mod telegram;
mod pixiv;

use std::fs::File;
use std::error::Error;

use serde::Deserialize;

use crate::config::pixiv::PixivConfig;
use crate::config::telegram::TelegramConfig;

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
    pub pixiv: PixivConfig,
}

impl BotConfig {
    pub fn read_config(path: &str) -> Result<BotConfig, ConfigError> {
        let file = File::open(path)?;
        let config: BotConfig = serde_json::from_reader(file)?;
        Ok(config)
    }
}