mod sticker_set_download;

use std::process::Stdio;
use std::sync::Arc;

use async_tempfile::TempFile;
use teloxide::{payloads, prelude::*};
use teloxide::dispatching::UpdateHandler;
use teloxide::requests::MultipartRequest;
use teloxide::types::{Animation, Document, InputFile, PhotoSize, ReplyParameters, Sticker, Video};
use teloxide::utils::command::BotCommands;
use tokio::io::AsyncWriteExt;

use crate::download::{download_file, path_to_filename, FileName};
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

            bot.send_message(msg.chat.id, "接下來，我會將貼紙轉換為圖片或動圖、將圖片或動圖轉換為貼紙。\n請發送想要轉換的貼紙、圖片或動圖～\n如果要退出轉換，請輸入 /exit 來退出轉換模式～").await?;
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

            bot.send_message(msg.chat.id, "接下來請發送一張想要下載的貼紙包中的貼紙～\n如果要退出下載，請輸入 /exit 來退出貼紙包下載模式～").await?;
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
            } else if let Some(photos) = msg.photo() {
                photo_to_sticker_processor(bot, &msg, photos).await?;
            } else if let Some(animation) = msg.animation() {
                animation_to_sticker_processor(bot, &msg, animation).await?;
            } else if let Some(video) = msg.video() {
                video_to_sticker_processor(bot, &msg, video).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送貼紙、圖片或動圖～ 如果要退出轉換，請輸入 /exit 來退出轉換模式～").await?;
            }
        },
        ChatStickerState::StickerSetDownload => {
            if let Some(sticker) = msg.sticker() {
                sticker_set_download_processor(bot, &msg, sticker).await?;
            } else {
                bot.send_message(msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～ 如果要退出下載，請輸入 /exit 來退出貼紙包下載模式～").await?;
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
        "[ChatID: {}, {}] Requested sticker to media conversion", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
    );

    let (content, file_name) = download_file(bot.clone(), sticker.file.id.clone()).await?;

    if file_name.extension != "webp" && file_name.extension != "webm" {
        bot.send_message(msg.chat.id, "現在還不支援 WebP 和 WebM 格式外的貼紙哦……").await?;
        return Ok(());
    }

    let new_file_basename = format!(
        "{}_{}_{}",
        sticker.set_name.clone().unwrap_or("noset".to_string()),
        msg.chat.id,
        msg.id.0
    );

    // Save to temp
    let source_name = format!("{}_source.{}", new_file_basename, file_name.extension);
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

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {}] Requested media to sticker conversion with document {}", 
         msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), doc.file_name.clone().unwrap_or("(NULL)".to_string())
    );

    let mut file = download_file(bot.clone(), doc.file.id.clone()).await?;

    if let Some(file_name) = path_to_filename(doc.file_name.clone().unwrap_or_default().as_str()) {
        file.1 = file_name
    }

    file_to_sticker_processor(bot, msg, file).await?;

    Ok(())
}

async fn photo_to_sticker_processor(
    bot: Bot,
    msg: &Message,
    photos: &[PhotoSize]
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {}] Requested media to sticker conversion with {} photos", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), photos.len()
    );

    // Choose the largest one of the photo
    let mut selected_photo: Option<&PhotoSize> = None;
    for photo in photos {
        if photo.width > selected_photo.map(|photo| photo.width).unwrap_or_default() &&
           photo.height > selected_photo.map(|photo| photo.height).unwrap_or_default() {
            selected_photo.replace(photo);
        }
    }

    if let Some(photo) = selected_photo {
        let file = download_file(bot.clone(), photo.file.id.clone()).await?;
        file_to_sticker_processor(bot.clone(), msg, file).await?;
    } else {
        log::warn!(
            target: "media_to_sticker",
            "[ChatID: {}, {}] Failed to select a photo", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
        );
        bot.send_message(msg.chat.id, "似乎沒有看到有圖片或動圖呢……").await?;
    }

    Ok(())
}

async fn animation_to_sticker_processor(
    bot: Bot,
    msg: &Message,
    anim: &Animation
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {}] Requested media to sticker conversion with animation", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
    );

    let mut file = download_file(bot.clone(), anim.file.id.clone()).await?;

    if let Some(file_name) = path_to_filename(anim.file_name.clone().unwrap_or_default().as_str()) {
        file.1 = file_name
    }

    file_to_sticker_processor(bot, msg, file).await?;

    Ok(())
}

async fn video_to_sticker_processor(
    bot: Bot,
    msg: &Message,
    video: &Video
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {}] Requested media to sticker conversion with video", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
    );

    let mut file = download_file(bot.clone(), video.file.id.clone()).await?;

    if let Some(file_name) = path_to_filename(video.file_name.clone().unwrap_or_default().as_str()) {
        file.1 = file_name
    }

    file_to_sticker_processor(bot, msg, file).await?;

    Ok(())
}

async fn file_to_sticker_processor(
    bot: Bot,
    msg: &Message,
    file: (Vec<u8>, FileName)
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    let (content, file_name) = file;

    const SUPPORTED_FORMAT: &[&'static str] = &["png", "jpg", "webp", "gif", "mp4", "webm"];
    const STATIC_SOURCE_FORMAT: &[&'static str] = &["png", "jpg", "webp"];
    const VIDEO_SOURCE_FORMAT: &[&'static str] = &["gif", "mp4", "webm"];

    if SUPPORTED_FORMAT.iter().all(|supported| file_name.extension.ne(supported)) {
        bot.send_message(msg.chat.id, format!("目前只支援 {} 格式的圖片或動圖呢……", SUPPORTED_FORMAT.join(" "))).await?;
    }

    // Save to temp
    let basename = format!("{}_{}_{}", file_name.basename, msg.chat.id, msg.id);

    let source_name = format!("{}_source.{}", basename, file_name.extension);
    let mut source_file = TempFile::new_with_name(&source_name).await?;
    source_file.write_all(&content).await?;
    let source_path = source_file.file_path().to_string_lossy();

    if STATIC_SOURCE_FORMAT.iter().any(|supported| file_name.extension.eq(supported)) {
        let webp_name = format!("{}.webp", basename);
        let webp_file = TempFile::new_with_name(&webp_name).await?;
        let webp_path = webp_file.file_path().to_string_lossy();

        log::info!(
            target: "media_to_sticker",
            "[ChatID: {}, {}] Converting {} to {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), source_path, webp_path
        );

        let conversion = tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-vf", "scale=512:512:force_original_aspect_ratio=1", "-y", &webp_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
            .wait().await?;
        if !conversion.success() {
            bot.send_message(msg.chat.id, "文件轉碼失敗惹……").await?;
            return Ok(())
        }

        log::info!(
            target: "media_to_sticker",
            "[ChatID: {}, {}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), webp_path
        );

        let upload_webp_file = InputFile::file(webp_file.file_path());

        let sticker_payload = payloads::SendSticker::new(msg.chat.id, upload_webp_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), sticker_payload).await?;
        
    } else if VIDEO_SOURCE_FORMAT.iter().any(|supported| file_name.extension.eq(supported)) {
        let webm_name = format!("{}.webm", basename);
        let webm_file = TempFile::new_with_name(&webm_name).await?;
        let webm_path = webm_file.file_path().to_string_lossy();

        log::info!(
            target: "media_to_sticker",
            "[ChatID: {}, {}] Converting {} to {}", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), source_path, webm_path
        );

        let conversion = tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-vf", "scale=512:512:force_original_aspect_ratio=1", "-c:v", "libvpx-vp9", "-an", "-y", &webm_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
            .wait().await?;
        if !conversion.success() {
            bot.send_message(msg.chat.id, "文件轉碼失敗惹……").await?;
            return Ok(())
        }

        let upload_webm_file = InputFile::file(webm_file.file_path());

        let sticker_payload = payloads::SendSticker::new(msg.chat.id, upload_webm_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), sticker_payload).await?;
    }

    
    Ok(())
}