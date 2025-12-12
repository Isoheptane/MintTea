pub mod config;
mod pixiv_illust_info;
mod pixiv_illust;
mod pixiv_animation;
mod pixiv_download;

use std::sync::Arc;

use frankenstein::types::Message;
use futures::future::BoxFuture;
use regex::Regex;

use crate::pixiv::pixiv_illust::{DownloadOptions, SendMode, pixiv_illust_handler};
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

    // Command handling
    let command = message_command(&msg);
    if command.is_some_and(|command| command == "pixiv") {
        pixiv_command_handler(ctx, msg).await?;
        return Ok(std::ops::ControlFlow::Break(()));
    }
    
    // Link detection
    if let std::ops::ControlFlow::Break(_) = pixiv_try_link_handler(ctx, msg).await? {
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
    let mut files_mode = false;
    let mut archive_mode = false;
    for arg in args.iter().skip(2) {
        if *arg == "nolim" { no_page_limit = true; }
        if *arg == "archive" { archive_mode = true; }
        if *arg == "files" { files_mode = true; }
    }

    let send_mode = match (files_mode, archive_mode) {
        (false, false) => SendMode::Photos,
        (true, false) => SendMode::Files,
        (false, true) => SendMode::Archive,
        (true, true) => SendMode::Archive
    };

    let options = DownloadOptions {
        no_page_limit,
        silent_page_limit: false,
        send_mode
    };

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::Typing).await?;

    pixiv_illust_handler(ctx, msg, id, options).await?;

    Ok(())
}

async fn pixiv_try_link_handler(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {

    let Some(text) = msg.text.as_ref() else {
        return Ok(std::ops::ControlFlow::Continue(()));
    };

    let re = Regex::new(r"^(?:(?:https?:\/\/)?(?:www.)?pixiv.net(?:\/en)?\/artworks\/)([0-9]+)$")?;   
    let Some((_, [id_str])) = re.captures(&text).map(|c| c.extract()) else {
        return Ok(std::ops::ControlFlow::Continue(()));
    };
    let Ok(id) = u64::from_str_radix(&id_str, 10) else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 pixiv ID 呢……").await?;
        return Ok(std::ops::ControlFlow::Break(()));
    };

    let options = DownloadOptions {
        no_page_limit: false,
        silent_page_limit: true, 
        send_mode: SendMode::Photos
    };

    pixiv_illust_handler(ctx, msg, id, options).await?;

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn pixiv_send_command_help(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    const HELP_MSG : &'static str = 
        "/pixiv 指令幫助\n\
        - 使用方法：/pixiv <id> [nolim|files|archive]\n\
        \n\
        參數說明：\n\
        - nolim: 允許 10 頁插畫以上的畫廊\n\
        - files: 發送插畫文件\n\
        - archive: 發送畫廊的 zip 歸檔\n\
        ";
    bot_actions::send_message(&ctx.bot, msg.chat.id, HELP_MSG).await?;
    Ok(())
}