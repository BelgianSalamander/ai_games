use std::path::Path;

use async_std::path::PathBuf;
use async_trait::async_trait;
use deadpool::unmanaged::Pool;
use gamedef::game_interface::GameInterface;

use crate::{isolate::sandbox::{IsolateSandbox, RunningJob}, util::{temp_file::random_dir, RUN_DIR}};

use super::files::ClientFiles;

pub struct PreparedProgram {
    pub dir: PathBuf,
    pub src: Option<PathBuf>
}

impl PreparedProgram {
    pub fn add_file(&self, path: &str, content: &str) {
        let full_path = self.dir.join(path);

        std::fs::write(full_path, content).unwrap();
    }

    pub fn add_src_file(&mut self, path: &str, content: &str) {
        match &self.src {
            None => {
                let full_path = self.dir.join(path);
                std::fs::write(full_path.clone(), content).unwrap();
                self.src = Some(full_path);
            },
            Some(s) => {
                panic!("Source is already set to {:?}", s)
            }
        }
        
    }

    pub fn dir_as_string(&self) -> String {
        self.dir.to_str().unwrap().to_string()
    }
}

impl PreparedProgram {
    pub fn new() -> Self {
        let dir = PathBuf::from(random_dir(RUN_DIR));

        Self {
            dir,
            src: None
        }
    }
}

#[async_trait]
pub trait Language: Send + Sync {
    fn name(&self) -> &'static str;
    fn id(&self) -> &'static str;
    fn extension(&self) -> &'static str;

    fn generate(&self, game_interface: &GameInterface) -> ClientFiles;

    //TODO: Make prepare async to allow for compiled languages to work
    async fn prepare(&self, src: &str, out: &mut PreparedProgram, game_interface: &GameInterface, sandboxes: Pool<IsolateSandbox>) -> Result<(), String>;

    fn launch(&self, data_dir: &str, sandbox: &IsolateSandbox, itf: &GameInterface) -> RunningJob;

    fn get_dir(&self, itf: &GameInterface) -> String {
        format!("gen/{}/{}", itf.name, self.id())
    }

    fn prepare_files(&self, itf: &GameInterface) -> ClientFiles {
        let files = self.generate(itf);

        for (name, file) in &files.files {
            let path = format!("{}/{}", self.get_dir(itf), name);
            let path = Path::new(&path);
            println!("Writing file: {:?}", path);

            if !path.exists() {
                std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
                std::fs::File::create(path).unwrap();
            }

            std::fs::write(path, &file.content).unwrap();
        }

        files
    }
}