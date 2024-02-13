use sea_orm::prelude::*;
use sea_orm::{sea_query::OnConflict, ActiveValue};
use std::error::Error;
use teloxide::prelude::*;

use crate::{entity::monitoring_statuses, keyboards::get_main_keyboard};

pub async fn disable_monitoring(
    bot: &Bot,
    chat_id: ChatId,
    message_id: teloxide::types::MessageId,
    connection: &DatabaseConnection,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    monitoring_statuses::Entity::insert(monitoring_statuses::ActiveModel {
        chat_id: ActiveValue::Set(chat_id.0),
        enabled: ActiveValue::Set(false),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(monitoring_statuses::Column::ChatId)
            .update_column(monitoring_statuses::Column::Enabled)
            .to_owned(),
    )
    .exec(connection)
    .await?;

    let keyboard = get_main_keyboard(connection, chat_id).await;
    bot.edit_message_reply_markup(chat_id, message_id)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}
