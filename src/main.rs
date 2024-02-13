use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, JoinType,
    QueryFilter, QuerySelect,
};
use std::error::Error;
use teloxide::dispatching::dialogue::{GetChatId, InMemStorage};
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tera::Tera;
use trusty_tail::commands::disable::disable_monitoring;
use trusty_tail::commands::enable::enable_monitoring;
use trusty_tail::commands::start::start_command;
use trusty_tail::config::Config;
use trusty_tail::types::{BotDialogState, BotDialogue};
use trusty_tail::{connection, entity::*};

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(rename_rule = "snake_case")]
enum MessageCommand {
    Start,
    Menu,
    #[command(description = "Показать доступные команды")]
    SetEmergencyText,
    #[command(description = "Показать текст на экстренный случай")]
    GetEmergencyText,
    #[command(description = "Отметиться, что все хорошо")]
    ImOk,
    #[command(description = "Включить мониторинг")]
    EnableMonitoring,
    #[command(description = "Выключить мониторинг")]
    DisableMonitoring,
    #[command(description = "Получить статус мониторинга")]
    GetMonitoring,
    #[command(description = "Получить код для приглашения экстренного контакта")]
    GetInvite,
    #[command(description = "Принять приглашение экстренного контакта")]
    AcceptInvite,
    #[command(description = "Показать экстренные контакты")]
    GetSecondaryOwners,
}

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(rename_rule = "snake_case")]
enum CallbackCommand {
    Enable,
    Disable,
}

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

async fn ask_for_emergency_info(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    tera: Tera,
) -> HandlerResult {
    dialogue
        .update(BotDialogState::WaitingEmergencyText)
        .await?;

    let context = tera::Context::new();
    let answer = tera.render("emergency_info_fill.html", &context).unwrap();

    bot.send_message(message.chat.id, answer).await?;
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

    bot.send_message(message.chat.id, "Текст на экстренный случай обновлен")
        .await?;
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
            bot.send_message(message.chat.id, "Не нашел текст на экстренный случай 🤷")
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
    bot.send_message(message.chat.id, "Пожалуйста введите код приглашения.")
        .await?;
    Ok(())
}

async fn accept_invite(
    bot: &Bot,
    message: &Message,
    connection: &DatabaseConnection,
) -> Result<Option<BotDialogState>, Box<dyn Error + Send + Sync>> {
    let invite_code = message.text().unwrap_or("").to_string();
    let invite = invites::Entity::find()
        .filter(invites::Column::Invite.eq(invite_code))
        .one(connection)
        .await
        .ok()
        .flatten();

    if invite.is_none() {
        bot.send_message(message.chat.id, "Неизвестный код приглашения.")
            .await?;
        return Ok(None);
    }
    let invite = invite.unwrap();

    secondary_owners::Entity::insert(secondary_owners::ActiveModel {
        primary_owner_chat_id: ActiveValue::Set(invite.chat_id),
        secondary_owner_chat_id: ActiveValue::Set(message.chat.id.0),
        ..Default::default()
    })
    .exec(connection)
    .await?;

    bot.send_message(message.chat.id, "Принято!").await?;
    Ok(None)
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

    if profiles.is_empty() {
        bot.send_message(message.chat.id, "Нет резервных контактов")
            .await?;
    } else {
        let formatted_profiles = profiles
            .iter()
            .map(|profile| format!("@{}", profile.username.clone()))
            .collect::<Vec<_>>()
            .join("\n");

        bot.send_message(message.chat.id, formatted_profiles)
            .await?;
    }
    Ok(())
}

async fn mark_alive(message: Message, connection: DatabaseConnection) {
    let _ = alive_events::Entity::insert(alive_events::ActiveModel {
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
    .await
    .unwrap();
}

async fn update_profile(message: Message, connection: DatabaseConnection) {
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
    connection: DatabaseConnection,
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
        None => return Err("Unknown command".into()),
    };

    match command {
        CallbackCommand::Enable => {
            enable_monitoring(&bot, chat_id, message_id, &connection).await?
        }
        CallbackCommand::Disable => {
            disable_monitoring(&bot, chat_id, message_id, &connection).await?
        }
        _ => (),
    }

    Ok(())
}

async fn message_handler(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
    connection: DatabaseConnection,
    tera: Tera,
) -> HandlerResult {
    let text = message.text().unwrap_or_default();
    let command = MessageCommand::parse(&text, "").ok();

    // Match command first
    let next_state = if let Some(command) = command {
        match command {
            MessageCommand::Start => start_command(&bot, &message, &tera, &connection).await?,
            _ => None,
        }
    // Match state second
    } else if let Some(state) = dialogue.get().await.ok().flatten() {
        match state {
            BotDialogState::WaitingForInvite => accept_invite(&bot, &message, &connection).await?,
            _ => None,
        }
    // Default handler
    } else {
        bot.send_message(message.chat.id, "Команда не найдена")
            .await?;
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
                .inspect_async(mark_alive)
                .inspect_async(update_profile)
                .endpoint(message_handler),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler));
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::SetEmergencyText)))
    //         .endpoint(ask_for_emergency_info),
    // )
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::GetEmergencyText)))
    //         .endpoint(get_emergency_info),
    // )
    // .branch(dptree::filter(|command| matches!(command, Some(Command::ImOk))).endpoint(im_ok))
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::EnableMonitoring)))
    //         .endpoint(enable_monitoring),
    // )
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::DisableMonitoring)))
    //         .endpoint(disable_monitoring),
    // )
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::GetMonitoring)))
    //         .endpoint(get_monitoring),
    // )
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::GetInvite)))
    //         .endpoint(get_invite_code),
    // )
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::AcceptInvite)))
    //         .endpoint(ask_for_invite),
    // )
    // .branch(
    //     dptree::filter(|command| matches!(command, Some(Command::GetSecondaryOwners)))
    //         .endpoint(get_secondary_owners),
    // )
    // // Dialogs
    // .branch(
    //     dptree::filter(|state: BotDialogState| {
    //         matches!(state, BotDialogState::WaitingEmergencyText)
    //     })
    //     .endpoint(set_emergency_info),
    // )
    // .branch(
    //     dptree::filter(|state: BotDialogState| {
    //         matches!(state, BotDialogState::WaitingForInvite)
    //     })
    //     .endpoint(accept_invite),
    // )
    // .endpoint(|bot: Bot, message: Message| async move {
    //     bot.send_message(message.chat.id, "Unknown command!")
    //         .await?;
    //     Ok(())
    // });

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
