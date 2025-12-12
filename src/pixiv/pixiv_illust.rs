use std::sync::Arc;

use anyhow::Result;
use frankenstein::AsyncTelegramApi;
use frankenstein::input_media::{InputMediaPhoto, MediaGroupInputMedia};
use frankenstein::methods::{SendMediaGroupParams};
use frankenstein::types::Message;
use serde::Deserialize;
use serde_json::Value;
use tempfile::NamedTempFile;

use crate::helper::bot_actions;
use crate::context::Context;
use crate::helper::tempfile::save_to_tempfile;
use crate::pixiv::pixiv_download::pixiv_download_image;
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

    let Some(ref_url) = info.urls.original else {
        bot_actions::send_message(&ctx.bot, msg.chat.id, "圖源的鏈接被屏蔽了呢……").await?;
        return Ok(());
    };

    let Some((base_url, file_name)) = ref_url.rsplit_once("/") else {
        log::error!(
            target: "pixiv_command",
            "[ChatID: {}, {:?}] Failed to get base url from url {}",
            msg.chat.id, msg.chat.username, ref_url
        );
        bot_actions::send_message(&ctx.bot, msg.chat.id, "圖源的鏈接好像有點問題呢……？").await?;
        return Ok(());
    };

    let file_name = FileName::from(file_name);
    let file_ext = file_name.extension_str();

    let mut pics: Vec<(u64, NamedTempFile)> = vec![];

    for i in 0..info.page_count {

        let pic_url = format!("{base_url}/{}_p{}.{file_ext}", id, i);

        log::info!(
            target: "pixiv_download",
            "[ChatID: {}, {:?}] Downloading pixiv illust {}", 
            msg.chat.id, msg.chat.username, pic_url
        );

        let content = match pixiv_download_image(Some(client.clone()), &pic_url).await {
            Ok(content) => content,
            Err(e) => {
                log::error!(
                    target: "pixiv_command",
                    "[ChatID: {}, {:?}] Failed to download image from {} : {e}",
                    msg.chat.id, msg.chat.username, pic_url
                );
                continue;
            }
        };

        let tempfile = save_to_tempfile(
            &format!("{}_{}_{}_p{}.{file_ext}", msg.chat.id, msg.message_id, info.id, i),
            &ctx.temp_dir,
            0,
            content
        )?;

        pics.push((i, tempfile));
    }

    log::info!(
        target: "pixiv_download",
        "[ChatID: {}, {:?}] Upload gallery {}", 
        msg.chat.id, msg.chat.username, id
    );

    let media_list: Vec<MediaGroupInputMedia> = pics.iter().map(|(i, pic)| {
        let photo = InputMediaPhoto::builder()
            .media(pic.path().to_path_buf())
            .parse_mode(frankenstein::ParseMode::Html)
            .caption(
                format!(
                    "<a href=\"https://www.pixiv.net/artworks/{}\">{}</a> / <a href=\"https://www.pixiv.net/users/{}\">{}</a> ({}/{})",
                    info.id, info.title, info.author_id, info.author_name, i + 1, info.page_count
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