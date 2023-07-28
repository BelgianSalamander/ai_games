use async_trait::async_trait;

use crate::isolate::sandbox::RunningJob;

pub mod oxo;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerResult {
    pub score: f32,
    pub error: Option<String>,
}

#[async_trait]
pub trait Game {
    fn num_players() -> usize;
    fn name() -> &'static str;

    async fn run(players: Vec<RunningJob>) -> Vec<PlayerResult>;
}