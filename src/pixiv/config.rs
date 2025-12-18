use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PixivConfig {
    pub client_user_agent: Option<String>,
    pub php_sessid: Option<String>,
    #[serde(default = "default_enable_link_detection")]
    pub enable_link_detection: bool,
    #[serde(default = "default_spoiler_nsfw")]
    pub spoiler_nsfw: bool,
    #[serde(default = "default_spoiler_r18g")]
    pub spoiler_r18g: bool,
}

fn default_enable_link_detection() -> bool { false }
fn default_spoiler_nsfw() -> bool { true }
fn default_spoiler_r18g() -> bool { true }