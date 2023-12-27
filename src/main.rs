mod config;

use config::Config;
use entity::{prelude::*, *};
use log::error;
use migration::Migrator;
use sea_orm::{ActiveValue, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm_migration::prelude::*;
use std::error::Error;
use std::sync::Arc;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};

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
    UpdateEmergencyText,
    #[command(description = "Get text regarding what to do in case of emergency")]
    GetEmergencyText,
}

#[derive(Clone, Default)]
enum DialogState {
    #[default]
    Start,
    AwaitingAge,
    AwaitingName,
    AwaitingCity,
}

type MyDialogue = Dialogue<DialogState, InMemStorage<DialogState>>;

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
    log::info!("Connected to dabase...");
    let schema_manager = SchemaManager::new(&connection);
    Migrator::up(&connection, None).await?;
    assert!(schema_manager.has_table("emergency_info").await?);
    log::info!("Applied migrations...");

    let bot = Bot::from_env();
    let connection = Arc::new(connection);

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_map(|msg: Message| Command::parse(&msg.text().unwrap_or_default(), "").ok())
                .branch(dptree::case![Command::Start].endpoint(
                    |bot: Bot, msg: Message| async move {
                        bot.send_message(msg.chat.id, "Start!").await?;
                        respond(())
                    },
                ))
                .branch(dptree::case![Command::Help].endpoint(
                    |bot: Bot, msg: Message| async move {
                        bot.send_message(msg.chat.id, "Help!").await?;
                        respond(())
                    },
                ))
                .branch(
                    dptree::case![Command::UpdateEmergencyText]
                        .enter_dialogue::<Message, InMemStorage<DialogState>, DialogState>()
                        .endpoint(|bot: Bot, msg: Message, state: MyDialogue| async move {
                            let res = state.update(DialogState::Start).await;
                            if res.is_err() {
                                panic!("{:?}", res.err())
                            }
                            bot.send_message(msg.chat.id, "UpdateEmergencyText!")
                                .await?;
                            respond(())
                        }),
                ),
        )
        .branch(
            dptree::entry()
                .enter_dialogue::<Message, InMemStorage<DialogState>, DialogState>()
                .filter_async(|state: MyDialogue| async move { state.get().await.is_ok() })
                .branch(dptree::case![DialogState::Start].endpoint(
                    |bot: Bot, state: MyDialogue, msg: Message| async move {
                        let _ = state.update(DialogState::AwaitingName).await;
                        bot.send_message(msg.chat.id, "Awaiting name!").await?;
                        respond(())
                    },
                ))
                .branch(dptree::case![DialogState::AwaitingName].endpoint(
                    |bot: Bot, state: MyDialogue, msg: Message| async move {
                        let _ = state.update(DialogState::AwaitingAge).await;
                        bot.send_message(msg.chat.id, "Awaiting Age!").await?;
                        respond(())
                    },
                ))
                .branch(dptree::case![DialogState::AwaitingAge].endpoint(
                    |bot: Bot, state: MyDialogue, msg: Message| async move {
                        let _ = state.exit().await;
                        bot.send_message(msg.chat.id, "Finished!").await?;
                        respond(())
                    },
                )),
        )
        .endpoint(|bot: Bot, msg: Message| async move {
            bot.send_message(msg.chat.id, "Unknown!").await?;
            respond(())
        });

    log::info!("Started listening...");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<DialogState>::new()])
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
//         Command::Start | Command::Help => {
//             let _ = bot
//                 .send_message(message.chat.id, Command::descriptions().to_string())
//                 .await;
//         }
//         Command::UpdateEmergencyText => {
//             let _ = dialogue.update(DialogState::Start).await;

//             // let new_emergency_info = emergency_info::ActiveModel {
//             //     text: ActiveValue::Set(emergency_text),
//             //     chat_id: ActiveValue::Set(msg.chat.id.0),
//             //     ..Default::default()
//             // };
//             // let res = EmergencyInfo::insert(new_emergency_info)
//             //     .on_conflict(
//             //         OnConflict::column(emergency_info::Column::ChatId)
//             //             .update_column(emergency_info::Column::Text)
//             //             .to_owned()
//             //     )
//             //     .exec(connection)
//             //     .await;

//             // match res {
//             //     Ok(_) => bot.send_message(msg.chat.id, format!("Added!")).await?,
//             //     Err(err) => {
//             //         error!("Can't add an entry: {:?}", err);
//             //         bot.send_message(msg.chat.id, format!("Can't update!"))
//             //             .await?
//             //     }
//             // }
//         }
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
