use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, JoinType, PaginatorTrait, QueryFilter,
    QuerySelect,
};
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tera::Tera;
use trusty_tail::connection;
use trusty_tail::entity::{alive_events, emergency_info, profiles};
use trusty_tail::profiles::utils::{
    select_active_profiles, select_emergency_contacts, select_profile,
};
use trusty_tail::statuses::utils::set_monitoring;

async fn send_alert(
    bot: &Bot,
    connection: &DatabaseConnection,
    chat_id: ChatId,
    tera: &Tera,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let alert_text = emergency_info::Entity::find()
        .filter(emergency_info::Column::ChatId.eq(chat_id.0))
        .one(connection)
        .await?
        .map(|x| x.text)
        .unwrap_or("(Текст на экстренный случай не задан)".to_string());

    let context = tera::Context::new();
    let message = tera.render("alert_owner.html", &context).unwrap();
    bot.send_message(chat_id, message).await?;

    set_monitoring(&connection, chat_id, false).await?;

    let username = select_profile(chat_id).one(connection).await?.map_or_else(
        || "Владелец питомца".to_owned(),
        |x| format!("@{}", x.username),
    );
    let recipents = select_emergency_contacts(chat_id).all(connection).await?;
    let mut context = tera::Context::new();
    context.insert("username", &username);
    context.insert("emergency_text", &alert_text);
    let message = tera.render("alert_contact.html", &context).unwrap();

    for recipient in recipents {
        log::info!("Notifying {:?}", recipient);
        bot.send_message(ChatId(recipient.secondary_owner_chat_id), message.clone())
            .parse_mode(ParseMode::Html)
            .await?;
    }
    Ok(())
}

async fn run(
    connection: &DatabaseConnection,
    bot: &Bot,
    tera: &Tera,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!("Checking statuses...");

    let mut profiles = select_active_profiles()
        .join_rev(
            JoinType::LeftJoin,
            alive_events::Entity::belongs_to(profiles::Entity)
                .from(alive_events::Column::ChatId)
                .to(profiles::Column::ChatId)
                .into(),
        )
        .filter(
            alive_events::Column::Timestamp
                .lt(chrono::Utc::now().naive_utc() - chrono::Duration::days(2))
                .or(alive_events::Column::Timestamp.is_null()),
        )
        .paginate(connection, 50);

    while let Some(profiles) = profiles.fetch_and_next().await? {
        for profile in profiles {
            let result = send_alert(&bot, &connection, ChatId(profile.chat_id), &tera).await;
            if result.is_err() {
                log::error!("Got error: {:?}", result);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting...");

    let connection = connection::init().await?;
    let bot = Bot::from_env();

    let tera = match Tera::new("templates/**/*") {
        Ok(tera) => tera,
        Err(message) => panic!("Tera error: {}", message),
    };

    run(&connection, &bot, &tera).await.unwrap();

    Ok(())
}
