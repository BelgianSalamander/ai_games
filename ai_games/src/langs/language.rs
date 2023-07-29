use std::{path::Path, fmt::format};

use async_std::path::PathBuf;
use gamedef::game_interface::{GameInterface, self};
use rand::Rng;

use crate::isolate::sandbox::{IsolateSandbox, RunningJob};

use super::files::ClientFiles;

pub struct PreparedProgram {
    pub dir: PathBuf,

    frozen: bool
}

impl PreparedProgram {
    pub fn add_file(&self, path: &str, content: &str) {
        let full_path = self.dir.join(path);

        std::fs::write(full_path, content).unwrap();
    }

    pub fn dir_as_string(&self) -> String {
        self.dir.to_str().unwrap().to_string()
    }
}

impl PreparedProgram {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        let name_bytes: Vec<u8> = rng.sample_iter(&rand::distributions::Alphanumeric).take(20).collect();
        let name = String::from_utf8_lossy(&name_bytes).to_string();

        let dir = PathBuf::from(format!("./tmp/{}", name));

        std::fs::create_dir_all(&dir).unwrap();

        Self {
            dir,

            frozen: false
        }
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }
}

impl Drop for PreparedProgram {
    fn drop(&mut self) {
        if self.frozen {
            return;
        }
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

pub trait Language {
    fn name(&self) -> &'static str;
    fn id(&self) -> &'static str;
    fn extension(&self) -> &'static str;

    fn generate(&self, game_interface: &GameInterface) -> ClientFiles;

    fn prepare(&self, src: &str, out: &mut PreparedProgram, game_interface: &GameInterface, files: &ClientFiles);

    fn launch(&self, program: &PreparedProgram, sandbox: &IsolateSandbox, itf: &GameInterface, files: &ClientFiles) -> RunningJob;

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