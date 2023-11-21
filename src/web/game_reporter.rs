use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc, time::Duration, ops::Index};

use async_std::{sync::Mutex, net::{TcpListener, TcpStream}};
use async_trait::async_trait;
use async_tungstenite::{accept_async, WebSocketStream, tungstenite::Message};
use futures::{StreamExt, SinkExt, FutureExt};
use log::{error, info};
use serde_json::{Value, json};

use crate::{
    games::Game,
    players::{
        self,
        auto_exec::GameRunner,
        reporting::{EndCallback, StartCallback, UpdateCallback},
    },
};

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
    ws: WebSocketStream<TcpStream>,
    game_request: Option<GameConnectRequest>,
    curr_game: Option<usize>,

    error: bool
}

impl Spectator {
    async fn send_packet(&mut self, kind: &str, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let packet = json!({
            "kind": kind,
            "data": data
        });

        let s = packet.to_string();

        self.ws.send(Message::Text(s)).await?;

        Ok(())
    }

    pub async fn connect_to_game(&mut self, game: &GameRecord)  -> Result<(), Box<dyn std::error::Error>> {
        let data = json!({
            "kind": game.kind,
            "players": game.players,
            "history": game.history
        });

        self.send_packet("connect", &data).await
    }

    pub async fn end_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_packet("end", &Value::Null).await
    }

    pub async fn update_game(&mut self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        self.send_packet("update", data).await
    }
}

struct SharedInner {
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

    async fn handle_ws(&mut self, ws: WebSocketStream<TcpStream>) {
        self.spectators.push(Arc::new(Mutex::new(Spectator {
            ws,
            game_request: None,
            curr_game: None,
            error: false
        })));
    }

    async fn check_ws(&mut self) {
        'outer: for spectator in &self.spectators {
            let mut lock = spectator.lock().await;
            if lock.error {
                continue;
            }
            
            match lock.ws.next().now_or_never() {
                None | Some(None) => {},
                Some(Some(Err(e))) => {
                    error!("WS Error {:?}", e);
                    lock.error = true;
                    continue 'outer;
                },
                Some(Some(Ok(Message::Text(s)))) => {
                    if let Ok(req) = serde_json::from_str::<GameConnectRequest>(&s) {
                        info!("Received game connect request {:?}", req);
                        lock.game_request = Some(req);

                        let mut remove_from = None;

                        for (id, game) in &mut self.games {
                            if Some(*id) != lock.curr_game && req.matches(game) {
                                if let Err(e) = lock.connect_to_game(game).await {
                                    error!("WS Error {:?}", e);
                                    lock.error = true;
                                    continue 'outer;
                                }

                                remove_from =  lock.curr_game.replace(*id);

                                lock.game_request = None;

                                game.spectators.push(spectator.clone());

                                break;
                            }
                        }

                        if let Some(other_game) = remove_from {
                            if let Some(record) = self.games.get_mut(&other_game) {
                                record.spectators.retain(move |x| !Arc::ptr_eq(x, spectator));
                            }
                        }
                    } else {
                        error!("Invalid game connect request {:?}", s);
                    }
                },
                x => {
                    error!("Unexpected message from ws {:?}", x);
                    lock.error = true;
                }
            }
        }

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
            if let Some(req) = lock.game_request {
                if req.matches(&self.games.get(&id).unwrap()) {
                    if let Err(e) = lock.connect_to_game(self.games.get(&id).unwrap()).await {
                        error!("WS Error {:?}", e);
                        lock.error = true;
                    }

                    if let Some(game) = lock.curr_game.replace(id) {
                        if let Some(record) = self.games.get_mut(&game) {
                            record.spectators.retain(move |x| !Arc::ptr_eq(x, spectator));
                        }
                    }

                    lock.game_request = None;

                    self.games.get_mut(&id).unwrap().spectators.push(spectator.clone());
                }
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

                if let Err(e) = lock.end_game().await {
                    error!("WS Error: {:?}", e);
                    lock.error = true;
                }

                lock.curr_game = None;
            }
        }

        self.games.remove(&id);
    }

    async fn update_game(&mut self, id: usize, data: &Value) {
        if let Some(record) = self.games.get_mut(&id) {
            record.history.push(data.to_string());

            for spectator in &record.spectators {
                let mut lock = spectator.lock().await;

                if let Err(e) = lock.update_game(data).await {
                    error!("WS Error: {:?}", e);
                    lock.error = true;
                }
            }
        }
    }
}

pub struct GameReporter {
    inner: Arc<Mutex<SharedInner>>,
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
        let addr = SocketAddr::from(([0, 0, 0, 0], 42070));

        info!("Binding websocket server to {:?}", addr);
        let tcp_listener = TcpListener::bind(addr).await.unwrap();

        let shared_inner = self.inner.clone();
        async_std::task::spawn(async move {
            loop {
                {shared_inner.lock().await.check_ws().await;}
                async_std::task::sleep(Duration::from_millis(25)).await;
            }
        });

        loop {
            let (stream, addr) = tcp_listener.accept().await?;

            info!("New websocket connection from {:?}", addr);
            let ws = match accept_async(stream).await {
                Ok(x) => x,
                Err(e) => {
                    error!("Websocket handshake failed! {:?}", e);
                    continue;
                }
            };

            self.inner.lock().await.handle_ws(ws).await;
        }
    }
}
