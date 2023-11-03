use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration, collections::{HashMap, HashSet}};

use async_std::{sync::Mutex, fs::File};
use gamedef::{game_interface::GameInterface, parser::parse_game_interface};
use log::{debug, error};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, sea_query::{Func, SimpleExpr}, QuerySelect, OrderedStatement, QueryTrait, ActiveValue, ActiveModelTrait};

use crate::{
    games::Game, isolate::sandbox::IsolateSandbox, langs::{get_all_languages, language::Language}, util::{pool::Pool, temp_file::TempFile}, entities::{agent, self},
};

use super::{player::Player, player_list::PlayerList};
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
    sandboxes: Arc<Mutex<Pool<IsolateSandbox>>>,
    db: DatabaseConnection,

    pub game: Arc<T>,
    pub itf: GameInterface,
    languages: Vec<Arc<dyn Language>>
}

impl<T: Game + 'static> GameRunner<T> {
    pub async fn new(game: T, name: &str, num_sandboxes: usize, db: DatabaseConnection, languages: Vec<Arc<dyn Language>>) -> Self {
        let itf_path = format!("res/games/{}.game", name);
        let itf = std::fs::read_to_string(itf_path).unwrap();
        let itf = parse_game_interface(&itf, name.to_string()).unwrap();

        for lang in get_all_languages() {
            lang.prepare_files(&itf);
        }

        let pool = Pool::new_async(num_sandboxes, |i| IsolateSandbox::new(i as _)).await;
        let pool = Arc::new(Mutex::new(pool));

        let players = Arc::new(Mutex::new(PlayerList::new()));

        Self {
            sandboxes: pool,
            db,

            game: Arc::new(game),
            itf,
            languages
        }
    }

    pub async fn add_player(&self, player: Player) {
        let src = match &player.program.src {
            Some(path) => {
                let res = TempFile::with_extra(".program_src");

                match std::fs::copy(path, &res.path) {
                    Ok(_) => Some(res),
                    Err(e) => {
                        error!("Error while copying file! {:?}", e);
                        None
                    }
                }
            },
            None => None
        };

        self.scores.lock().await.insert(player.id, PlayerInfo {
            id: player.id,
            name: player.name.clone(),
            language: player.language.name(),

            rating: 1000.0,

            average_score: 0.0,
            num_games: 0,

            removed: false,
            error: None,
            src
        });

        self.players.lock().await.add_player(player);
    }

    pub fn get_language(&self, language: &str) -> Option<&Arc<dyn Language>> {
        self.languages.iter().find(|l| l.id() == language)
    }

    pub async fn run(&self) {
        loop {
            async_std::task::sleep(Duration::from_secs(1)).await;

            let mut players = Agent::find()
                .filter(agent::Column::InGame.eq(false))
                .filter(agent::Column::Removed.eq(false))
                .order_by_asc(SimpleExpr::FunctionCall(Func::random()))
                .limit(self.game.num_players() as u64)
                .all(&self.db).await.unwrap();

            if players.len() < self.game.num_players() {
                continue;
            }

            let mut pool = self.sandboxes.lock().await;
            if pool.num_available() < self.game.num_players() {
                continue;
            }

            let mut players: Vec<_> = futures::future::join_all(players.into_iter().map(|p| {
                let active: entities::agent::ActiveModel = p.into();
                active.in_game = ActiveValue::Set(true);
                active.update(&self.db)
            })).await.into_iter().map(|x| x.unwrap()).collect();

            let mut agents = vec![];
            let mut sandboxes_to_free = vec![];

            for player in players {
                let language = self.get_language(&player.language).unwrap();

                let (sandbox_id, sandbox) = pool.get().unwrap();

                agents.push(language.launch(&player.directory, sandbox, &self.itf));
                sandboxes_to_free.push(sandbox_id);
            }

            let game_copy = self.game.clone();
            let dp_copy = self.db.clone();

            async_std::task::spawn(async move {
                let results = game_copy.run(agents, None).await;

                Self::update_ratings(players, results, db_copy);
            });

            while players.num_available() >= self.game.num_players()
                && pool.num_available() >= self.game.num_players()
            {
                let mut ids = vec![];
                let mut agents = vec![];

                for _ in 0..self.game.num_players() {
                    let player = players.get_random_player().unwrap();
                    ids.push(player.id);
                    let (sandbox_idx, sandbox) = pool.get().unwrap();

                    //debug!("Launching player {} in sandbox {}", player.id, sandbox_idx);
                    let mut job = player.launch(sandbox, &self.itf);

                    let players_copy = self.players.clone();
                    let sandboxes_copy = self.sandboxes.clone();

                    job.add_post_exit(move |job: &mut crate::isolate::sandbox::RunningJob| {
                        async_std::task::block_on(async {
                            if let Some(err) = job.get_error() {
                                const MAX_READ: usize = 10 * 1024;

                                let stderr_contents = job.read_stderr(Some(MAX_READ)).await;
                                let displayed_error = format!("Error: {}\nStderr:\n{}", err, stderr_contents);

                                let stderr_store = TempFile::new();
                                stderr_store.write_string_async(&displayed_error).await;

                                player.on_removal(&displayed_error);
                            } else {
                                players_copy.lock().await.add_player(player);
                            }

                            sandboxes_copy.lock().await.release(sandbox_idx);
                        });
                    });

                    agents.push(job);
                }

                let game_copy = self.game.clone();

                let scores_copy = self.scores.clone();
                async_std::task::spawn(async move {
                    let results = game_copy.run(agents, None).await;
                    debug!("Game results: {:?}", results);

                    Self::update_ratings(ids, results, scores_copy).await;
                });
            }
        }
    }

    async fn update_ratings(players: Vec<PlayerId>, results: Vec<f32>, scores: Arc<Mutex<HashMap<PlayerId, PlayerInfo>>>) {
        let mut lock = scores.lock().await;

        for i in 0..players.len() {
            let id = players[i];
            let result = results[i];

            let score = lock.get_mut(&id).unwrap();

            score.average_score = (score.average_score * score.num_games as f32 + result) / (score.num_games + 1) as f32;
            score.num_games += 1;
        }

        let n = players.len();
        let mut merged = players.into_iter().zip(results).collect::<Vec<_>>();

        const K: f32 = 20.0;
        const D: f32 = 400.0;
        const B: f32 = 10.0;

        merged.sort_by_key(|(_, score)| *score as i32);

        let curr_ratings = merged.iter().map(|(id, _)| lock[id].rating).collect::<Vec<_>>();

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
            lock.get_mut(&merged[i].0).unwrap().rating = new_scores[i];
        }
    }

    pub async fn get_score(&self, id: PlayerId) -> Option<i32> {
        self.scores.lock().await.get(&id).map(|x| x.rating as i32)
    }

    pub fn make_id(&self) -> PlayerId {
        let res = self.id_counter.fetch_add(1, Ordering::SeqCst);
        PlayerId::new(res)
    }
}
