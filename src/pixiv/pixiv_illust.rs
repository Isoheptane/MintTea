use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use frankenstein::AsyncTelegramApi;
use frankenstein::input_media::{InputMediaPhoto, MediaGroupInputMedia};
use frankenstein::methods::{SendMediaGroupParams};
use frankenstein::types::Message;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::helper::bot_actions;
use crate::context::Context;
use crate::pixiv::pixiv_download::pixiv_download_image_to_path;
use crate::types::FileName;

#[derive(Clone, Debug, Deserialize)]
struct PixivIllustResponse {
    error: bool,
    message: String,
    body: Value
}

#[derive(Clone, Debug, Deserialize)]
struct PixivIllustInfoUrls {
    #[allow(unused)]
    mini: Option<String>,
    #[allow(unused)]
    thumb: Option<String>,
    #[allow(unused)]
    small: Option<String>,
    #[allow(unused)]
    regular: Option<String>,
    original: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct PixivIllustInfo {
    id: String,
    title: String,
    description: String,
    #[serde(rename = "userId")]
    author_id: String,
    #[serde(rename = "userName")]
    author_name: String,
    #[serde(rename = "pageCount")]
    page_count: u64,
    urls: PixivIllustInfoUrls,
}

pub async fn pixiv_illust_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
) -> Result<()> {

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
        .timeout(Duration::from_secs(5))
        .build()?;
    // TODO: the user agent should be customizable maybe?
    
    let info_url = format!("https://www.pixiv.net/ajax/illust/{}", id);
    log::info!(
        target: "pixiv_command",
        "[ChatID: {}, {:?}] Requesting pixiv API: {}", 
        msg.chat.id, msg.chat.username, info_url
    );

    let request = client.get(info_url);
    // Add cookie
    let request = if let Some(php_sessid) = ctx.config.pixiv.php_sessid.as_ref() {
        request.header("Cookie", format!("PHPSESSID={}", php_sessid))
    } else {
        request
    };

    let response: PixivIllustResponse = request.send().await?.json().await?;

    // Check response successful
    if response.error {
        if response.body.as_array().is_some_and(|array| array.is_empty()) {
            bot_actions::send_message(&ctx.bot, msg.chat.id, "沒有找到這個 pixiv 畫廊呢……").await?;
        } else {
            log::error!(
                target: "pixiv_command",
                "[ChatID: {}, {:?}] Pixiv returned error: {}", 
                msg.chat.id, msg.chat.username, response.message
            )
        }
        return Ok(());
    }

    // Get the basic informations
    let info = match PixivIllustInfo::deserialize(response.body) {
        Ok(info) => info,
        Err(e) => {
            log::error!(
                target: "pixiv_command",
                "[ChatID: {}, {:?}] Failed to extract illustration info from response: {:?}", 
                msg.chat.id, msg.chat.username, e
            );
            return Ok(());
        }
    };

    let original_quality = false;
    // Notice when to use regular and when to use original
    let ref_url = match original_quality {
        true => info.urls.original,
        false => info.urls.regular,
    };
    let Some(ref_url) = ref_url else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "圖源的鏈接被屏蔽了呢……").await?;
        return Ok(());
    };

    let Some((base_url, ref_file_name)) = ref_url.rsplit_once("/") else {
        log::error!(
            target: "pixiv_command",
            "[ChatID: {}, {:?}] Failed to get base url from url {}",
            msg.chat.id, msg.chat.username, ref_url
        );
        bot_actions::send_message(&ctx.bot, msg.chat.id, "圖源的鏈接好像有點問題呢……？").await?;
        return Ok(());
    };

    // Create tempfile start download all files
    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;

    let task_queue: VecDeque<PixivDownloadTask> = (0..info.page_count)
        .map(|page| PixivDownloadTask { 
            page: page,
            file_name: ref_file_name.replace("p0", &format!("p{}", page))
        })
        .collect();
    let task_queue: Arc<Mutex<VecDeque<PixivDownloadTask>>> = Arc::new(Mutex::new(task_queue));
    let completed: Arc<Mutex<Vec<PixivDownloadResult>>> = Arc::new(Mutex::new(Vec::new()));
    
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

    // For higher page count, show download process
    if info.page_count >= 5 {
        let mut progress_text = format!("開始下載插畫…… (共 {} 頁）", info.page_count);
        let progress_message = bot_actions::send_message(&ctx.bot, msg.chat.id, &progress_text).await?;
        loop {
            if join_handle_list.iter().map(|h| h.is_finished()).all(|s| s == true) {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            let count = completed.lock().await.len();
            let new_text = format!("正在下載插畫…… ({}/{})", count, info.page_count);
            if new_text != progress_text {
                progress_text = new_text;
                bot_actions::edit_message_text(&ctx.bot, msg.chat.id, progress_message.message_id, &progress_text).await?;
            }
        }
    }
    
    for task in join_handle_list {
        task.await?;
    }

    let completed = completed.lock().await.clone();

    log::info!(
        target: "pixiv_download",
        "[ChatID: {}, {:?}] Uploading gallery {}", 
        msg.chat.id, msg.chat.username, id
    );

    let media_list: Vec<MediaGroupInputMedia> = completed.into_iter().map(|result| {
        let photo = InputMediaPhoto::builder()
            .media(result.save_path)
            .parse_mode(frankenstein::ParseMode::Html)
            .caption(
                format!(
                    "<a href=\"https://www.pixiv.net/artworks/{}\">{}</a> / <a href=\"https://www.pixiv.net/users/{}\">{}</a> ({}/{})",
                    info.id, info.title, info.author_id, info.author_name, result.page + 1, info.page_count
                ))
            .build();
        MediaGroupInputMedia::Photo(photo)
    }).collect();

    let send_media_group_param = SendMediaGroupParams::builder()
        .chat_id(msg.chat.id)
        .media(media_list)
        .build();
    ctx.bot.send_media_group(&send_media_group_param).await?;

    Ok(())
}

#[derive(Debug, Clone)]
struct PixivDownloadTask {
    file_name: String,
    page: u64,
}
#[derive(Debug, Clone)]
struct PixivDownloadResult {
    save_path: PathBuf,
    page: u64
}

async fn pixiv_illust_download_worker(
    worker_id: u64,
    base_url: String,
    save_dir_path: PathBuf,
    queue: Arc<Mutex<VecDeque<PixivDownloadTask>>>,
    completed: Arc<Mutex<Vec<PixivDownloadResult>>>
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
            target: &format!("pixiv_download worker#{}", worker_id),
            "Downloading {} from {}...",
            task.file_name, url
        );
        
        if let Err(e) = pixiv_download_image_to_path(None, &url, &save_path).await {
            log::warn!(
                target: &format!("pixiv_download worker#{}", worker_id),
                "Failed to download illust file {} from {}: {}",
                task.file_name, url, e
            );
        }

        {
            let mut guard = completed.lock().await;
            guard.push(PixivDownloadResult { save_path, page: task.page });
        }
    }
}