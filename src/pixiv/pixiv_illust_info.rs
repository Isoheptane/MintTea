
use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;

use crate::context::Context;

#[derive(Clone, Debug, Deserialize)]
pub struct ImageUrls {
    #[allow(unused)]
    pub mini: Option<String>,
    #[allow(unused)]
    pub thumb: Option<String>,
    #[allow(unused)]
    pub small: Option<String>,
    #[allow(unused)]
    pub regular: Option<String>,
    pub original: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tag {
    pub tag: String,
    pub romaji: Option<String>,
    pub translation: Option<HashMap<String, String>>
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tags {
    pub tags: Vec<Tag>
    // There are more elements but i'm not going to add them right now
}

impl Tags {
    pub fn contains_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|list_tag| list_tag.tag == tag)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PixivIllustInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "userId")]
    pub author_id: String,
    #[serde(rename = "userName")]
    pub author_name: String,
    #[serde(rename = "pageCount")]
    pub page_count: u64,
    pub urls: ImageUrls,
    pub tags: Tags,
}

pub fn have_spoiler(ctx :&Arc<Context>, info: &PixivIllustInfo) -> bool {
    let nsfw = info.tags.contains_tag("R-18");
    let r18g = info.tags.contains_tag("R-18G");
    (ctx.config.pixiv.spoiler_r18g && r18g) || (ctx.config.pixiv.spoiler_nsfw && (nsfw || r18g))
}

pub fn pixiv_illust_caption(info: &PixivIllustInfo, page: Option<u64>) -> String {

    let nsfw = info.tags.contains_tag("R-18");
    let r18g = info.tags.contains_tag("R-18G");

    let prefix = match (nsfw, r18g) {
        (false, false) => "",
        (true, false) => "#NSFW ",
        (_, true) => "#NSFW #R18G "
    };

    let page_num_str = match page {
        Some(page) => format!(" ({}/{})", page, info.page_count),
        None => "".to_string()
    };

    format!(
        "{prefix}<a href=\"https://www.pixiv.net/artworks/{}\">{}</a> / <a href=\"https://www.pixiv.net/users/{}\">{}</a>{page_num_str}",
        info.id, info.title, info.author_id, info.author_name,
    )
}