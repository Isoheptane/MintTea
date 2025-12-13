mod sticker_set_download;
mod sticker_to_media;
mod media_to_sticker;
pub mod config;

use std::str::FromStr;
use std::sync::Arc;

use frankenstein::methods::SendChatActionParams;
use frankenstein::types::ChatType;
use frankenstein::types::Message;
use frankenstein::AsyncTelegramApi;
use futures::future::BoxFuture;

use crate::handler::HandlerResult;
use crate::handler::ModalHandlerResult;
use crate::helper::bot_actions;
use crate::helper::log::LogSource;
use crate::helper::message_utils::{message_chat_sender, message_command};
use crate::context::{Context, ModalState};
use crate::sticker::media_to_sticker::{animation_to_sticker_processor, document_to_sticker_processor, photo_to_sticker_processor, video_to_sticker_processor};
use crate::sticker::sticker_set_download::sticker_set_download_processor;
use crate::sticker::sticker_to_media::sticker_to_media_processor;

#[derive(Debug, PartialEq, Clone)]
pub enum StickerModalState {
    StickerConvert,
    StickerSetDownload
}

#[derive(Debug, PartialEq, Clone)]
pub enum StickerCommand {
    StickerConvert,
    StickerSetDownload
}

impl FromStr for StickerCommand {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sticker_convert" => Ok(StickerCommand::StickerConvert),
            "sticker_set_download" => Ok(StickerCommand::StickerSetDownload),
            _ => Err(())
        }
    }
}

pub const COMMAND_LIST: &[(&'static str, &'static str)] = &[
    ("sticker_convert", "轉換貼紙、圖片和動圖"),
    ("sticker_set_download", "下載貼紙包")
];

pub fn sticker_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = sticker_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn sticker_handler_impl(
    ctx: Arc<Context>,
    msg: Arc<Message>
) -> HandlerResult {

    let command = message_command(&msg);
    let Some(command) = command else {
        return Ok(std::ops::ControlFlow::Continue(()));
    };
    let Ok(command) = StickerCommand::from_str(&command) else {
        return Ok(std::ops::ControlFlow::Continue(()));
    };

    if msg.chat.type_field != ChatType::Private {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "貼紙指令只能在私聊中使用哦——").await?;
        return Ok(std::ops::ControlFlow::Break(()));
    }

    match command {
        StickerCommand::StickerConvert => {
            ctx.modal_states.set_state(
                message_chat_sender(&msg), 
                ModalState::Sticker(StickerModalState::StickerConvert)
            ).await;

            log::info!(
                target: "sticker_command",
                "{} Switched to sticker conversion mode", 
                LogSource(&msg)
            );

            bot_actions::send_message(&ctx.bot, msg.chat.id, "請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit").await?;
        }
        StickerCommand::StickerSetDownload => {
            ctx.modal_states.set_state(
                message_chat_sender(&msg), 
                ModalState::Sticker(StickerModalState::StickerSetDownload)
            ).await;

            log::info!(
                target: "sticker_command",
                "{} Switched to sticker set download mode", 
                LogSource(&msg)
            );

            bot_actions::send_message(&ctx.bot, msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
        }
    }

    return Ok(std::ops::ControlFlow::Break(()));
}

pub async fn sticker_modal_handler(
    ctx: Arc<Context>,
    msg: Arc<Message>,
    state: StickerModalState
) -> ModalHandlerResult {
    match state {
        StickerModalState::StickerConvert => {
            bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::Typing).await?;
            ctx.bot.send_chat_action(&SendChatActionParams::builder().chat_id(msg.chat.id).action(frankenstein::types::ChatAction::Typing).build()).await?;
            // Check message type and decide conversion type
            if let Some(sticker) = msg.sticker.as_ref() {
                sticker_to_media_processor(ctx.clone(), &msg, sticker).await?;
            } else if let Some(document) = msg.document.as_ref() {
                document_to_sticker_processor(ctx.clone(), &msg, document).await?;
            } else if let Some(photos) = msg.photo.as_ref() {
                photo_to_sticker_processor(ctx.clone(), &msg, photos).await?;
            } else if let Some(animation) = msg.animation.as_ref() {
                animation_to_sticker_processor(ctx.clone(), &msg, animation).await?;
            } else if let Some(video) = msg.video.as_ref() {
                video_to_sticker_processor(ctx.clone(), &msg, video).await?;
            } else {
                bot_actions::send_message(&ctx.bot, msg.chat.id, "請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit").await?;
            }
        },
        StickerModalState::StickerSetDownload => {
            if let Some(sticker) = msg.sticker.as_ref() {
                sticker_set_download_processor(ctx.clone(), &msg, sticker).await?;
            } else {
                bot_actions::send_message(&ctx.bot, msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
            }
        },
    }
    
    Ok(())
}