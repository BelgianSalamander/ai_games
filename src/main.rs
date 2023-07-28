use isolate::sandbox::IsolateSandbox;
use proc_gamedef::make_server;
use std::{io::Write, path::Path, process::Command, time::Duration};

use gamedef::parser::parse_game_interface;

use crate::{isolate::sandbox::LaunchOptionsBuilder, langs::{python::Python, language::{Language, PreparedProgram}}, games::{oxo::TicTacToe, Game}};

pub mod isolate;
pub mod util;
pub mod langs;
pub mod games;

make_server!("../res/games/test_game.game");

fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("ai_games=debug")
            .default_write_style_or("always"),
    )
    .format_timestamp(None)
    .format_module_path(false)
    .init();

    const PATH: &str = "res/games/tic_tac_toe.game";
    let content = std::fs::read_to_string(PATH).unwrap();

    let game_interface = parse_game_interface(&content, "tic_tac_toe".to_string()).unwrap();

    let client_files = Python.prepare_files(&game_interface);

    let mut program = PreparedProgram::new();

    Python.prepare(
        &read_file("./res/test/oxo.py"),
        &mut program,
        &game_interface,
        &client_files
    );

    async_std::task::block_on(async {
        let sandbox_one = IsolateSandbox::new(0).await;
        let sandbox_two = IsolateSandbox::new(1).await;

        let program_a = Python.launch(&program, &sandbox_one, &game_interface, &client_files);
        let program_b = Python.launch(&program, &sandbox_two, &game_interface, &client_files);

        let programs = vec![program_a, program_b];

        let results = TicTacToe::run(programs).await;

        println!("Results: {:?}", results);
    });
}
