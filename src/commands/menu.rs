use sea_orm::prelude::*;
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};
use tera::{Context, Tera};

use crate::{entity::monitoring_statuses_utils::is_enabled, types::BotDialogState};

pub async fn get_menu_keyboard(
    connection: &DatabaseConnection,
    chat_id: ChatId,
) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "⚠️️ Экстренная информация",
        "/emergency_info",
    )]);
    let enabled = is_enabled(connection, chat_id.0).await;
    if enabled {
        keyboard.push(vec![InlineKeyboardButton::callback(
            "✅ Включено",
            "/disable",
        )]);
    } else {
        keyboard.push(vec![InlineKeyboardButton::callback(
            "❌️ Выключено",
            "/enable",
        )]);
    }
    keyboard.push(vec![InlineKeyboardButton::callback(
        "🤝 Принять приглашение",
        "/ask_for_invite",
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn show_menu(
    bot: &Bot,
    chat_id: ChatId,
    connection: &DatabaseConnection,
    tera: &Tera,
) -> Result<Option<BotDialogState>, Box<dyn Error + Sync + Send>> {
    let keyboard = get_menu_keyboard(connection, chat_id).await;
    let context = Context::new();
    let answer = tera.render("menu.html", &context).unwrap();
    bot.send_message(chat_id, answer)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(None)
}
