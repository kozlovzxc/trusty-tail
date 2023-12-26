mod config;

use config::Config;
use entity::{prelude::*, *};
use log::error;
use migration::Migrator;
use sea_orm::{ActiveValue, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm_migration::prelude::*;
use std::error::Error;
use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let config = Config::init();
    let database_full_url = format!(
        "postgres://{}:{}@{}/{}",
        config.db_user, config.db_password, config.db_url, config.db_name
    );
    let connection = Database::connect(database_full_url).await?;
    let schema_manager = SchemaManager::new(&connection);
    Migrator::up(&connection, None).await?;
    assert!(schema_manager.has_table("emergency_info").await?);

    log::info!("Starting bot...");

    let bot = Bot::from_env();

    let connection = Arc::new(connection);
    Command::repl(bot, move |bot, msg, cmd| {
        answer(bot, msg, cmd, Arc::clone(&connection))
    })
    .await;

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Add text regarding what to do in case of emergency")]
    UpdateEmergencyText(String),
    #[command(description = "Get text regarding what to do in case of emergency")]
    GetEmergencyText,
}

async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    connection: Arc<DatabaseConnection>,
) -> ResponseResult<()> {
    let connection = &*connection;

    match cmd {
        Command::Start | Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::UpdateEmergencyText(emergency_text) => {
            let new_emergency_info = emergency_info::ActiveModel {
                text: ActiveValue::Set(emergency_text),
                chat_id: ActiveValue::Set(msg.chat.id.0),
                ..Default::default()
            };
            let res = EmergencyInfo::insert(new_emergency_info)
                .on_conflict(
                    OnConflict::column(emergency_info::Column::ChatId)
                        .update_column(emergency_info::Column::Text)
                        .to_owned()
                )
                .exec(connection)
                .await;

            match res {
                Ok(_) => bot.send_message(msg.chat.id, format!("Added!")).await?,
                Err(err) => {
                    error!("Can't add an entry: {:?}", err);
                    bot.send_message(msg.chat.id, format!("Can't update!"))
                        .await?
                }
            }
        }
        Command::GetEmergencyText => {
            let res = EmergencyInfo::find()
                .filter(emergency_info::Column::ChatId.eq(msg.chat.id.0))
                .one(connection)
                .await;

            if res.is_err() {
                error!("Can't read an entry: {:?}", res.err());
                bot.send_message(msg.chat.id, format!("Can't read!"))
                    .await?;
                return Ok(());
            }

            let res = res.unwrap();
            if res.is_none() {
                bot.send_message(msg.chat.id, format!("Can't find an entry!"))
                    .await?;
                return Ok(());
            }

            let res = res.unwrap();

            bot.send_message(
                msg.chat.id,
                format!("Current emergency text:\n{}", res.text),
            )
            .await?
        }
    };

    Ok(())
}
