mod config;

use config::Config;
use entity::{prelude::*, *};
use migration::Migrator;
use sea_orm::{ActiveValue, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm_migration::prelude::*;
use std::error::Error;
use std::fmt::{self, Debug};
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    Start,
    #[command(description = "display this text.")]
    Help,
    TestError,
    #[command(description = "Add text regarding what to do in case of emergency")]
    UpdateEmergencyText,
    #[command(description = "Get text regarding what to do in case of emergency")]
    GetEmergencyText,
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

#[derive(Debug)]
pub enum TestError {
    SimpleError(String),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TestError::SimpleError(ref msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for TestError {}

async fn trigger_error() -> HandlerResult {
    Err(Box::new(TestError::SimpleError("Simple error".into())))
}

async fn print_help_info(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(message.chat.id, "Help!").await?;
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
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::Start)))
                .endpoint(print_start_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::TestError)))
                .endpoint(trigger_error),
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
            dptree::filter(|state: BotDialogState| {
                matches!(state, BotDialogState::WaitingEmergencyText)
            })
            .endpoint(update_emergency_info),
        )
        .branch(
            dptree::filter(|command| matches!(command, Some(Command::GetEmergencyText)))
                .endpoint(get_emergency_info),
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

// async fn command_handler(
//     bot: &Bot,
//     message: &Message,
//     command: Command,
//     connection: Arc<DatabaseConnection>,
//     dialogue: Dialogue<DialogState, InMemStorage<DialogState>>,
// ) {
//     let connection = &*connection;

//     match command {
//         Command::GetEmergencyText => {
//             let res = EmergencyInfo::find()
//                 .filter(emergency_info::Column::ChatId.eq(message.chat.id.0))
//                 .one(connection)
//                 .await;

//             if res.is_err() {
//                 error!("Can't read an entry: {:?}", res.err());
//                 let _ = bot
//                     .send_message(message.chat.id, format!("Can't read!"))
//                     .await;
//                 return;
//             }

//             let res = res.unwrap();
//             if res.is_none() {
//                 let _ = bot
//                     .send_message(message.chat.id, format!("Can't find an entry!"))
//                     .await;
//                 return;
//             }

//             let res = res.unwrap();

//             let _ = bot
//                 .send_message(
//                     message.chat.id,
//                     format!("Current emergency text:\n{}", res.text),
//                 )
//                 .await;
//         }
//     };
// }
