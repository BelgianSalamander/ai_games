use async_std::sync::Mutex;
use isolate::sandbox::IsolateSandbox;
use log::{info, debug};
use proc_gamedef::make_server;
use sea_orm::{DbErr, Database, EntityTrait, QueryFilter, ColumnTrait};
use util::DATABASE_URL;
use std::{io::Write, path::Path, process::{Command, exit}, time::Duration, sync::Arc, env};

use gamedef::parser::parse_game_interface;

use crate::{langs::{python::Python, language::{Language, PreparedProgram}, javascript::make_js_deserializers, cpp::CppLang}, games::{oxo::TicTacToe, Game}, util::{RUN_DIR}, players::{auto_exec::GameRunner}, web::api, entities::agent};

pub mod isolate;
pub mod util;
pub mod langs;
pub mod games;
pub mod players;
pub mod web;
pub mod entities;

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

async fn db_test() -> Result<(), DbErr> {
    let db = Database::connect(DATABASE_URL).await?;

    Ok(())
}

fn main() {
    ensure_root();

    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("ai_games=debug")
            .default_write_style_or("always"),
    )
    .format_timestamp(None)
    .format_module_path(false)
    .init();

    info!("Clearing tmp/ directory");
    std::fs::remove_dir_all("./tmp").unwrap_or(());

    if !Path::new(RUN_DIR).is_dir() {
        std::fs::create_dir(RUN_DIR).unwrap();
    }

    let itf_path = format!("res/games/{}.game", "snake");
    let itf = std::fs::read_to_string(itf_path).unwrap();
    let itf = parse_game_interface(&itf, "snake".to_string()).unwrap();
    CppLang.prepare_files(&itf);

    let lang = Arc::new(Python);

    let game: Box<dyn Game> = Box::new(TicTacToe);

    async_std::task::block_on(async {
        let db = Database::connect(DATABASE_URL).await.unwrap();
        let db_copy = db.clone();
        entities::prelude::Agent::delete_many()
            .filter(agent::Column::Partial.eq(true))
            .exec(&db).await.unwrap();

        let runner = Arc::new(GameRunner::new(game, "tic_tac_toe", 10, db).await);

        //Launch api on new thread
        let runner_copy = runner.clone();
        let itf = runner.itf.clone();
        std::thread::spawn(move || {
            async_std::task::block_on(async {
                api::launch_and_run_api(runner_copy, itf, db_copy).await.unwrap();
            });
        });

        /*for i in 0..10 {
            let mut program = PreparedProgram::new();

            lang.prepare(
                &read_file("./res/test/oxo.py"),
                &mut program, 
                &runner.itf
            ).unwrap();

            let id = runner.add_player(format!("Player {}", i), lang.id().to_string(), program.dir_as_string(), program.src.map(|x| x.to_str().unwrap().to_string())).await;
            debug!("Added player with id {}", id);
        }*/

        info!("Starting executor");
        runner.run().await;
    });
}
