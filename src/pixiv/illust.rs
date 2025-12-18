use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use frankenstein::AsyncTelegramApi;
use frankenstein::input_media::{InputMediaDocument, InputMediaPhoto, MediaGroupInputMedia};
use frankenstein::methods::{SendDocumentParams, SendMediaGroupParams};
use frankenstein::types::Message;
use serde::Deserialize;
use tempfile::TempDir;
use tokio::sync::Mutex;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::helper::{bot_actions, param_builders};
use crate::context::Context;
use crate::pixiv::helper::{have_spoiler, illust_caption};
use crate::pixiv::types::{IllustInfo, IllustRequest, PixivResponse, SendMode};
use crate::pixiv::ugoira::pixiv_ugoira_handler;
use crate::pixiv::download::download_to_path;

pub async fn pixiv_illust_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    options: IllustRequest
) -> anyhow::Result<()> {

    log::info!(
        target: "pixiv_illust",
        "[Pixiv: {id}] Requested pixiv illust download with options: {options:?}",
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
        .timeout(Duration::from_secs(5))
        .build()?;
    // TODO: the user agent should be customizable maybe?
    
    let info_url = format!("https://www.pixiv.net/ajax/illust/{}", id);
    log::info!(
        target: "pixiv_illust",
        "[Pixiv: {id}] Requesting pixiv API: {}",
        info_url
    );

    let request = client.get(info_url);
    // Add cookie
    let request = if let Some(php_sessid) = ctx.config.pixiv.php_sessid.as_ref() {
        request.header("Cookie", format!("PHPSESSID={}", php_sessid))
    } else {
        request
    };

    let response: PixivResponse = request.send().await?.json().await?;

    // Check response successful
    if response.error {
        if response.body.as_array().is_some_and(|array| array.is_empty()) {
            bot_actions::send_reply_message(
                &ctx.bot, msg.chat.id, "沒有找到這個 pixiv 畫廊呢……",
                msg.message_id, None
            ).await?;
        } else {
            log::error!(
                target: "pixiv_illust",
                "[Pixiv: {id}] pixiv returned error: {}",
                response.message
            )
        }
        return Ok(());
    }

    // Get the basic informations
    let info = match IllustInfo::deserialize(response.body) {
        Ok(info) => info,
        Err(e) => {
            log::error!(
                target: "pixiv_illust",
                "[Pixiv: {id}] Failed to extract illustration info from response: {e:?}"
            );
            return Ok(());
        }
    };

    // Check if it is not in archive mode and page limit exists
    if options.send_mode != SendMode::Archive && !options.no_page_limit && info.page_count > 10 {
        if !options.silent_page_limit {
            bot_actions::send_reply_message(
                &ctx.bot, msg.chat.id, "在不使用 nolim 或 archive 參數的情況下，最多支持有 10 張圖的畫廊哦。",
                msg.message_id, None
            ).await?;
        }
        return Ok(());
    }

    // Ugoira if "ugoira0" is present in the original link
    let Some(original_url) = info.urls.original.as_ref() else {
        bot_actions::send_reply_message(&ctx.bot, msg.chat.id, "圖源的鏈接被屏蔽了呢……", msg.message_id, None).await?;
        return Ok(());
    };
    if original_url.contains("ugoira0.jpg") {
        log::info!(
            target: "pixiv_illust",
            "[Pixiv: {id}] Animation detected, go to animation processing"
        );
        
        pixiv_ugoira_handler(ctx, msg, id, info, options).await?;

        return Ok(());
    }

    // Set ref url for corresponding quality
    let original_quality = !(options.send_mode == SendMode::Photos);
    // Notice when to use regular and when to use original
    let ref_url = match original_quality {
        true => info.urls.original.as_ref(),
        false => info.urls.regular.as_ref(),
    };
    let Some(ref_url) = ref_url else {
        bot_actions::send_reply_message(&ctx.bot, msg.chat.id, "圖源的鏈接被屏蔽了呢……", msg.message_id, None).await?;
        return Ok(());
    };

    let Some((base_url, ref_file_name)) = ref_url.rsplit_once("/") else {
        log::error!(
            target: "pixiv_illust",
            "[Pixiv: {id}] Failed to get base url from url {ref_url}"
        );
        bot_actions::send_reply_message(&ctx.bot, msg.chat.id, "圖源的鏈接好像有點問題呢……？", msg.message_id, None).await?;
        return Ok(());
    };

    log::info!(
        target: "pixiv_illust",
        "[Pixiv: {id}] Downloading gallery from {base_url}"
    );

    // About to download, send a typing status
    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::Typing).await?;

    // Create tempfile start download all files
    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;

    let task_queue: VecDeque<PixivDownloadTask> = (0..info.page_count)
        .map(|page| PixivDownloadTask { 
            page: page,
            file_name: ref_file_name.replace("p0", &format!("p{}", page))
        })
        .collect();
    let task_queue: Arc<Mutex<VecDeque<PixivDownloadTask>>> = Arc::new(Mutex::new(task_queue));
    let completed: Arc<Mutex<Vec<PixivDownloadFile>>> = Arc::new(Mutex::new(Vec::new()));
    
    let mut join_handle_list = vec![];
    const WORKER_COUNT: u64 = 4;
    for worker_id in 0..u64::min(WORKER_COUNT, info.page_count) {
        let queue_cloned = task_queue.clone();
        let base_url = base_url.to_string();
        let completed_clone = completed.clone();
        let path = temp_dir.path().to_path_buf();
        join_handle_list.push(tokio::task::spawn(async move {
            pixiv_illust_download_worker(
                worker_id,
                base_url,
                path,
                queue_cloned,
                completed_clone
            ).await;
        }));
    }

    if info.page_count >= 5 {
        let mut progress_text = format!("開始下載插畫…… (共 {} 頁）", info.page_count);
        let progress_message = bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, &progress_text, msg.message_id, None
        ).await?;
        loop {
            if join_handle_list.iter().map(|h| h.is_finished()).all(|s| s == true) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            let count = completed.lock().await.len();

            log::info!(
                target: "pixiv_illust",
                "[Pixiv: {id}] Downloading gallery ({}/{})", 
                count, info.page_count
            );

            let new_text = format!("正在下載插畫…… ({}/{})", count, info.page_count);
            if new_text != progress_text {
                progress_text = new_text;
                bot_actions::edit_message_text(&ctx.bot, msg.chat.id, progress_message.message_id, &progress_text).await?;
            }
        }
        bot_actions::delete_message(&ctx.bot, progress_message.chat.id, progress_message.message_id).await?;
    }
    for task in join_handle_list { task.await? }

    let mut files = completed.lock().await.clone();
    files.sort_by(|a, b| {
        a.page.cmp(&b.page)
    });

    if files.len() != info.page_count as usize {
        let fail_count = info.page_count as usize - files.len();
        log::warn!(
            target: "pixiv_illust",
            "[Pixiv: {id}] Incomplete gallery download: {}/{} downloaded, {} failed",
            files.len(), info.page_count, fail_count
        );
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, format!("畫廊下載完成了，但似乎有 {} 頁插畫下載失敗了呢……", fail_count), msg.message_id, None
        ).await?;
    }

    match options.send_mode {
        SendMode::Photos =>     { pixiv_illust_send_photos(ctx, msg, id, info, files).await? }
        SendMode::Files =>      { pixiv_illust_send_files(ctx, msg, id, info, files).await? }
        SendMode::Archive =>    { pixiv_illust_send_archive(ctx, msg, id, info, files, temp_dir).await? }
    }

    Ok(())
}

async fn pixiv_illust_send_files(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    info: IllustInfo,
    files: Vec<PixivDownloadFile>,
) -> anyhow::Result<()> {

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::UploadDocument).await?;

    let chunks = files.chunks(10);
    let chunk_count = chunks.len();

    for (chunk_i, chunk) in chunks.enumerate() {
        log::info!(
            target: "pixiv_illust",
            "[Pixiv: {id}] Uploading gallery ({}/{})", 
            chunk_i + 1, chunk_count
        );
        let media_list: Vec<MediaGroupInputMedia> = chunk.into_iter().map(|result| {
            let doc = InputMediaDocument::builder()
                .media(result.save_path.clone())
                .parse_mode(frankenstein::ParseMode::Html)
                .caption(illust_caption(&info, Some(result.page + 1)))
                .build();
            MediaGroupInputMedia::Document(doc)
        }).collect();
        let send_media_group_param = SendMediaGroupParams::builder()
            .chat_id(msg.chat.id)
            .media(media_list)
            .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
            .build();
        ctx.bot.send_media_group(&send_media_group_param).await?;
        // let send_document_param = SendDocumentParams::builder()
        //     .chat_id(msg.chat.id)
        //     .document(file.save_path)
        //     .caption(pixiv_illust_caption(&info, file.page + 1))
        //     .parse_mode(frankenstein::ParseMode::Html)
        //     .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        //     .build();
        // ctx.bot.send_document(&send_document_param).await?;
    }

    Ok(())
}

async fn pixiv_illust_send_archive(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    info: IllustInfo,
    files: Vec<PixivDownloadFile>,
    temp_dir: TempDir
) -> anyhow::Result<()> {

    let archive_file_name = format!("{}.zip", info.id);
    let archive_path = temp_dir.path().join(&archive_file_name);
    let archive_path_clone = archive_path.clone();
    // Blocking zip and write operation
    let archiving_task = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let archive_file = std::fs::File::create(archive_path_clone)?;
        let mut archive = zip::ZipWriter::new(archive_file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);
        for download_file in files {
            let mut file = match std::fs::File::open(&download_file.save_path) {
                Ok(f) => f,
                Err(e) => {
                    log::warn!(
                        target: "pixiv_illust archiver",
                        "[Pixiv: {id}] Failed to open downloaded file {} for archiving: {}", 
                        download_file.save_path.to_string_lossy(), e
                    );
                    continue;
                }
            };

            archive.start_file(download_file.file_name, options)?;
            std::io::copy(&mut file, &mut archive)?;
        }
        archive.finish()?;
        Ok(())
    });

    // Notice, JoinError is passed up by "?"
    if let Err(e) = archiving_task.await? {
        log::warn!(
            target: "pixiv_illust",
            "[Pixiv: {id}] Failed to archive file {}: {}", 
            archive_file_name, e
        );
        return Ok(())
    }

    log::info!(
        target: "pixiv_illust",
        "[Pixiv: {id}] Upolading archive"
    );

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::UploadDocument).await?;

    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(archive_path)
        .parse_mode(frankenstein::ParseMode::Html)
        .caption(illust_caption(&info, None))
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_document(&send_document_param).await?;

    Ok(())
}

async fn pixiv_illust_send_photos(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    info: IllustInfo,
    files: Vec<PixivDownloadFile>
) -> anyhow::Result<()> {

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::UploadPhoto).await?;

    let chunks = files.chunks(10);
    let chunk_count = chunks.len();

    for (chunk_i, chunk) in chunks.enumerate() {
        log::info!(
            target: "pixiv_illust",
            "[Pixiv: {id}] Uploading gallery ({}/{})", 
            chunk_i + 1, chunk_count
        );
        let media_list: Vec<MediaGroupInputMedia> = chunk.into_iter().map(|result| {
            let photo = InputMediaPhoto::builder()
                .media(result.save_path.clone())
                .parse_mode(frankenstein::ParseMode::Html)
                .caption(illust_caption(&info, if info.page_count == 1 { None } else { Some(result.page + 1) }))
                .has_spoiler(have_spoiler(&ctx.config.pixiv, &info))
                .build();
            MediaGroupInputMedia::Photo(photo)
        }).collect();
        let send_media_group_param = SendMediaGroupParams::builder()
            .chat_id(msg.chat.id)
            .media(media_list)
            .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
            .build();
        ctx.bot.send_media_group(&send_media_group_param).await?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct PixivDownloadTask {
    file_name: String,
    page: u64,
}
#[derive(Debug, Clone)]
struct PixivDownloadFile {
    file_name: String,
    save_path: PathBuf,
    page: u64
}

async fn pixiv_illust_download_worker(
    worker_id: u64,
    base_url: String,
    save_dir_path: PathBuf,
    queue: Arc<Mutex<VecDeque<PixivDownloadTask>>>,
    completed: Arc<Mutex<Vec<PixivDownloadFile>>>
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

        let url = format!("{base_url}/{}", task.file_name);
        let save_path = save_dir_path.join(&task.file_name);

        log::debug!(
            target: &format!("pixiv_illust download worker#{}", worker_id),
            "Downloading {} from {}",
            task.file_name, url
        );
        
        if let Err(e) = download_to_path(None, &url, &save_path).await {
            log::warn!(
                target: &format!("pixiv_illust download worker#{}", worker_id),
                "Failed to download illust file {} from {}: {}",
                task.file_name, url, e
            );
            continue;
        }

        {
            let mut guard = completed.lock().await;
            guard.push(PixivDownloadFile {
                page: task.page,
                save_path: save_path,
                file_name: task.file_name
            });
        }
    }
}