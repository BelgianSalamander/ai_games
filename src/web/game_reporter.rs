use std::{collections::HashMap, sync::Arc, time::Duration};

use async_std::{sync::Mutex, net::TcpStream, io::WriteExt};
use async_trait::async_trait;
use log::{error, info};
use serde_json::{Value, json};

use crate::{
    games::Game,
    players::{
        auto_exec::GameRunner,
        reporting::{EndCallback, StartCallback, UpdateCallback},
    },
};

use super::{http::{Request, Response, Status}, web_errors::WebError};

#[derive(Debug)]
struct GameRecord {
    kind: String,
    players: Vec<i32>,
    history: Vec<String>,

    spectators: Vec<Arc<Mutex<Spectator>>>
}

unsafe impl Send for GameRecord {}
unsafe impl Sync for GameRecord {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub enum GameConnectRequest {
    Any,
    WithPlayer(i32)
}

impl GameConnectRequest {
    fn matches(&self, game: &GameRecord) -> bool {
        match self {
            Self::Any => true,
            Self::WithPlayer(x) => game.players.contains(&x)
        }
    }
}

#[derive(Debug)]
struct Spectator {
    stream: TcpStream,
    game_request: GameConnectRequest,
    curr_game: Option<usize>,

    error: bool
}

impl Spectator {
    async fn send_packet(&mut self, kind: &str, data: &Value) -> Result<(), std::io::Error> {
        let packet = json!({
            "kind": kind,
            "data": data
        });

        let s = packet.to_string();

        let mut bytes = "data: ".as_bytes().to_vec();
        bytes.extend(s.into_bytes());
        bytes.extend("\n\n".as_bytes());

        self.stream.write_all(&bytes).await?;

        Ok(())
    }

    pub async fn connect_to_game(&mut self, game: &GameRecord)  -> Result<(), std::io::Error> {
        let data = json!({
            "kind": game.kind,
            "players": game.players,
            "history": game.history
        });

        self.send_packet("connect", &data).await
    }

    pub async fn end_game(&mut self) -> Result<(), std::io::Error> {
        self.send_packet("end", &Value::Null).await
    }

    pub async fn update_game(&mut self, data: &Value) -> Result<(), std::io::Error> {
        self.send_packet("update", data).await
    }
}

pub struct SharedInner {
    games: HashMap<usize, GameRecord>,
    by_player: HashMap<i32, usize>,
    spectators: Vec<Arc<Mutex<Spectator>>>
}

unsafe impl Send for SharedInner {}
unsafe impl Sync for SharedInner {}

impl SharedInner {
    fn new() -> Self {
        info!("Examples: {:?}\n\n{:?}", serde_json::to_value(GameConnectRequest::Any), serde_json::to_value(GameConnectRequest::WithPlayer(5)));

        Self {
            games: HashMap::new(),
            by_player: HashMap::new(),
            spectators: Vec::new()
        }
    }

    pub async fn handle_stream(&mut self, mut stream: TcpStream, request: &Request) {
        let data = match request.path.get("req").map(|x| urlencoding::decode(&x)) {
            Ok(Ok(x)) => x,
            Ok(Err(e)) => {
                let response = WebError::InvalidData(format!("Couldn't decode req {:?}", e)).into_response();
                if let Err(e2) = response.write_async(&mut stream).await {
                    error!("{:?} occured while handling {:?}", e2, e)
                }
                return;
            }
            Err(e) => {
                let response = e.into_response();
                if let Err(e2) = response.write_async(&mut stream).await {
                    error!("{:?} occured while handling web error", e2)
                }
                return;
            }
        }.to_string();
        
        let req = match serde_json::from_str::<GameConnectRequest>(&data) {
            Ok(x) => x,
            Err(e) => {
                let response = WebError::InvalidData(format!("Invalid connction request! {:?}", e)).into_response();
                if let Err(e2) = response.write_async(&mut stream).await {
                    error!("{:?} occured while handling {:?}", e2, e)
                }
                return;
            }
        };

        let spectator = Arc::new(Mutex::new(Spectator {
            stream,
            game_request: req,
            curr_game: None,
            error: false
        }));

        let mut lock = spectator.lock().await;
        self.spectators.push(spectator.clone());

        let mut response = Response::new();
        response.set_status(Status::Ok);

        response.set_header("Content-Type", "text/event-stream");
        response.set_header("Cache-Control", "no-cache");
        response.set_header("Connection", "keep-alive");

        let _ = response.write_async(&mut lock.stream).await;

        for (id, game) in &mut self.games {
            if lock.game_request.matches(game) {
                if let Err(e) = lock.connect_to_game(game).await {
                    let response = WebError::InternalServerError(format!("Error while conncting to game {:?}", e)).into_response();
                    let _ = response.write_async(&mut lock.stream).await;
                    return;
                }

                lock.curr_game = Some(*id);

                game.spectators.push(spectator.clone());

                break;
            }
        }
    }

    async fn check_streams(&mut self) {
        for i in (0..self.spectators.len()).rev() {
            if self.spectators[i].lock().await.error {
                self.spectators.swap_remove(i);
            }
        }
    }

    async fn start_game(&mut self, id: usize, name: &str, players: &[i32]) {
        self.games.insert(
            id,
            GameRecord {
                kind: name.to_string(),
                players: players.to_vec(),
                history: vec![],
                spectators: vec![]
            },
        );

        for player in players {
            self.by_player.insert(*player, id);
        }


        for spectator in &self.spectators {
            let mut lock = spectator.lock().await;

            if lock.error {
                continue;
            }

            if lock.game_request.matches(&self.games.get(&id).unwrap()) && lock.curr_game.is_none() {
                println!("Connecting to game!");
                if let Err(e) = lock.connect_to_game(self.games.get(&id).unwrap()).await {
                    error!("WS Error {:?}", e);
                    lock.error = true;
                }

                lock.curr_game = Some(id);

                self.games.get_mut(&id).unwrap().spectators.push(spectator.clone());
            }
        }
    }

    async fn end_game(&mut self, id: usize) {
        if let Some(record) = self.games.get(&id) {
            for player in &record.players {
                self.by_player.remove(player);
            }

            for spectator in &record.spectators {
                let mut lock = spectator.lock().await;

                if lock.error {
                    continue;
                }

                if let Err(e) = lock.end_game().await {
                    error!("WS Error: {:?}", e);
                    lock.error = true;
                }

                lock.curr_game = None;
                let _ = lock.stream.shutdown(std::net::Shutdown::Both);
                lock.error = true;
            }
        }

        self.games.remove(&id);
    }

    async fn update_game(&mut self, id: usize, data: &Value) {
        if let Some(record) = self.games.get_mut(&id) {
            record.history.push(data.to_string());

            for spectator in &record.spectators {
                let mut lock = spectator.lock().await;

                if lock.error {
                    continue;
                }

                if let Err(e) = lock.update_game(data).await {
                    error!("WS Error: {:?}", e);
                    lock.error = true;
                }
            }
        }
    }
}

pub struct GameReporter {
    pub inner: Arc<Mutex<SharedInner>>,
}

unsafe impl Send for GameReporter {}
unsafe impl Sync for GameReporter {}

struct ReporterStartCallback(Arc<Mutex<SharedInner>>);
#[async_trait]
impl StartCallback for ReporterStartCallback {
    async fn call(&mut self, id: usize, name: &str, players: &[i32]) {
        self.0.lock().await.start_game(id, name, players).await;
    }
}

struct ReporterEndCallback(Arc<Mutex<SharedInner>>);
#[async_trait]
impl EndCallback for ReporterEndCallback {
    async fn call(&mut self, id: usize) {
        self.0.lock().await.end_game(id).await;
    }
}

struct ReporterUpdateCallback(Arc<Mutex<SharedInner>>);
#[async_trait]
impl UpdateCallback for ReporterUpdateCallback {
    async fn call(&mut self, id: usize, value: &Value) {
        self.0.lock().await.update_game(id, value).await;
    }
}

impl GameReporter {
    pub async fn new<T: Game>(executor: &GameRunner<T>) -> Self {
        let res = Self {
            inner: Arc::new(Mutex::new(SharedInner::new())),
        };

        let copy = res.inner.clone();
        executor
            .reporting
            .add_start_game_callback(Box::new(ReporterStartCallback(copy)))
            .await;

        let copy = res.inner.clone();
        executor
            .reporting
            .add_end_game_callback(Box::new(ReporterEndCallback(copy)))
            .await;

        let copy = res.inner.clone();
        executor
            .reporting
            .add_update_callback(Box::new(ReporterUpdateCallback(copy)))
            .await;

        res
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let shared_inner = self.inner.clone();
        loop {
            {shared_inner.lock().await.check_streams().await;}
            async_std::task::sleep(Duration::from_millis(25)).await;
        }
    }
}
