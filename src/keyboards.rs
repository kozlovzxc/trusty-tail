use sea_orm::DatabaseConnection;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::entity::monitoring_statuses_utils::is_enabled;

pub async fn get_main_keyboard(
    connection: &DatabaseConnection,
    chat_id: ChatId,
) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "✍️ Экстренный текст",
        "/get_emergency_text",
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

    InlineKeyboardMarkup::new(keyboard)
}
