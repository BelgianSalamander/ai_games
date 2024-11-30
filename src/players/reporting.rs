use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

use async_std::sync::Mutex;
use async_trait::async_trait;
use log::error;
use serde_json::{Value, json};

use crate::games::Game;

#[async_trait]
pub trait StartCallback: Sync + Send {
    async fn call(&mut self, id: usize, name: &str, players: &[i32]);
}

#[async_trait]
pub trait EndCallback: Sync + Send  {
    async fn call(&mut self, id: usize);
}

#[async_trait]
pub trait UpdateCallback: Sync + Send  {
    async fn call(&mut self, id: usize, data: &Value);
}

struct CallbackHandler {
    on_start_game: Vec<Box<dyn StartCallback>>,
    on_end_game: Vec<Box<dyn EndCallback>>,
    on_update: Vec<Box<dyn UpdateCallback>>
}

impl CallbackHandler {
    pub fn new() -> Self {
        Self {
            on_start_game: vec![],
            on_end_game: vec![],
            on_update: vec![]
        }
    }

    pub fn add_start_game_callback(&mut self, callback: Box<dyn StartCallback>) {
        self.on_start_game.push(callback);
    }

    pub fn add_end_game_callback(&mut self, callback: Box<dyn EndCallback>) {
        self.on_end_game.push(callback);
    }

    pub fn add_update_callback(&mut self, callback: Box<dyn UpdateCallback>) {
        self.on_update.push(callback);
    }

    pub async fn start_game(&mut self, id: usize, name: &str, players: &[i32]) {
        for callback in &mut self.on_start_game {
            callback.call(id, name, players).await;
        }
    }

    pub async fn end_game(&mut self, id: usize) {
        for callback in &mut self.on_end_game {
            callback.call(id).await;
        }
    }

    pub async fn update_game(&mut self, id: usize, data: &Value) {
        for callback in &mut self.on_update {
            callback.call(id, data).await;
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

    pub async fn update<T: serde::Serialize>(&mut self, data: &T, kind: &str) {
        let val = match serde_json::to_value(data) {
            Ok(x) => x,
            Err(e) => {
                error!("Failed to turn data to json to update viewers! {:?}", e);
                return;
            }
        };

        let val = json!([kind, val]);

        self.callbacks.lock().await.update_game(self.id, &val).await;
    }
}

impl Drop for GameReporter {
    fn drop(&mut self) {
        let callbacks_clone = self.callbacks.clone();
        let id = self.id;
        async_std::task::spawn(async move {
            callbacks_clone.lock().await.end_game(id).await;
        });
    }
}

pub struct Reporter {
    id_counter: AtomicUsize,
    callbacks: Arc<Mutex<CallbackHandler>>
}

impl Reporter {
    pub fn new() -> Self {
        Self {
            id_counter: AtomicUsize::new(0),
            callbacks: Arc::new(Mutex::new(CallbackHandler::new()))
        }
    }

    pub async fn start_game<GameType: Game>(&self, game: &GameType, players: &[i32]) -> GameReporter {
        let id = self.id_counter.fetch_add(1, Ordering::AcqRel);

        self.callbacks.lock().await.start_game(id, game.name(), players).await;

        GameReporter::new(self.callbacks.clone(), id)
    }

    pub async fn add_start_game_callback(&self, callback: Box<dyn StartCallback>) {
        self.callbacks.lock().await.add_start_game_callback(callback);
    }

    pub async fn add_end_game_callback(&self, callback: Box<dyn EndCallback>) {
        self.callbacks.lock().await.add_end_game_callback(callback);
    }

    pub async fn add_update_callback(&self, callback: Box<dyn UpdateCallback>) {
        self.callbacks.lock().await.add_update_callback(callback);
    }
}