pub use sea_orm_migration::prelude::*;

mod m20231103_000001_create_agent;
mod m20231104_000002_create_user;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231103_000001_create_agent::Migration),
            Box::new(m20231104_000002_create_user::Migration)
        ]
    }
}
