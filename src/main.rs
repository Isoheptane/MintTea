mod config;
mod helper;
mod types;
mod shared;
mod sticker;
mod download;


use std::sync::Arc;

use crate::config::BotConfig;

use crate::helper::{message_chat_sender, message_command};
use crate::shared::{ChatStateStorage, SharedData};
use crate::sticker::sticker_handler;

use tokio::time::{sleep, Duration};

use frankenstein::methods::{GetUpdatesParams, SendMessageParams, SetMyCommandsParams};
use frankenstein::types::{BotCommand, Message};
use frankenstein::updates::Update;
use frankenstein::AsyncTelegramApi;
use frankenstein::client_reqwest::Bot;

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

    let bot = Bot::new(&config.telegram.token.clone());

    // Initialize commands 
    if let Err(e) = bot.set_my_commands(&SetMyCommandsParams::builder().commands(get_bot_commands()).build()).await {
        log::warn!(target: "init", "Failed to set commands: {e}");
    }

    let mut update_id: i64 = 0;
    'update_loop: loop {
        let result = match bot.get_updates(&GetUpdatesParams::builder()
            .offset(update_id)
            .timeout(15)
            .build()
        ).await {
            Ok(result) => result,
            Err(e) => {
                log::error!(target: "update_loop", "Failed to get updates: {e}");
                sleep(Duration::from_secs(5)).await;
                continue 'update_loop;
            }
        }.result;
        for update in result {
            update_id = i64::max(update_id, update.update_id as i64 + 1);
            let bot_clone = bot.clone();
            let data = arc_shared.clone();
            tokio::spawn(async move {
                handle_update(bot_clone, data, update).await;
            });
        }
    }
}

async fn handle_update(bot: Bot, data: Arc<SharedData>, update: Update) {
    match update.content {
        frankenstein::updates::UpdateContent::Message(message) => {
            handle_message(bot, data, *message).await
        }
        _ => {
            log::debug!(target: "update_handler", "Ignoring unhandled type {}", std::any::type_name_of_val(&update.content));
        }
    };
}

async fn handle_message(bot: Bot, data: Arc<SharedData>, msg: Message) {
    // Try basic help
    if basic_command_handler(bot.clone(), data.clone(), &msg).await.0 {

    } else if sticker_handler(bot.clone(), data.clone(), &msg).await.0 {

    } else {
        log::warn!(target: "message_handler", "Unhandled message: {:?}", msg.text);
    }
}

async fn basic_command_handler(bot: Bot, data: Arc<SharedData>, msg: &Message) -> (bool, Option<Box<dyn std::error::Error + Send + Sync + 'static>>) {
    let command = message_command(&msg);
    if let Some(command) = command {
        match command.as_str() {
            "exit" => {
                data.chat_state_storage.release_state(message_chat_sender(&msg)).await;
                return (true, None);
            }
            "help" => {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(msg.chat.id)
                    .text(HELP_MSG)
                    .build();
                let e = bot.send_message(&send_message_params).await.map_err(Into::into);
                return (true, e.err());
            }
            _ => {
                return (false, None);
            }
        }
    }
    return (false, None);
}

fn get_bot_commands() -> Vec<BotCommand> {
    let commands = vec![
        ("help", "顯示幫助信息"),
        ("exit", "退出當前的功能"),
        ("sticker_conv", "轉換貼紙、圖片和動圖"),
        ("sticker_set_download", "下載貼紙包"),
    ];
    commands.into_iter().map(|(command, desc)| 
        BotCommand::builder()
        .command(command)
        .description(desc)
        .build()  
    )
    .collect()
}

const HELP_MSG : &'static str = 
"這裡是薄荷茶～ 目前支持這些功能\n\
- /help : 顯示幫助信息\n\
\n\
貼紙轉換和貼紙下載\n\
/sticker_convert : 轉換貼紙、圖片和動圖\n\
/sticker_set_download : 下載貼紙包";
