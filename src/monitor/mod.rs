pub mod context;

mod rules;

use std::sync::Arc;

use frankenstein::methods::{CopyMessageParams, ForwardMessageParams};
use frankenstein::{AsyncTelegramApi, methods::SendMessageParams};
use frankenstein::types::{ChatShared, KeyboardButton, KeyboardButtonRequestChat, KeyboardButtonRequestUsers, Message, ReplyKeyboardMarkup, ReplyMarkup, SharedUser};
use futures::future::BoxFuture;
use uuid::Uuid;

use crate::helper::{bot_actions, param_builders};
use crate::helper::message_utils::{get_chat_sender, get_command};
use crate::handler::{HandlerResult, ModalHandlerResult};
use crate::context::{Context, ModalState};
use crate::helper::log::LogOp;
use crate::monitor::rules::{FilterRule, MonitorRule};

#[derive(Debug, PartialEq, Clone)]
pub enum MonitorModalState {
    SendUser,
    SendChat(Option<SharedUser>),
    SendKeyword(Option<SharedUser>, Option<ChatShared>),
}

pub fn monitor_command_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_command_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_command_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {
    let command = get_command(&msg);
    let Some(command) = command else {
        return Ok(std::ops::ControlFlow::Continue(()));
    };
    if command != "monitor" {
        return Ok(std::ops::ControlFlow::Continue(()));
    }

    monitor_into_modal(ctx, msg).await?;
    
    Ok(std::ops::ControlFlow::Continue(()))
}

async fn monitor_into_modal(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {

    log::debug!("{} Going into monitor setup modal", LogOp(&msg));

    let button_req_user = KeyboardButtonRequestUsers::builder()
        .request_id(0)
        .request_name(true)
        .build();
    let button = KeyboardButton::builder()
        .request_users(button_req_user)
        .text("選擇用戶")
        .build();
    let markup = ReplyKeyboardMarkup::builder()
        .keyboard(vec![vec![button]])
        .one_time_keyboard(true)
        .resize_keyboard(true)
        .build();

    let param = SendMessageParams::builder()
        .chat_id(msg.chat.id)
        .text("請選擇一個要監視的用戶～\n如果不需要根據用戶篩選的話，請發送任意消息。")
        .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(markup))
        .build();

    ctx.bot.send_message(&param).await?;
    
    ctx.modal_states.set_state(get_chat_sender(&msg), ModalState::Monitor(MonitorModalState::SendUser)).await;

    Ok(())
}

pub async fn monitor_modal_handler(ctx: Arc<Context>, msg: Arc<Message>, state: MonitorModalState) -> ModalHandlerResult {

    match state {
        MonitorModalState::SendUser => {

            log::debug!(
                target: "monitor_add_modal", "{} Received (optional) shared user", 
                LogOp(&msg)
            );

            let shared_user = match msg.users_shared.as_ref() {
                Some(shared) => shared.users.get(0).cloned(),
                None => None
            };
            let next_state = MonitorModalState::SendChat(shared_user);
            
            let button_req_chat = KeyboardButtonRequestChat::builder()
                .request_id(0)
                .chat_is_channel(false)
                .request_title(true)
                .build();

            let button = KeyboardButton::builder()
                .request_chat(button_req_chat)
                .text("選擇群組")
                .build();

            let markup = ReplyKeyboardMarkup::builder()
                .keyboard(vec![vec![button]])
                .one_time_keyboard(true)
                .resize_keyboard(true)
                .build();

            let param = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text("請選擇一個要監視的群組～\n如果不需要根據群組篩選的話，請發送任意消息。")
                .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(markup))
                .build();

            ctx.bot.send_message(&param).await?;
    
            ctx.modal_states.set_state(
                get_chat_sender(&msg), 
                ModalState::Monitor(next_state)
            ).await;

        },
        MonitorModalState::SendChat(shared_user) => {

            log::debug!(
                target: "monitor_add_modal", "{} Received (optional) shared chat", 
                LogOp(&msg)
            );

            let shared_chat = match msg.chat_shared.as_ref() {
                Some(shared) => Some(*shared.to_owned()),
                None => None
            };

            // Check rules, at least one should be defined
            if shared_user.is_none() && shared_chat.is_none() {
                bot_actions::send_message(&ctx.bot, msg.chat.id, "監視的用戶和要監視的群組需要指定至少一個——").await?;
                // Remove modal
                ctx.modal_states.release_state(get_chat_sender(&msg)).await;
                return Ok(())
            }

            let next_state = MonitorModalState::SendKeyword(shared_user, shared_chat);

            bot_actions::send_message(
                &ctx.bot, msg.chat.id, 
                "請列出要監視的關鍵詞，以空格分隔。字符數量不超過 64 字。\n如果不需要根據關鍵詞篩選的話，請發送 /finish"
            ).await?;

            ctx.modal_states.set_state(
                get_chat_sender(&msg), 
                ModalState::Monitor(next_state)
            ).await;

        },
        MonitorModalState::SendKeyword(sender, chat) => {
            let keywords: Vec<String> = match msg.text.as_ref() {
                Some(text) => {
                    if text == "/finish" {
                        vec![]
                    } else {
                        text.split_whitespace().into_iter()
                            .map(|s| s.to_string())
                            .collect()
                    }
                }
                None => vec![] // 
            };

            // Check keywords, make sure total character count is less than 64
            let total_length: u64 = keywords.iter().map(|s| s.len() as u64).sum();
            if total_length > 64 {
                bot_actions::send_reply_message(
                    &ctx.bot, msg.chat.id, 
                    "字符數量超過 64 個字了呢，請重新發送關鍵詞列表——", 
                    msg.message_id, None
                ).await?;
            }

            // pre calculate title
            let user_nickname = match sender.as_ref() {
                Some(s) => {
                    let first_name = s.first_name.as_ref().map(|s| s.as_str()).unwrap_or("<no name>");
                    let last_name = s.last_name.as_ref().map(|s| s.as_str()).unwrap_or("");
                    Some(format!("{} {}", first_name, last_name))
                }
                None => None
            };

            let chat_title = match chat.as_ref() {
                Some(c) => {
                    let title = c.title.as_ref().map(|s| s.as_str()).unwrap_or("<no title>");
                    Some(title)
                }
                None => None
            };

            let bot_message = format!(
                "創建監視規則：\n - 用戶: {}\n - 群組: {}\n - 關鍵詞: {}",
                user_nickname.as_ref().map(|s| s.as_str()).unwrap_or(""),
                chat_title.unwrap_or(""),
                keywords.join(", ")
            );

            // Create rules
            let filter_rule = FilterRule {
                sender_id: sender.map(|u| u.user_id as i64),
                chat_id: chat.as_ref().map(|c| c.chat_id),
                keywords: keywords
            };

            let rule = MonitorRule {
                filter: filter_rule,
                forward_to: msg.chat.id,
                user_nickname,
                chat_title: chat_title.map(|s| s.to_string()),
            };

            let uuid = Uuid::new_v4();

            log::debug!(
                target: "monitor_add_modal", "{} Adding monitor rule {}", 
                LogOp(&msg), uuid
            );

            ctx.monitor.ruleset.add_rule(rule, uuid);

            ctx.modal_states.release_state(get_chat_sender(&msg)).await;

            bot_actions::send_message(&ctx.bot, msg.chat.id, bot_message).await?;

        },
    }


    Ok(())
}

/// This is a monitor handler, will always return Continue
pub fn monitor_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {

    // This function needs early return
    tokio::spawn(async move {
        monitor_handler_worker(ctx, msg).await
    });

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn monitor_handler_worker(ctx: Arc<Context>, msg: Arc<Message>) {
    
    let forward_to = ctx.monitor.ruleset.check_message(&msg);

    for chat_id in forward_to {
        let ctx = ctx.clone();
        let msg = msg.clone();
        tokio::spawn(async move {

            let param = CopyMessageParams::builder()
                .chat_id(chat_id)
                .from_chat_id(msg.chat.id)
                .message_id(msg.message_id)
                .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
                .build();

            if let Err(e) = ctx.bot.copy_message(&param).await {
                log::warn!(
                    target: "monitor_forward_worker", "{} Failed to make a portal message: {e}", 
                    LogOp(&msg)
                );
            }
        });
    }

}