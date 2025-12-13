use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use frankenstein::AsyncTelegramApi;
use frankenstein::methods::{SendDocumentParams, SendVideoParams};
use frankenstein::types::Message;
use serde::Deserialize;
use serde_json::Value;
use tempfile::TempDir;

use crate::helper::{bot_actions, param_builders};
use crate::pixiv::pixiv_download::pixiv_download_to_path;
use crate::pixiv::pixiv_illust_info::{PixivIllustInfo, have_spoiler, pixiv_illust_caption};
use crate::pixiv::pixiv_illust::{IllustOptions, SendMode};
use crate::context::Context;
use crate::pixiv::pixiv_ugoira_meta::PixivUgoiraMeta;

// https://www.pixiv.net/ajax/illust/134231396/ugoira_meta?lang=en

#[derive(Clone, Debug, Deserialize)]
struct PixivUgoiraResponse {
    error: bool,
    message: String,
    body: Value
}

pub async fn pixiv_ugoira_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    info: PixivIllustInfo,
    options: IllustOptions,
) -> anyhow::Result<()> {

    // The previous part is similar to PixivIllust

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
        .timeout(Duration::from_secs(5))
        .build()?;
    
    let meta_url = format!("https://www.pixiv.net/ajax/illust/{}/ugoira_meta", info.id);
    log::info!(
        target: "pixiv_ugoira",
        "[Pixiv: {id}] Requesting pixiv API: {meta_url}"
    );

    let request = client.get(meta_url);
    // Add cookie
    let request = if let Some(php_sessid) = ctx.config.pixiv.php_sessid.as_ref() {
        request.header("Cookie", format!("PHPSESSID={}", php_sessid))
    } else {
        request
    };

    let response: PixivUgoiraResponse = request.send().await?.json().await?;

    // Check response successful
    if response.error {
        if response.body.as_array().is_some_and(|array| array.is_empty()) {
            bot_actions::send_reply_message(
                &ctx.bot, msg.chat.id, "沒有找到這個 pixiv 动图呢……",
                msg.message_id, None
            ).await?;
        } else {
            log::error!(
                target: "pixiv_ugoira",
                "[Pixiv: {id}] pixiv returned error: {}", 
                response.message
            )
        }
        return Ok(());
    }

    // Get the basic informations
    let ugoira_meta = match PixivUgoiraMeta::deserialize(response.body) {
        Ok(meta) => meta,
        Err(e) => {
            log::error!(
                target: "pixiv_ugoira",
                "[Pixiv: {id}] Failed to extract illustration info from response: {e:?}"
            );
            return Ok(());
        }
    };

    let ugoira_url = ugoira_meta.original_src.as_str();
    let Some((_, file_name)) = ugoira_url.rsplit_once("/") else {
        log::error!(
            target: "pixiv_ugoira",
            "[Pixiv: {id}] Failed to get base url from url {ugoira_url}"
        );
        bot_actions::send_reply_message(&ctx.bot, msg.chat.id, "圖源的鏈接好像有點問題呢……？", msg.message_id, None).await?;
        return Ok(());
    };

    // About to download, send a typing status
    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::Typing).await?;

    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;
    let ugoira_zip_path = temp_dir.path().join(file_name);
    
    log::info!(
        target: "pixiv_ugoira",
        "[Pixiv: {id}] Downloading animation zip file from {ugoira_url}",
    );

    if let Err(e) = pixiv_download_to_path(None, &ugoira_url, &ugoira_zip_path).await {
        log::warn!(
            target: "pixiv_ugoira",
            "[Pixiv: {id}] Failed to download animation zip file from {ugoira_url} : {e}"
        );
    }

    match options.send_mode {
        SendMode::Photos |
        SendMode::Files => {
            pixiv_ugoira_send_encoded_video(ctx, msg, id, info, ugoira_meta, temp_dir, ugoira_zip_path).await?;
        }
        SendMode::Archive => {
            pixiv_ugoira_send_archive(ctx, msg, id, info, ugoira_zip_path).await?;
        }
    }

    Ok(())
}

pub async fn pixiv_ugoira_send_archive(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    info: PixivIllustInfo,
    ugoira_zip_path: PathBuf
) -> anyhow::Result<()> {

    log::info!(
        target: "pixiv_ugoira",
        "[Pixiv: {id}] Uploading original animation archive"
    );

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::UploadDocument).await?;

    let send_document_param = SendDocumentParams::builder()
        .chat_id(msg.chat.id)
        .document(ugoira_zip_path)
        .parse_mode(frankenstein::ParseMode::Html)
        .caption(pixiv_illust_caption(&info, None))
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();
    ctx.bot.send_document(&send_document_param).await?;

    Ok(())
}

pub async fn pixiv_ugoira_send_encoded_video(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    id: u64,
    info: PixivIllustInfo,
    ugoira_meta: PixivUgoiraMeta,
    temp_dir: TempDir,
    ugoira_zip_path: PathBuf
) -> anyhow::Result<()> {

    let extract_dir = temp_dir.path().to_path_buf();
    let zip_path = ugoira_zip_path.clone();
    let unzip_task = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let zip_file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(zip_file)?;
        archive.extract(extract_dir)?;
        Ok(())
    });

    if let Err(e) = unzip_task.await {
        log::warn!(
            target: "pixiv_ugoira",
            "[Pixiv: {id}] Failed to extract archive file {} : {e}", 
            ugoira_zip_path.to_string_lossy()
        );
        return Ok(())
    }

    let input_glob = format!("{}", temp_dir.path().join("*.jpg").to_string_lossy());

    let file_name = format!("{}.mp4", info.id);
    let output_path = temp_dir.path().join(&file_name);
    let output_path_str = output_path.to_string_lossy();

    // Calculate framerate 
    let length_ms: u64 = ugoira_meta.frames.iter().map(|frame| frame.delay ).sum();
    let avg_delay: f64 = length_ms as f64 / ugoira_meta.frames.len() as f64;
    let avg_frame_rate: f64 = 1000.0 / avg_delay;

    log::info!(
        target: "pixiv_ugoira",
        "[Pixiv: {id}] Caclculated ugoira info length: {} ms, average delay: {:.3} ms, average frame rate: {:.3}", 
        length_ms, avg_delay, avg_frame_rate
    );
    let frame_rate_str = format!("{:.3}", avg_frame_rate);

    let ffmpeg_args = vec![
        // "-f", "lavfi", "-i", "anullsrc=channel_layout=stereo:sample_rate=48000",     // silent audio stream
        "-framerate", &frame_rate_str, "-pattern_type", "glob", "-i", &input_glob,      // 
        "-vf", "crop=trunc(iw/2)*2:trunc(ih/2)*2",                                      // crop to make witdh/height is divisible by 2 (yuv420p) requirement
        "-c:v", "libx264", "-preset", "medium", "-crf", "23", "-pix_fmt", "yuv420p",    // yuv420p is required to playable on android
        // "-c:a", "aac", "shortest",                                                   // audio stream codec (from silent audio stream)
        "-y", &output_path_str
    ];

    log::info!(
        target: "pixiv_ugoira",
        "[Pixiv: {id}] ffmpeg converting image series to {file_name}", 
    );

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
        target: "pixiv_ugoira",
        "[Pixiv: {id}] Uploading video {file_name}", 
    );

    bot_actions::sent_chat_action(&ctx.bot, msg.chat.id, frankenstein::types::ChatAction::UploadVideo).await?;

    let param = SendVideoParams::builder()
        .chat_id(msg.chat.id)
        .video(output_path)
        .parse_mode(frankenstein::ParseMode::Html)
        .caption(pixiv_illust_caption(&info, None))
        .has_spoiler(have_spoiler(&ctx, &info))
        .reply_parameters(param_builders::reply_parameters(msg.message_id, Some(msg.chat.id)))
        .build();

    ctx.bot.send_video(&param).await?;

    Ok(())
}