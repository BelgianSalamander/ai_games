use std::sync::Arc;

use gamedef::game_interface::GameInterface;

use crate::{langs::language::{PreparedProgram, Language}, isolate::sandbox::{IsolateSandbox, RunningJob}};

use super::auto_exec::PlayerId;

pub struct Player {
    pub id: PlayerId,
    pub name: String,

    pub program: PreparedProgram,
    pub language: Arc<dyn Language>,
}

impl Player {
    pub fn new(id: PlayerId, name: String, program: PreparedProgram, language: Arc<dyn Language>) -> Self {
        Self {
            id,
            name,
            program,
            language,
        }
    }

    pub fn launch(&self, sandbox: &IsolateSandbox, itf: &GameInterface) -> RunningJob {
        self.language.launch(&self.program, sandbox, itf)
    }

    pub fn on_removal(&self, message: &str) {
        println!("Player {} removed: {}", self.name, message)
    }
}