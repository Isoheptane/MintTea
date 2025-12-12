use std::{error::Error, fmt::Display};

use reqwest::{Client, StatusCode};

#[derive(Debug)]
pub enum PixivDownloadError {
    ReqwestError(reqwest::Error),
    Unsuccess(StatusCode)
}

impl Display for PixivDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for PixivDownloadError {}

pub async fn pixiv_download_image(
    client: Option<Client>,
    url: &str,
) -> anyhow::Result<Vec<u8>> {

    let client = match client {
        Some(client) => client,
        None => Client::new()
    };

    let resp = client.get(url)
        .header("Referer", "https://www.pixiv.net/")
        .send().await
        .map_err(|e| PixivDownloadError::ReqwestError(e))?;

    let status = resp.status();

    if status.is_success() {
        let content = resp.bytes().await?.to_vec();
        Ok(content) 
    } else {
        Err(PixivDownloadError::Unsuccess(status).into())
    }
}