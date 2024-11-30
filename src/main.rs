use log::{info, debug};
use migration::MigratorTrait;
use players::auto_exec::GameRunner;
use proc_gamedef::make_server;
use sea_orm::{Database, EntityTrait, QueryFilter, ColumnTrait, DatabaseConnection};
use util::DATABASE_URL;
use std::{path::{Path, PathBuf}, sync::Arc, collections::HashSet};

use crate::{games::{Game, nzoi_snake::NzoiSnake}, util::RUN_DIR, web::{api, game_reporter::GameReporter}, entities::agent};

pub mod isolate;
pub mod util;
pub mod langs;
pub mod games;
pub mod players;
pub mod web;
pub mod entities;

make_server!("res/games/test_game.game");

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

    let file = std::fs::File::open("res/configs/snake_small.json").unwrap();
    let game: Box<dyn Game> = Box::new(serde_json::from_reader::<std::fs::File, NzoiSnake>(file).unwrap());

    async_std::task::block_on(async {
        let db = Database::connect(DATABASE_URL).await.unwrap();

        migration::Migrator::up(&db, None).await.unwrap();
        
        let db_copy = db.clone();
        entities::prelude::Agent::delete_many()
            .filter(agent::Column::Partial.eq(true).or(agent::Column::Removed.eq(true)))
            .exec(&db).await.unwrap();

        cleanup_files(&db).await;

        let runner = Arc::new(GameRunner::new(game, "nzoi_snake", 8, db).await);

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
