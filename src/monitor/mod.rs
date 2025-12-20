pub mod context;

mod rules;
mod add_rule;

use std::sync::Arc;

use frankenstein::methods::{CopyMessageParams, SendMessageParams};
use frankenstein::AsyncTelegramApi;
use frankenstein::types::{ChatShared, Message, SharedUser};
use futures::future::BoxFuture;

use crate::helper::{bot_actions, param_builders};
use crate::helper::message_utils::{get_command, get_sender_id, get_withspace_split};
use crate::handler::{HandlerResult, ModalHandlerResult};
use crate::context::Context;
use crate::helper::log::LogOp;
use crate::monitor::add_rule::{into_add_rule_modal, monitor_add_rule_modal_handler};



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

    let args = get_withspace_split(&msg);
    let Some(subcommand) = args.get(1) else {
        into_add_rule_modal(ctx, msg).await?;
        return Ok(std::ops::ControlFlow::Break(()));
    };
    let subcommand = *subcommand;

    if subcommand == "list" {
        list_rules(ctx, msg).await?;
        return Ok(std::ops::ControlFlow::Break(()));
    }
    
    Ok(std::ops::ControlFlow::Continue(()))
}

pub async fn list_rules(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    let Some(receiver_id) = get_sender_id(&msg) else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "好像找不到你的 ID 呢……").await?;
        return Ok(())
    };
    
    let rules = ctx.monitor.ruleset.get_receiver_rules(receiver_id);

    let mut lines: Vec<String> = vec![];
    lines.push(format!("你有 {} 條監視規則哦：", rules.len()));
    
    for (uuid, rule) in rules {
        lines.push("".to_string());
        lines.push(format!("<code>{}</code>", uuid));
        if let Some(id) = rule.filter.sender_id {
            let nickname_suffix = match rule.user_nickname {
                Some(nickname) => format!(" ({})", nickname),
                None => "".to_string(),
            };
            lines.push(format!(" - 用戶: {}{}", id, nickname_suffix));
        }
        if let Some(id) = rule.filter.chat_id {
            let nickname_suffix = match rule.chat_title {
                Some(title) => format!(" ({})", title),
                None => "".to_string(),
            };
            lines.push(format!(" - 群組: {}{}", id, nickname_suffix));
        }
        lines.push(format!(" - 關鍵詞: {}", rule.filter.keywords.join(", ")));
    }

    let params = SendMessageParams::builder()
        .chat_id(msg.chat.id)
        .reply_parameters(param_builders::reply_parameters(msg.message_id, None))
        .parse_mode(frankenstein::ParseMode::Html)
        .text(lines.join("\n"))
        .build();
    ctx.bot.send_message(&params).await?;

    return Ok(())
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

pub async fn monitor_modal_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>, 
    state: MonitorModalState
) -> ModalHandlerResult {
    monitor_add_rule_modal_handler(ctx, msg, state).await
}