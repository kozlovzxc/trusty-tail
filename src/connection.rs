use crate::{config::Config, migration::Migrator};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::{MigratorTrait, SchemaManager};
use std::error::Error;

pub async fn init() -> Result<DatabaseConnection, Box<dyn Error>> {
    let config = Config::init();

    let database_url = format!(
        "postgres://{}:{}@{}/{}",
        config.db_user, config.db_password, config.db_url, config.db_name
    );
    let mut opt = ConnectOptions::new(database_url);
    opt.sqlx_logging_level(log::LevelFilter::Debug);
    let connection = Database::connect(opt).await?;
    log::info!("Connected to database...");

    let schema_manager = SchemaManager::new(&connection);
    Migrator::up(&connection, None).await?;
    assert!(schema_manager.has_table("emergency_info").await?);
    log::info!("Applied migrations...");

    Ok(connection)
}
