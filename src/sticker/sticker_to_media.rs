use std::process::Stdio;
use std::sync::Arc;

use async_tempfile::TempFile;
use frankenstein::client_reqwest::Bot;
use frankenstein::methods::{SendDocumentParams, SendMessageParams, SendPhotoParams};
use frankenstein::stickers::Sticker;
use frankenstein::types::{Message, ReplyParameters};
use frankenstein::AsyncTelegramApi;
use tokio::io::AsyncWriteExt;

use crate::download::{download_file, FileBaseExt};
use crate::shared::SharedData;

pub async fn sticker_to_media_processor(
    bot: Bot,
    data: Arc<SharedData>,
    msg: &Message,
    sticker: &Sticker
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    log::info!(
        target: "sticker_to_media",
        "[ChatID: {}, {:?}] Requested sticker to media conversion", 
        msg.chat.id, msg.chat.username
    );

    let file = download_file(bot.clone(), data, &sticker.file_id).await?;

    let (file_content, file_name) = match file {
        Some(x) => x,
        None => {
            log::warn!("File path is empty for file_id {}", &sticker.file_id);
            let send_message_params = SendMessageParams::builder()
                .chat_id(msg.chat.id)
                .text("文件下載失敗惹……")
                .build();
            bot.send_message(&send_message_params).await?;
            return Ok(())
        }
    };

    let base_ext = FileBaseExt::from(file_name);

    if base_ext.extension.ne("webp") && base_ext.extension.ne("webm") {
        let send_message_params = SendMessageParams::builder()
            .chat_id(msg.chat.id)
            .text("現在還不支援 WebP 和 WebM 格式外的貼紙哦……")
            .build();
        bot.send_message(&send_message_params).await?;
        return Ok(());
    }

    let new_file_basename = format!(
        "{}_{}_{}",
        sticker.set_name.clone().unwrap_or("noset".to_string()),
        msg.chat.id,
        msg.message_id
    );

    // Save to temp
    let source_name = format!("{}_source.{}", new_file_basename, base_ext.extension);
    let mut source_file = TempFile::new_with_name(&source_name).await?;
    source_file.write_all(&file_content).await?;
    let source_path = source_file.file_path().to_string_lossy();
    
    // Picture
    if base_ext.extension == "webp" {
        let png_name = format!("{}.png", new_file_basename);
        let png_file = TempFile::new_with_name(&png_name).await?;
        let png_path = png_file.file_path().to_string_lossy();

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {:?}] Converting {} to {}", 
            msg.chat.id, msg.chat.username, source_path, png_path
        );

        let conversion = tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-y", &png_path])
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

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {:?}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username, png_name
        );

        let send_document_param = SendDocumentParams::builder()
            .chat_id(msg.chat.id)
            .document(png_file.file_path().clone())
            .reply_parameters(ReplyParameters::builder().message_id(msg.message_id).build())
            .build();
        bot.send_document(&send_document_param).await?;

        let send_photo_param = SendPhotoParams::builder()
            .chat_id(msg.chat.id)
            .photo(png_file.file_path().clone())
            .reply_parameters(ReplyParameters::builder().message_id(msg.message_id).build())
            .build();
        bot.send_photo(&send_photo_param).await?;

        let send_message_params = SendMessageParams::builder()
            .chat_id(msg.chat.id)
            .text("轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit")
            .build();
        bot.send_message(&send_message_params).await?;

    } else if base_ext.extension == "webm" {
        let gif_name = format!("{}.gif", new_file_basename);
        let gif_file = TempFile::new_with_name(&gif_name).await?;
        let gif_path = gif_file.file_path().to_string_lossy();

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {:?}] Converting {} to {}", 
            msg.chat.id, msg.chat.username, source_path, gif_path
        );

        let conversion = tokio::process::Command::new("ffmpeg")
            .args(vec!["-i", &source_path, "-y", &gif_path])
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
        // after conversion

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {:?}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username, source_name
        );

        let send_document_param = SendDocumentParams::builder()
            .chat_id(msg.chat.id)
            .document(source_file.file_path().clone())
            .reply_parameters(ReplyParameters::builder().message_id(msg.message_id).build())
            .build();
        bot.send_document(&send_document_param).await?;

        log::info!(
            target: "sticker_to_media",
            "[ChatID: {}, {:?}] Uploading converted file {}", 
            msg.chat.id, msg.chat.username, gif_name
        );

        let send_document_param = SendDocumentParams::builder()
            .chat_id(msg.chat.id)
            .document(gif_file.file_path().clone())
            .reply_parameters(ReplyParameters::builder().message_id(msg.message_id).build())
            .build();
        bot.send_document(&send_document_param).await?;

        let send_message_params = SendMessageParams::builder()
            .chat_id(msg.chat.id)
            .text("轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit")
            .build();
        bot.send_message(&send_message_params).await?;

    }

    Ok(())
}