use serde::Deserialize;

#[derive(Clone, Debug, Deserialize  )]
pub struct FrameTimestamp {
    #[allow(unused)]
    pub file: String,
    pub delay: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PixivUgoiraMeta {
    #[allow(unused)]
    pub src: String,
    #[serde(rename = "originalSrc")]
    pub original_src: String,
    #[allow(unused)]
    pub mime_type: String,
    pub frames: Vec<FrameTimestamp>
}