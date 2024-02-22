use sea_orm::{prelude::*, JoinType, QuerySelect};
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
};
use tera::{Context, Tera};

use crate::{
    entity::{profiles, secondary_owners},
    types::BotDialogState,
};

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

pub fn get_secondary_menu_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "👈 Меню владельца питомца",
        "/owner_menu",
    )]);

    keyboard.push(vec![InlineKeyboardButton::callback(
        "🤝 Принять приглашение",
        "/ask_for_invite",
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn show_contact_menu(
    bot: &Bot,
    chat_id: ChatId,
    connection: &DatabaseConnection,
    tera: &Tera,
) -> Result<Option<BotDialogState>, Box<dyn Error + Sync + Send>> {
    let primary_owners = get_primary_owners(connection, chat_id).await;
    let primary_owners = format_owners(primary_owners);

    let keyboard = get_secondary_menu_keyboard();
    let mut context = Context::new();
    context.insert("primary_owners", &primary_owners);
    let answer = tera.render("contact_menu.html", &context).unwrap();
    bot.send_message(chat_id, answer)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .disable_web_page_preview(true)
        .await?;

    Ok(None)
}
