pub mod context;

mod rules;
mod add_rule;

use std::sync::Arc;

use frankenstein::methods::CopyMessageParams;
use frankenstein::AsyncTelegramApi;
use frankenstein::types::{ChatShared, Message, SharedUser};
use futures::future::BoxFuture;

use crate::helper::param_builders;
use crate::helper::message_utils::get_command;
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

    into_add_rule_modal(ctx, msg).await?;
    
    Ok(std::ops::ControlFlow::Continue(()))
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