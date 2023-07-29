use std::{future::Future, io::Error};

use async_trait::async_trait;

use crate::isolate::sandbox::RunningJob;

pub mod oxo;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerResult {
    pub score: f32,
    pub error: Option<String>,
}

pub async fn await_seconds<Fut, T>(fut: Fut, seconds: f32) -> Result<T, String>
where
    Fut: Future<Output = Result<T, Error>>,
{
    let timeout = async_std::future::timeout(std::time::Duration::from_secs_f32(seconds), fut);

    match timeout.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("Timeout".to_string()),
    }
}

#[async_trait]
pub trait Game {
    fn num_players() -> usize;
    fn name() -> &'static str;

    async fn run(players: Vec<RunningJob>) -> Vec<PlayerResult>;
}
