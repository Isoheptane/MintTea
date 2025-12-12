use std::path::Path;

use frankenstein::client_reqwest::Bot;
use frankenstein::methods::GetFileParams;
use frankenstein::{reqwest, AsyncTelegramApi};
use tokio::io::AsyncWriteExt;
#[derive(Debug, Clone, Default)]
pub struct TelegramFileInfo {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
}

impl TelegramFileInfo {
    pub fn new(path: String, size: u64) -> Self {
        TelegramFileInfo {
            file_name: path_to_filename(&path),
            file_path: path,
            file_size: size
        }
    }
}

/// Convert a path */filename to filename, return original string if / is not present
fn path_to_filename(path: impl Into<String>) -> String {
    let path: String = path.into(); 
    match path.rsplit_once("/") {
        Some((_, filename)) => filename.to_string(),
        None => path
    }
}

pub async fn get_telegram_file_info(
    bot: &Bot,
    file_id: &str,
) -> anyhow::Result<Option<TelegramFileInfo>> {
    let result = bot.get_file(&GetFileParams::builder().file_id(file_id).build()).await?.result;
    let inner = match (result.file_path, result.file_size) {
        (Some(path), Some(size)) => Some(TelegramFileInfo::new(path, size)),
        _ => None
    };
    Ok(inner)
}

pub async fn download_url_to_memory(
    url: &str,
) -> anyhow::Result<Vec<u8>> {
    Ok(reqwest::get(url).await?.bytes().await?.to_vec())
}

pub async fn download_url_to_file(
    url: &str,
    save_file: &mut tokio::fs::File
) -> anyhow::Result<()> {
    let mut response = reqwest::get(url).await?;

    while let Some(chunk) = response.chunk().await? {
        save_file.write_all(&chunk).await?;
    };
    save_file.flush().await?;

    Ok(())
}

pub async fn download_url_to_path<P: AsRef<Path>>(
    url: &str,
    save_path: P
) -> anyhow::Result<tokio::fs::File> {
    let mut save_file = tokio::fs::File::create(save_path).await?;
    download_url_to_file(url, &mut save_file).await?;
    Ok(save_file)
}

fn get_telegram_file_link(token: &str, file_path: &str,) -> String {
    format!("https://api.telegram.org/file/bot{}/{}", token, file_path)
}

pub async fn download_telegram_to_memory(
    token: &str,
    file_path: &str,
) -> anyhow::Result<Vec<u8>> {
    download_url_to_memory(&get_telegram_file_link(token, file_path)).await
}

pub async fn download_telegram_file_to_file(
    token: &str,
    file_path: &str,
    save_file: &mut tokio::fs::File
) -> anyhow::Result<()> {
    download_url_to_file(&get_telegram_file_link(token, file_path), save_file).await

}

pub async fn download_telegram_file_to_path<P: AsRef<Path>>(
    token: &str,
    file_path: &str,
    save_path: P
) -> anyhow::Result<tokio::fs::File> {
    download_url_to_path(&get_telegram_file_link(token, file_path), save_path).await
}