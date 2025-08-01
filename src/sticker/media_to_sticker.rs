use std::process::Stdio;

use async_tempfile::TempFile;
use teloxide::{payloads, prelude::*};
use teloxide::requests::MultipartRequest;
use teloxide::types::{Animation, Document, FileMeta, InputFile, PhotoSize, ReplyParameters, Video};
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

    let file_name= path_to_filename(doc.file_name.clone().unwrap_or_default().as_str());

    file_to_sticker_processor(bot, msg, doc.file.clone(), file_name).await?;

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
        file_to_sticker_processor(bot.clone(), msg, photo.file.clone(), None).await?;
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

    let file_name = path_to_filename(anim.file_name.clone().unwrap_or_default().as_str());

    file_to_sticker_processor(bot, msg, anim.file.clone(), file_name).await?;

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

    let file_name = path_to_filename(video.file_name.clone().unwrap_or_default().as_str());

    file_to_sticker_processor(bot, msg, video.file.clone(), file_name).await?;

    Ok(())
}

async fn file_to_sticker_processor(
    bot: Bot,
    msg: &Message,
    file: FileMeta,
    media_file_name: Option<FileName>
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    let (content, mut file_name) = match download_file(bot.clone(), file.id).await {
        Ok(x) => x,
        Err(e) => {
            log::error!("Failed to download media file: {}", e);
            bot.send_message(msg.chat.id, "文件下載失敗惹……").await?;
            return Ok(())
        }
    };

    if let Some(media_file_name) = media_file_name {
        file_name = media_file_name;
    }
    
    // Identify file type
    const STATIC_SOURCE_FORMAT: &[&'static str] = &["png", "jpg", "webp"];
    const VIDEO_SOURCE_FORMAT: &[&'static str] = &["gif", "mp4", "webm"];

    let is_animated = {
        if VIDEO_SOURCE_FORMAT.iter().any(|supported| file_name.extension.eq(supported)) {
            true
        } else if STATIC_SOURCE_FORMAT.iter().any(|supported| file_name.extension.eq(supported)) {
            false
        } else {
            bot.send_message(msg.chat.id, format!(
                "目前只支援 {} 格式的圖片和 {} 格式的動圖呢……", 
                STATIC_SOURCE_FORMAT.join(" "),
                VIDEO_SOURCE_FORMAT.join(" ")
            )).await?;
            return Ok(());
        }
    };

    // Save to temp
    let basename = format!("{}_{}_{}", file_name.basename, msg.chat.id, msg.id);

    let source_name = format!("{}_source.{}", basename, file_name.extension);
    let mut source_file = TempFile::new_with_name(&source_name).await?;
    source_file.write_all(&content).await?;
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
        "[ChatID: {}, {}] Converting {} to {}", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), source_path, output_path
    );

    let conversion = tokio::process::Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?
        .wait().await?;
    if !conversion.success() {
        bot.send_message(msg.chat.id, "文件轉碼失敗惹……").await?;
        return Ok(())
    }

    // Finally upload
    let upload_file = InputFile::file(output_file.file_path());

    let sticker_payload = payloads::SendSticker::new(msg.chat.id, upload_file)
        .reply_parameters(ReplyParameters::new(msg.id));
    MultipartRequest::new(bot.clone(), sticker_payload).await?;

    bot.send_message(msg.chat.id, "轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
    
    Ok(())
}