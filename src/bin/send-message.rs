use sea_orm::{Database, EntityTrait, PaginatorTrait};
use std::{
    error::Error,
    io::{self, Read},
};
use teloxide::{requests::Requester, types::ChatId, Bot};
use trusty_tail::config::Config;
use trusty_tail::entity::profiles;

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

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read input");

    let input = input.trim();

    if input.is_empty() {
        log::error!("Input is empty");
        return Ok(());
    }

    let mut profile_pages = profiles::Entity::find().paginate(&connection, 50);

    while let Some(profiles) = profile_pages.fetch_and_next().await? {
        for profile in profiles {
            let _ = bot.send_message(ChatId(profile.chat_id), input).await;
        }
    }

    log::info!("Finished!");

    Ok(())
}
