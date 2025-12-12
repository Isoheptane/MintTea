use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use frankenstein::AsyncTelegramApi;
use frankenstein::stickers::Sticker;
use frankenstein::types::Message;
use frankenstein::methods::{GetStickerSetParams, SendDocumentParams};
use tokio::sync::Mutex;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::helper::download::{download_telegram_file_to_path, get_telegram_file_info};
use crate::helper::{bot_actions, param_builders};
use crate::context::Context;
use crate::types::FileName;

#[derive(Debug, Clone)]
struct StickerDownloadTask {
    name_suffix: String,
    file_id: String,
}
#[derive(Debug, Clone)]
struct StickerDownloadResult {
    file_name: String,
    path: PathBuf
}

pub async fn sticker_set_download_processor(
    ctx: Arc<Context>,
    msg: &Message,
    sticker: &Sticker
) -> anyhow::Result<()> {
    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Requested sticker set download", 
        msg.chat.id, msg.chat.username
    );
    let set_name = match sticker.set_name.clone() {
        Some(x) => x,
        None => {
            bot_actions::send_message(&ctx.bot, msg.chat.id, "這張貼紙不屬於任何貼紙包呢……").await?;
            return Ok(());
        }
    };

    let get_sticker_set_param = GetStickerSetParams::builder()
        .name(&set_name)
        .build();
    let set = ctx.bot.get_sticker_set(&get_sticker_set_param).await;

    let set = match set {
        Ok(set) => set.result,
        Err(e) => {
            log::warn!("Get sticker set failed: {}", e);
            bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎找不到那個貼紙包呢……").await?;
            return Ok(());
        }
    };

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Sticker set name: {}, downloading sticker set", 
        msg.chat.id, msg.chat.username, set.name
    );

    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;
    // Allocate mission queue
    let mut task_queue: VecDeque<StickerDownloadTask> = set.stickers
        .into_iter()
        .enumerate()
        .map(|(i, sticker)| StickerDownloadTask { 
            name_suffix: format!("{:03}", i), file_id: sticker.file_id
        })
        .collect();
    if let Some(thumbnail) = set.thumbnail {
        task_queue.push_back(StickerDownloadTask { 
            name_suffix: "thumbnail".to_string(), 
            file_id: thumbnail.file_id }
        );
    }
    let sticker_count = task_queue.len();
    let task_queue: Arc<Mutex<VecDeque<StickerDownloadTask>>> = Arc::new(Mutex::new(task_queue));
    let completed = Arc::new(Mutex::new(Vec::<StickerDownloadResult>::new()));
    // Concurrent download stickers

    let mut join_handle_list = vec![];
    const WORKER_COUNT: usize = 8;
    for worker_id in 0..WORKER_COUNT {
        let ctx_cloned = ctx.clone();
        let queue_cloned = task_queue.clone();
        let completed_cloned = completed.clone();
        let set_name_cloned = set_name.clone();
        let path = temp_dir.path().to_path_buf();
        join_handle_list.push(tokio::spawn(async move {
            sticker_download_worker(
                ctx_cloned,
                worker_id,
                path,
                set_name_cloned,
                queue_cloned,
                completed_cloned,
            ).await;
        }))
    }

    let progress_message = bot_actions::send_message(&ctx.bot, msg.chat.id, format!("開始下載貼紙…… (共 {} 張）", sticker_count)).await?;
    loop {
        if join_handle_list.iter().map(|h| h.is_finished()).all(|s| s == true) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        let count = completed.lock().await.len();
        bot_actions::edit_message_text(&ctx.bot, msg.chat.id, progress_message.message_id, format!("正在下載貼紙…… ({}/{})", count, sticker_count)).await?;
    }

    let completed = completed.lock().await.clone();

    if completed.len() == sticker_count {
        bot_actions::edit_message_text(
            &ctx.bot, msg.chat.id, progress_message.message_id, 
            "貼紙下載完成了～"
        ).await?;
    } else {
        let fail_count = sticker_count - completed.len();
        log::warn!(
            target: "sticker_set_download",
            "[ChatID: {}, {:?}] Incomplete sticker set {} download: {}/{} downloaded, {} failed", 
            msg.chat.id, msg.chat.username, set.name, completed.len(), sticker_count, fail_count
        );
        bot_actions::edit_message_text(
            &ctx.bot, msg.chat.id, progress_message.message_id, 
            format!("貼紙下載完成了～ ({} 張貼紙下載失敗)", fail_count)
        ).await?;
    }

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Sticker set name: {}, archiving sticker set", 
        msg.chat.id, msg.chat.username, set.name
    );

    let archive_file_name = format!("{}.zip", set_name);
    let archive_path = temp_dir.path().join(&archive_file_name);
    let archive_path_clone = archive_path.clone();
    // Blocking zip and write operation
    let archiving_task = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let archive_file = std::fs::File::create(archive_path_clone)?;
        let mut archive = zip::ZipWriter::new(archive_file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);
        for result in completed {
            let mut source_file = match std::fs::File::open(&result.path) {
                Ok(f) => f,
                Err(e) => {
                    log::warn!(
                        target: "sticker_set_download archiver",
                        "Failed to open downloaded file {} for archiving: {}", 
                        result.path.to_string_lossy(), e
                    );
                    continue;
                }
            };

            archive.start_file(result.file_name, options)?;
            std::io::copy(&mut source_file, &mut archive)?;
        }
        archive.finish()?;
        Ok(())
    });
    
    // Notice, JoinError is passed up by "?"
    if let Err(e) = archiving_task.await? {
        log::warn!(
            target: "sticker_set_download",
            "[ChatID: {}, {:?}] Failed to archive file {}: {}", 
            msg.chat.id, msg.chat.username, archive_file_name, e
        );
        return Ok(())
    }

    log::info!(
        target: "sticker_set_download",
        "[ChatID: {}, {:?}] Sticker set name: {}, upolading archive...", 
        msg.chat.id, msg.chat.username, set.name
    );

    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(archive_path)
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_document(&send_document_param).await?;

    bot_actions::send_message(&ctx.bot, msg.chat.id, "下載完成啦～\n您可以繼續發送要下載的貼紙包～\n如果要退出，請點擊指令 -> /exit").await?;

    Ok(())
}

async fn sticker_download_worker(
    ctx: Arc<Context>,
    worker_id: usize,
    save_dir_path: PathBuf,
    set_name: String,
    queue: Arc<Mutex<VecDeque<StickerDownloadTask>>>,
    completed: Arc<Mutex<Vec<StickerDownloadResult>>>
) {
    loop {
        let task = {
            let mut guard = queue.lock().await;
            guard.pop_front()
        };
        
        let Some(task) = task else {
            // Tasks are empty now
            return;
        };
        
        let file = match get_telegram_file_info(&ctx.bot, &task.file_id).await {
            Ok(Some(x)) => x,
            Ok(None) => {
                log::warn!(
                    target: &format!("sticker_set_download worker#{}", worker_id),
                    "Sticker file info is empty, #{} (file_id: {})",
                    task.name_suffix, task.file_id
                );
                continue;
            }
            Err(e) => {
                log::warn!(
                    target: &format!("sticker_set_download worker#{}", worker_id),
                    "Failed to get sticker file info #{} (file_id: {}): {}",
                    task.name_suffix, task.file_id, e
                );
                continue;
            }
        };

        let file_name = format!("{}_{}.{}", set_name, task.name_suffix, FileName::from(file.file_name).extension_str());
        let save_path = save_dir_path.join(&file_name);
        
        if let Err(e) = download_telegram_file_to_path(
            &ctx.config.telegram.token, 
            &file.file_path, 
            &save_path,
        ).await {
            log::warn!(
                target: &format!("sticker_set_download worker#{}", worker_id),
                "Failed to download sticker file #{} (file_id: {}) to {}: {}",
                task.name_suffix, task.file_id, save_path.to_string_lossy(), e
            );
            continue;
        }

        {
            let mut guard = completed.lock().await;
            guard.push(StickerDownloadResult {
                file_name,
                path: save_path
            });
        }
    }
}