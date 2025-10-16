mod sticker_set_download;
mod sticker_to_media;
mod media_to_sticker;

use std::sync::Arc;

use frankenstein::client_reqwest::Bot;
use frankenstein::methods::SendChatActionParams;
use frankenstein::methods::SendMessageParams;
use frankenstein::types::ChatType;
use frankenstein::types::Message;
use frankenstein::AsyncTelegramApi;

use crate::helper::message_chat_sender;
use crate::helper::message_command;
use crate::shared::SharedData;
use crate::shared::ChatState;
use crate::sticker::media_to_sticker::{animation_to_sticker_processor, document_to_sticker_processor, photo_to_sticker_processor, video_to_sticker_processor};
use crate::sticker::sticker_set_download::sticker_set_download_processor;
use crate::sticker::sticker_to_media::sticker_to_media_processor;

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

pub async fn sticker_handler(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message
) -> (bool, Option<Box<dyn std::error::Error + Send + Sync + 'static>>) {
    let state = data.chat_state_storage.get_state(message_chat_sender(msg)).await;

    if let Some(ChatState::Sticker(sticker_state)) = state {
        let e = sticker_message_processor(bot, data, msg, sticker_state).await.err();
        return (true, e)
    }

    let command = message_command(&msg);
    if let Some(command) = command {
        match command.as_str() {
            "sticker_convert" => {
                let e = sticker_command_processor(bot, data, msg, StickerCommand::StickerConvert).await.err();
                return (true, e)
            },
            "sticker_set_download" => {
                let e = sticker_command_processor(bot, data, msg, StickerCommand::StickerSetDownload).await.err();
                return (true, e)
            }
            _ => {}
        }
    }

    return (false, None)
}

async fn sticker_command_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    cmd: StickerCommand
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    if msg.chat.type_field != ChatType::Private {
        let send_message_params = SendMessageParams::builder()
            .chat_id(msg.chat.id)
            .text("貼紙指令只能在私聊中使用哦——")
            .build();
        bot.send_message(&send_message_params).await?;
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

            let send_message_params = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text("請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit")
                .build();
            bot.send_message(&send_message_params).await?;
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

            let send_message_params = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text("請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit")
                .build();
            bot.send_message(&send_message_params).await?;
        }
    }
    
    Ok(())
}

async fn sticker_message_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    sticker_state: ChatStickerState
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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
                let send_message_params = SendMessageParams::builder()
                    .chat_id(msg.chat.id)
                    .text("請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit")
                    .build();
                bot.send_message(&send_message_params).await?;
            }
        },
        ChatStickerState::StickerSetDownload => {
            if let Some(sticker) = msg.sticker.as_ref() {
                sticker_set_download_processor(bot, data, &msg, sticker).await?;
            } else {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(msg.chat.id)
                    .text("請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit")
                    .build();
                bot.send_message(&send_message_params).await?;
            }
        },
    }
    
    Ok(())
}