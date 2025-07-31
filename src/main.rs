mod config;
mod shared;
mod sticker;
mod download;

use std::sync::Arc;

use crate::config::BotConfig;

use teloxide::prelude::*;
use teloxide::dispatching::UpdateHandler;
use teloxide::utils::command::BotCommands;

use crate::shared::{ChatStateStorage, SharedData};

#[tokio::main]
async fn main() {
    let env = env_logger::Env::new().default_filter_or("info");
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

    if let Err(e) = bot.set_my_commands(CommandList::bot_commands()).await {
        log::warn!("Failed to set commands for bot: {}", e);
    }

    Dispatcher::builder(bot, update_handler())
    .dependencies(dptree::deps![arc_shared])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

#[derive(BotCommands, PartialEq, Clone, Debug)]
#[command(rename_rule = "snake_case")]
enum CommandList {
    #[command(description = "顯示幫助信息")]
    Help,
    #[command(description = "退出當前的功能")]
    Exit,
    #[command(description = "轉換貼紙、圖片和動圖")]
    StickerConvert,
    #[command(description = "下載貼紙包")]
    StickerSetDownload
}

#[derive(BotCommands, PartialEq, Clone, Debug)]
#[command(rename_rule = "snake_case")]
enum BasicCommand {
    Exit, // Remove state
    Help, // Help
    Start, // Start
}

const HELP_MSG : &'static str = 
"這裡是薄荷茶～ 目前支持這些功能\n\
- /help : 顯示幫助信息\n\
\n\
貼紙轉換和貼紙下載\n\
/sticker_convert : 轉換貼紙、圖片和動圖\n\
/sticker_set_download : 下載貼紙包";

async fn basic_command_processor(
    shared: Arc<SharedData>,
    bot: Bot,
    msg: Message,
    command: BasicCommand
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match command {
        BasicCommand::Exit => {
            shared.chat_state_storage.release_state(msg.chat.id).await;
            // bot.send_message(msg.chat.id, "").await?;
        },
        BasicCommand::Help |
        BasicCommand::Start => {
            bot.send_message(msg.chat.id, HELP_MSG).await?;
        }
    }
    Ok(())
}

fn update_handler() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    Update::filter_message()
    .branch(
        dptree::entry()
        .filter_command::<BasicCommand>()
        .endpoint(basic_command_processor)
    )
    .branch(
        sticker::sticker_handler()
    )
}
