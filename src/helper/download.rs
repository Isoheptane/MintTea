use std::sync::Arc;

use frankenstein::methods::GetFileParams;
use frankenstein::{reqwest, AsyncTelegramApi};

use crate::context::Context;

#[derive(Debug, Clone, Default)]
pub struct DownloadedFile {
    pub file_name: String,
    pub data: Vec<u8>
}

impl DownloadedFile {
    pub fn new(file_name: String, data: Vec<u8>) -> Self {
        DownloadedFile { file_name, data }
    }
}

fn path_to_filename(path: impl Into<String>) -> Option<String> {
    let path: String = path.into(); 
    path.split('/').last().map(|path| path.to_string())
}

pub async fn download_file(
    ctx: Arc<Context>,
    file_id: impl Into<String>,
) -> anyhow::Result<Option<DownloadedFile>> {
    let file_info = ctx.bot.get_file(&GetFileParams::builder().file_id(file_id).build()).await?.result;
    let path = match file_info.file_path {
        Some(x) => x,
        None => return Ok(None)
    };
    let file_name = path_to_filename(&path).unwrap_or("".to_string());
    
    let bytes = reqwest::get(format!("https://api.telegram.org/file/bot{}/{}", ctx.config.telegram.token, path)).await?
        .bytes().await?
        .to_vec();
    return Ok(Some(DownloadedFile::new(file_name, bytes)));
}