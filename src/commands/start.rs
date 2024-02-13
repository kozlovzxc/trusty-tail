use sea_orm::DatabaseConnection;
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tera::Tera;

use crate::keyboards::get_main_keyboard;
use crate::types::BotDialogState;

pub async fn start_command(
    bot: &Bot,
    message: &Message,
    tera: &Tera,
    connection: &DatabaseConnection,
) -> Result<Option<BotDialogState>, Box<dyn Error + Send + Sync>> {
    let keyboard = get_main_keyboard(connection, message.chat.id).await;
    let context = tera::Context::new();
    let answer = tera.render("start.html", &context).unwrap();
    bot.parse_mode(ParseMode::Html)
        .send_message(message.chat.id, answer)
        .reply_markup(keyboard)
        .await?;

    Ok(None)
}
