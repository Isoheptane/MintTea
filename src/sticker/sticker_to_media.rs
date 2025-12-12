use std::process::Stdio;
use std::sync::Arc;
use std::io::Write;
use std::io::Read;

use tempfile::NamedTempFile;
use frankenstein::methods::{SendDocumentParams, SendPhotoParams};
use frankenstein::stickers::Sticker;
use frankenstein::types::Message;
use frankenstein::AsyncTelegramApi;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;


use crate::helper::download::download_telegram_file;
use crate::helper::{bot_actions, param_builders};
use crate::context::Context;
use crate::types::FileName;

pub async fn sticker_to_media_processor(
    ctx: Arc<Context>,
    msg: &Message,
    sticker: &Sticker
) -> anyhow::Result<()> {

    log::info!(
        target: "sticker_to_media",
        "[ChatID: {}, {:?}] Requested sticker to media conversion", 
        msg.chat.id, msg.chat.username
    );

    let file = download_telegram_file(ctx.clone(), &sticker.file_id).await?;

    let file = match file {
        Some(x) => x,
        None => {
            log::warn!("File path is empty for file_id {}", &sticker.file_id);
            bot_actions::send_message(&ctx.bot, msg.chat.id, "文件下載失敗惹……").await?;
            return Ok(())
        }
    };

    let base_ext = FileName::from(file.file_name);

    let is_animated = if base_ext.extension_str().to_lowercase() == "webm" {
        true
    } else if base_ext.extension_str().to_lowercase() == "webp" {
        false
    } else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "現在還不支援 WebP 和 WebM 格式外的貼紙哦……").await?;
        return Ok(());
    };

    let basename = sticker.set_name.clone().unwrap_or("noset".to_string());
    // Save to temp
    let input_name = format!("{}_input.{}", basename, base_ext.extension_str());
    let mut input_file = NamedTempFile::with_suffix_in(&input_name, ctx.temp_dir.path())?;
    input_file.write_all(&file.data)?;
    let input_path = input_file.path().to_string_lossy();

    let output_name = format!("{}_output.{}", basename, if is_animated { "gif" } else { "png" });
    let mut output_file = NamedTempFile::with_suffix_in(&output_name, ctx.temp_dir.path())?;
    let output_path = output_file.path().to_string_lossy();

    log::info!(
        target: "sticker_to_media",
        "[ChatID: {}, {:?}] Converting {} to {}", 
        msg.chat.id, msg.chat.username, input_path, output_path
    );

    let ffmpeg_args = if is_animated {
        vec!["-i", &input_path, "-y", &output_path]
    } else {
        vec!["-i", &input_path, "-y", &output_path]
    };

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

    log::info!(
        target: "sticker_to_media",
        "[ChatID: {}, {:?}] Uploading converted file {}", 
        msg.chat.id, msg.chat.username, output_name
    );

    // Always try send output document, GIF may be re-encoded
    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(output_file.path().to_path_buf())
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_document(&send_document_param).await?;

    if is_animated {
        let mut output_data = Vec::<u8>::new();
        output_file.read_to_end(&mut output_data)?;

        // Make an archive for upload, wrap gif file and input file in a archive
        let mut archive_data = Vec::<u8>::new();
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(&mut archive_data));

        let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o755);

        archive.start_file(input_name, options)?;
        archive.write_all(&file.data)?;
        archive.start_file(output_name, options)?;
        archive.write_all(&output_data)?;
        archive.finish()?;

        // Make an archive file
        let mut archive_file = NamedTempFile::with_suffix_in(format!("{}.zip", basename), ctx.temp_dir.path())?;
        archive_file.write_all(&archive_data)?;

        let send_document_param = SendDocumentParams::builder()
            .chat_id(msg.chat.id)
            .document(archive_file.path().to_path_buf())
            .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
            .build();
        ctx.bot.send_document(&send_document_param).await?;
        
    } else {
        // Send a preview photo
        let send_photo_param = SendPhotoParams::builder()
            .chat_id(msg.chat.id)
            .photo(output_file.path().to_path_buf())
            .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
            .build();
        ctx.bot.send_photo(&send_photo_param).await?;
    }
    
    bot_actions::send_message(&ctx.bot, msg.chat.id, "轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;

    Ok(())
}