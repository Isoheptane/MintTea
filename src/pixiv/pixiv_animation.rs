use std::option;
use std::path::PathBuf;
use std::sync::Arc;

use frankenstein::types::Message;
use tempfile::TempDir;

use crate::helper::bot_actions;
use crate::pixiv::pixiv_download::pixiv_download_to_path;
use crate::pixiv::pixiv_illust_info::PixivIllustInfo;
use crate::pixiv::pixiv_illust::DownloadOptions;
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

    if let Err(e) = pixiv_download_to_path(None, &ugoira_url, ugoira_zip_path).await {
        log::warn!(
            target: "pixiv_animation",
            "[ChatID: {}, {:?}] Failed to download animation zip file from {} : {e}",
            msg.chat.id, msg.chat.username, ugoira_url
        );
    }

    match options.send_mode {
        crate::pixiv::pixiv_illust::SendMode::Photos => {},
        crate::pixiv::pixiv_illust::SendMode::Files => {},
        crate::pixiv::pixiv_illust::SendMode::Archive => {

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

    

    Ok(())
}