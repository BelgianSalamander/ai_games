use std::{future::Future, io::Error, time::{Duration, Instant}};

use async_trait::async_trait;

use crate::isolate::sandbox::RunningJob;

pub mod oxo;

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

pub struct Waiter {
    pub min_delay: Option<Duration>,
    pub last_tick: Instant
}

impl Waiter {
    pub fn new(min_delay: Option<Duration>) -> Self {
        Self {
            min_delay,
            last_tick: Instant::now()
        }
    }

    pub async fn wait(&mut self) {
        if let Some(min_delay) = self.min_delay {
            let elapsed = self.last_tick.elapsed();
            if elapsed < min_delay {
                async_std::task::sleep(min_delay - elapsed).await;
            }
        }
        self.last_tick = Instant::now();
    }
}

#[async_trait]
pub trait Game: Sync + Send {
    fn num_players(&self) -> usize;
    fn name(&self) -> &'static str;

    async fn run(&self, players: &mut Vec<RunningJob>, min_delay: Option<Duration>) -> Vec<f32>;
}

#[async_trait]
impl Game for Box<dyn Game> {
    fn num_players(&self) -> usize {
        (**self).num_players()
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }

    async fn run(&self, players: &mut Vec<RunningJob>, min_delay: Option<Duration>) -> Vec<f32> {
        (**self).run(players, min_delay).await
    }
}