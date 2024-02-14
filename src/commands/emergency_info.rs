use sea_orm::prelude::*;
use sea_orm::{sea_query::OnConflict, ActiveValue};
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};
use tera::{Context, Tera};

use crate::{entity::emergency_info, types::BotDialogState};

async fn get_emerengecy_info_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "👈 Главное меню",
        "/main_menu",
    )]);

    keyboard.push(vec![InlineKeyboardButton::callback(
        "✍️ Задать экстренную информацию",
        "/ask_for_emergency_info",
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn show_emergency_info(
    bot: &Bot,
    chat_id: ChatId,
    connection: &DatabaseConnection,
    tera: &Tera,
) -> Result<Option<BotDialogState>, Box<dyn Error + Sync + Send>> {
    let emergency_text = emergency_info::Entity::find()
        .filter(emergency_info::Column::ChatId.eq(chat_id.0))
        .one(connection)
        .await
        .ok()
        .flatten()
        .map(|x| x.text);

    let context = match emergency_text {
        Some(emergency_text) => {
            let mut context = Context::new();
            context.insert("emergency_text", &emergency_text);
            context
        }
        None => Context::new(),
    };

    let answer = tera.render("emergency_info.html", &context).unwrap();
    let keyboard = get_emerengecy_info_keyboard().await;
    bot.send_message(chat_id, answer)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(None)
}

pub async fn ask_for_emergency_info(
    bot: &Bot,
    chat_id: ChatId,
    tera: &Tera,
) -> Result<Option<BotDialogState>, Box<dyn Error + Sync + Send>> {
    let context = tera::Context::new();
    let answer = tera.render("emergency_info_fill.html", &context).unwrap();
    bot.send_message(chat_id, answer)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(Some(BotDialogState::WaitingEmergencyText))
}

pub async fn set_emergency_info(
    bot: &Bot,
    message: &Message,
    connection: &DatabaseConnection,
) -> Result<Option<BotDialogState>, Box<dyn Error + Sync + Send>> {
    let new_emergency_info = emergency_info::ActiveModel {
        text: ActiveValue::Set(message.text().unwrap_or("").to_string()),
        chat_id: ActiveValue::Set(message.chat.id.0),
        ..Default::default()
    };
    emergency_info::Entity::insert(new_emergency_info)
        .on_conflict(
            OnConflict::column(emergency_info::Column::ChatId)
                .update_column(emergency_info::Column::Text)
                .to_owned(),
        )
        .exec(connection)
        .await?;

    Ok(None)
}
