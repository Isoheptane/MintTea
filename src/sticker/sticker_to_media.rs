use std::process::Stdio;
use std::sync::Arc;

use frankenstein::methods::{SendDocumentParams, SendPhotoParams};
use frankenstein::stickers::Sticker;
use frankenstein::types::Message;
use frankenstein::AsyncTelegramApi;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::helper::download::download_telegram_file_to_path;
use crate::helper::download::get_telegram_file_info;
use crate::helper::log::LogSource;
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
        "{} Requested sticker to media conversion", 
        LogSource(&msg)
    );

    let file = match get_telegram_file_info(&ctx.bot, &sticker.file_id).await {
        Ok(Some(x)) => x,
        Ok(None) => {
            bot_actions::send_message(&ctx.bot, msg.chat.id, "好像找不到那個文件呢……").await?;
            return Ok(())
        }
        Err(e) => {
            log::error!(
                target: "sticker_to_media",
                "Failed to get file info with file_id {}: {e}", 
                sticker.file_id
            );
            bot_actions::send_message(&ctx.bot, msg.chat.id, "獲取文件信息失敗惹……").await?;
            return Ok(());
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

    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;

    // The sticker set name may be None
    let set_name = sticker.set_name.clone().unwrap_or("noset".to_string());
    let basename = format!("{}_{}_{}", set_name, msg.chat.id, msg.message_id);

    let input_name = format!("{}_input.{}", basename, base_ext.extension_str());
    let input_path = temp_dir.path().join(&input_name);
    let input_path_str = input_path.to_string_lossy();

    if let Err(e) = download_telegram_file_to_path(&ctx, &file.file_path, &input_path).await {
        log::error!(
            target: "sticker_to_media",
            "Failed to download file from path {}: {e}", 
            file.file_path
        );
        bot_actions::send_message(&ctx.bot, msg.chat.id, "下載文件失敗惹……").await?;
        return Ok(());
    }

    let output_name = format!("{}_output.{}", basename, if is_animated { "gif" } else { "png" });
    let output_path = temp_dir.path().join(&output_name);
    let output_path_str = output_path.to_string_lossy();

    log::info!(
        target: "sticker_to_media",
        "{} Converting {} to {}", 
        LogSource(&msg), input_path_str, output_path_str
    );

    // TODO: make different for gif
    let ffmpeg_args = if is_animated {
        vec!["-i", &input_path_str, "-y", &output_path_str]
    } else {
        vec!["-i", &input_path_str, "-y", &output_path_str]
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
        "{} Uploading converted file {}", 
        LogSource(&msg), output_name
    );

    // Always try send output document, GIF may be re-encoded
    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(output_path.clone())
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_document(&send_document_param).await?;

    if is_animated {

        let archive_file_name = format!("{}.zip", basename);
        let archive_path = temp_dir.path().join(&archive_file_name);
        let archive_path_clone = archive_path.clone();
        // Blocking zip and write operation
        let archiving_task = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let archive_file = std::fs::File::create(archive_path_clone)?;
            let mut archive = zip::ZipWriter::new(archive_file);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o755);

            let mut input_file= std::fs::File::open(input_path)?;
            let mut output_file = std::fs::File::open(output_path)?;
            
            archive.start_file(input_name, options)?;
            std::io::copy(&mut input_file, &mut archive)?;
            archive.start_file(output_name, options)?;
            std::io::copy(&mut output_file, &mut archive)?;

            archive.finish()?;
            Ok(())
        });
        
        // Notice, JoinError is passed up by "?"
        if let Err(e) = archiving_task.await? {
            log::warn!(
                target: "sticker_set_download",
                "{} Failed to archive file {}: {}", 
                LogSource(&msg), archive_file_name, e
            );
            return Ok(())
        }

        let send_document_param = SendDocumentParams::builder()
            .chat_id(msg.chat.id)
            .document(archive_path)
            .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
            .build();
        ctx.bot.send_document(&send_document_param).await?;
        
    } else {
        // Send a preview photo
        let send_photo_param = SendPhotoParams::builder()
            .chat_id(msg.chat.id)
            .photo(output_path)
            .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
            .build();
        ctx.bot.send_photo(&send_photo_param).await?;
    }
    
    bot_actions::send_message(&ctx.bot, msg.chat.id, "轉換完成啦～\n您可以繼續發送要轉換的貼紙～\n如果要退出，請點擊指令 -> /exit").await?;

    Ok(())
}