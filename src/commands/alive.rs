use std::error::Error;

use teloxide::prelude::*;

use crate::types::BotDialogue;

pub async fn im_ok(
    bot: Bot,
    message: Message,
    dialogue: BotDialogue,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    dialogue.exit().await?;

    bot.send_message(message.chat.id, "Хорошего дня, все отметили")
        .await?;

    Ok(())
}
