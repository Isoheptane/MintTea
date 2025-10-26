mod config;
mod helper;
mod types;
mod shared;
mod sticker;
mod download;
mod basic_commands;
mod handler;

use std::sync::Arc;

use crate::basic_commands::basic_command_handler;
use crate::config::BotConfig;

use crate::shared::{ChatStateStorage, SharedData};
use crate::sticker::sticker_handler;

use tokio::time::{sleep, Duration};

use frankenstein::methods::{GetUpdatesParams, SetMyCommandsParams};
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
            handle_message(&bot, &data, &*message).await
        }
        _ => {
            log::debug!(target: "update_handler", "Ignoring unhandled type {}", std::any::type_name_of_val(&update.content));
        }
    };
}

async fn handle_message(bot: &Bot, data: &Arc<SharedData>, msg: &Message) {
    let handlers = vec![
        basic_command_handler(&bot, &data, &msg),
        sticker_handler(&bot, &data, &msg)
    ];

    for handler in handlers {
        let result = handler.await;
        let action = match result {
            Ok(action) => action,
            Err(e) => {
                log::error!("Handler execution failed: {e}");
                return;
            }
        };
        match action {
            std::ops::ControlFlow::Continue(_) => { continue; }
            std::ops::ControlFlow::Break(_) => { return; }
        }
    }
    log::warn!("Message is rejected by all handlers: {:?}", msg.text);
}

fn get_bot_commands() -> Vec<BotCommand> {
    let commands = vec![
        basic_commands::COMMAND_LIST,
        sticker::COMMAND_LIST,
    ].concat();
    commands.into_iter().map(|(command, desc)| 
        BotCommand::builder()
        .command(command)
        .description(desc)
        .build()  
    )
    .collect()
}