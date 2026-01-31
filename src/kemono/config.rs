use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoConfig {
    pub client_user_agent: Option<String>,
    #[serde(default = "default_enable_kemono_link_detection")]
    pub enable_kemono_link_detection: bool,
    #[serde(default = "default_enable_fanbox_link_detection")]
    pub enable_fanbox_link_detection: bool,
}

fn default_enable_kemono_link_detection() -> bool { false }
fn default_enable_fanbox_link_detection() -> bool { false }