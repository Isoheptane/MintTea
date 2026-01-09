use serde::Deserialize;


#[derive(Debug, Clone, Deserialize)]
pub struct CreatorProfile {
    pub id: String,
    pub name: String,
    pub public_id: String,
}