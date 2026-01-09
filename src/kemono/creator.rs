use serde::Deserialize;


#[derive(Debug, Clone, Deserialize)]
pub struct CreatorProfile {
    #[allow(unused)]
    pub id: String,
    #[allow(unused)]
    pub name: String,
    pub public_id: String,
}