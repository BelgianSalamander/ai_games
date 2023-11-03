use std::pin::Pin;

use futures::Future;

pub mod temp_file;
pub mod pool;
pub mod asyncio;

pub const DATABASE_URL: &str = "sqlite:./sqlite.db?mode=rwc";