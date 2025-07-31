use std::collections::VecDeque;
use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_tempfile::TempFile;
use teloxide::{payloads, prelude::*};
use teloxide::dispatching::UpdateHandler;
use teloxide::requests::MultipartRequest;
use teloxide::types::{Document, FileMeta, InputFile, ReplyParameters, Sticker};
use teloxide::utils::command::BotCommands;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::download::{download_file, FileName};
use crate::shared::SharedData;
use crate::shared::ChatState;

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
            bot.send_message(msg.chat.id, "接下來，我會將貼紙轉換為圖片或動圖、將圖片或動圖轉換為貼紙。\n請發送想要轉換的貼紙、圖片或動圖～").await?;
        },
        StickerCommand::StickerSetDownload => {
            shared.chat_state_storage.set_state(
                msg.chat.id, 
                ChatState::Sticker(ChatStickerState::StickerSetDownload)
            ).await;
            bot.send_message(msg.chat.id, "接下來請發送一張想要下載的貼紙包中的貼紙～ 輸入 /exit 退出。").await?;
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
                sticker_to_picture_processor(bot, &msg, sticker).await?;
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
                bot.send_message(msg.chat.id, "請發送一張想要下載的貼紙包中的貼紙～ 如果想要退出下載，請輸入 /exit 來退出轉換模式～").await?;
            }
        },
    }
    
    Ok(())
}

async fn sticker_to_picture_processor(
    bot: Bot,
    msg: &Message,
    sticker: &Sticker
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

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
    let mut source_file = TempFile::new_with_name(source_name).await?;
    source_file.write_all(&content).await?;
    let source_path = source_file.file_path().to_string_lossy();
    
    // Picture
    if file_name.extension == "webp" {
        let png_name = format!("{}.png", new_file_basename);
        let png_file = TempFile::new_with_name(png_name).await?;
        let png_path = png_file.file_path().to_string_lossy();

        log::debug!("Converting {} to {}", source_path, png_path);
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

        let document_payload = payloads::SendDocument::new(msg.chat.id, upload_file.clone())
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), document_payload).await?;

        let photo_payload = payloads::SendPhoto::new(msg.chat.id, upload_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), photo_payload).await?;

        return Ok(());
    }

    // Animation
    if file_name.extension == "webm" {
        let gif_name = format!("{}.gif", new_file_basename);
        let gif_file = TempFile::new_with_name(gif_name).await?;
        let gif_path = gif_file.file_path().to_string_lossy();

        log::debug!("Converting {} to {}", source_path, gif_path);
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

        let webm_payload = payloads::SendDocument::new(msg.chat.id, upload_webm_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), webm_payload).await?;

        let gif_payload = payloads::SendAnimation::new(msg.chat.id, upload_gif_file)
            .reply_parameters(ReplyParameters::new(msg.id));
        MultipartRequest::new(bot.clone(), gif_payload).await?;

        return Ok(());
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

#[derive(Debug, Clone)]
struct StickerDownloadResult {
    sticker_no: usize,
    content: Vec<u8>,
    file_name: FileName
}

async fn sticker_set_download_processor(
    bot: Bot,
    msg: &Message,
    sticker: &Sticker
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let set_name = match &sticker.set_name {
        Some(x) => x,
        None => {
            bot.send_message(msg.chat.id, "這張貼紙不屬於任何貼紙包呢……").await?;
            return Ok(());
        }
    };

    let set = match bot.get_sticker_set(set_name).await {
        Ok(x) => x,
        Err(e) => {
            log::warn!("Get sticker set failed: {}", e);
            bot.send_message(msg.chat.id, "似乎找不到那個貼紙包呢……").await?;
            return Ok(());
        }
    };

    let sticker_count = set.stickers.len();

    // Allocate missions
    let sticker_queue: VecDeque<(usize, FileMeta)> = set.stickers
        .into_iter()
        .enumerate()
        .map(|(i, sticker)| (i, sticker.file))
        .collect();
    let queue: Arc<Mutex<VecDeque<(usize, FileMeta)>>> = Arc::new(Mutex::new(sticker_queue));
    let results = Arc::new(Mutex::new(Vec::<StickerDownloadResult>::new()));

    // Concurrent download stickers
    let progress_message = bot.send_message(msg.chat.id, format!("開始下載貼紙 (共 {} 張）……", sticker_count)).await?;

    let mut join_handle_list = vec![];
    const WORKER_COUNT: usize = 8;
    for worker_id in 0..WORKER_COUNT {
        let bot_cloned = bot.clone();
        let ref_queue = queue.clone();
        let ref_results = results.clone();
        join_handle_list.push(tokio::spawn(async move {
            sticker_download_worker(bot_cloned, worker_id, ref_queue, ref_results).await;
        }))
    }

    loop {
        if join_handle_list.iter().map(|h| h.is_finished()).all(|s| s == true) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        let count = results.lock().await.len();
        bot.edit_message_text(
            msg.chat.id, 
            progress_message.id,
            format!("正在下載貼紙喵…… ({}/{})", count, sticker_count)
        ).await?;
    }

    // Add stickers to archive
    let mut archive_data = Vec::<u8>::new();
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(&mut archive_data));
    for result in results.lock().await.iter() {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);
        archive.start_file(format!("{}_{}.{}", set_name, result.sticker_no, result.file_name.extension), options)?;
        archive.write_all(&result.content)?;
    }

    // Download thumbnail if exists
    if let Some(thumbnail) = set.thumbnail {
        let (content, file_name) = download_file(bot.clone(), thumbnail.file.id).await?;
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);
        archive.start_file(format!("{}_thumbnail.{}", set_name, file_name.extension), options)?;
        archive.write_all(&content)?;
    }

    archive.finish()?;

    bot.edit_message_text(
        msg.chat.id, 
        progress_message.id,
        "貼紙下載完成了～"
    ).await?;

    let archive_file = InputFile::memory(archive_data).file_name(format!("{}.zip", set_name));
    let webm_payload = payloads::SendDocument::new(msg.chat.id, archive_file)
        .reply_parameters(ReplyParameters::new(msg.id));
    MultipartRequest::new(bot.clone(), webm_payload).await?;

    Ok(())
}

async fn sticker_download_worker(
    bot: Bot,
    worker_id: usize,
    queue: Arc<Mutex<VecDeque<(usize, FileMeta)>>>,
    results: Arc<Mutex<Vec<StickerDownloadResult>>>
) {
    loop {
        let task = {
            let mut guard = queue.lock().await;
            guard.pop_front()
        };
        let (sticker_no, file_meta) = match task {
            Some(task) => task,
            None => { return }
        };

        let (content, file_name) = match download_file(bot.clone(), file_meta.id).await {
            Ok(x) => x,
            Err(e) => {
                log::warn!(
                    "Sticker download worker {} failed to download sticker #{}: {}",
                    worker_id, sticker_no, e
                );
                continue;
            }
        };
        
        {
            let mut guard = results.lock().await;
            guard.push(StickerDownloadResult { 
                sticker_no: sticker_no, 
                content: content, 
                file_name: file_name 
            });
        }
    }
}