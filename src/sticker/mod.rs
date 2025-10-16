mod sticker_set_download;
mod sticker_to_media;
mod media_to_sticker;

use std::sync::Arc;

use async_trait::async_trait;
use frankenstein::client_reqwest::Bot;
use frankenstein::methods::SendChatActionParams;
use frankenstein::types::ChatType;
use frankenstein::types::Message;
use frankenstein::AsyncTelegramApi;

use crate::handler::HandlerResult;
use crate::handler::UpdateHandler;
use crate::helper::bot_actions;
use crate::helper::message_utils::{message_chat_sender, message_command};
use crate::shared::{ChatState, SharedData};
use crate::sticker::media_to_sticker::{animation_to_sticker_processor, document_to_sticker_processor, photo_to_sticker_processor, video_to_sticker_processor};
use crate::sticker::sticker_set_download::sticker_set_download_processor;
use crate::sticker::sticker_to_media::sticker_to_media_processor;

pub const COMMAND_LIST: &[(&'static str, &'static str)] = &[
    ("sticker_convert", "轉換貼紙、圖片和動圖"),
    ("sticker_set_download", "下載貼紙包")
];

#[derive(Debug, PartialEq, Clone)]
pub enum ChatStickerState {
    StickerConvert,
    StickerSetDownload
}

#[derive(Debug, PartialEq, Clone)]
pub enum StickerCommand {
    StickerConvert,
    StickerSetDownload
}

pub struct StickerHandler {}
#[async_trait]
impl UpdateHandler<Arc<SharedData>, Message> for StickerHandler {
    async fn handle(&self, bot: Bot, data: &Arc<SharedData>, update: &Message) -> HandlerResult {
        sticker_handler(bot, data, update).await
    }
}

pub async fn sticker_handler(
    bot: Bot,
    data: &Arc<SharedData>,
    msg: &Message
) -> HandlerResult {
    let state = data.chat_state_storage.get_state(message_chat_sender(msg)).await;

    let command = message_command(&msg);
    if let Some(command) = command {
        match command.as_str() {
            "sticker_convert" => {
                sticker_command_processor(bot, data, msg, StickerCommand::StickerConvert).await?;
                return Ok(std::ops::ControlFlow::Break(()))
            },
            "sticker_set_download" => {
                sticker_command_processor(bot, data, msg, StickerCommand::StickerSetDownload).await?;
                return Ok(std::ops::ControlFlow::Break(()))
            }
            _ => {}
        }
    }

    if let Some(ChatState::Sticker(sticker_state)) = state {
        sticker_message_processor(bot, data, msg, sticker_state).await?;
        return Ok(std::ops::ControlFlow::Break(()));
    }

    return Ok(std::ops::ControlFlow::Continue(()))
}

async fn sticker_command_processor(
    bot: Bot,
    data: &Arc<SharedData>,
    msg: &Message,
    cmd: StickerCommand
) -> anyhow::Result<()> {
    if msg.chat.type_field != ChatType::Private {
        bot_actions::send_message(&bot, msg.chat.id, "貼紙指令只能在私聊中使用哦——").await?;
        return Ok(());
    }

    match cmd {
        StickerCommand::StickerConvert => {
            data.chat_state_storage.set_state(
                message_chat_sender(&msg), 
                ChatState::Sticker(ChatStickerState::StickerConvert)
            ).await;

            log::info!(
                target: "sticker_command",
                "[ChatID: {}, {:?}] Switched to sticker conversion mode", 
                msg.chat.id, msg.chat.username
            );

            bot_actions::send_message(&bot, msg.chat.id, "請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit").await?;
        },
        StickerCommand::StickerSetDownload => {
            data.chat_state_storage.set_state(
                message_chat_sender(&msg), 
                ChatState::Sticker(ChatStickerState::StickerSetDownload)
            ).await;

            log::info!(
                target: "sticker_command",
                "[ChatID: {}, {:?}] Switched to sticker set download mode", 
                msg.chat.id, msg.chat.username
            );

            bot_actions::send_message(&bot, msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
        }
    }
    
    Ok(())
}

async fn sticker_message_processor(
    bot: Bot,
    data: &Arc<SharedData>,
    msg: &Message,
    sticker_state: ChatStickerState
) -> anyhow::Result<()> {
    match sticker_state {
        ChatStickerState::StickerConvert => {
            bot.send_chat_action(&SendChatActionParams::builder().chat_id(msg.chat.id).action(frankenstein::types::ChatAction::Typing).build()).await?;
            // Check message type and decide conversion type
            if let Some(sticker) = msg.sticker.as_ref() {
                sticker_to_media_processor(bot, data, &msg, sticker).await?;
            } else if let Some(document) = msg.document.as_ref() {
                document_to_sticker_processor(bot, data, &msg, document).await?;
            } else if let Some(photos) = msg.photo.as_ref() {
                photo_to_sticker_processor(bot, data, &msg, photos).await?;
            } else if let Some(animation) = msg.animation.as_ref() {
                animation_to_sticker_processor(bot, data, &msg, animation).await?;
            } else if let Some(video) = msg.video.as_ref() {
                video_to_sticker_processor(bot, data, &msg, video).await?;
            } else {
                bot_actions::send_message(&bot, msg.chat.id, "請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit").await?;
            }
        },
        ChatStickerState::StickerSetDownload => {
            if let Some(sticker) = msg.sticker.as_ref() {
                sticker_set_download_processor(bot, data, &msg, sticker).await?;
            } else {
                bot_actions::send_message(&bot, msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
            }
        },
    }
    
    Ok(())
}