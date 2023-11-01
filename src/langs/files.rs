use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClientFile {
    pub name: String,
    pub content: String,
    pub hidden: bool
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

    pub fn add_file(&mut self, name: &str, content: String, hidden: bool) {
        self.files.insert(name.to_string(), ClientFile {
            name: name.to_string(),
            content,
            hidden
        });
    }

    pub fn include_client_file(&mut self, name: &str, dir: &str) {
        const DIR: &str = "res/client_files/";

        let path = format!("{}{}", DIR, name);

        let content = std::fs::read_to_string(&path).unwrap();

        self.add_file(&format!("{}/{}", dir, name), content, false);
    }
}