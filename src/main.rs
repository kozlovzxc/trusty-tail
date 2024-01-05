mod config;

use chrono::Utc;
use config::Config;
use entity::{prelude::*, *};
use migration::Migrator;
use sea_orm::{ActiveValue, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm_migration::prelude::*;
use std::error::Error;
use std::fmt::Debug;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(
    rename_rule = "snake_case",
    description = "These commands are supported:"
)]
enum Command {
    Start,
    Help,
    UpdateEmergencyText,
    GetEmergencyText,
    Alive,
    Enable,
    Disable,
    Status,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum BotDialogState {
    #[default]
    Idle,
    WaitingEmergencyText,
}

type BotDialogue = Dialogue<BotDialogState, InMemStorage<BotDialogState>>;

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

async fn print_start_info(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(message.chat.id, "Start!").await?;
    Ok(())
}

async fn print_help_info(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(message.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn ask_for_emergency_info(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
) -> HandlerResult {
    dialogue
        .update(BotDialogState::WaitingEmergencyText)
        .await?;
    bot.send_message(message.chat.id, "Input your emergency text!")
        .await?;
    Ok(())
}

async fn update_emergency_info(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;
    let new_emergency_info = emergency_info::ActiveModel {
        text: ActiveValue::Set(message.text().unwrap_or("").to_string()),
        chat_id: ActiveValue::Set(message.chat.id.0),
        ..Default::default()
    };
    EmergencyInfo::insert(new_emergency_info)
        .on_conflict(
            OnConflict::column(emergency_info::Column::ChatId)
                .update_column(emergency_info::Column::Text)
                .to_owned(),
        )
        .exec(&connection)
        .await?;

    bot.send_message(message.chat.id, "Updated!").await?;
    Ok(())
}

async fn get_emergency_info(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;
    let emergency_info = EmergencyInfo::find()
        .filter(emergency_info::Column::ChatId.eq(message.chat.id.0))
        .one(&connection)
        .await?;
    match emergency_info {
        Some(emergency_info) => {
            bot.send_message(message.chat.id, emergency_info.text)
                .await?;
        }
        None => {
            bot.send_message(message.chat.id, "There is no saved emergency info")
                .await?;
        }
    }
    Ok(())
}

async fn mark_alive(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    AliveEvents::insert(alive_events::ActiveModel {
        chat_id: ActiveValue::Set(message.chat.id.0),
        timestamp: ActiveValue::Set(Utc::now().naive_utc()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(alive_events::Column::ChatId)
            .update_column(alive_events::Column::Timestamp)
            .to_owned(),
    )
    .exec(&connection)
    .await?;

    bot.send_message(message.chat.id, "Marked as alive!")
        .await?;
    Ok(())
}

async fn enable(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    MonitoringStatuses::insert(monitoring_statuses::ActiveModel {
        chat_id: ActiveValue::Set(message.chat.id.0),
        enabled: ActiveValue::Set(true),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(monitoring_statuses::Column::ChatId)
            .update_column(monitoring_statuses::Column::Enabled)
            .to_owned(),
    )
    .exec(&connection)
    .await?;

    bot.send_message(message.chat.id, "Enabled!").await?;
    Ok(())
}

async fn disable(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    MonitoringStatuses::insert(monitoring_statuses::ActiveModel {
        chat_id: ActiveValue::Set(message.chat.id.0),
        enabled: ActiveValue::Set(false),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(monitoring_statuses::Column::ChatId)
            .update_column(monitoring_statuses::Column::Enabled)
            .to_owned(),
    )
    .exec(&connection)
    .await?;

    bot.send_message(message.chat.id, "Disabled!").await?;
    Ok(())
}

async fn get_status(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    let monitoring_status = MonitoringStatuses::find()
        .filter(monitoring_statuses::Column::ChatId.eq(message.chat.id.0))
        .one(&connection)
        .await?;

    match monitoring_status {
        Some(monitoring_status) => {
            bot.send_message(
                message.chat.id,
                format!("Monitoring status: {}", monitoring_status.enabled),
            )
            .await?;
        }
        None => {
            bot.send_message(message.chat.id, "Monitoring is not set")
                .await?;
        }
    }

    Ok(())
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
    let schema_manager = SchemaManager::new(&connection);
    Migrator::up(&connection, None).await?;
    assert!(schema_manager.has_table("emergency_info").await?);
    log::info!("Applied migrations...");

    let bot = Bot::from_env();

    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<BotDialogState>, BotDialogState>()
        .map(|message: Message| {
            let text = message.text().unwrap_or_default();
            Command::parse(&text, "").ok()
        })
        .map_async(|dialogue: BotDialogue| async move { dialogue.get().await.ok().flatten() })
        // Commands
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::Start)))
                .endpoint(print_start_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::Help)))
                .endpoint(print_help_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::UpdateEmergencyText)))
                .endpoint(ask_for_emergency_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::GetEmergencyText)))
                .endpoint(get_emergency_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::Alive))).endpoint(mark_alive),
        )
        .branch(dptree::filter(|command| matches!(command, Some(Command::Enable))).endpoint(enable))
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::Disable))).endpoint(disable),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::Status))).endpoint(get_status),
        )
        // Dialogs
        .branch(
            dptree::filter(|state: BotDialogState| {
                matches!(state, BotDialogState::WaitingEmergencyText)
            })
            .endpoint(update_emergency_info),
        )
        .endpoint(|bot: Bot, message: Message| async move {
            bot.send_message(message.chat.id, "Unknown command!")
                .await?;
            Ok(())
        });

    log::info!("Started listening...");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            InMemStorage::<BotDialogState>::new(),
            connection
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
