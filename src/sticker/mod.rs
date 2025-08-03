mod sticker_set_download;
mod sticker_to_media;
mod media_to_sticker;

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::dispatching::UpdateHandler;
use teloxide::types::ChatAction;
use teloxide::utils::command::BotCommands;

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

#[derive(BotCommands, PartialEq, Clone, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
enum StickerCommand {
    StickerConvert,
    StickerSetDownload
}

pub fn sticker_handler() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    dptree::entry()
        .branch(
            dptree::filter_map_async(async |shared: Arc<SharedData>, msg: Message|  {
                let state = shared.chat_state_storage.get_state(msg.chat.id).await;
                if let Some(ChatState::Sticker(sticker_state)) = state {
                    return Some(sticker_state);
                } else {
                    return None
                }
            })
            .endpoint(sticker_message_processor)
        )
        .branch(
            dptree::entry()
            .filter_command::<StickerCommand>()
            .endpoint(sticker_command_processor)
        )
}

async fn sticker_command_processor(
    shared: Arc<SharedData>,
    bot: Bot,
    msg: Message,
    cmd: StickerCommand
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    if !msg.chat.is_private() {
        bot.send_message(msg.chat.id, "貼紙指令只能在私聊中使用哦——").await?;
        return Ok(());
    }

    match cmd {
        StickerCommand::StickerConvert => {
            shared.chat_state_storage.set_state(
                msg.chat.id, 
                ChatState::Sticker(ChatStickerState::StickerConvert)
            ).await;

            log::info!(
                target: "sticker_command",
                "[ChatID: {}, {}] Switched to sticker conversion mode", 
                msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
            );

            bot.send_message(msg.chat.id, "請發送想要轉換的貼紙、圖片或動圖～").await?;
        },
        StickerCommand::StickerSetDownload => {
            shared.chat_state_storage.set_state(
                msg.chat.id, 
                ChatState::Sticker(ChatStickerState::StickerSetDownload)
            ).await;

            log::info!(
                target: "sticker_command",
                "[ChatID: {}, {}] Switched to sticker set download mode", 
                msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
            );

            bot.send_message(msg.chat.id, "接下來請發送一張想要下載的貼紙包中的貼紙～").await?;
        }
    }
    
    Ok(())
}

async fn sticker_message_processor(
    // shared: Arc<SharedData>,
    bot: Bot,
    msg: Message,
    sticker_state: ChatStickerState
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match sticker_state {
        ChatStickerState::StickerConvert => {
            bot.send_chat_action(msg.chat.id, ChatAction::Typing).await?;
            // Check message type and decide conversion type
            if let Some(sticker) = msg.sticker() {
                sticker_to_media_processor(bot, &msg, sticker).await?;
            } else if let Some(document) = msg.document() {
                document_to_sticker_processor(bot, &msg, document).await?;
            } else if let Some(photos) = msg.photo() {
                photo_to_sticker_processor(bot, &msg, photos).await?;
            } else if let Some(animation) = msg.animation() {
                animation_to_sticker_processor(bot, &msg, animation).await?;
            } else if let Some(video) = msg.video() {
                video_to_sticker_processor(bot, &msg, video).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送想要轉換的貼紙、圖片或動圖～\n如果要退出，請點擊指令 -> /exit").await?;
            }
        },
        ChatStickerState::StickerSetDownload => {
            if let Some(sticker) = msg.sticker() {
                sticker_set_download_processor(bot, &msg, sticker).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
            }
        },
    }
    
    Ok(())
}