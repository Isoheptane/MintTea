use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use teloxide::{payloads, prelude::*};
use teloxide::requests::MultipartRequest;
use teloxide::types::{FileMeta, InputFile, ReplyParameters, Sticker};
use tokio::sync::Mutex;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::download::{download_file, FileName};

#[derive(Debug, Clone)]
struct StickerDownloadResult {
    sticker_no: usize,
    content: Vec<u8>,
    file_name: FileName
}

pub async fn sticker_set_download_processor(
    bot: Bot,
    msg: &Message,
    sticker: &Sticker
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {}] Requested sticker set download", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous")
    );
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

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {}] Sticker set name: {}, downloading sticker set", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), set.name
    );

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
    let progress_message = bot.send_message(msg.chat.id, format!("開始下載貼紙喵…… (共 {} 張）", sticker_count)).await?;

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

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {}] Sticker set name: {}, archiving sticker set", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), set.name
    );

    // Add stickers to archive
    let mut archive_data = Vec::<u8>::new();
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(&mut archive_data));
    for result in results.lock().await.iter() {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);
        archive.start_file(format!("{}_{:03}.{}", set_name, result.sticker_no, result.file_name.extension), options)?;
        archive.write_all(&result.content)?;
    }

    // Download thumbnail if exists
    if let Some(thumbnail) = set.thumbnail {
        log::info!(
            target: "sticker_set_download",
            "[ChatID: {}, {}] Sticker set name: {}, downloading thumbnail", 
            msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), set.name
        );
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

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {}] Sticker set name: {}, upolading archive", 
        msg.chat.id, msg.chat.username().unwrap_or("Anonymous"), set.name
    );

    let archive_file = InputFile::memory(archive_data).file_name(format!("{}.zip", set_name));
    let webm_payload = payloads::SendDocument::new(msg.chat.id, archive_file)
        .reply_parameters(ReplyParameters::new(msg.id));
    MultipartRequest::new(bot.clone(), webm_payload).await?;

    bot.send_message(msg.chat.id, "下載完成啦～\n您可以繼續發送要下載的貼紙包～\n如果要退出，請點擊指令 -> /exit").await?;

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
                    target: &format!("sticker_set_download worker#{}", worker_id),
                    "Failed to download sticker #{}: {}",
                    sticker_no, e
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