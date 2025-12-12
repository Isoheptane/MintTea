use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use frankenstein::AsyncTelegramApi;
use frankenstein::methods::{SendDocumentParams, SendVideoParams};
use frankenstein::types::Message;
use tempfile::TempDir;

use crate::helper::{bot_actions, param_builders};
use crate::pixiv::pixiv_download::pixiv_download_to_path;
use crate::pixiv::pixiv_illust_info::{PixivIllustInfo, have_spoiler, pixiv_illust_caption};
use crate::pixiv::pixiv_illust::{DownloadOptions, SendMode};
use crate::context::Context;

pub async fn pixiv_animation_handler(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    info: PixivIllustInfo,
    ugoira_url: String,
    options: DownloadOptions,
) -> anyhow::Result<()> {

    let Some((_, file_name)) = ugoira_url.rsplit_once("/") else {
        log::error!(
            target: "pixiv_animation",
            "[ChatID: {}, {:?}] Failed to get base url from url {}",
            msg.chat.id, msg.chat.username, ugoira_url
        );
        bot_actions::send_reply_message(&ctx.bot, msg.chat.id, "圖源的鏈接好像有點問題呢……？", msg.message_id, None).await?;
        return Ok(());
    };

    let temp_dir = tempfile::tempdir_in(&ctx.temp_root_path)?;
    let ugoira_zip_path = temp_dir.path().join(file_name);
    
    log::info!(
        target: "pixiv_animation",
        "[ChatID: {}, {:?}] Downloading animation zip file from {}",
        msg.chat.id, msg.chat.username, ugoira_url
    );

    if let Err(e) = pixiv_download_to_path(None, &ugoira_url, &ugoira_zip_path).await {
        log::warn!(
            target: "pixiv_animation",
            "[ChatID: {}, {:?}] Failed to download animation zip file from {} : {e}",
            msg.chat.id, msg.chat.username, ugoira_url
        );
    }

    match options.send_mode {
        SendMode::Photos |
        SendMode::Files => {
            pixiv_animation_send_encoded_video(ctx, msg, info, temp_dir, ugoira_zip_path).await?;
        }
        SendMode::Archive => {
            pixiv_animation_send_archive(ctx, msg, info, temp_dir, ugoira_zip_path).await?;
        }
    }

    Ok(())
}

pub async fn pixiv_animation_send_archive(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    info: PixivIllustInfo,
    temp_dir: TempDir,
    ugoira_zip_path: PathBuf
) -> anyhow::Result<()> {

    log::info!(
        target: "pixiv_animation",
        "[ChatID: {}, {:?}] Uploading original animation {} archive", 
        msg.chat.id, msg.chat.username, info.id
    );

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

pub async fn pixiv_animation_send_encoded_video(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    info: PixivIllustInfo,
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
            target: "pixiv_animation",
            "[ChatID: {}, {:?}] Failed to extract archive file {} : {e}", 
            msg.chat.id, msg.chat.username, ugoira_zip_path.to_string_lossy()
        );
        return Ok(())
    }

    let input_glob = format!("{}", temp_dir.path().join("*.jpg").to_string_lossy());

    let file_name = format!("{}.mp4", info.id);
    let output_path = temp_dir.path().join(&file_name);
    let output_path_str = output_path.to_string_lossy();

    let ffmpeg_args = vec![
        "-framerate", "8", "-pattern_type", "glob", "-i", &input_glob, 
        "-c:v", "libx264", "-preset" ,"medium" ,"-crf" ,"17" ,
        "-y", &output_path_str
    ];

    log::info!(
        target: "pixiv_animation",
        "[ChatID: {}, {:?}] Converting image series to {}", 
        msg.chat.id, msg.chat.username, file_name
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
        target: "pixiv_animation",
        "[ChatID: {}, {:?}] Uploading video {}", 
        msg.chat.id, msg.chat.username, file_name
    );

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