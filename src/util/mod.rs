use sea_orm::{ActiveValue, Value};

pub mod temp_file;
pub mod asyncio;

pub const DATABASE_URL: &str = "sqlite:./sqlite.db?mode=rwc";
pub const RUN_DIR: &str = "./run";

pub trait ActiveValueExtension<T> {
    fn get(&self) -> Option<&T>;
}

impl<T: Into<Value>> ActiveValueExtension<T> for ActiveValue<T> {
    fn get(&self) -> Option<&T> {
        match self {
            ActiveValue::Set(x) | ActiveValue::Unchanged(x) => Some(x),
            _ => None
        }
    }
}