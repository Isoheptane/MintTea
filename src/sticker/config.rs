use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StickerConfig {
    #[serde(default = "size_limit_kb_default")]
    pub size_limit_kb: u64
}

fn size_limit_kb_default() -> u64 { 16384 }