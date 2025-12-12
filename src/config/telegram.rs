use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
    pub bot_api_server: String,
}