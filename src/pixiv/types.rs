use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

/* User request */

#[derive(Clone, Debug, PartialEq)]
pub enum SendMode {
    Photos,
    Files,
    Archive
}

#[derive(Clone, Debug)]
pub struct IllustRequest {
    pub no_page_limit: bool,
    pub silent_page_limit: bool,
    pub send_mode: SendMode
}

/* Server Response */

#[derive(Clone, Debug, Deserialize)]
pub struct PixivResponse {
    pub error: bool,
    pub message: String,
    pub body: Value
}

/* Pixiv Illust */

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
    #[allow(unused)]
    pub romaji: Option<String>,
    #[allow(unused)]
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
pub struct IllustInfo {
    pub id: String,
    pub title: String,
    #[allow(unused)]
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

/* Pixiv Ugoira */

#[derive(Clone, Debug, Deserialize  )]
pub struct FrameTimestamp {
    #[allow(unused)]
    pub file: String,
    pub delay: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UgoiraMeta {
    #[allow(unused)]
    pub src: String,
    #[serde(rename = "originalSrc")]
    pub original_src: String,
    #[allow(unused)]
    pub mime_type: String,
    pub frames: Vec<FrameTimestamp>
}