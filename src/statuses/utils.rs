use sea_orm::prelude::*;
use sea_orm::{sea_query::OnConflict, ActiveValue, ColumnTrait};
use std::error::Error;
use teloxide::prelude::*;

use crate::entity::statuses;

pub async fn is_enabled(connection: &DatabaseConnection, chat_id: ChatId) -> bool {
    statuses::Entity::find()
        .filter(statuses::Column::ChatId.eq(chat_id.0))
        .one(connection)
        .await
        .ok()
        .flatten()
        .map_or(false, |x| x.enabled)
}

pub async fn set_monitoring(
    connection: &DatabaseConnection,
    chat_id: ChatId,
    status: bool,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    statuses::Entity::insert(statuses::ActiveModel {
        chat_id: ActiveValue::Set(chat_id.0),
        enabled: ActiveValue::Set(status),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(statuses::Column::ChatId)
            .update_column(statuses::Column::Enabled)
            .to_owned(),
    )
    .exec(connection)
    .await?;

    Ok(())
}
