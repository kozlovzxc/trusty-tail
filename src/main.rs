use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};
use std::error::Error;
use teloxide::dispatching::dialogue::{GetChatId, InMemStorage};
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tera::Tera;
use trusty_tail::commands::alive::mark_alive_callback;
use trusty_tail::commands::contact_menu::show_contact_menu;
use trusty_tail::commands::emergency_info::{
    ask_for_emergency_info, set_emergency_info, show_emergency_info,
};
use trusty_tail::commands::invites::{accept_invite, ask_for_invite};
use trusty_tail::commands::owner_menu::{
    handle_disable_monitoring, handle_enable_monitoring, show_owner_menu,
};
use trusty_tail::commands::start::show_start_info;
use trusty_tail::config::Config;
use trusty_tail::types::{BotDialogState, BotDialogue};
use trusty_tail::{connection, entity::*};

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(rename_rule = "snake_case")]
enum MessageCommand {
    Start,
    // Legacy
    Menu,
    OwnerMenu,
    ContactMenu,
}

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(rename_rule = "snake_case")]
enum CallbackCommand {
    Enable,
    Disable,
    EmergencyInfo,
    AskForEmergencyInfo,
    OwnerMenu,
    ContactMenu,
    AskForInvite,
    MarkAlive,
}

async fn update_profile_middleware(message: Message, connection: DatabaseConnection) {
    let username = message
        .from()
        .and_then(|user| user.username.clone())
        .unwrap_or("Unknown".to_string());

    let _ = profiles::Entity::insert(profiles::ActiveModel {
        chat_id: ActiveValue::Set(message.chat.id.0),
        username: ActiveValue::Set(username),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(profiles::Column::ChatId)
            .update_column(profiles::Column::Username)
            .to_owned(),
    )
    .exec(&connection)
    .await
    .unwrap();
}

async fn callback_handler(
    bot: Bot,
    query: CallbackQuery,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
    tera: Tera,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = match query.chat_id() {
        Some(chat_id) => chat_id,
        None => return Err("No chat id".into()),
    };

    let message_id = match query.message.map(|x| x.id) {
        Some(message_id) => message_id,
        None => return Err("No message id".into()),
    };

    let command = match query
        .data
        .map(|x| CallbackCommand::parse(&x, "").ok())
        .flatten()
    {
        Some(command) => command,
        None => {
            bot.send_message(chat_id, "Команда не найдена").await?;
            show_owner_menu(&bot, chat_id, &connection, &tera).await?;
            return Err("Unknown command".into());
        }
    };

    let next_state = match command {
        CallbackCommand::Enable => {
            handle_enable_monitoring(&bot, chat_id, message_id, &connection).await?
        }
        CallbackCommand::Disable => {
            handle_disable_monitoring(&bot, chat_id, message_id, &connection).await?
        }
        CallbackCommand::EmergencyInfo => {
            show_emergency_info(&bot, chat_id, &connection, &tera).await?
        }
        CallbackCommand::AskForEmergencyInfo => {
            ask_for_emergency_info(&bot, chat_id, &tera).await?
        }
        CallbackCommand::OwnerMenu => show_owner_menu(&bot, chat_id, &connection, &tera).await?,
        CallbackCommand::ContactMenu => {
            show_contact_menu(&bot, chat_id, &connection, &tera).await?
        }
        CallbackCommand::AskForInvite => ask_for_invite(&bot, chat_id).await?,
        CallbackCommand::MarkAlive => {
            mark_alive_callback(&bot, chat_id, message_id, &connection).await?
        }
    };

    // Update state
    if let Some(next_state) = next_state {
        dialogue.update(next_state).await?;
    } else {
        dialogue.exit().await?;
    }

    Ok(())
}

async fn message_handler(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
    tera: Tera,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let text = message.text().unwrap_or_default();
    let command = MessageCommand::parse(&text, "").ok();

    // Match command first
    let next_state = if let Some(command) = command {
        match command {
            MessageCommand::Start => show_start_info(&bot, &message, &connection, &tera).await?,
            MessageCommand::Menu | MessageCommand::OwnerMenu => {
                show_owner_menu(&bot, message.chat.id, &connection, &tera).await?
            }
            MessageCommand::ContactMenu => {
                show_contact_menu(&bot, message.chat.id, &connection, &tera).await?
            }
        }
    // Match state second
    } else if let Some(state) = dialogue.get().await.ok().flatten() {
        match state {
            BotDialogState::WaitingEmergencyText => {
                set_emergency_info(&message, &connection).await?;
                show_emergency_info(&bot, message.chat.id, &connection, &tera).await?
            }
            BotDialogState::WaitingForInvite => {
                accept_invite(&bot, &message, &connection).await?;
                show_contact_menu(&bot, message.chat.id, &connection, &tera).await?
            }
            BotDialogState::Idle => {
                bot.send_message(message.chat.id, "Команда не найдена")
                    .await?;
                None
            }
        }
    } else {
        None
    };

    // Update state
    if let Some(next_state) = next_state {
        dialogue.update(next_state).await?;
    } else {
        dialogue.exit().await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting...");
    let config = Config::init();

    let tera = match Tera::new("templates/**/*") {
        Ok(tera) => tera,
        Err(message) => panic!("Tera error: {}", message),
    };

    let _guard = sentry::init((
        config.sentry_url,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let connection = connection::init().await?;

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<BotDialogState>, BotDialogState>()
                .inspect_async(update_profile_middleware)
                .endpoint(message_handler),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<CallbackQuery, InMemStorage<BotDialogState>, BotDialogState>()
                .endpoint(callback_handler),
        );

    log::info!("Started listening...");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            InMemStorage::<BotDialogState>::new(),
            connection,
            tera
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
