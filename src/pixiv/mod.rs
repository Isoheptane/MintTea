pub mod config;
pub mod context;
mod types;
mod download;
mod illust;
mod ugoira;
mod helper;
mod parser;
mod fanbox;

use std::sync::Arc;

use frankenstein::types::Message;
use futures::future::BoxFuture;

use crate::pixiv::fanbox::fanbox_to_kemono_handler;
use crate::pixiv::illust::pixiv_illust_handler;
use crate::pixiv::parser::{parse_fanbox_link, parse_pixiv_command, parse_pixiv_link};
use crate::pixiv::types::IllustRequest;
use crate::handler::HandlerResult;
use crate::helper::message_utils::get_command;
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
    let command = get_command(&msg);
    let Some(text) = msg.text.as_ref() else {
        return Ok(std::ops::ControlFlow::Continue(()))
    };

    if let Some(command) = command {
        match command.as_str() {
            "pixiv" => {
                match parse_pixiv_command(text) {
                    parser::PixivCommandParseResult::Success(req) => {
                        pixiv_illust_handler(ctx, msg, req).await?;
                    }
                    parser::PixivCommandParseResult::InvalidId => {
                        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 pixiv ID 呢……").await?;
                    }
                    parser::PixivCommandParseResult::ShowHelp => {
                        send_pixiv_command_help(ctx, msg).await?;
                    }
                }
                return Ok(std::ops::ControlFlow::Break(()));
            }
            _ => return Ok(std::ops::ControlFlow::Continue(()))
        }
    }
    
    // Link detection for pixiv
    if ctx.config.pixiv.enable_pixiv_link_detection {
        match parse_pixiv_link(&text) {
            parser::PixivLinkParseResult::Success(id) => {
                let req = IllustRequest::link_default(id);
                pixiv_illust_handler(ctx, msg, req).await?;
                return Ok(std::ops::ControlFlow::Break(()))
            }
            parser::PixivLinkParseResult::InvalidId => {
                bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 pixiv ID 呢……").await?;
                return Ok(std::ops::ControlFlow::Break(()));
            }
            parser::PixivLinkParseResult::NotMatch => {},
        }
    }
    // Link detection for fanbox
    if ctx.config.pixiv.enable_fanbox_link_detection {
        match parse_fanbox_link(&text) {
            parser::FanboxLinkParseResult::Success { name, post_id } => {
                fanbox_to_kemono_handler(ctx, msg, name, post_id).await?;
            },
            parser::FanboxLinkParseResult::InvalidPostId => {
                bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 FANBOX Post ID 呢……").await?;
            },
            parser::FanboxLinkParseResult::EmptyName |
            parser::FanboxLinkParseResult::NotMatch => {},
        }
    }

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn send_pixiv_command_help(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    const HELP_MSG : &'static str = 
        "/pixiv 指令幫助\n\
        - 使用方法：/pixiv <id> [nolim|files|archive|detail|metaonly]\n\
        \n\
        參數說明：\n\
        - nolim: 允許 10 頁插畫以上的畫廊\n\
        - files: 發送插畫文件\n\
        - archive: 發送畫廊的 zip 歸檔\n\
        - detail: 詳細插畫描述信息\n\
        - metaonly: 只發送元數據（插畫描述信息）\n\
        ";
    bot_actions::send_message(&ctx.bot, msg.chat.id, HELP_MSG).await?;
    Ok(())
}