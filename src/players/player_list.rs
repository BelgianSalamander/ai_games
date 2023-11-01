use std::sync::Arc;

use async_std::sync::Mutex;
use gamedef::game_interface::GameInterface;
use rand::{rngs::StdRng, SeedableRng, Rng};

use crate::{util::pool::Pool, isolate::sandbox::{IsolateSandbox, RunningJob}};

use super::player::Player;

pub struct PlayerList {
    pub players: Vec<Player>,
    rng: StdRng
}

impl PlayerList {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            rng: StdRng::from_entropy()
        }
    }

    pub fn num_available(&self) -> usize {
        self.players.len()
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn get_random_player(&mut self) -> Option<Player> {
        let num_players = self.players.len();

        if num_players == 0 {
            return None;
        }

        let index = self.rng.gen_range(0..num_players);
        let player = self.players.swap_remove(index);

        Some(player)
    }

    pub async fn launch_random_in(&mut self, self_ref: Arc<Mutex<Self>>, pool_ref: &Arc<Mutex<Pool<IsolateSandbox>>>, pool: &mut Pool<IsolateSandbox>, itf: &GameInterface) -> RunningJob {
        let player = self.get_random_player().unwrap();
        println!("Player: {}", player.name);

        let (idx, sandbox) = pool.get().unwrap();
        
        let mut res = player.launch(sandbox, itf);

        let arc = pool_ref.clone();
        res.set_on_exit(move |_| {
            async_std::task::spawn(async move {
                let mut pool = arc.lock().await;
                pool.release(idx);
                drop(pool);

                let mut players = self_ref.lock().await;
                players.players.push(player);
            });
        });

        res
    }
}