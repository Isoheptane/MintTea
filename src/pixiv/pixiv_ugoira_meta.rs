use serde::Deserialize;

#[derive(Clone, Debug, Deserialize  )]
pub struct FrameTimestamp {
    pub file: String,
    pub delay: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PixivUgoiraMeta {
    pub src: String,
    #[serde(rename = "originalSrc")]
    pub original_src: String,
    pub mime_type: String,
    pub frames: Vec<FrameTimestamp>
}