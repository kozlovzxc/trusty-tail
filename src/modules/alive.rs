use std::error::Error;

use chrono::prelude::*;
use sea_orm::{prelude::*, sea_query::OnConflict, ActiveValue};
use teloxide::{prelude::*, types::MessageId};

use crate::{entity::alive_events, types::BotDialogState};

pub async fn mark_alive(
    connection: &DatabaseConnection,
    chat_id: ChatId,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    alive_events::Entity::insert(alive_events::ActiveModel {
        chat_id: ActiveValue::Set(chat_id.0),
        timestamp: ActiveValue::Set(Utc::now().naive_utc()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(alive_events::Column::ChatId)
            .update_column(alive_events::Column::Timestamp)
            .to_owned(),
    )
    .exec(connection)
    .await?;

    Ok(())
}

pub async fn mark_alive_callback(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    connection: &DatabaseConnection,
) -> Result<Option<BotDialogState>, Box<dyn Error + Send + Sync>> {
    mark_alive(connection, chat_id).await?;
    bot.delete_message(chat_id, message_id).await?;
    Ok(None)
}
