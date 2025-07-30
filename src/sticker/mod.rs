use std::sync::Arc;

use async_tempfile::TempFile;
use teloxide::net::Download;
use teloxide::{payloads, prelude::*};
use teloxide::dispatching::UpdateHandler;
use teloxide::requests::MultipartRequest;
use teloxide::types::{InputFile, ReplyParameters, Sticker};
use teloxide::utils::command::BotCommands;
use tokio::io::AsyncWriteExt;

use crate::shared::SharedData;
use crate::shared::ChatState;
use crate::shared::ChatStickerState;

#[derive(BotCommands, PartialEq, Clone, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
enum StickerCommand {
    S2P, // Sticker to Picture
    P2S, // Picture to Sticker
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
        StickerCommand::S2P => {
            shared.chat_state_storage.set_state(
                msg.chat.id, 
                ChatState::Sticker(ChatStickerState::StickerToPicture)
            ).await;
            bot.send_message(msg.chat.id, "接下來，請發送貼紙，然後我會將貼紙轉換為圖片～").await?;
        },
        StickerCommand::P2S => {
            shared.chat_state_storage.set_state(
                msg.chat.id, 
                ChatState::Sticker(ChatStickerState::PictureToSticker)
            ).await;
            bot.send_message(msg.chat.id, "接下來，請發送圖片或動圖，然後我會將貼紙轉換為貼紙～").await?;
        }
        StickerCommand::StickerSetDownload => {
            shared.chat_state_storage.set_state(
                msg.chat.id, 
                ChatState::Sticker(ChatStickerState::StickerSetDownload)
            ).await;
            bot.send_message(msg.chat.id, "接下來請發送一張想要下載的貼紙包中的貼紙～ 輸入 /exit 退出。").await?;
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
        ChatStickerState::StickerToPicture => {
            if let Some(sticker) = msg.sticker() {
                sticker_to_picture_processor(bot, &msg, sticker).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送貼紙").await?;
            }
        },
        ChatStickerState::PictureToSticker => {
            
        },
        ChatStickerState::StickerSetDownload => {
            bot.send_message(msg.chat.id, "接下來請發送一張想要下載的貼紙包中的貼紙～").await?;
        },
    }
    
    Ok(())
}

const STICKER_TEMP_DIR: &'static str = "./temp/sticker";

async fn sticker_to_picture_processor(
    bot: Bot,
    msg: &Message,
    sticker: &Sticker
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let file_info = bot.get_file(sticker.file.id.clone()).await?;

    let file_name = file_info.path.split('/').last();
    let file_name = match file_name {
        Some(name) => name.to_string(),
        None => {
            log::warn!("Failed to fetch file name in URL: {}", file_info.path);
            bot.send_message(msg.chat.id, "似乎出了什麼問題……").await?;
            return Ok(())
        }
    };

    let file_ext = file_name.split('.').last();
    let file_ext = match file_ext {
        Some(ext) => ext.to_string(),
        None => {
            log::warn!("Failed to fetch file extension in file name: {}", file_info.path);
            bot.send_message(msg.chat.id, "似乎出了什麼問題……").await?;
            return Ok(())
        }
    };
    
    if file_ext != "webp" && file_ext != "webm" {
        bot.send_message(msg.chat.id, "現在還不支持 WebP 和 WebM 格式外的貼紙哦——").await?;
        return Ok(());
    }

    let new_file_basename = format!(
        "{}_{}_{}",
        sticker.set_name.clone().unwrap_or("noset".to_string()),
        msg.chat.id,
        msg.id.0
    );

    // Download Sticker
    log::debug!("Downloading file {} ({} Bytes)", file_name, file_info.size);
    let mut file_content = Vec::<u8>::new();
    file_content.reserve(file_info.size as usize);
    bot.download_file(&file_info.path, &mut file_content).await?;
    // Check Temp Folder
    if !tokio::fs::try_exists(STICKER_TEMP_DIR).await? {
        tokio::fs::create_dir_all(STICKER_TEMP_DIR).await?;
    }
    // Save to temp folder
    let source_name = format!("{}.{}", new_file_basename, file_ext);
    let mut source_file = TempFile::new_with_name(source_name).await?;
    source_file.write_all(&file_content).await?;
    let source_path = source_file.file_path().to_string_lossy();
    
    // Picture
    if !sticker.is_animated() && file_ext == "webp" {
        let png_name = format!("{}.png", new_file_basename);
        let png_file = TempFile::new_with_name(png_name).await?;
        let png_path = png_file.file_path().to_string_lossy();

        log::debug!("Converting {} to {}", source_path, png_path);
        tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-y", &png_path])
        .spawn()?.wait().await?;
        // after conversion
        let upload_file = InputFile::file(png_file.file_path());

        let document_payload = payloads::SendDocument::new(msg.chat.id, upload_file.clone())
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), document_payload).await?;

        let photo_payload = payloads::SendPhoto::new(msg.chat.id, upload_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), photo_payload).await?;

        return Ok(());
    }

    if file_ext == "webm" {
        let gif_name = format!("{}.gif", new_file_basename);
        let gif_file = TempFile::new_with_name(gif_name).await?;
        let gif_path = gif_file.file_path().to_string_lossy();

        log::debug!("Converting {} to {}", source_path, gif_path);
        tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-y", &gif_path])
        .spawn()?.wait().await?;
        // after conversion
        let upload_webm_file = InputFile::file(source_file.file_path());
        let upload_gif_file = InputFile::file(gif_file.file_path());

        let webm_payload = payloads::SendDocument::new(msg.chat.id, upload_webm_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), webm_payload).await?;

        let gif_payload = payloads::SendDocument::new(msg.chat.id, upload_gif_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), gif_payload).await?;

        return Ok(());
    }
    

    Ok(())
}