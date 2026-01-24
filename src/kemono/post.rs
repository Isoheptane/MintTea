use serde::de::Deserializer;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoFile {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoPost {
    pub service: String,
    pub id: String,
    pub user: String,
    pub title: String,
    #[allow(unused)]
    pub content: String,
    #[serde(deserialize_with = "file_or_empty_object")]
    pub file: Option<KemonoFile>,
    pub attachments: Vec<KemonoFile>
}

#[derive(Debug, Clone, Deserialize)]
pub struct KemonoPostResponse {
    pub post: KemonoPost
}

fn file_or_empty_object<'de, D>(d: D) -> Result<Option<KemonoFile>, D::Error> where D: Deserializer<'de> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Helper {
        Data(KemonoFile),
        Empty {},
        Null,
    }

    match Helper::deserialize(d) {
        Ok(Helper::Data(file)) => Ok(Some(file)),
        Ok(_) => Ok(None),
        Err(e) => Err(e)
    }
}