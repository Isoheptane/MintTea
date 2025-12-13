use std::process::Stdio;
use std::sync::Arc;

use frankenstein::methods::SendStickerParams;
use frankenstein::types::{Animation, Document, Message, PhotoSize, Video};
use frankenstein::AsyncTelegramApi;

use crate::helper::download::{download_telegram_file_to_path, get_telegram_file_info};
use crate::helper::log::LogSource;
use crate::helper::{bot_actions, param_builders};
use crate::context::Context;
use crate::types::FileName;

pub async fn document_to_sticker_processor(
    ctx: Arc<Context>,
    msg: &Message,
    doc: &Document
) -> anyhow::Result<()> {

    log::info!(
        target: "media_to_sticker",
        "{} Requested media to sticker conversion with document {:?}", 
        LogSource(&msg), doc.file_name
    );

    file_to_sticker_processor(ctx, msg, doc.file_id.clone(), doc.file_name.clone()).await?;

    Ok(())
}

pub async fn photo_to_sticker_processor(
    ctx: Arc<Context>,
    msg: &Message,
    photos: &[PhotoSize]
) -> anyhow::Result<()> {

    log::info!(
        target: "media_to_sticker",
        "{} Requested media to sticker conversion with {} photos", 
        LogSource(&msg), photos.len()
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
        file_to_sticker_processor(ctx, msg, photo.file_id.clone(), None).await?;
    } else {
        log::warn!(
            target: "media_to_sticker",
            "{} Failed to select a photo", 
            LogSource(&msg)
        );
        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有看到有圖片或動圖呢……").await?;
    }

    Ok(())
}

pub async fn animation_to_sticker_processor(
    ctx: Arc<Context>,
    msg: &Message,
    anim: &Animation
) -> anyhow::Result<()> {

    log::info!(
        target: "media_to_sticker",
        "{} Requested media to sticker conversion with animation", 
        LogSource(&msg)
    );

    file_to_sticker_processor(ctx, msg, anim.file_id.clone(), anim.file_name.clone()).await?;

    Ok(())
}

pub async fn video_to_sticker_processor(
    ctx: Arc<Context>,
    msg: &Message,
    video: &Video
) -> anyhow::Result<()> {

    log::info!(
        target: "media_to_sticker",
        "{} Requested media to sticker conversion with video", 
        LogSource(&msg)
    );

    file_to_sticker_processor(ctx, msg, video.file_id.clone(), video.file_name.clone()).await?;

    Ok(())
}

async fn file_to_sticker_processor(
    ctx: Arc<Context>,
    msg: &Message,
    file_id: String,
    media_file_name: Option<String>,
) -> anyhow::Result<()> {

    let file = match get_telegram_file_info(&ctx.bot, &file_id).await {
        Ok(Some(x)) => x,
        Ok(None) => {
            bot_actions::send_message(&ctx.bot, msg.chat.id, "好像找不到那個文件呢……").await?;
            return Ok(());
        }
        Err(e) => {
            log::error!(
                target: "sticker_to_media",
                "Failed to get file info with file_id {}: {e}", 
                file_id
            );
            bot_actions::send_message(&ctx.bot, msg.chat.id, "獲取文件信息失敗惹……").await?;
            return Ok(());
        }
    };

    let mut file_name = file.file_name;
    if let Some(media_file_name) = media_file_name {
        file_name = media_file_name;
    }
    let file_name = FileName::from(file_name);
    
    // Identify file type
    const STATIC_SOURCE_FORMAT: &[&'static str] = &["png", "jpg", "webp"];
    const VIDEO_SOURCE_FORMAT: &[&'static str] = &["gif", "mp4", "webm"];

    let is_animated = if VIDEO_SOURCE_FORMAT.iter().any(|supported| file_name.extension_str().to_ascii_lowercase() == *supported) {
        true
    } else if STATIC_SOURCE_FORMAT.iter().any(|supported| file_name.extension_str().to_ascii_lowercase() == *supported) {
        false
    } else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, 
            format!(
                "目前只支援 {} 格式的圖片和 {} 格式的動圖呢……", 
                STATIC_SOURCE_FORMAT.join(" "),
                VIDEO_SOURCE_FORMAT.join(" ")
            )
        ).await?;
        return Ok(());
    };

    // Size limit
    if file.file_size > ctx.config.sticker.size_limit_kb * 1024 {
        bot_actions::send_message(&ctx.bot, msg.chat.id, 
            format!(
                "目前只支持最大 {} KiB 的文件呢……", 
                ctx.config.sticker.size_limit_kb
            )
        ).await?;
        return Ok(());
    }

    // predefine the file at first
    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;
    let input_name = format!("{}_input.{}", file_name.basename, file_name.extension_str());
    let input_path = temp_dir.path().to_path_buf().join(&input_name);
    let input_path_str = input_path.to_string_lossy();

    // input file handle is intentionally ignored as we won't use it right now
    if let Err(e) = download_telegram_file_to_path(&ctx, &file.file_path, &input_path).await {
        log::error!(
            target: "sticker_to_media",
            "Failed to download file from path {}: {e}", 
            file.file_path
        );
        bot_actions::send_message(&ctx.bot, msg.chat.id, "下載文件失敗惹……").await?;
        return Ok(());
    }

    // Start conversion
    let output_name = format!("{}_output.{}", file_name.basename, if is_animated { "webm" } else { "webp" });
    let output_path = temp_dir.path().to_path_buf().join(&output_name);
    let output_path_str = output_path.to_string_lossy();

    let ffmpeg_args = if is_animated {
        vec![
            "-i", &input_path_str, 
            "-vf", "scale=512:512:force_original_aspect_ratio=1", "-c:v", "libvpx-vp9", "-an", 
            "-y", &output_path_str
        ]
    } else {
        vec![
            "-i", &input_path_str, 
            "-vf", "scale=512:512:force_original_aspect_ratio=1", 
            "-y", &output_path_str
        ]
    };

    log::info!(
        target: "media_to_sticker",
        "{} Converting {} to {}", 
        LogSource(&msg), input_name, output_name
    );

    let conversion = tokio::process::Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?
        .wait().await?;
    if !conversion.success() {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "文件轉碼失敗惹……").await?;
        return Ok(())
    }

    // Send sticker
    let send_sticker_param = SendStickerParams::builder()
        .chat_id(msg.chat.id)
        .sticker(output_path_str.to_string())
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_sticker(&send_sticker_param).await?;

    bot_actions::send_message(&ctx.bot, msg.chat.id, "轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;
    
    Ok(())
}