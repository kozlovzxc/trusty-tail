use rand::{distributions::Alphanumeric, Rng};
use sea_orm::{prelude::*, ActiveValue, JoinType, QuerySelect};
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};
use tera::{Context, Tera};

use crate::{
    entity::{invites, monitoring_statuses_utils::is_enabled, profiles, secondary_owners},
    types::BotDialogState,
};

async fn get_secondary_owners(
    connection: &DatabaseConnection,
    chat_id: ChatId,
) -> Vec<profiles::Model> {
    profiles::Entity::find()
        .join_rev(
            JoinType::InnerJoin,
            secondary_owners::Entity::belongs_to(profiles::Entity)
                .from(secondary_owners::Column::SecondaryOwnerChatId)
                .to(profiles::Column::ChatId)
                .into(),
        )
        .filter(secondary_owners::Column::PrimaryOwnerChatId.eq(chat_id.0))
        .all(connection)
        .await
        .unwrap_or(vec![])
}

async fn get_primary_owners(
    connection: &DatabaseConnection,
    chat_id: ChatId,
) -> Vec<profiles::Model> {
    profiles::Entity::find()
        .join_rev(
            JoinType::InnerJoin,
            secondary_owners::Entity::belongs_to(profiles::Entity)
                .from(secondary_owners::Column::PrimaryOwnerChatId)
                .to(profiles::Column::ChatId)
                .into(),
        )
        .filter(secondary_owners::Column::SecondaryOwnerChatId.eq(chat_id.0))
        .all(connection)
        .await
        .unwrap_or(vec![])
}

fn format_owners(owners: Vec<profiles::Model>) -> String {
    let owners = owners
        .iter()
        .map(|profile| format!("@{}", profile.username.clone()))
        .collect::<Vec<_>>();

    if owners.is_empty() {
        "Нет контактов".to_string()
    } else {
        owners.join("\n")
    }
}

async fn get_invite_code(connection: &DatabaseConnection, chat_id: ChatId) -> Option<String> {
    match invites::Entity::find()
        .filter(invites::Column::ChatId.eq(chat_id.0))
        .one(connection)
        .await
        .ok()
        .flatten()
    {
        Some(invite) => Some(invite.invite),
        None => {
            let invite_code = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect::<String>();

            let invite = invites::ActiveModel {
                chat_id: ActiveValue::Set(chat_id.0),
                invite: ActiveValue::Set(invite_code.clone()),
                ..Default::default()
            };

            match invite.insert(connection).await {
                Ok(_) => Some(invite_code.clone()),
                Err(_) => None,
            }
        }
    }
}

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
    let secondary_owners = get_secondary_owners(connection, chat_id).await;
    let secondary_owners = format_owners(secondary_owners);

    let primary_owners = get_primary_owners(connection, chat_id).await;
    let primary_owners = format_owners(primary_owners);

    let invite_code = get_invite_code(connection, chat_id)
        .await
        .unwrap_or("Ошибка".to_string());

    let keyboard = get_menu_keyboard(connection, chat_id).await;
    let mut context = Context::new();
    context.insert("primary_owners", &primary_owners);
    context.insert("secondary_owners", &secondary_owners);
    context.insert("invite_code", &invite_code);
    let answer = tera.render("menu.html", &context).unwrap();
    bot.send_message(chat_id, answer)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(None)
}
