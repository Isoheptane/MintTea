use std::time::Duration;

use reqwest::Client;

use crate::pixiv::config::PixivConfig;

#[derive(Debug)]
pub struct PixivContext {
    pub client: Client,
}

impl PixivContext {
    pub fn from_config(config: &PixivConfig) -> anyhow::Result<PixivContext> {
        let client = reqwest::Client::builder();
        let client = if let Some(ua) = config.client_user_agent.as_ref() { 
            client.user_agent(ua) 
        } else {client };
        let client = client
            .timeout(Duration::from_secs(10))
            .build()?;

        return Ok(PixivContext {
            client,
        })
    }
}

