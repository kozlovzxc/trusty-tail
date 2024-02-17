use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType, PaginatorTrait,
    QueryFilter, QuerySelect,
};
use std::error::Error;
use teloxide::prelude::*;
use trusty_tail::connection;
use trusty_tail::entity::monitoring_statuses_utils::set_monitoring;
use trusty_tail::entity::{
    alive_events, emergency_info, monitoring_statuses, profiles, secondary_owners,
};

#[derive(Debug, FromQueryResult, Clone, PartialEq)]
pub struct MonitoringStatusesAliveJoin {
    pub chat_id: i64,
    pub enabled: bool,
    pub timestamp: Option<NaiveDateTime>,
}

async fn send_alert(
    bot: &Bot,
    connection: &DatabaseConnection,
    chat_id: ChatId,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let info = emergency_info::Entity::find()
        .filter(emergency_info::Column::ChatId.eq(chat_id.0))
        .one(connection)
        .await?;
    let alert_text = info.clone().map(|x| x.text).unwrap_or("---".to_string());

    bot.send_message(
        chat_id,
        "🚨 Высылаем текст на экстренный случай всем экстренным контактам, а пока ставим бота на паузу."
    ).await?;

    set_monitoring(&connection, chat_id, false).await?;

    let username = profiles::Entity::find()
        .filter(profiles::Column::ChatId.eq(chat_id.0))
        .one(connection)
        .await?
        .map_or_else(
            || "Владелец питомца".to_owned(),
            |x| format!("@{}", x.username),
        );

    let recipents = secondary_owners::Entity::find()
        .filter(secondary_owners::Column::PrimaryOwnerChatId.eq(chat_id.0))
        .into_model::<secondary_owners::Model>()
        .all(connection)
        .await?;

    for recipient in recipents {
        log::info!("{:?}", recipient);
        bot.send_message(
            ChatId(recipient.secondary_owner_chat_id),
            format!(
                "🚨 {} не вышел на связь в течение нескольких дней. Пожалуйста, проверьте, что все в порядке. Вот текст на экстренный случай:\n\n{}",
                username,
                alert_text
            )
        )
        .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting...");

    let connection = connection::init().await?;
    let bot = Bot::from_env();

    log::info!("Checking statuses...");
    let mut statuses_pages = monitoring_statuses::Entity::find()
        .filter(monitoring_statuses::Column::Enabled.eq(true))
        .column_as(alive_events::Column::Timestamp, "timestamp")
        .join_rev(
            JoinType::LeftJoin,
            alive_events::Entity::belongs_to(monitoring_statuses::Entity)
                .from(alive_events::Column::ChatId)
                .to(monitoring_statuses::Column::ChatId)
                .into(),
        )
        .filter(
            alive_events::Column::Timestamp
                .lt(chrono::Utc::now().naive_utc() - chrono::Duration::days(2))
                .or(alive_events::Column::Timestamp.is_null()),
        )
        .into_model::<MonitoringStatusesAliveJoin>()
        .paginate(&connection, 50);

    while let Some(statuses) = statuses_pages.fetch_and_next().await? {
        for status in statuses {
            let result = send_alert(&bot, &connection, ChatId(status.chat_id)).await;
            if result.is_err() {
                log::error!("{:?}", result);
            }
        }
    }

    Ok(())
}
