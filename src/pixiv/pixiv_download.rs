use std::{error::Error, fmt::Display, path::Path};

use reqwest::{Client, StatusCode};
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub enum PixivDownloadError {
    ReqwestError(reqwest::Error),
    IoError(std::io::Error),
    Unsuccess(StatusCode),
}

impl Display for PixivDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixivDownloadError::ReqwestError(error) => write!(f, "ReqwestError: {:?}", error),
            PixivDownloadError::IoError(error) => write!(f, "IoError: {:?}", error),
            PixivDownloadError::Unsuccess(status_code) => write!(f, "Unsuccess Request: {:?}", status_code),
        }
    }
}

impl From<reqwest::Error> for PixivDownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl From<std::io::Error> for PixivDownloadError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl Error for PixivDownloadError {}

pub async fn pixiv_download_image_to_file(
    client: Option<Client>,
    url: &str,
    file: &mut tokio::fs::File
) -> Result<(), PixivDownloadError> {

    let client = match client {
        Some(client) => client,
        // Possibly use other user agent
        None => Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")
            .build()?
    };

    let mut resp = client.get(url)
        .header("Referer", "https://www.pixiv.net/")
        .send().await
        .map_err(|e| PixivDownloadError::ReqwestError(e))?;

    let status = resp.status();

    if status.is_success() {
        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?
        }
        Ok(())
    } else {
        Err(PixivDownloadError::Unsuccess(status).into())
    }
}

pub async fn pixiv_download_image_to_path<P: AsRef<Path>>(
    client: Option<Client>,
    url: &str,
    save_path: P
) -> anyhow::Result<tokio::fs::File> {
    let mut save_file = tokio::fs::File::create(save_path).await?;
    pixiv_download_image_to_file(client, url, &mut save_file).await?;
    Ok(save_file)
}