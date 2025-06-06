use std::{sync::Arc, time::Duration};

use colors_transform::{Hsl, Color};
use deadpool::unmanaged::Pool;
use gamedef::{game_interface::GameInterface, parser::parse_game_interface};
use log::{debug, warn, info, error};
use rand::Rng;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, sea_query::{Func, SimpleExpr}, QuerySelect, ActiveValue, ActiveModelTrait, Value, DbErr};

use crate::{
    games::Game, isolate::sandbox::IsolateSandbox, langs::{get_all_languages, language::Language, files::ClientFiles}, util::{temp_file::{TempFile, random_file}, ActiveValueExtension, RUN_DIR}, entities::{agent, self}
};

use crate::entities::prelude::*;

use super::reporting::Reporter;

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
    pub languages: Vec<(Arc<dyn Language>, ClientFiles)>,

    pub reporting: Reporter
}

impl<T: Game + 'static> GameRunner<T> {
    pub async fn new(game: T, name: &str, num_sandboxes: usize, db: DatabaseConnection) -> Self {
        let itf_path = format!("res/games/{}.game", name);
        println!("Loading interface at {}", itf_path);
        let itf = std::fs::read_to_string(itf_path).unwrap();
        let itf = parse_game_interface(&itf, name.to_string()).unwrap();

        let languages = get_all_languages().into_iter().map(|lang| {
            let files = lang.prepare_files(&itf);
            (lang, files)
        }).collect();

        let pool = Pool::new(num_sandboxes);

        for i in 0..num_sandboxes {
            pool.add(IsolateSandbox::new(i as u32).await).await.unwrap();
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
            languages,

            reporting: Reporter::new()
        }
    }

    pub async fn add_player(&self, name: String, language: String, directory: String, source_file: Option<String>, owner_id: Option<i32>, partial: bool) -> Result<i32, DbErr> {
        let rgb = {
            let mut rand = rand::thread_rng();
            let hsl = Hsl::from(
                rand.gen_range(0.0..360.0),
                rand.gen_range(20.0..100.0),
                rand.gen_range(30.0..90.0)
            );
            hsl.to_rgb()
        };
        
        let agent = agent::ActiveModel {
            name: ActiveValue::Set(name),
            language: ActiveValue::Set(language),
            directory: ActiveValue::Set(directory),
            source_file: ActiveValue::Set(source_file),
            owner_id: ActiveValue::Set(owner_id),
            partial: ActiveValue::Set(partial),
            colour: ActiveValue::Set(rgb.to_css_hex_string().to_ascii_uppercase()),
            ..Default::default()
        };
        let res = Agent::insert(agent).exec(&self.db).await?;

        Ok(res.last_insert_id)
    }

    pub fn get_language(&self, language: &str) -> Option<&Arc<dyn Language>> {
        self.languages.iter().find(|(l, _)| l.id() == language).map(|x| &x.0)
    }

    pub async fn run(&self) -> ! {
        loop {
            async_std::task::sleep(Duration::from_secs(1)).await;

            let players = match Agent::find()
                .filter(agent::Column::InGame.eq(false))
                .filter(agent::Column::Removed.eq(false))
                .filter(agent::Column::Partial.eq(false))
                .order_by_asc(SimpleExpr::FunctionCall(Func::random()))
                .limit(self.game.num_players() as u64)
                .all(&self.db).await {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Encountered error while selecting players! {}", e);
                        continue;
                    }
                };

            debug!("Found {} available players ({:?})", players.len(), players);

            if players.len() < self.game.num_players() {
                continue;
            }

            let mut sanboxes = vec![];

            for _ in 0..self.game.num_players() {
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
            })).await.into_iter().filter_map(|x| match x {
                Ok(x) => Some(x),
                Err(e) => {
                    panic!("Encountered error while setting player active! Cannot recover! {}", e);
                }
            }).collect();


            let mut agents = vec![];
            let mut ids = vec![];

            for (mut sandbox, player) in sanboxes.into_iter().zip(players.iter()) {
                sandbox.initialize().await;
                let language = self.get_language(&player.language).unwrap();

                //TODO: Free sandbox as soon as it can be freed?
                let mut job = language.launch(&player.directory, sandbox.as_ref(), &self.itf);

                job.add_post_exit(move |_| {
                    async_std::task::block_on(sandbox.cleanup());
                    drop(sandbox);
                });

                agents.push(job);
                ids.push(player.id);
            }

            let game_copy = self.game.clone();
            let db_copy = self.db.clone();

            let reporter = self.reporting.start_game(game_copy.as_ref(), &ids).await;

            async_std::task::spawn(async move {
                info!("Starting a game!");
                let results = game_copy.run(&mut agents, Some(Duration::from_millis(40)), reporter).await;

                let mut players: Vec<entities::agent::ActiveModel> = players.into_iter().map(|p| p.into()).collect();

                for i in 0..agents.len() {
                    const MAX_READ: usize = 10 * 1024;
                    let stderr_contents = agents[i].read_stderr(Some(MAX_READ)).await;

                    let stderr_store = match &players[i].error_file {
                        ActiveValue::NotSet | ActiveValue::Unchanged(None) | ActiveValue::Set(None) => {
                            let res = random_file(RUN_DIR, ".error");
                            players[i].error_file = ActiveValue::Set(Some(res.clone()));
                            res
                        },
                        ActiveValue::Unchanged(Some(x)) | ActiveValue::Set(Some(x)) => x.to_string()
                    };

                    if let Some(err) = agents[i].get_error() {
                        let displayed_error = format!("Error: {}\nStderr:\n{}", err, stderr_contents);

                        if let Err(e) = async_std::fs::write(stderr_store.clone(), displayed_error.clone()).await {
                            error!("Encountered error while saving error! {}", e);
                        }

                        players[i].removed = ActiveValue::Set(true);

                        warn!("Player {} removed.\n{}", players[i].name.get().unwrap(), displayed_error);
                    } else {
                        if let Err(e) = async_std::fs::write(stderr_store, stderr_contents).await {
                            error!("Encountered error while saving stderr! {}", e);
                        }
                    }
                }

                Self::update_ratings(&mut players, results).await;

                for mut player in players {
                    player.in_game = ActiveValue::Set(false);
                    let _ = player.update(&db_copy).await;
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

    pub async fn get_score(&self, id: i32) -> Result<Option<i32>, DbErr> {
        Ok(Agent::find_by_id(id).one(&self.db).await?.map(|x| x.rating as i32))
    }
}
