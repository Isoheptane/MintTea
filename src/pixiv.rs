pub mod config;
mod pixiv_illust;
mod pixiv_download;

use std::sync::Arc;

use frankenstein::types::Message;
use futures::future::BoxFuture;
use regex::Regex;

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

    // Send help if the first argument is not present / or equals to help
    let Some(raw_input) = args.get(1) else {
        // Send help message if pixiv link is not present
        pixiv_send_command_help(ctx, msg).await?;
        return Ok(());
    };
    if *raw_input == "help" {
        pixiv_send_command_help(ctx, msg).await?;
        return Ok(());
    }

    // Recognize link or id
    let re = Regex::new(r"^(?:(?:https?:\/\/)?(?:www.)?pixiv.net(?:\/en)?\/artworks\/)?([0-9]+)$")?;    
    let Some((_, [id_str])) = re.captures(raw_input).map(|c| c.extract()) else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 pixiv 鏈接或 ID 呢……").await?;
        return Ok(());
    };
    let Ok(id) = u64::from_str_radix(&id_str, 10) else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 pixiv ID 呢……").await?;
        return Ok(());
    };

    // Recognize arguments (2 and latter arguments)
    let mut no_page_limit = false;
    let mut archive_mode = false;
    for arg in args.iter().skip(2) {
        if *arg == "nolim" { no_page_limit = true; }
        if *arg == "archive" { archive_mode = true; }
    }

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::Typing).await?;

    pixiv_illust_handler(ctx, msg, id, no_page_limit, archive_mode).await?;

    Ok(())
}

async fn pixiv_send_command_help(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    const HELP_MSG : &'static str = 
        "/pixiv 指令幫助\n\
        - 使用方法：/pixiv <id> [nolim|archive]\n";
    bot_actions::send_message(&ctx.bot, msg.chat.id, HELP_MSG).await?;
    Ok(())
}