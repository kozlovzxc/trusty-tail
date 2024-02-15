use chrono::NaiveDateTime;
use sea_orm::prelude::*;
use sea_orm::{Database, EntityTrait, FromQueryResult, JoinType, PaginatorTrait, QuerySelect};
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use trusty_tail::config::Config;
use trusty_tail::entity::{alive_events, monitoring_statuses};

#[derive(Debug, FromQueryResult, Clone, PartialEq)]
pub struct MonitoringStatusesAliveJoin {
    pub chat_id: i64,
    pub enabled: bool,
    pub timestamp: Option<NaiveDateTime>,
}

pub fn get_alive_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![InlineKeyboardButton::callback(
        "Все хорошо",
        "/mark_alive",
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting...");
    let config = Config::init();
    log::info!("Initialized config...");
    let database_full_url = format!(
        "postgres://{}:{}@{}/{}",
        config.db_user, config.db_password, config.db_url, config.db_name
    );
    let connection = Database::connect(database_full_url).await?;
    log::info!("Connected to database...");
    let bot = Bot::from_env();

    log::info!("Checking statuses...");
    let mut statuses_pages = monitoring_statuses::Entity::find()
        .column_as(alive_events::Column::Timestamp, "timestamp")
        .join_rev(
            JoinType::LeftJoin,
            alive_events::Entity::belongs_to(monitoring_statuses::Entity)
                .from(alive_events::Column::ChatId)
                .to(monitoring_statuses::Column::ChatId)
                .into(),
        )
        .filter(monitoring_statuses::Column::Enabled.eq(true))
        .filter(
            alive_events::Column::Timestamp
                .lt(chrono::Utc::now().naive_utc() - chrono::Duration::days(1))
                .or(alive_events::Column::Timestamp.is_null()),
        )
        .into_model::<MonitoringStatusesAliveJoin>()
        .paginate(&connection, 50);

    let keyboard = get_alive_keyboard();
    while let Some(statuses) = statuses_pages.fetch_and_next().await? {
        for status in statuses {
            bot.send_message(
                ChatId(status.chat_id),
                "Пожалуйста подтвердите, что с вами все хорошо",
            )
            .reply_markup(keyboard.clone())
            .await?;
        }
    }

    Ok(())
}
