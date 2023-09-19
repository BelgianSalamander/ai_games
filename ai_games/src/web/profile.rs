use lazy_static::lazy_static;
use rand::seq::SliceRandom;

lazy_static! {
    pub static ref WORDS: Vec<String> = {
        let mut words = Vec::new();

        let file = std::fs::read_to_string("./res/words.txt").unwrap();

        for line in file.lines() {
            words.push(line.to_string());
        }

        words
    };
}

fn generate_password() -> String {
    let mut rng = rand::thread_rng();

    let mut password = (0..4).map(|_| WORDS.choose(&mut rng).unwrap().clone()).collect::<Vec<_>>().join("-");

    password
}

pub struct Profile {
    pub id: u32,
    pub username: String,
    pub password: String,

    pub num_agents_allowed: usize,
    pub agents: Vec<AgentInfo>
}

impl Profile {
    pub fn new(id: u32, username: String, num_agents: usize) -> Self {
        Self {
            id,
            username,
            password: generate_password(),

            num_agents_allowed: num_agents,
            agents: Vec::new(),
        }
    }

    pub fn regenerate_password(&mut self) {
        self.password = generate_password();
    }
}

pub struct AgentInfo {

}