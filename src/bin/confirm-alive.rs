use sea_orm::prelude::*;
use sea_orm::{EntityTrait, JoinType, PaginatorTrait, QuerySelect};
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use trusty_tail::connection;
use trusty_tail::entity::{alive_events, statuses};
use trusty_tail::profiles::utils::select_active_profiles;

pub fn get_alive_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "👍 Все хорошо",
        "/mark_alive",
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn run(
    connection: &DatabaseConnection,
    bot: &Bot,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!("Checking statuses...");
    let mut profiles = select_active_profiles()
        .join_rev(
            JoinType::LeftJoin,
            alive_events::Entity::belongs_to(statuses::Entity)
                .from(alive_events::Column::ChatId)
                .to(statuses::Column::ChatId)
                .into(),
        )
        .filter(
            alive_events::Column::Timestamp
                .lt(chrono::Utc::now().naive_utc() - chrono::Duration::days(1))
                .or(alive_events::Column::Timestamp.is_null()),
        )
        .paginate(connection, 50);

    let keyboard = get_alive_keyboard();
    while let Some(profiles) = profiles.fetch_and_next().await? {
        for profile in profiles {
            log::info!("Notifying {:?}", profile);
            bot.send_message(
                ChatId(profile.chat_id),
                "Пожалуйста подтвердите, что с вами все хорошо 🙏",
            )
            .reply_markup(keyboard.clone())
            .await?;
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

    run(&connection, &bot).await.unwrap();

    Ok(())
}
