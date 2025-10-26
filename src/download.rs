use std::sync::Arc;

use frankenstein::client_reqwest::Bot;
use frankenstein::methods::GetFileParams;
use frankenstein::{reqwest, AsyncTelegramApi};

use crate::shared::SharedData;

#[derive(Debug, Clone, Default)]
pub struct FileBaseExt {
    pub basename: String,
    pub extension: String,
}

impl FileBaseExt {
    pub fn new(basename: String, extension: String) -> FileBaseExt {
        FileBaseExt { basename, extension }
    }
}

impl<T> From<T> for FileBaseExt where T: Into<String> {
    fn from(name: T) -> Self {
        let name: String = name.into();
        let split: Vec<&str> = name.split('.').collect();
        let extension = split.last().map(|ext| ext.to_string()).unwrap_or("".to_string());
        let basename = split[0..(split.len() - 1)].join(".");
        return FileBaseExt::new(basename, extension);
    }
}

impl ToString for FileBaseExt {
    fn to_string(&self) -> String {
        format!("{}.{}", self.basename, self.extension)
    }
}

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
    bot: Bot,
    data: &Arc<SharedData>,
    file_id: impl Into<String>,
) -> anyhow::Result<Option<DownloadedFile>> {
    let file_info = bot.get_file(&GetFileParams::builder().file_id(file_id).build()).await?.result;
    let path = match file_info.file_path {
        Some(x) => x,
        None => return Ok(None)
    };
    let file_name = path_to_filename(&path).unwrap_or("".to_string());
    
    let bytes = reqwest::get(format!("https://api.telegram.org/file/bot{}/{}", data.config.telegram.token, path)).await?
        .bytes().await?
        .to_vec();
    return Ok(Some(DownloadedFile::new(file_name, bytes)));
}