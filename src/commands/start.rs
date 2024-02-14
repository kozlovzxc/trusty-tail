use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tera::Tera;

use crate::types::BotDialogState;

pub async fn show_start_info(
    bot: &Bot,
    message: &Message,
    tera: &Tera,
) -> Result<Option<BotDialogState>, Box<dyn Error + Send + Sync>> {
    let context = tera::Context::new();
    let answer = tera.render("start.html", &context).unwrap();
    bot.parse_mode(ParseMode::Html)
        .send_message(message.chat.id, answer)
        .await?;

    Ok(None)
}
