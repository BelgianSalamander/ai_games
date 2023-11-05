use std::fs::File;
use std::rc::Rc;
use async_std::path::Path;
use log::{trace, warn};
use rand::distributions::Alphanumeric;
use rand::Rng;

pub fn rand_str(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let name_bytes: Vec<u8> = rng.sample_iter(&Alphanumeric).take(len).collect();
    String::from_utf8_lossy(&name_bytes).to_string()
}

pub fn random_file(dir: &str, ext: &str) -> String {
    let filename = format!("{}/{}{}", dir, rand_str(25), ext);

    filename
}

pub fn random_dir(parent: &str) -> String {
    let dir = format!("{}/{}", parent, rand_str(25));

    std::fs::create_dir(&dir).unwrap();

    dir
}

pub struct TempFile {
    pub path: String,
    frozen: bool
}

impl TempFile {
    pub fn new() -> TempFile {
        TempFile::with_extra("")
    }

    pub fn with_extra(extra: &str) -> TempFile {
        let path = TempFile::get_temp_file_path(extra);

        trace!("Creating temp file: {}", path);

        TempFile {
            path,
            frozen: false
        }
    }

    pub fn get_temp_file_path(extra: &str) -> String {
        let mut path = "./tmp/".to_string();

        //Check if tmp dir exists
        if !std::path::Path::new("./tmp").exists() {
            std::fs::create_dir("./tmp").unwrap();
        }

        let mut rng = rand::thread_rng();
        let name_bytes: Vec<u8> = rng.sample_iter(&Alphanumeric).take(20).collect();
        let name = String::from_utf8_lossy(&name_bytes).to_string();

        path.push_str(&name);

        if !extra.is_empty() {
            path.push_str(&extra);
        }

        // Create the file
        {std::fs::File::create(&path).unwrap();}

        path
    }

    pub fn get_file_read(&self) -> File {
        File::open(&self.path).unwrap()
    }

    pub async fn get_file_async_read(&self) -> async_std::fs::File {
        async_std::fs::File::open(&self.path).await.unwrap()
    }

    pub async fn write_string_async(&self, data: &str) {
        async_std::fs::write(&self.path, data).await.unwrap();
    }

    pub fn get_file_write(&self) -> File {
        File::create(&self.path).unwrap()
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.frozen {
            trace!("Temp file is frozen, not deleting: {}", self.path);
            return;
        }

        trace!("Deleting temp file: {}", self.path);
        match std::fs::remove_file(&self.path) {
            Ok(_) => {},
            Err(e) => {
                warn!("Failed to delete temp file: {}", e);
            }
        }
    }
}

pub struct SharedTempFile {
    pub file: Rc<TempFile>
}

impl SharedTempFile {
    pub fn new() -> SharedTempFile {
        SharedTempFile {
            file: Rc::new(TempFile::new())
        }
    }

    pub fn get_file_read(&self) -> File {
        self.file.get_file_read()
    }

    pub fn get_file_write(&self) -> File {
        self.file.get_file_write()
    }
}