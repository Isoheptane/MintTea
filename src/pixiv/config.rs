use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PixivConfig {
    pub php_sessid: Option<String>,
    #[serde(default = "default_enable_link_detection")]
    pub enable_link_detection: bool,
}

fn default_enable_link_detection() -> bool { false }