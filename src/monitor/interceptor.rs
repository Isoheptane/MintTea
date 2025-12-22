use std::sync::Arc;

use frankenstein::AsyncTelegramApi;
use frankenstein::methods::CopyMessageParams;
use frankenstein::types::Message;
use futures::future::BoxFuture;

use crate::helper::param_builders;
use crate::handler::HandlerResult;
use crate::context::Context;
use crate::helper::log::LogOp;

/// This is a monitor handler, will always return Continue
pub fn monitor_interceptor(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_interceptor_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_interceptor_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {

    // This function needs early return
    tokio::spawn(async move {
        monitor_interceptor_worker(ctx, msg).await
    });

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn monitor_interceptor_worker(ctx: Arc<Context>, msg: Arc<Message>) {
    
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