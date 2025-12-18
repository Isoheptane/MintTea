use std::{error::Error, fmt::Display, path::Path};

use reqwest::{Client, StatusCode};
use tokio::io::AsyncWriteExt;

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

pub async fn download_to_file(
    client: Option<&Client>,
    url: &str,
    file: &mut tokio::fs::File
) -> Result<(), DownloadError> {

    let client = match client {
        Some(client) => client,
        // Possibly use other user agent
        None => &Client::builder().build()?
    };

    let mut resp = client.get(url)
        .header("Referer", "https://www.pixiv.net/")
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

pub async fn download_to_path<P: AsRef<Path>>(
    client: Option<&Client>,
    url: &str,
    save_path: P
) -> anyhow::Result<tokio::fs::File> {
    let mut save_file = tokio::fs::File::create(save_path).await?;
    download_to_file(client, url, &mut save_file).await?;
    Ok(save_file)
}