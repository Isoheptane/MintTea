mod config;
mod helper;
mod types;
mod context;
mod handler;
mod basic_commands;

mod sticker;
mod pixiv;
mod monitor;

use std::sync::Arc;

use crate::basic_commands::{basic_command_handler};
use crate::config::BotConfig;

use crate::context::{Context, ModalState};
use crate::handler::HandlerResult;
use crate::helper::log::MessageDisplay;
use crate::helper::message_utils::get_chat_sender;
use crate::pixiv::pixiv_handler;
use crate::sticker::{sticker_handler, sticker_modal_handler};
use crate::types::ChatSender;

use futures::future::BoxFuture;
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

    let bot = Bot::new_url(format!("{}/bot{}", config.telegram.bot_api_server, config.telegram.token));

    // make temp directory
    let cur_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("Failed to get current directory: {e}");
            panic!();
        }
    };

    let temp_path = cur_dir.join("temp");
    if let Err(e) = std::fs::create_dir_all(&temp_path) {
        log::error!("Failed to create temp directory: {e}");
        panic!();
    }

    let ctx = Arc::new(Context::new(bot, config, temp_path));

    // Initialize commands 
    if let Err(e) = ctx.bot.set_my_commands(&SetMyCommandsParams::builder().commands(get_bot_commands()).build()).await {
        log::warn!(target: "init", "Failed to set commands: {e}");
    }

    log::info!("Bot initialized");

    let mut update_id: i64 = 0;
    'update_loop: loop {
        let result = match ctx.bot.get_updates(&GetUpdatesParams::builder()
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
            let ctx_clone = ctx.clone();
            tokio::spawn(async move {
                handle_update(ctx_clone, update).await;
            });
        }
    }
}

async fn handle_update(ctx: Arc<Context>, update: Update) {
    match update.content {
        frankenstein::updates::UpdateContent::Message(message) => {
            handle_message(ctx, Arc::new(*message)).await
        }
        _ => {
            log::debug!(target: "update_handler", "Ignoring unhandled type {:?}", update.content);
        }
    };
}

static HANDLERS: &[fn(Arc<Context>, Arc<Message>) -> BoxFuture<'static, HandlerResult>] = &[
    sticker_handler,
    pixiv_handler
];

async fn handle_message(ctx: Arc<Context>, msg: Arc<Message>) {

    log::debug!(
        "Chat ID: {}, From ID: {:?}, SenderChat ID: {:?}", 
        msg.chat.id, 
        msg.from.as_ref().map(|f| f.id), 
        msg.sender_chat.as_ref().map(|c| c.id)
    );
    // Print message first
    print!(
        "{}",
        MessageDisplay(&msg)
    );
    
    // Basic handler is handled prior to all handlers & routers
    match basic_command_handler(ctx.clone(), msg.clone()).await {
        Ok(std::ops::ControlFlow::Continue(_)) => {}
        Ok(std::ops::ControlFlow::Break(_)) => { return; }
        Err(e) => {
            log::error!("Handler execution failed: {e}, detail: {e:?}");
            return;
        }
    }

    // Route to modal handler
    if let Some(state) = ctx.modal_states.get_state(get_chat_sender(&msg)).await {
        let result = match state {
            ModalState::Sticker(state) => sticker_modal_handler(ctx, msg, state).await
        };
        if let Err(e) = result {
            log::error!("Modal handler execution failed: {e}, detail: {e:?}");
        }
        return;
    };

    // Normal handler
    for handler in HANDLERS {
        let result = handler(ctx.clone(), msg.clone()).await;
        let action = match result {
            Ok(action) => action,
            Err(e) => {
                log::error!("Handler execution failed: {e}, detail: {e:?}");
                return;
            }
        };
        match action {
            std::ops::ControlFlow::Continue(_) => { continue; }
            std::ops::ControlFlow::Break(_) => { return; }
        }
    }
    // log::debug!("Message is rejected by all handlers: {:?}");
}

fn get_bot_commands() -> Vec<BotCommand> {
    let commands = vec![
        basic_commands::COMMAND_LIST,
        sticker::COMMAND_LIST,
        pixiv::COMMAND_LIST,
    ].concat();
    commands.into_iter().map(|(command, desc)| 
        BotCommand::builder()
        .command(command)
        .description(desc)
        .build()  
    )
    .collect()
}