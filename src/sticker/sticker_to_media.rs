use std::process::Stdio;

use async_tempfile::TempFile;
use teloxide::{payloads, prelude::*};
use teloxide::requests::MultipartRequest;
use teloxide::types::{InputFile, ReplyParameters, Sticker};
use tokio::io::AsyncWriteExt;

use crate::download::download_file;

pub async fn sticker_to_media_processor(
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