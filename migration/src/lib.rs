pub use sea_orm_migration::prelude::*;

mod m20261025_000001_initial;
mod m20261109_000002_platform;
mod m20261111_000003_report_keys;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261025_000001_initial::Migration),
            Box::new(m20261109_000002_platform::Migration),
            Box::new(m20261111_000003_report_keys::Migration),
        ]
    }
}
