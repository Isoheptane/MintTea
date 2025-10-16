use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use async_tempfile::TempFile;
use frankenstein::AsyncTelegramApi;
use frankenstein::stickers::Sticker;
use frankenstein::client_reqwest::Bot;
use frankenstein::types::Message;
use frankenstein::methods::{GetStickerSetParams, SendDocumentParams};
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::helper::{bot_actions, param_builders};
use crate::{download::{download_file, FileBaseExt}, shared::SharedData};

#[derive(Debug, Clone)]
struct StickerDownloadResult {
    sticker_no: usize,
    content: Vec<u8>,
    file_name: FileBaseExt
}

pub async fn sticker_set_download_processor(
    bot: Bot,
    data: &Arc<SharedData>,
    msg: &Message,
    sticker: &Sticker
) -> anyhow::Result<()> {
    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Requested sticker set download", 
        msg.chat.id, msg.chat.username
    );
    let set_name = match &sticker.set_name {
        Some(x) => x,
        None => {
            bot_actions::send_message(&bot, msg.chat.id, "這張貼紙不屬於任何貼紙包呢……").await?;
            return Ok(());
        }
    };

    let get_sticker_set_param = GetStickerSetParams::builder()
        .name(set_name)
        .build();
    let set = bot.get_sticker_set(&get_sticker_set_param).await;

    let set = match set {
        Ok(set) => set.result,
        Err(e) => {
            log::warn!("Get sticker set failed: {}", e);
            bot_actions::send_message(&bot, msg.chat.id, "似乎找不到那個貼紙包呢……").await?;
            return Ok(());
        }
    };

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Sticker set name: {}, downloading sticker set", 
        msg.chat.id, msg.chat.username, set.name
    );

    let sticker_count = set.stickers.len();

    // Allocate missions
    let sticker_queue: VecDeque<(usize, String)> = set.stickers
        .into_iter()
        .enumerate()
        .map(|(i, sticker)| (i, sticker.file_id))
        .collect();
    let queue: Arc<Mutex<VecDeque<(usize, String)>>> = Arc::new(Mutex::new(sticker_queue));
    let results = Arc::new(Mutex::new(Vec::<StickerDownloadResult>::new()));

    // Concurrent download stickers
    let progress_message = bot_actions::send_message(&bot, msg.chat.id, format!("開始下載貼紙喵…… (共 {} 張）", sticker_count)).await?;

    let mut join_handle_list = vec![];
    const WORKER_COUNT: usize = 8;
    for worker_id in 0..WORKER_COUNT {
        let bot_cloned = bot.clone();
        let data_cloned = data.clone();
        let ref_queue = queue.clone();
        let ref_results = results.clone();
        join_handle_list.push(tokio::spawn(async move {
            sticker_download_worker(bot_cloned, data_cloned, worker_id, ref_queue, ref_results).await;
        }))
    }

    loop {
        if join_handle_list.iter().map(|h| h.is_finished()).all(|s| s == true) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        let count = results.lock().await.len();
        bot_actions::edit_message_text(&bot, msg.chat.id, progress_message.message_id, format!("正在下載貼紙喵…… ({}/{})", count, sticker_count)).await?;
    }

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Sticker set name: {}, archiving sticker set", 
        msg.chat.id, msg.chat.username, set.name
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
            "[ChatID: {}, {:?}] Sticker set name: {}, downloading thumbnail", 
            msg.chat.id, msg.chat.username, set.name
        );
        let file = download_file(bot.clone(), data, &thumbnail.file_id).await?;

        match file {
            Some((file_context, file_name)) => {
                let options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Stored)
                    .unix_permissions(0o755);
                archive.start_file(format!("{}_thumbnail.{}", set_name, FileBaseExt::from(file_name).extension), options)?;
                archive.write_all(&file_context)?;
            },
            None => {
                log::warn!(
                    target: "sticker_set_download",
                    "Failed to download empty sticker thumbnail: {}",
                    thumbnail.file_id
                );
            }
        };
    }

    archive.finish()?;

    bot_actions::edit_message_text(&bot, msg.chat.id, progress_message.message_id, "貼紙下載完成了～").await?;

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Sticker set name: {}, upolading archive", 
        msg.chat.id, msg.chat.username, set.name
    );

    let mut archive_file = TempFile::new_with_name(format!("{}.zip", set_name)).await?;
    archive_file.write_all(&archive_data).await?;

    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(archive_file.file_path().clone())
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    bot.send_document(&send_document_param).await?;

    bot_actions::send_message(&bot, msg.chat.id, "下載完成啦～\n您可以繼續發送要下載的貼紙包～\n如果要退出，請點擊指令 -> /exit").await?;

    Ok(())
}

async fn sticker_download_worker(
    bot: Bot,
    data: Arc<SharedData>,
    worker_id: usize,
    queue: Arc<Mutex<VecDeque<(usize, String)>>>,
    results: Arc<Mutex<Vec<StickerDownloadResult>>>
) {
    loop {
        let task = {
            let mut guard = queue.lock().await;
            guard.pop_front()
        };
        let (sticker_no, file_id) = match task {
            Some(task) => task,
            None => { return }
        };

        let file = match download_file(bot.clone(), &data, &file_id).await {
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

        let (file_context, file_name) = match file {
            Some(x) => x,
            None => {
                log::warn!(
                    target: &format!("sticker_set_download worker#{}", worker_id),
                    "Failed to download empty sticker #{}: {}",
                    sticker_no, file_id
                );
                continue;
            }
        };
        
        {
            let mut guard = results.lock().await;
            guard.push(StickerDownloadResult { 
                sticker_no: sticker_no, 
                content: file_context, 
                file_name: FileBaseExt::from(file_name)
            });
        }
    }
}