use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, Database, EntityTrait, FromQueryResult, JoinType, PaginatorTrait, QueryFilter,
    QuerySelect,
};
use std::error::Error;
use teloxide::{requests::Requester, types::ChatId, Bot};
use trusty_tail::config::Config;
use trusty_tail::entity::{alive_events, emergency_info, monitoring_statuses, secondary_owners};

#[derive(Debug, FromQueryResult, Clone, PartialEq)]
pub struct MonitoringStatusesAliveJoin {
    pub chat_id: i64,
    pub enabled: bool,
    pub timestamp: NaiveDateTime,
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
            JoinType::InnerJoin,
            alive_events::Entity::belongs_to(monitoring_statuses::Entity)
                .from(alive_events::Column::ChatId)
                .to(monitoring_statuses::Column::ChatId)
                .into(),
        )
        .filter(
            alive_events::Column::Timestamp
                .lt(chrono::Utc::now().naive_utc() - chrono::Duration::days(2)),
        )
        .into_model::<MonitoringStatusesAliveJoin>()
        .paginate(&connection, 50);

    while let Some(statuses) = statuses_pages.fetch_and_next().await? {
        for status in statuses {
            let info = emergency_info::Entity::find()
                .filter(emergency_info::Column::ChatId.eq(status.chat_id))
                .one(&connection)
                .await?;
            let alert_text = info.clone().map(|x| x.text).unwrap_or("---".to_string());

            let recipents = secondary_owners::Entity::find()
                .filter(secondary_owners::Column::PrimaryOwnerChatId.eq(status.chat_id))
                .into_model::<secondary_owners::Model>()
                .all(&connection)
                .await?;

            for recipient in recipents {
                log::info!("{:?}", recipient);
                bot.send_message(
                    ChatId(recipient.secondary_owner_chat_id),
                    format!(
                        "Владелец питомца не вышел на связь в течение нескольких дней. Пожалуйста, проверьте, что с ним и с его животным все в порядке. Вот текст на экстренный случай:\n\n{}",
                        alert_text
                    )
                )
                .await?;
            }
        }
    }

    Ok(())
}
