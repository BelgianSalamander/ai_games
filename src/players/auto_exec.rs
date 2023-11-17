use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration, collections::{HashMap, HashSet}};

use async_std::{sync::Mutex, fs::File};
use deadpool::unmanaged::Pool;
use gamedef::{game_interface::GameInterface, parser::parse_game_interface};
use log::{debug, error, warn, info};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, sea_query::{Func, SimpleExpr}, QuerySelect, OrderedStatement, QueryTrait, ActiveValue, ActiveModelTrait, Value};

use crate::{
    games::Game, isolate::sandbox::IsolateSandbox, langs::{get_all_languages, language::Language}, util::{temp_file::{TempFile, random_file}, ActiveValueExtension, RUN_DIR}, entities::{agent, self},
};

use crate::entities::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(usize);

impl PlayerId {
    pub fn new(id: usize) -> Self {
        PlayerId(id)
    }

    pub fn get(&self) -> usize {
        let PlayerId(x) = self;
        *x
    }
}

pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub language: &'static str,

    pub rating: f32,

    pub average_score: f32,
    pub num_games: usize,

    pub removed: bool,
    pub error: Option<TempFile>,
    pub src: Option<TempFile>
}

pub struct GameRunner<T: Game + 'static> {
    pub sandboxes: Pool<IsolateSandbox>,
    db: DatabaseConnection,

    pub game: Arc<T>,
    pub itf: GameInterface,
    languages: Vec<Arc<dyn Language>>
}

impl<T: Game + 'static> GameRunner<T> {
    pub async fn new(game: T, name: &str, num_sandboxes: usize, db: DatabaseConnection) -> Self {
        let itf_path = format!("res/games/{}.game", name);
        let itf = std::fs::read_to_string(itf_path).unwrap();
        let itf = parse_game_interface(&itf, name.to_string()).unwrap();

        for lang in get_all_languages() {
            lang.prepare_files(&itf);
        }

        let pool = Pool::new(num_sandboxes);

        for i in 0..num_sandboxes {
            pool.add(IsolateSandbox::new(i as u32).await).await;
        }

        //Set all agents to not in_game
        Agent::update_many()
            .col_expr(agent::Column::InGame, SimpleExpr::Value(Value::Bool(Some(false))))
            .exec(&db)
            .await.unwrap();

        Self {
            sandboxes: pool,
            db,

            game: Arc::new(game),
            itf,
            languages: get_all_languages()
        }
    }

    pub async fn add_player(&self, name: String, language: String, directory: String, source_file: Option<String>, owner_id: Option<i32>, partial: bool) -> i32 {
        let agent = agent::ActiveModel {
            name: ActiveValue::Set(name),
            language: ActiveValue::Set(language),
            directory: ActiveValue::Set(directory),
            source_file: ActiveValue::Set(source_file),
            owner_id: ActiveValue::Set(owner_id),
            partial: ActiveValue::Set(partial),
            ..Default::default()
        };
        let res = Agent::insert(agent).exec(&self.db).await.unwrap();

        res.last_insert_id
    }

    pub fn get_language(&self, language: &str) -> Option<&Arc<dyn Language>> {
        self.languages.iter().find(|l| l.id() == language)
    }

    pub async fn run(&self) -> ! {
        loop {
            async_std::task::sleep(Duration::from_secs(1)).await;

            let players = Agent::find()
                .filter(agent::Column::InGame.eq(false))
                .filter(agent::Column::Removed.eq(false))
                .filter(agent::Column::Partial.eq(false))
                .order_by_asc(SimpleExpr::FunctionCall(Func::random()))
                .limit(self.game.num_players() as u64)
                .all(&self.db).await.unwrap();

            debug!("Found {} available players ({:?})", players.len(), players);

            if players.len() < self.game.num_players() {
                continue;
            }

            let mut sanboxes = vec![];

            for i in 0..self.game.num_players() {
                if let Ok(sandbox) = self.sandboxes.try_get() {
                    sanboxes.push(sandbox);
                } else {
                    break;
                }
            }

            if sanboxes.len() != players.len() {
                continue;
            }
 
            let players: Vec<_> = futures::future::join_all(players.into_iter().map(|p| {
                let mut active: entities::agent::ActiveModel = p.into();
                active.in_game = ActiveValue::Set(true);
                active.update(&self.db)
            })).await.into_iter().map(|x| x.unwrap()).collect();

            let mut agents = vec![];

            for (sandbox, player) in sanboxes.into_iter().zip(players.iter()) {
                let language = self.get_language(&player.language).unwrap();

                //TODO: Free sandbox as soon as it can be freed?
                let mut job = language.launch(&player.directory, sandbox.as_ref(), &self.itf);

                job.add_post_exit(move |_| {
                    drop(sandbox);
                });

                agents.push(job);
            }

            let game_copy = self.game.clone();
            let db_copy = self.db.clone();

            async_std::task::spawn(async move {
                info!("Starting a game!");
                let results = game_copy.run(&mut agents, None).await;

                let mut players: Vec<entities::agent::ActiveModel> = players.into_iter().map(|p| p.into()).collect();

                for i in 0..agents.len() {
                    if let Some(err) = agents[i].get_error() {
                        const MAX_READ: usize = 10 * 1024;

                        let stderr_contents = agents[i].read_stderr(Some(MAX_READ)).await;
                        let displayed_error = format!("Error: {}\nStderr:\n{}", err, stderr_contents);

                        //TODO: Don't use a temp file cause it shoudl persist (this goes for a lot of other things as well). Current workaround is to freeze
                        let stderr_store = random_file(RUN_DIR, ".error");
                        async_std::fs::write(stderr_store.clone(), displayed_error.clone()).await.unwrap();

                        players[i].error_file = ActiveValue::Set(Some(stderr_store.clone()));
                        players[i].removed = ActiveValue::Set(true);

                        warn!("Player {} removed.\n{}", players[i].name.get().unwrap(), displayed_error);
                    }
                }

                Self::update_ratings(&mut players, results).await;

                for mut player in players {
                    player.in_game = ActiveValue::Set(false);
                    player.update(&db_copy).await.unwrap();
                }
            });
        }
    }

    async fn update_ratings(players: &mut Vec<entities::agent::ActiveModel>, results: Vec<f32>) {
        for i in 0..players.len() {
            let player = &mut players[i];

            player.total_score = ActiveValue::Set(player.total_score.get().unwrap() + results[i] as f64);
            player.num_games = ActiveValue::Set(player.num_games.get().unwrap() + 1);
        }

        let n = players.len();
        let mut merged = players.into_iter().zip(results).collect::<Vec<_>>();

        const K: f32 = 20.0;
        const D: f32 = 400.0;
        const B: f32 = 10.0;

        merged.sort_by_key(|(_, score)| *score as i32);

        let curr_ratings = merged.iter().map(|(player, _)| *player.rating.get().unwrap() as f32).collect::<Vec<_>>();

        let num_pairings = n * (n - 1) / 2;

        let expected_scores = (0..n).map(|i| {
            let mut sum = 0.0;

            for j in 0..n {
                if i == j {
                    continue;
                }

                let diff = (curr_ratings[j] - curr_ratings[i]) / D;
                let expected = 1.0 / (1.0 + B.powf(diff));
                sum += expected;
            }
            
            sum / (num_pairings as f32)
        }).collect::<Vec<_>>();

        let mut actual_scores = (0..n).map(|i| i as f32 / (num_pairings as f32)).collect::<Vec<_>>();
    
        let mut last_idx = 0;

        for i in 0..n {
            if i == n - 1 || (merged[i].1 - merged[i+1].1).abs() > 1e-5 {
                let mut sum = 0.0;

                for j in last_idx..=i {
                    sum += actual_scores[j];
                }

                let avg = sum / ((i - last_idx + 1) as f32);

                for j in last_idx..=i {
                    actual_scores[j] = avg;
                }

                last_idx = i + 1;
            }
        }

        let new_scores = (0..n).map(|i| {
            let curr = curr_ratings[i];
            let expected = expected_scores[i];
            let actual = actual_scores[i];

            let diff = actual - expected;
            let new = curr + K * diff;

            new
        }).collect::<Vec<_>>();

        for i in 0..n {
            merged[i].0.rating = ActiveValue::Set(new_scores[i] as _);
        }
    }

    pub async fn get_score(&self, id: i32) -> Option<i32> {
        Agent::find_by_id(id).one(&self.db).await.unwrap().map(|x| x.rating as i32)
    }
}
