pub mod config;
mod parser;
mod post;
mod creator;
mod telegraph;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use frankenstein::AsyncTelegramApi;
use frankenstein::methods::SendDocumentParams;
use frankenstein::types::Message;
use futures::future::BoxFuture;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::sync::Mutex;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::handler::HandlerResult;
use crate::helper::download::download_url_to_path;
use crate::helper::log::LogOp;
use crate::helper::message_utils::get_command;
use crate::helper::{bot_actions, param_builders};
use crate::context::Context;
use crate::kemono::creator::CreatorProfile;
use crate::kemono::parser::{FanboxRequest, KemonoCommandParam, KemonoRequest, parse_fanbox_link, parse_kemono_command, parse_kemono_link};
use crate::kemono::post::{KemonoFile, KemonoPostResponse};
use crate::kemono::telegraph::send_telegraph_preview;

pub const COMMAND_LIST: &[(&'static str, &'static str)] = &[
    ("kemono", "預覽或下載 kemono.cr 上的歸檔"),
];

pub fn kemono_handler(ctx: Arc<Context>, msg: Arc<Message>) -> BoxFuture<'static, HandlerResult> {
    let fut = kemono_handler_impl(ctx, msg);
    return Box::pin(fut);
}

async fn kemono_handler_impl(ctx: Arc<Context>, msg: Arc<Message>) -> HandlerResult {

    // Command handling
    let command = get_command(&msg);
    let Some(text) = msg.text.as_ref() else {
        return Ok(std::ops::ControlFlow::Continue(()))
    };

    if let Some(command) = command {
        match command.as_str() {
            "kemono" => {
                match parse_kemono_command(text) {
                    parser::KemonoCommandParseResult::Kemono(req) => {
                        kemono_download_handler(ctx, msg, req).await?;
                    }
                    parser::KemonoCommandParseResult::Fanbox(req) => {
                        fanbox_download_handler(ctx, msg, req).await?;
                    }
                    parser::KemonoCommandParseResult::InvalidLink => {
                        bot_actions::send_message(&ctx.bot, msg.chat.id, "似乎沒有識別到正確的 kemono.cr 鏈接呢……").await?;
                    }
                    parser::KemonoCommandParseResult::ShowHelp => {
                        send_kemono_command_help(ctx, msg).await?;
                    }
                }
                return Ok(std::ops::ControlFlow::Break(()));
            }
            _ => return Ok(std::ops::ControlFlow::Continue(()))
        }
    }
    
    // Link detection for kemono
    if ctx.config.kemono.enable_kemono_link_detection {
        if let Some((service, user_id, post_id)) = parse_kemono_link(text) {
            let request = KemonoRequest {
                service,
                user_id,
                post_id,
                param: KemonoCommandParam::link_default()
            };
            kemono_download_handler(ctx, msg, request).await?;
            return Ok(std::ops::ControlFlow::Continue(()));
        }
    }
    // Link detection for fanbox
    if ctx.config.kemono.enable_fanbox_link_detection {
        if let Some((username, post_id)) = parse_fanbox_link(text) {
            let request = FanboxRequest {
                username,
                post_id,
                param: KemonoCommandParam::link_default()
            };
            fanbox_download_handler(ctx, msg, request).await?;
            return Ok(std::ops::ControlFlow::Continue(()));
        }
    }

    Ok(std::ops::ControlFlow::Continue(()))
}

async fn send_kemono_command_help(ctx: Arc<Context>, msg: Arc<Message>) -> anyhow::Result<()> {
    const HELP_MSG : &'static str = 
        "/kemono 指令幫助\n\
        ";
    bot_actions::send_message(&ctx.bot, msg.chat.id, HELP_MSG).await?;
    Ok(())
}

async fn fanbox_download_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    request: FanboxRequest
) -> anyhow::Result<()> {

    #[derive(Clone, Debug, Deserialize)]
    struct FanboxCreatorGetResponse {
        pub error: Option<String>,
        pub body: Option<FanboxCreatorGetBody>,
    }

    #[derive(Clone, Debug, Deserialize)]
    struct FanboxCreatorGetBody {
        pub user: FanboxUser
    }

    #[derive(Clone, Debug, Deserialize)]
    struct FanboxUser {
        #[serde(rename = "userId")]
        pub user_id: String,
        #[allow(unused)]
        pub name: String
    }

    let api_url = format!("https://api.fanbox.cc/creator.get?creatorId={}", request.username);

    log::info!(
        target: "kemono_download_fanbox",
        "{} Requesting Fanbox API for user ID: {}",
        LogOp(&msg), api_url
    );

    let fanbox_req = ctx.pixiv.client.get(api_url)
        .header("Origin", "https://www.fanbox.cc");

    let response: FanboxCreatorGetResponse = fanbox_req.send().await?.json().await?;

    // NOTICE:
    // Fanbox returns general error when requesting a non-existing artist
    let Some(response_body) = response.body else {

        let error_msg = match response.error.as_ref() {
            Some(msg) => msg.as_str(),
            None => "<no error message>",
        };

        log::info!(
            target: "kemono_download_fanbox",
            "{} Fanbox API response body is empty, error message: {}",
            LogOp(&msg), error_msg
        );

        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, "似乎沒找到這個創作者呢…… (也有可能是查詢 FANBOX 創作者失敗了)",
            msg.message_id, None
        ).await?;

        return Ok(())
    };

    // It should be number, but we keep it a string here
    let user_id = response_body.user.user_id;

    // Return kemono link if post_id is not specified
    let Some(post_id) = request.post_id else {
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, 
            format!("該作者可能的 kemono.cr 主頁： {}", format!("https://kemono.cr/fanbox/user/{user_id}")),
            msg.message_id, None
        ).await?;
        return Ok(())
    };

    // Handover to kemono download handler
    kemono_download_handler(ctx, msg, KemonoRequest {
        service: "fanbox".to_string(),
        user_id: user_id,
        post_id: post_id,
        param: request.param
    }).await?;

    return Ok(())
}

async fn kemono_download_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    request: KemonoRequest,
) -> anyhow::Result<()> {

    let client_builder = match ctx.config.kemono.client_user_agent.as_ref() {
        Some(ua) => Client::builder().user_agent(ua),
        None => Client::builder()
    };
    let client = client_builder
        .timeout(Duration::from_mins(10))
        .build()?;

    let url = format!(
        "https://kemono.cr/api/v1/{}/user/{}/post/{}", 
        request.service, request.user_id, request.post_id
    );

    log::info!(
        target: "kemono_download",
        "{} Requesting {}",
        LogOp(&msg), url
    );

    let response = client.get(url)
        .header("Accept", "text/css")
        .send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        log::info!(
            target: "kemono_download",
            "{} kemono.cr page not found",
            LogOp(&msg)
        );
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, "沒有找到這個 kemono.cr 頁面呢……",
            msg.message_id, None
        ).await?;
        return Ok(());
    }

    let response: KemonoPostResponse = response.json().await?;
    let post = response.post;

    let url = format!("https://kemono.cr/api/v1/{}/user/{}/profile", request.service, request.user_id);
    let creator: CreatorProfile = client.get(url)
        .header("Accept", "text/css")
        .send().await?
        .json().await?;

    // Send telegraph first
    if request.param.as_telegraph {
        log::info!(
            target: "kemono_download",
            "{} Sending telegraph link",
            LogOp(&msg)
        );
        if let Err(e) = send_telegraph_preview(ctx.clone(), msg.clone(), &post, &creator).await {
            log::warn!(
                target: "kemono_download",
                "{} Failed to send telegraph link: {}",
                LogOp(&msg), e
            );
        }
    }

    // Skip download if media is not required
    if !request.param.as_media && !request.param.as_archive {
        return Ok(())
    }

    // Create tempfile start download all files
    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;

    // Construct tasks and worker queue
    let mut queue = VecDeque::<DownloadTask>::new();
    if let Some(banner) = post.file {
        queue.push_back(DownloadTask::from_kemono_file(
            banner, "banner_".to_string(), temp_dir.path()
        ));
    }
    for (idx, file) in post.attachments.iter().enumerate() {
        queue.push_back(DownloadTask::from_kemono_file(
            file.clone(), 
            format!("{:02}_", idx + 1),
            temp_dir.path()
        ));
    }

    let task_count = queue.len();

    // Create metadata checking task queue
    let metadata_queue: VecDeque<String> = queue.iter().map(
        |task| task.url.clone()
    ).collect();
    let metadata_tasks = Arc::new(Mutex::new(metadata_queue));

    // Create list of files for later scanning
    let scan_file_list: Vec<PathBuf> = queue.iter().map(
        |task| task.save_path.clone()
    ).collect();

    /* Get file size */
    log::info!(
        target: "kemono_download",
        "{} Requesting file metadata...",
        LogOp(&msg)
    );

    let total_size = Arc::new(Mutex::new(0 as u64));

    let mut meta_join_handle_list = vec![];
    const META_WORKER_COUNT: u64 = 8;
    for worker_id in 0..u64::min(META_WORKER_COUNT, task_count as u64) {
        let client_cloned = client.clone();
        let tasks_cloned = metadata_tasks.clone();
        let total_size_cloned = total_size.clone();
        meta_join_handle_list.push(tokio::task::spawn(async move {
            kemono_metadata_worker(
                worker_id,
                client_cloned,
                tasks_cloned,
                total_size_cloned,
            ).await;
        }));
    }

    let progress_message = bot_actions::send_reply_message(
        &ctx.bot, msg.chat.id, "正在請求文件元數據……", msg.message_id, None
    ).await?;

    for handle in meta_join_handle_list {
        handle.await?
    }

    let total_size = *total_size.lock().await;
    let total_size_mib = total_size as f64 / (1024.0 * 1024.0);

    // Check file size
    // Use 
    if total_size > 49_000_000 {
        bot_actions::edit_message_text(
            &ctx.bot, msg.chat.id, progress_message.message_id,
            format!("文件總大小超過 50 MB 了呢…… ({:.1} MiB)\n請自行前往 kemono.cr 下載。", total_size_mib)
        ).await?;
        return Ok(())
    }

    /* Real download */
    log::info!(
        target: "kemono_download",
        "{} Downloading files ({} bytes)",
        LogOp(&msg), total_size
    );

    let download_tasks = Arc::new(Mutex::new(queue));

    let completed = Arc::new(Mutex::new(Vec::<DownloadItem>::new()));

    let mut join_handle_list = vec![];
    const WORKER_COUNT: u64 = 4;
    for download_worker_id in 0..u64::min(WORKER_COUNT, task_count as u64) {
        let client_cloned = client.clone();
        let tasks_cloned = download_tasks.clone();
        let completed_cloned = completed.clone();
        join_handle_list.push(tokio::task::spawn(async move {
            kemono_download_worker(
                download_worker_id,
                client_cloned,
                tasks_cloned,
                completed_cloned,
            ).await;
        }));
    }

    // Wait until all done
    let mut progress_text = format!(
        "開始從 kemono.cr 下載文件…… ({:.1} MiB, 共 {} 個文件）", 
        total_size_mib, task_count
    );
    
    bot_actions::edit_message_text(
        &ctx.bot, msg.chat.id, progress_message.message_id, &progress_text
    ).await?;

    loop {
        if join_handle_list.iter().map(|h| h.is_finished()).all(|s| s == true) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;

        let count: u64 = completed.lock().await.len() as u64;

        let mut current_size: u64 = 0;
        for path in &scan_file_list {
            if let Ok(meta) = tokio::fs::metadata(path).await {
                current_size += meta.len()
            }
        }

        let current_size_mib = current_size as f64 / (1024.0 * 1024.0);
    
        log::info!(
            target: "kemono_download",
            "{} Downloading ({}/{}) attachments ({:.1} MiB / {:.1} MiB)", 
            LogOp(&msg), count, task_count, current_size_mib, total_size_mib
        );
    
        let new_text = format!(
            "正在從 kemono.cr 下載文件 ({:.1} MiB / {:.1} MiB, {}/{} 個文件）", 
            current_size_mib, total_size_mib, count, task_count
        );

        if new_text != progress_text {
            progress_text = new_text;
            bot_actions::edit_message_text(&ctx.bot, msg.chat.id, progress_message.message_id, &progress_text).await?;
        }
    }
    bot_actions::delete_message(&ctx.bot, progress_message.chat.id, progress_message.message_id).await?;

    let files = completed.lock().await.clone();

    if files.len() != task_count as usize {
        let fail_count = task_count as usize - files.len();
        log::warn!(
            target: "kemono_download",
            "{} Incomplete attachment download: {}/{} downloaded, {} failed",
            LogOp(&msg), files.len(), task_count, fail_count
        );
        bot_actions::send_reply_message(
            &ctx.bot, msg.chat.id, format!("文件下載完成了，但似乎有 {} 個文件下載失敗了呢……", fail_count), msg.message_id, None
        ).await?;
    }

    let archive_file_name = format!("{}_{}_{}.zip", post.service, post.user, post.id);
    let archive_path = temp_dir.path().join(&archive_file_name);
    let archive_path_clone = archive_path.clone();
    let tmp_path = temp_dir.path().to_path_buf();
    // Archive file
    // Blocking zip and write operation
    let archiving_task = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let archive_file = std::fs::File::create(archive_path_clone)?;
        let mut archive = zip::ZipWriter::new(archive_file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);
        for download_file in files {
            let path = tmp_path.join(&download_file.save_name);
            let mut file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    log::warn!(
                        target: "kemono_download archiver",
                        "Failed to open downloaded file {} for archiving: {}", 
                        path.to_string_lossy(), e
                    );
                    continue;
                }
            };

            archive.start_file(download_file.save_name, options)?;
            std::io::copy(&mut file, &mut archive)?;
        }
        archive.finish()?;
        Ok(())
    });

    // Notice, JoinError is passed up by "?"
    if let Err(e) = archiving_task.await? {
        log::warn!(
            target: "kemono_download",
            "{} Failed to archive file {}: {}", 
            LogOp(&msg), archive_file_name, e
        );
        return Ok(())
    }

    log::info!(
        target: "kemono_download",
        "{} Upolading archive",
        LogOp(&msg),
    );

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::UploadDocument).await?;

    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(archive_path)
        .parse_mode(frankenstein::ParseMode::Html)
        .caption(format!("<b>{}</b>", post.title))
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_document(&send_document_param).await?;

    Ok(())
}

#[derive(Debug, Clone)]
struct DownloadTask {
    pub url: String,
    pub save_name: String,
    pub save_path: PathBuf,
}

impl DownloadTask {
    pub fn from_kemono_file<P: AsRef<Path>>(file: KemonoFile, prefix: String, root_dir: P) -> DownloadTask {
        const KEMONO_BASE_URL: &'static str = "https://kemono.cr";
        let url = format!("{KEMONO_BASE_URL}{}", file.path);
        let save_name = format!("{}{}", prefix, file.name);
        DownloadTask {
            url,
            save_path: root_dir.as_ref().join(&save_name),
            save_name,
        }
    }
}

#[derive(Debug, Clone)]
struct DownloadItem {
    pub save_name: String,
}

async fn kemono_metadata_worker(
    worker_id: u64,
    client: Client,
    queue: Arc<Mutex<VecDeque<String>>>,
    downloading_size: Arc<Mutex<u64>>
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

        let resp = match client.head(&task).send().await {
            Ok(resp) => resp,
            Err(e) => {
                log::warn!(
                    target: &format!("kemono download meta worker#{}", worker_id),
                    "Failed to check header of {} : {}",
                    task, e
                );
                continue;
            }
        };

        let length = resp.headers().get("content-length")
            .and_then(|val| val.to_str().ok())
            .and_then(|val| val.parse::<u64>().ok());

        *downloading_size.lock().await += length.unwrap_or(0);
    }
}

async fn kemono_download_worker(
    worker_id: u64,
    client: Client,
    queue: Arc<Mutex<VecDeque<DownloadTask>>>,
    completed: Arc<Mutex<Vec<DownloadItem>>>,
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

        log::debug!(
            target: &format!("kemono download worker#{}", worker_id),
            "Downloading {} from {}",
            task.save_name, task.url
        );

        if let Err(e) = download_url_to_path(
            Some(client.clone()), &task.url, task.save_path
        ).await {
            log::warn!(
                target: &format!("kemono download worker#{}", worker_id),
                "Failed to download file {} : {}",
                task.save_name, e
            );
            continue;
        }

        {
            let mut guard = completed.lock().await;
            guard.push(DownloadItem { save_name: task.save_name });
        }
    }
}