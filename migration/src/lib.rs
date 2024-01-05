pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_emergency_info;
mod m20240105_150139_create_alive_events_table;
mod m20240105_155622_create_monitoring_statuses_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_emergency_info::Migration),
            Box::new(m20240105_150139_create_alive_events_table::Migration),
            Box::new(m20240105_155622_create_monitoring_statuses_table::Migration),
        ]
    }
}
