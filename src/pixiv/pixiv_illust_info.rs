use std::collections::HashMap;

use serde::Deserialize;

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