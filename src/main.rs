use async_std::sync::Mutex;
use isolate::sandbox::IsolateSandbox;
use log::info;
use proc_gamedef::make_server;
use std::{io::Write, path::Path, process::{Command, exit}, time::Duration, sync::Arc, env};

use gamedef::parser::parse_game_interface;

use crate::{isolate::{sandbox::LaunchOptionsBuilder}, langs::{python::Python, language::{Language, PreparedProgram}, javascript::make_js_deserializers}, games::{oxo::TicTacToe, Game}, util::pool::Pool, players::{player_list::PlayerList, player::Player, auto_exec::GameRunner}, web::api};

pub mod isolate;
pub mod util;
pub mod langs;
pub mod games;
pub mod players;
pub mod web;

make_server!("../res/games/test_game.game");

fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn ensure_root() {
    match env::var("USER") {
        Err(e) => {
            println!("Something went wrong: {:?}", e);
            exit(2);
        },
        Ok(name) => {
            if name != "root" {
                println!("Must be started as root");
                exit(1);
            }
        }
    }
}

fn main() {
    ensure_root();

    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("ai_games=info")
            .default_write_style_or("always"),
    )
    .format_timestamp(None)
    .format_module_path(false)
    .init();

    info!("Clearing tmp/ directory");
    std::fs::remove_dir_all("./tmp").unwrap_or(());

    let lang = Arc::new(Python);

    let game: Box<dyn Game> = Box::new(TicTacToe);


    async_std::task::block_on(async { 
        let runner = Arc::new(GameRunner::new(game, "tic_tac_toe", 10).await);

        //Launch api on new thread
        let runner_copy = runner.clone();
        let itf = runner.itf.clone();
        std::thread::spawn(move || {
            async_std::task::block_on(async {
                let mut languages: Vec<Arc<dyn Language>> = vec![];
                languages.push(Arc::new(Python));

                api::launch_and_run_api(runner_copy, languages, itf).await.unwrap();
            });
        });

        for i in 0..9 {
            let mut program = PreparedProgram::new();
    
            lang.prepare(
                &read_file("./res/test/oxo.py"), 
                &mut program,
                &runner.itf
            );
    
            let player = Player::new(
                runner.make_id(),
                format!("Player {}", i),
                program,
                lang.clone()
            );
    
            runner.add_player(player).await;
        }

        let mut program = PreparedProgram::new();

        lang.prepare(
            &read_file("./res/test/half_smart.py"), 
            &mut program,
            &runner.itf
        );

        let player = Player::new(
            runner.make_id(),
            format!("Smartass"),
            program,
            lang.clone()
        );

        runner.add_player(player).await;

        info!("Starting game");
        runner.run().await;
    });
}
