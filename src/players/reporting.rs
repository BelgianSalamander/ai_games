use std::sync::Arc;

use async_std::sync::Mutex;
use log::error;
use serde_json::Value;

use crate::games::Game;

pub type StartCallback = Box<dyn FnMut(usize, &str, &[i32]) + Sync + Send>;
pub type EndCallback = Box<dyn FnMut(usize) + Sync + Send>;
pub type UpdateCallback = Box<dyn FnMut(usize, &Value) + Sync + Send>;

struct CallbackHandler {
    on_start_game: Vec<StartCallback>,
    on_end_game: Vec<EndCallback>,
    on_update: Vec<UpdateCallback>
}

impl CallbackHandler {
    pub fn new() -> Self {
        Self {
            on_start_game: vec![],
            on_end_game: vec![],
            on_update: vec![]
        }
    }

    pub fn add_start_game_callback(&mut self, callback: StartCallback) {
        self.on_start_game.push(callback);
    }

    pub fn add_end_game_callback(&mut self, callback: EndCallback) {
        self.on_end_game.push(callback);
    }

    pub fn add_update_callback(&mut self, callback: UpdateCallback) {
        self.on_update.push(callback);
    }

    pub fn start_game(&mut self, id: usize, name: &str, players: &[i32]) {
        for callback in &mut self.on_start_game {
            callback(id, name, players);
        }
    }

    pub fn end_game(&mut self, id: usize) {
        for callback in &mut self.on_end_game {
            callback(id);
        }
    }

    pub fn update_game(&mut self, id: usize, data: &Value) {
        for callback in &mut self.on_update {
            callback(id, data);
        }
    }
}

pub struct GameReporter {
    callbacks: Arc<Mutex<CallbackHandler>>,
    id: usize
}

impl GameReporter {
    fn new(callbacks: Arc<Mutex<CallbackHandler>>, id: usize) -> Self {
        Self {
            callbacks,
            id
        }
    }

    pub async fn update<T: serde::Serialize>(&mut self, data: &T) {
        let val = match serde_json::to_value(data) {
            Ok(x) => x,
            Err(e) => {
                error!("Failed to turn data to json to update viewers! {:?}", e);
                return;
            }
        };

        self.callbacks.lock().await.update_game(self.id, &val);
    }
}

impl Drop for GameReporter {
    fn drop(&mut self) {
        let callbacks_clone = self.callbacks.clone();
        let id = self.id;
        async_std::task::spawn(async move {
            callbacks_clone.lock().await.end_game(id);
        });
    }
}

pub struct Reporter {
    id_counter: usize,
    callbacks: Arc<Mutex<CallbackHandler>>
}

impl Reporter {
    pub fn new() -> Self {
        Self {
            id_counter: 0,
            callbacks: Arc::new(Mutex::new(CallbackHandler::new()))
        }
    }

    pub async fn start_game<GameType: Game>(&mut self, game: GameType, players: &[i32]) -> GameReporter {
        let id = self.id_counter;
        self.id_counter += 1;

        self.callbacks.lock().await.start_game(id, game.name(), players);

        GameReporter::new(self.callbacks.clone(), id)
    }

    pub async fn add_start_game_callback(&mut self, callback: StartCallback) {
        self.callbacks.lock().await.add_start_game_callback(callback);
    }

    pub async fn add_end_game_callback(&mut self, callback: EndCallback) {
        self.callbacks.lock().await.add_end_game_callback(callback);
    }

    pub async fn add_update_callback(&mut self, callback: UpdateCallback) {
        self.callbacks.lock().await.add_update_callback(callback);
    }
}