use sea_orm::{prelude::*, ActiveValue};
use std::error::Error;
use teloxide::prelude::*;

use crate::{
    entity::{invites, secondary_owners},
    types::BotDialogState,
};

pub async fn ask_for_invite(
    bot: &Bot,
    chat_id: ChatId,
) -> Result<Option<BotDialogState>, Box<dyn Error + Sync + Send>> {
    bot.send_message(
        chat_id,
        "Пожалуйста отправьте код приглашения следующим сообщением.",
    )
    .await?;
    Ok(Some(BotDialogState::WaitingForInvite))
}

pub async fn accept_invite(
    bot: &Bot,
    message: &Message,
    connection: &DatabaseConnection,
) -> Result<Option<BotDialogState>, Box<dyn Error + Send + Sync>> {
    let invite_code = message.text().unwrap_or("").to_string();
    let invite = invites::Entity::find()
        .filter(invites::Column::Invite.eq(invite_code))
        .one(connection)
        .await
        .ok()
        .flatten();

    if invite.is_none() {
        bot.send_message(message.chat.id, "Неизвестный код приглашения.")
            .await?;
        return Ok(None);
    }
    let invite = invite.unwrap();

    secondary_owners::Entity::insert(secondary_owners::ActiveModel {
        primary_owner_chat_id: ActiveValue::Set(invite.chat_id),
        secondary_owner_chat_id: ActiveValue::Set(message.chat.id.0),
        ..Default::default()
    })
    .exec(connection)
    .await?;

    Ok(None)
}
