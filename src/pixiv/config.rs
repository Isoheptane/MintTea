use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PixivConfig {
    pub php_sessid: Option<String>,
}