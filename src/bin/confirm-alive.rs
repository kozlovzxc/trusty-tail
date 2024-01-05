use entity::{alive_events, prelude::*};
use sea_orm::{ColumnTrait, Database, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use std::error::Error;
use teloxide::{requests::Requester, types::ChatId, Bot};
use trusty_tail::config::Config;

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
    let mut statuses_pages = MonitoringStatuses::find().paginate(&connection, 50);
    while let Some(statuses) = statuses_pages.fetch_and_next().await? {
        for status in statuses {
            let alive_event = AliveEvents::find()
                .filter(alive_events::Column::ChatId.eq(status.chat_id))
                .order_by_desc(alive_events::Column::Timestamp)
                .one(&connection)
                .await?;
            match alive_event {
                Some(alive_event) => {
                    let now = chrono::Utc::now().naive_utc();
                    let diff = now - alive_event.timestamp;
                    if diff.num_days() > 3 {
                        bot.send_message(
                            ChatId(status.chat_id),
                            "Please confirm that you are alive by using\n/alive",
                        )
                        .await?;
                    }
                }
                None => {
                    // pass
                }
            }
        }
    }

    Ok(())
}
