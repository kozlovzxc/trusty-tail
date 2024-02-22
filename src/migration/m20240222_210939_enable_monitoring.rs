use sea_orm::{ActiveValue, EntityTrait};
use sea_orm_migration::prelude::*;

use crate::entity::statuses;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();

        statuses::Entity::update_many()
            .set(statuses::ActiveModel {
                enabled: ActiveValue::Set(true),
                ..Default::default()
            })
            .exec(connection)
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
