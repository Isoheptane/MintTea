mod pixiv_illust;
mod pixiv_download;

use std::sync::Arc;

use frankenstein::types::Message;
use futures::future::BoxFuture;

use crate::pixiv::pixiv_illust::pixiv_illust_handler;
use crate::handler::HandlerResult;
use crate::helper::message_utils::message_command;
use crate::helper::bot_actions;
use crate::context::Context;

pub const COMMAND_LIST: &[(&'static str, &'static str)] = &[
    ("pixiv", "從 Pixiv 下載插畫"),
];

pub fn pixiv_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = pixiv_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn pixiv_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {

    let command = message_command(&msg);
    if command.is_some_and(|command| command == "pixiv") {
        pixiv_command_handler(ctx, msg).await?;
        return Ok(std::ops::ControlFlow::Break(()));
    }

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn pixiv_command_handler(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {

    // Command parser
    let Some(text) = msg.text.as_ref() else {
        return Ok(());
    };
    let args: Vec<&str> = text.split_whitespace().collect();
    // Check if the id is present
    let Some(raw_input) = args.get(1) else {
        // Send help message if pixiv link is not present
        const HELP_MSG : &'static str = 
            "/pixiv 指令幫助\n\
            - 使用方法：/pixiv <id>\n";
        bot_actions::send_message(&ctx.bot, msg.chat.id, HELP_MSG).await?;
        return Ok(());
    };
    // TODO: Add link support using regex
    let Ok(id) = u64::from_str_radix(&raw_input, 10) else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 pixiv ID 呢……").await?;
        return Ok(());
    };

    pixiv_illust_handler(ctx, msg, id).await?;

    Ok(())
}