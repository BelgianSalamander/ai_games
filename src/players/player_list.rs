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
}