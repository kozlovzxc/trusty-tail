use sea_orm::prelude::*;
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use tera::Tera;

use crate::types::BotDialogState;

use super::alive::mark_alive;

fn get_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "🐶 Меню для владельцев питомцев",
        "/owner_menu",
    )]);
    keyboard.push(vec![InlineKeyboardButton::callback(
        "🛟 Меню для резервных контактов",
        "/contact_menu",
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn show_start_info(
    bot: &Bot,
    message: &Message,
    connection: &DatabaseConnection,
    tera: &Tera,
) -> Result<Option<BotDialogState>, Box<dyn Error + Send + Sync>> {
    mark_alive(connection, message.chat.id).await?;

    let keyboard = get_keyboard();
    let context = tera::Context::new();
    let answer = tera.render("start.html", &context).unwrap();
    bot.parse_mode(ParseMode::Html)
        .send_message(message.chat.id, answer)
        .reply_markup(keyboard)
        .await?;

    Ok(None)
}
