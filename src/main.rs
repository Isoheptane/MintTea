mod config;
mod helper;
mod types;
mod context;
mod handler;
mod basic_commands;
mod sticker;

use std::sync::Arc;

use crate::basic_commands::{basic_command_handler};
use crate::config::BotConfig;

use crate::context::{Context, ModalState};
use crate::handler::HandlerResult;
use crate::helper::message_utils::message_chat_sender;
use crate::sticker::{sticker_handler, sticker_modal_handler};

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

    let bot = Bot::new(&config.telegram.token);

    let ctx = Arc::new(Context::new(bot, config));

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
            log::debug!(target: "update_handler", "Ignoring unhandled type {}", std::any::type_name_of_val(&update.content));
        }
    };
}

static HANDLERS: &[fn(Arc<Context>, Arc<Message>) -> BoxFuture<'static, HandlerResult>] = &[
    sticker_handler
];

async fn handle_message(ctx: Arc<Context>, msg: Arc<Message>) {
    
    // Basic handler is handled prior to all handlers & routers
    match basic_command_handler(ctx.clone(), msg.clone()).await {
        Ok(std::ops::ControlFlow::Continue(_)) => {}
        Ok(std::ops::ControlFlow::Break(_)) => { return; }
        Err(e) => {
            log::error!("Handler execution failed: {e}");
            return;
        }
    }

    // Route to modal handler
    if let Some(state) = ctx.modal_states.get_state(message_chat_sender(&msg)).await {
        let result = match state {
            ModalState::Sticker(state) => sticker_modal_handler(ctx, msg, state).await
        };
        if let Err(e) = result {
            log::error!("Modal handler execution failed: {e}");
        }
        return;
    };

    // Normal handler
    for handler in HANDLERS {
        let result = handler(ctx.clone(), msg.clone()).await;
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