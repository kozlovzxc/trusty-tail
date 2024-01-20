mod config;

use chrono::Utc;
use config::Config;
use entity::*;
use migration::Migrator;
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    JoinType, QueryFilter, QuerySelect,
};
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
    #[command(description = "display this text.")]
    Help,
    #[command(description = "set emergency text")]
    SetEmergencyText,
    #[command(description = "get emergency text")]
    GetEmergencyText,
    #[command(description = "mark that you are ok")]
    ImOk,
    #[command(description = "enable monitoring")]
    EnableMonitoring,
    #[command(description = "disable monitoring")]
    DisableMonitoring,
    #[command(description = "get monitoring status")]
    GetMonitoring,
    #[command(description = "get invite code")]
    GetInvite,
    #[command(description = "accept invite code")]
    AcceptInvite,
    #[command(description = "get secondary owners")]
    GetSecondaryOwners,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum BotDialogState {
    #[default]
    Idle,
    WaitingEmergencyText,
    WaitingForInvite,
}

type BotDialogue = Dialogue<BotDialogState, InMemStorage<BotDialogState>>;

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

async fn print_start_info(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    let username = message
        .from()
        .and_then(|user| user.username.clone())
        .unwrap_or("Unknown".to_string());

    let new_profile = profiles::ActiveModel {
        chat_id: ActiveValue::Set(message.chat.id.0),
        username: ActiveValue::Set(username),
        ..Default::default()
    };
    profiles::Entity::insert(new_profile)
        .on_conflict(
            OnConflict::column(profiles::Column::ChatId)
                .update_column(profiles::Column::Username)
                .to_owned(),
        )
        .exec(&connection)
        .await?;

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

async fn set_emergency_info(
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
    emergency_info::Entity::insert(new_emergency_info)
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
    let emergency_info = emergency_info::Entity::find()
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

async fn im_ok(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    alive_events::Entity::insert(alive_events::ActiveModel {
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

async fn enable_monitoring(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    monitoring_statuses::Entity::insert(monitoring_statuses::ActiveModel {
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

async fn disable_monitoring(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    monitoring_statuses::Entity::insert(monitoring_statuses::ActiveModel {
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

async fn get_monitoring(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    let monitoring_status = monitoring_statuses::Entity::find()
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

async fn get_invite_code(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    let invite = match invites::Entity::find()
        .filter(invites::Column::ChatId.eq(message.chat.id.0))
        .one(&connection)
        .await?
    {
        Some(invite) => invite,
        None => {
            let invite_code = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect();

            let invite = invites::ActiveModel {
                chat_id: ActiveValue::Set(message.chat.id.0),
                invite: ActiveValue::Set(invite_code),
                ..Default::default()
            };

            invite.insert(&connection).await?
        }
    };

    bot.send_message(message.chat.id, invite.invite).await?;
    Ok(())
}

async fn ask_for_invite(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.update(BotDialogState::WaitingForInvite).await?;

    bot.send_message(message.chat.id, "Please enter invite code.")
        .await?;
    Ok(())
}

async fn accept_invite(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    let invite_code = message.text().unwrap_or("").to_string();
    let invite = invites::Entity::find()
        .filter(invites::Column::Invite.eq(invite_code))
        .one(&connection)
        .await
        .ok()
        .flatten();

    if invite.is_none() {
        bot.send_message(message.chat.id, "Invalid invite code.")
            .await?;
        return Ok(());
    }
    let invite = invite.unwrap();

    secondary_owners::Entity::insert(secondary_owners::ActiveModel {
        primary_owner_chat_id: ActiveValue::Set(invite.chat_id),
        secondary_owner_chat_id: ActiveValue::Set(message.chat.id.0),
        ..Default::default()
    })
    .exec(&connection)
    .await?;

    bot.send_message(message.chat.id, "Accepted!").await?;
    Ok(())
}

async fn get_secondary_owners(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
) -> HandlerResult {
    dialogue.exit().await?;

    let profiles = profiles::Entity::find()
        .join_rev(
            JoinType::InnerJoin,
            secondary_owners::Entity::belongs_to(profiles::Entity)
                .from(secondary_owners::Column::SecondaryOwnerChatId)
                .to(profiles::Column::ChatId)
                .into(),
        )
        .filter(secondary_owners::Column::PrimaryOwnerChatId.eq(message.chat.id.0))
        .all(&connection)
        .await?;

    let formatted_profiles = profiles
        .iter()
        .map(|profile| format!("@{}", profile.username.clone()))
        .collect::<Vec<_>>()
        .join("\n");

    bot.send_message(message.chat.id, formatted_profiles)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting...");
    let config = Config::init();
    log::info!("Initialized config...");

    let _guard = sentry::init((
        config.sentry_url,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

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
            dptree::filter(|command| matches!(command, Some(Command::SetEmergencyText)))
                .endpoint(ask_for_emergency_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::GetEmergencyText)))
                .endpoint(get_emergency_info),
        )
        .branch(dptree::filter(|command| matches!(command, Some(Command::ImOk))).endpoint(im_ok))
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::EnableMonitoring)))
                .endpoint(enable_monitoring),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::DisableMonitoring)))
                .endpoint(disable_monitoring),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::GetMonitoring)))
                .endpoint(get_monitoring),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::GetInvite)))
                .endpoint(get_invite_code),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::AcceptInvite)))
                .endpoint(ask_for_invite),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::GetSecondaryOwners)))
                .endpoint(get_secondary_owners),
        )
        // Dialogs
        .branch(
            dptree::filter(|state: BotDialogState| {
                matches!(state, BotDialogState::WaitingEmergencyText)
            })
            .endpoint(set_emergency_info),
        )
        .branch(
            dptree::filter(|state: BotDialogState| {
                matches!(state, BotDialogState::WaitingForInvite)
            })
            .endpoint(accept_invite),
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
