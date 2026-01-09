use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoFile {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoPost {
    pub service: String,
    pub id: String,
    pub user: String,
    pub title: String,
    #[allow(unused)]
    pub content: String,
    pub file: KemonoFile,
    pub attachments: Vec<KemonoFile>
}

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoPostResponse {
    pub post: KemonoPost
}