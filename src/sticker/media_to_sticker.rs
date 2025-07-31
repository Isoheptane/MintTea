use std::process::Stdio;

use async_tempfile::TempFile;
use teloxide::{payloads, prelude::*};
use teloxide::requests::MultipartRequest;
use teloxide::types::{Animation, Document, InputFile, PhotoSize, ReplyParameters, Video};
use tokio::io::AsyncWriteExt;

use crate::download::{download_file, path_to_filename, FileName};

pub async fn document_to_sticker_processor(
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

pub async fn photo_to_sticker_processor(
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

pub async fn animation_to_sticker_processor(
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

pub async fn video_to_sticker_processor(
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