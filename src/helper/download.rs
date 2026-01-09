use std::error::Error;
use std::fmt::Display;
use std::path::Path;

use frankenstein::client_reqwest::Bot;
use frankenstein::methods::GetFileParams;
use frankenstein::{reqwest, AsyncTelegramApi};
use reqwest::{Client, StatusCode};
use tokio::io::AsyncWriteExt;

use crate::context::Context;
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
    let tail_delimiter = |c| "?#&".contains(c);
    let path = match path.split_once(tail_delimiter) {
        Some((stub, _)) => stub,
        None => &path
    };
    match path.rsplit_once("/") {
        Some((_, filename)) => filename.to_string(),
        None => path.to_string()
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

#[derive(Debug)]
pub enum DownloadError {
    ReqwestError(reqwest::Error),
    IoError(std::io::Error),
    Unsuccess(StatusCode),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::ReqwestError(error) => write!(f, "ReqwestError: {:?}", error),
            DownloadError::IoError(error) => write!(f, "IoError: {:?}", error),
            DownloadError::Unsuccess(status_code) => write!(f, "Unsuccess Request: {:?}", status_code),
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl Error for DownloadError {}

pub async fn download_url_to_memory(
    client: Option<Client>,
    url: &str,
) -> Result<Vec<u8>, DownloadError> {
    let client = match client {
        Some(client) => client,
        // Possibly use other user agent
        None => Client::builder().build()?
    };

    let resp = client.get(url)
        .send().await
        .map_err(|e| DownloadError::ReqwestError(e))?;

    let status = resp.status();

    if status.is_success() {
        Ok(resp.bytes().await?.to_vec())
    } else {
        Err(DownloadError::Unsuccess(status).into())
    }
}

pub async fn download_url_to_file(
    client: Option<Client>,
    url: &str,
    file: &mut tokio::fs::File
) -> Result<(), DownloadError> {

    let client = match client {
        Some(client) => client,
        // Possibly use other user agent
        None => Client::builder().build()?
    };

    let mut resp = client.get(url)
        .send().await
        .map_err(|e| DownloadError::ReqwestError(e))?;

    let status = resp.status();

    if status.is_success() {
        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?
        }
        Ok(())
    } else {
        Err(DownloadError::Unsuccess(status).into())
    }
}

pub async fn download_url_to_path<P: AsRef<Path>>(
    client: Option<Client>,
    url: &str,
    save_path: P
) -> anyhow::Result<tokio::fs::File> {
    let mut save_file = tokio::fs::File::create(save_path).await?;
    download_url_to_file(client, url, &mut save_file).await?;
    Ok(save_file)
}

fn get_telegram_file_link(api_url: &str, token: &str, file_path: &str,) -> String {
    format!("{}/file/bot{}/{}", api_url, token, file_path)
}

fn get_telegram_file_link_by_context(ctx: &Context, file_path: &str,) -> String {
    get_telegram_file_link(&ctx.config.telegram.bot_api_server, &ctx.config.telegram.token, file_path)
}

#[allow(unused)]
pub async fn download_telegram_to_memory(
    ctx: &Context,
    file_path: &str,
) -> anyhow::Result<Vec<u8>> {
    Ok(download_url_to_memory(None, &get_telegram_file_link_by_context(ctx, file_path)).await?)
}

#[allow(unused)]
pub async fn download_telegram_file_to_file(
    ctx: &Context,
    file_path: &str,
    save_file: &mut tokio::fs::File
) -> anyhow::Result<()> {
    Ok(download_url_to_file(None, &get_telegram_file_link_by_context(ctx, file_path), save_file).await?)
}

pub async fn download_telegram_file_to_path<P: AsRef<Path>>(
    ctx: &Context,
    file_path: &str,
    save_path: P
) -> anyhow::Result<tokio::fs::File> {
    Ok(download_url_to_path(None, &get_telegram_file_link_by_context(ctx, file_path), save_path).await?)
}