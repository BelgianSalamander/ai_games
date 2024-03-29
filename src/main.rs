use async_std::fs::File;
use gamedef::{game_interface::GameInterface, parser::parse_game_interface};
use log::{info, debug, error};
use proc_gamedef::make_server;
use sea_orm::{Database, EntityTrait, QueryFilter, ColumnTrait, DatabaseConnection, ActiveValue};
use util::DATABASE_URL;
use std::{path::{Path, PathBuf}, process::exit, sync::Arc, env, collections::HashSet};

use crate::{games::{oxo::TicTacToe, Game, nzoi_snake::NzoiSnake}, util::RUN_DIR, web::{api, game_reporter::GameReporter}, entities::agent, players::{auto_exec::GameRunner}, langs::{cpp::CppLang, language::Language}};

pub mod isolate;
pub mod util;
pub mod langs;
pub mod games;
pub mod players;
pub mod web;
pub mod entities;

make_server!("../res/games/test_game.game");

const USERS: [&str; 23] = [
    "yixiu",
    "siliconjz",
    "ezhqx",
    "Bobobobby",
    "Zinc",
    "Cyanberry",
    "ar88lo",
    "Vedang2006",
    "Figgles",
    "hause",
    "leastinfinformednerd",
    "SyntaxError",
    "NebulaDev",
    "olivermarsh",
    "HappyPanda",
    "EnaYin",
    "kiwirafe",
    "HappyCapybara",
    "B_Star",
    "thunderball",
    "ac122351",
    "Eason",
    "bertil"
];

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

fn cleanup_from_path(path: &PathBuf, dont_delete: &HashSet<PathBuf>) -> bool {
    debug!("Exploring {:?} for cleanup!", path);
    if dont_delete.contains(path) {
        debug!("  Still in use! Skipping!");
        return true;
    }

    if path.is_file() {
        debug!("  Removing file!");
        std::fs::remove_file(path).unwrap();
        return false;
    } else {
        let mut keep = false;

        for child in std::fs::read_dir(path).unwrap() {
            if let Ok(entry) = child {
                if cleanup_from_path(&entry.path(), dont_delete) {
                    keep = true;
                }
            }
        }

        if !keep {
            debug!("  Removing {:?}!", path);
            std::fs::remove_dir(path).unwrap();
        }

        return keep;
    }
}

pub async fn cleanup_files(db: &DatabaseConnection) {
    info!("Cleaning up files!");
    let target_dirs = vec!["./run"];

    let mut dont_delete: HashSet<PathBuf> = HashSet::new();

    for agent in agent::Entity::find().all(db).await.unwrap() {
        dont_delete.insert(PathBuf::from(agent.directory));

        if let Some(error_file) = agent.error_file {
            dont_delete.insert(PathBuf::from(error_file));
        }

        if let Some(src_file) = agent.source_file {
            dont_delete.insert(PathBuf::from(src_file));
        }
    }

    let dont_delete: HashSet<_> = dont_delete.into_iter().map(|x| x.canonicalize().unwrap()).collect();

    debug!("Discovered files:");
    dont_delete.iter().for_each(|x| debug!(" - {:?}", x));

    for dir in target_dirs {
        cleanup_from_path(&PathBuf::from(dir).canonicalize().unwrap(), &dont_delete);
    }
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

    let mut file = std::fs::File::open("res/configs/fours.json").unwrap();
    let game: Box<dyn Game> = Box::new(serde_json::from_reader::<std::fs::File, NzoiSnake>(file).unwrap());

    async_std::task::block_on(async {
        let db = Database::connect(DATABASE_URL).await.unwrap();
        let db_copy = db.clone();
        entities::prelude::Agent::delete_many()
            .filter(agent::Column::Partial.eq(true).or(agent::Column::Removed.eq(true)))
            .exec(&db).await.unwrap();

        cleanup_files(&db).await;

        for name in USERS {
            let username = name;
            info!("Adding default user '{}'", username);

            let other = entities::prelude::User::find()
                .filter(entities::user::Column::Username.eq(username))
                .one(&db)
                .await.unwrap();

            //Check if username is already taken
            if other.is_some() {
                error!("User already existed!");
                continue;
            }

            let num_agents_allowed = 2;

            let profile = entities::user::ActiveModel {
                username: ActiveValue::Set(username.to_string()),
                password: ActiveValue::Set(name.to_string()),
                num_agents_allowed: ActiveValue::Set(num_agents_allowed),
                ..Default::default()
            };

            entities::user::Entity::insert(profile).exec(&db).await.unwrap();
        }

        let runner = Arc::new(GameRunner::new(game, "nzoi_snake", 20, db).await);

        let mut reporter = GameReporter::new(&runner).await;
        let reporter_inner = reporter.inner.clone();

        std::thread::spawn(move || {
            async_std::task::block_on(async {
                reporter.run().await.unwrap();
            });
        });

        //Launch api on new thread
        let runner_copy = runner.clone();
        
        std::thread::spawn(move || {
            async_std::task::block_on(async {
                api::launch_and_run_api(runner_copy, reporter_inner, db_copy).await.unwrap();
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
