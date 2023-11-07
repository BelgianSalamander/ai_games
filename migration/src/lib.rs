pub use sea_orm_migration::prelude::*;

mod m20231105_000002_create_agent;
mod m20231105_000001_create_user;
mod m20231107_000003_add_partial;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231105_000002_create_agent::Migration),
            Box::new(m20231105_000001_create_user::Migration),
            Box::new(m20231107_000003_add_partial::Migration)
        ]
    }
}
