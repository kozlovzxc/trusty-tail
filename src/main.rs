use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, JoinType,
    QueryFilter, QuerySelect,
};
use std::error::Error;
use std::fmt::Debug;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;
use trusty_tail::config::Config;
use trusty_tail::{connection, entity::*};

#[derive(BotCommands, Clone, PartialEq, Eq)]
#[command(rename_rule = "snake_case", description = "Поддерживаются команды:")]
enum Command {
    Start,
    #[command(description = "Показать доступные команды")]
    Help,
    #[command(description = "Обновить текст на экстренный случай")]
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum BotDialogState {
    #[default]
    Idle,
    WaitingEmergencyText,
    WaitingForInvite,
}

type BotDialogue = Dialogue<BotDialogState, InMemStorage<BotDialogState>>;

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

const COMMAND_START_TEMPLATE: &str = "Привет 👋 Этот бот создан для заботы о питомцах, если с основным владельцем что-то случилось.

<strong>Для владельцев питомцев:</strong>
Время от времени, бот будет просить подтвердить, что с вами все в порядке. Если вы не сможете ответить несколько дней подряд, то мы оповестим ваши резервные контакты.

Для того, чтобы бот начал работать, задайте текст на экстренный случай с помощью команды /set_emergency_text и пригласите резервные контакты с помощью /get_invite.

<strong>Для резервных контактов:</strong>
Вам нужно лишь принять приглашение от владельца питомца с помощью команды /accept_invite. В случае, если владелец питомца не отвечает на запросы бота, вы получите уведомление.

Таким образом, за питомцем всегда присмотрят 🐶";

async fn print_start_info(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.exit().await?;

    bot.parse_mode(ParseMode::Html)
        .send_message(message.chat.id, COMMAND_START_TEMPLATE)
        .await?;

    Ok(())
}

async fn print_help_info(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(message.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

const COMMAND_ASK_FOR_INFO_TEMPLATE: &str = "Эта команда поможет вам настроить текст экстренного сообщения, который будет отправлен вашему резервному контакту, если вы не отвечаете в течение нескольких дней. Это важно, чтобы кто-то мог позаботиться о вашем питомце, если с вами что-то случится.

Пожалуйста, предоставьте следующую информацию:

1️⃣ Доступ к вашему дому: Как ваш резервный контакт может попасть в ваш дом, чтобы заботиться о вашем питомце? Это может быть телефон родственника, арендодателя или информация о ключе.

2️⃣ Документы на питомца: Где ваш резервный контакт может найти все необходимые документы на вашего питомца?

3️⃣ Здоровье питомца: Есть ли у вашего питомца какие-либо заболевания или особые потребности в уходе, о которых должен знать ваш резервный контакт?

4️⃣ Рекомендованная диета: Какую еду предпочитает ваш питомец и есть ли у него какие-либо диетические ограничения?

5️⃣ Особые инструкции: Есть ли какие-либо особые инструкции по уходу за вашим питомцем, которые должен знать ваш резервный контакт? Это может включать в себя информацию о прогулках, любимых игрушках, способах успокоения и т.д.

6️⃣ Ветеринар: Контактные данные вашего ветеринара, на случай, если питомцу потребуется медицинская помощь.";

async fn ask_for_emergency_info(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
) -> HandlerResult {
    dialogue
        .update(BotDialogState::WaitingEmergencyText)
        .await?;
    bot.send_message(message.chat.id, COMMAND_ASK_FOR_INFO_TEMPLATE)
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

async fn im_ok(bot: Bot, message: Message, dialogue: BotDialogue) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(message.chat.id, "Хорошего дня, все отметили")
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

    bot.send_message(message.chat.id, "Мониторинг включен")
        .await?;
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

    bot.send_message(message.chat.id, "Мониторинг выключен")
        .await?;
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
                format!(
                    "Статус мониторинга: {}",
                    if monitoring_status.enabled {
                        "Включен"
                    } else {
                        "Выключен"
                    }
                ),
            )
            .await?;
        }
        None => {
            bot.send_message(message.chat.id, "Мониторинг не задан")
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
        bot.send_message(message.chat.id, "Неизвестный код приглашения.")
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

    bot.send_message(message.chat.id, "Принято!").await?;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting...");
    let config = Config::init();

    let _guard = sentry::init((
        config.sentry_url,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let connection = connection::init().await?;

    let bot = Bot::from_env();

    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<BotDialogState>, BotDialogState>()
        .map(|message: Message| {
            let text = message.text().unwrap_or_default();
            Command::parse(&text, "").ok()
        })
        .map_async(|dialogue: BotDialogue| async move { dialogue.get().await.ok().flatten() })
        // Middleware
        .inspect_async(mark_alive)
        .inspect_async(update_profile)
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
