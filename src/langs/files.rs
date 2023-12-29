use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClientFile {
    pub name: String,
    pub content: String,
    pub hidden: bool,

    pub description: String,
    pub download_name: String
}

#[derive(Debug, Clone)]
pub struct ClientFiles {
    pub files: HashMap<String, ClientFile>
}

impl ClientFiles {
    pub fn new() -> Self {
        Self {
            files: HashMap::new()
        }
    }

    pub fn add_file(&mut self, name: &str, content: String, hidden: bool, description: &str, download_name: &str) {
        self.files.insert(name.to_string(), ClientFile {
            name: name.to_string(),
            content,
            hidden,
            description: description.to_string(),
            download_name: download_name.to_string()
        });
    }

    pub fn include_client_file(&mut self, name: &str, dir: &str, description: &str, download_name: &str) {
        const DIR: &str = "res/client_files/";

        let path = format!("{}{}", DIR, name);

        let content = std::fs::read_to_string(&path).unwrap();

        self.add_file(&format!("{}/{}", dir, name), content, false, description, download_name);
    }
}