use std::fs::File;
use std::error::Error;

use serde::Deserialize;

use crate::kemono::config::KemonoConfig;
use crate::pixiv::config::PixivConfig;
use crate::sticker::config::StickerConfig;

/* Config Error */

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    DeserializeError(serde_json::Error)
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::IoError(value)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        ConfigError::DeserializeError(value)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO Error: {}", e),
            ConfigError::DeserializeError(e) => write!(f, "Deserialize Error: {}", e),
        }
    }
}

impl Error for ConfigError {}

/* Config */

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    pub telegram: TelegramConfig,
    pub sticker: StickerConfig,
    pub pixiv: PixivConfig,
    pub kemono: KemonoConfig,
}

impl BotConfig {
    pub fn read_config(path: &str) -> Result<BotConfig, ConfigError> {
        let file = File::open(path)?;
        let config: BotConfig = serde_json::from_reader(file)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
    #[serde(default = "default_api_server")]
    pub bot_api_server: String,
}

pub fn default_api_server() -> String { "https://api.telegram.org".to_string() }