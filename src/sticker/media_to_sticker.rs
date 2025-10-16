use std::process::Stdio;
use std::sync::Arc;

use async_tempfile::TempFile;
use frankenstein::client_reqwest::Bot;
use frankenstein::methods::{SendMessageParams, SendStickerParams};
use frankenstein::types::{Animation, Document, Message, PhotoSize, ReplyParameters, Video};
use frankenstein::AsyncTelegramApi;
use tokio::io::AsyncWriteExt;

use crate::download::{download_file, FileBaseExt};
use crate::shared::SharedData;

pub async fn document_to_sticker_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    doc: &Document
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {:?}] Requested media to sticker conversion with document {:?}", 
        msg.chat.id, msg.chat.username, doc.file_name
    );

    file_to_sticker_processor(bot, data, msg, doc.file_id.clone(), doc.file_name.clone()).await?;

    Ok(())
}

pub async fn photo_to_sticker_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    photos: &[PhotoSize]
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {:?}] Requested media to sticker conversion with {} photos", 
        msg.chat.id, msg.chat.username, photos.len()
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
        file_to_sticker_processor(bot.clone(), data, msg, photo.file_id.clone(), None).await?;
    } else {
        log::warn!(
            target: "media_to_sticker",
            "[ChatID: {}, {:?}] Failed to select a photo", 
            msg.chat.id, msg.chat.username
        );
        let send_message_params = SendMessageParams::builder()
            .chat_id(msg.chat.id)
            .text("似乎沒有看到有圖片或動圖呢……")
            .build();
        bot.send_message(&send_message_params).await?;
    }

    Ok(())
}

pub async fn animation_to_sticker_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    anim: &Animation
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {:?}] Requested media to sticker conversion with animation", 
        msg.chat.id, msg.chat.username
    );

    file_to_sticker_processor(bot, data, msg, anim.file_id.clone(), anim.file_name.clone()).await?;

    Ok(())
}

pub async fn video_to_sticker_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    video: &Video
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {:?}] Requested media to sticker conversion with video", 
        msg.chat.id, msg.chat.username
    );

    file_to_sticker_processor(bot, data, msg, video.file_id.clone(), video.file_name.clone()).await?;

    Ok(())
}

async fn file_to_sticker_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    file_id: String,
    media_file_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    let file = match download_file(bot.clone(), data, &file_id).await {
        Ok(x) => x,
        Err(e) => {
            log::error!("Failed to download media file: {}", e);
            let send_message_params = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text("文件下載失敗惹……")
                .build();
            bot.send_message(&send_message_params).await?;
            return Ok(())
        }
    };

    let (file_content, mut file_name) = match file {
        Some(x) => x,
        None => {
            log::warn!("File path is empty for file_id {}", &file_id);
            let send_message_params = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text("文件下載失敗惹……")
                .build();
            bot.send_message(&send_message_params).await?;
            return Ok(())
        }
    };

    if let Some(media_file_name) = media_file_name {
        file_name = media_file_name;
    }

    let base_ext = FileBaseExt::from(file_name);
    
    // Identify file type
    const STATIC_SOURCE_FORMAT: &[&'static str] = &["png", "jpg", "webp"];
    const VIDEO_SOURCE_FORMAT: &[&'static str] = &["gif", "mp4", "webm"];

    let is_animated = {
        if VIDEO_SOURCE_FORMAT.iter().any(|supported| base_ext.extension.eq(supported)) {
            true
        } else if STATIC_SOURCE_FORMAT.iter().any(|supported| base_ext.extension.eq(supported)) {
            false
        } else {
            let msg_text = format!(
                "目前只支援 {} 格式的圖片和 {} 格式的動圖呢……", 
                STATIC_SOURCE_FORMAT.join(" "),
                VIDEO_SOURCE_FORMAT.join(" ")
            );
            let send_message_params = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text(msg_text)
                .build();
            bot.send_message(&send_message_params).await?;
            return Ok(());
        }
    };

    // Save to temp
    let basename = format!("{}_{}_{}", base_ext.basename, msg.chat.id, msg.message_id);

    let source_name = format!("{}_source.{}", basename, base_ext.extension);
    let mut source_file = TempFile::new_with_name(&source_name).await?;
    source_file.write_all(&file_content).await?;
    let source_path = source_file.file_path().to_string_lossy();

    // Start conversion
    let output_name = format!("{}.{}", basename, if is_animated { "webm" } else { "webp" });
    let output_file = TempFile::new_with_name(&output_name).await?;
    let output_path = output_file.file_path().to_string_lossy();

    let ffmpeg_args = if is_animated {
        vec!["-i", &source_path, "-vf", "scale=512:512:force_original_aspect_ratio=1", "-c:v", "libvpx-vp9", "-an", "-y", &output_path]
    } else {
        vec!["-i", &source_path, "-vf", "scale=512:512:force_original_aspect_ratio=1", "-y", &output_path]
    };

    log::info!(
        target: "media_to_sticker",
        "[ChatID: {}, {:?}] Converting {} to {}", 
        msg.chat.id, msg.chat.username, source_path, output_path
    );

    let conversion = tokio::process::Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?
        .wait().await?;
    if !conversion.success() {
        let send_message_params = SendMessageParams::builder()
            .chat_id(msg.chat.id)
            .text("文件轉碼失敗惹……")
            .build();
        bot.send_message(&send_message_params).await?;
        return Ok(())
    }

    // Send sticker
    let send_sticker_param = SendStickerParams::builder()
        .chat_id(msg.chat.id)
        .sticker(output_file.file_path().clone())
        .reply_parameters(ReplyParameters::builder().message_id(msg.message_id).build())
        .build();
    bot.send_sticker(&send_sticker_param).await?;

    let send_message_params = SendMessageParams::builder()
        .chat_id(msg.chat.id)
        .text("轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit")
        .build();
    bot.send_message(&send_message_params).await?;
    
    Ok(())
}