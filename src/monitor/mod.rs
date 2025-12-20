mod rules;

use std::sync::Arc;

use frankenstein::types::Message;
use futures::future::BoxFuture;

use crate::{context::Context, handler::HandlerResult};

pub const COMMAND_LIST: &[(&'static str, &'static str)] = &[
    ("monitor", ""),
];

pub fn monitor_command_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_command_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_command_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {
    Ok(std::ops::ControlFlow::Continue(()))
}

/// This is a monitor handler, will always return Continue
pub fn monitor_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = monitor_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn monitor_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {



    Ok(std::ops::ControlFlow::Continue(()))
}