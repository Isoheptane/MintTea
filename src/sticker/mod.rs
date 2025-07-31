mod sticker_set_download;

use std::process::Stdio;
use std::sync::Arc;

use async_tempfile::TempFile;
use teloxide::{payloads, prelude::*};
use teloxide::dispatching::UpdateHandler;
use teloxide::requests::MultipartRequest;
use teloxide::types::{Document, InputFile, ReplyParameters, Sticker};
use teloxide::utils::command::BotCommands;
use tokio::io::AsyncWriteExt;

use crate::download::download_file;
use crate::shared::SharedData;
use crate::shared::ChatState;
use crate::sticker::sticker_set_download::sticker_set_download_processor;

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

            bot.send_message(msg.chat.id, "接下來，我會將貼紙轉換為圖片或動圖、將圖片或動圖轉換為貼紙。\n請發送想要轉換的貼紙、圖片或動圖～\n如果想要退出轉換，請輸入 /exit 來退出轉換模式～").await?;
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

            bot.send_message(msg.chat.id, "接下來請發送一張想要下載的貼紙包中的貼紙～\n如果想要退出下載，請輸入 /exit 來退出貼紙包下載模式～").await?;
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
            if let Some(sticker) = msg.sticker() {
                sticker_to_media_processor(bot, &msg, sticker).await?;
            } else if let Some(document) = msg.document() {
                document_to_sticker_processor(bot, &msg, document).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送貼紙、圖片或動圖～ 如果想要退出轉換，請輸入 /exit 來退出轉換模式～").await?;
            }
        },
        ChatStickerState::StickerSetDownload => {
            if let Some(sticker) = msg.sticker() {
                sticker_set_download_processor(bot, &msg, sticker).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～ 如果想要退出下載，請輸入 /exit 來退出貼紙包下載模式～").await?;
            }
        },
    }
    
    Ok(())
}

async fn sticker_to_media_processor(
    bot: Bot,
    msg: &Message,
    sticker: &Sticker
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "sticker_to_media",
        "[ChatID: {}, {}] Requested sticker conversion", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
    );

    let (content, file_name) = download_file(bot.clone(), sticker.file.id.clone()).await?;

    if file_name.extension != "webp" && file_name.extension != "webm" {
        bot.send_message(msg.chat.id, "現在還不支持 WebP 和 WebM 格式外的貼紙哦……").await?;
        return Ok(());
    }

    let new_file_basename = format!(
        "{}_{}_{}",
        sticker.set_name.clone().unwrap_or("noset".to_string()),
        msg.chat.id,
        msg.id.0
    );

    // Save to temp
    let source_name = format!("{}.{}", new_file_basename, file_name.extension);
    let mut source_file = TempFile::new_with_name(&source_name).await?;
    source_file.write_all(&content).await?;
    let source_path = source_file.file_path().to_string_lossy();
    
    // Picture
    if file_name.extension == "webp" {
        let png_name = format!("{}.png", new_file_basename);
        let png_file = TempFile::new_with_name(&png_name).await?;
        let png_path = png_file.file_path().to_string_lossy();

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {}] Converting {} to {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), source_path, png_path
        );

        let conversion = tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-y", &png_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
            .wait().await?;
        if !conversion.success() {
            bot.send_message(msg.chat.id, "文件轉碼失敗惹……").await?;
            return Ok(())
        }
        // after conversion
        let upload_file = InputFile::file(png_file.file_path());

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), png_name
        );

        let document_payload = payloads::SendDocument::new(msg.chat.id, upload_file.clone())
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), document_payload).await?;

        let photo_payload = payloads::SendPhoto::new(msg.chat.id, upload_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), photo_payload).await?;
    } else if file_name.extension == "webm" {
        let gif_name = format!("{}.gif", new_file_basename);
        let gif_file = TempFile::new_with_name(&gif_name).await?;
        let gif_path = gif_file.file_path().to_string_lossy();

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {}] Converting {} to {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), source_path, gif_path
        );

        let conversion = tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-y", &gif_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
            .wait().await?;
        if !conversion.success() {
            bot.send_message(msg.chat.id, "文件轉碼失敗惹……").await?;
            return Ok(())
        }
        // after conversion
        let upload_webm_file = InputFile::file(source_file.file_path());
        let upload_gif_file = InputFile::file(gif_file.file_path());

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), source_name
        );

        let webm_payload = payloads::SendDocument::new(msg.chat.id, upload_webm_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), webm_payload).await?;

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), gif_name
        );

        let gif_payload = payloads::SendAnimation::new(msg.chat.id, upload_gif_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), gif_payload).await?;
    }

    Ok(())
}

async fn document_to_sticker_processor(
    bot: Bot,
    msg: &Message,
    doc: &Document
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    let (content, file_name) = download_file(bot.clone(), doc.file.id.clone()).await?;

    bot.send_message(msg.chat.id, format!("{}.{}", file_name.basename, file_name.extension)).await?;

    Ok(())
}