mod config;
mod shared;
mod sticker;

use std::sync::Arc;

use crate::config::BotConfig;

use teloxide::prelude::*;
use teloxide::dispatching::UpdateHandler;
use teloxide::utils::command::BotCommands;

use crate::shared::{ChatStateStorage, SharedData};

#[tokio::main]
async fn main() {
    let env = env_logger::Env::new().default_filter_or("debug");
    env_logger::init_from_env(env);

    let config = match BotConfig::read_config("config.json") {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to load config: {}", e);
            panic!()
        }
    };

    let shared = SharedData {
        config: config.clone(), 
        chat_state_storage: ChatStateStorage::default()
    };

    let arc_shared = Arc::new(shared);

    let bot = Bot::new(config.telegram.token.clone());

    Dispatcher::builder(bot, update_handler())
    .dependencies(dptree::deps![arc_shared])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

#[derive(BotCommands, PartialEq, Clone, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
enum ExitCommand {
    Exit, // Sticker to Picture
}

async fn exit_handler(
    shared: Arc<SharedData>,
    bot: Bot,
    msg: Message,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    shared.chat_state_storage.release_state(msg.chat.id).await;

    bot.send_message(msg.chat.id, "退出").await?;
    Ok(())
}

fn update_handler() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    Update::filter_message()
    .branch(
        dptree::entry()
        .filter_command::<ExitCommand>()
        .endpoint(exit_handler)
    )
    .branch(
        sticker::sticker_handler()
    )
}
