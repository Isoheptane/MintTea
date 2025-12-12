use std::sync::Arc;

use frankenstein::types::Message;

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
    return Ok(());
}