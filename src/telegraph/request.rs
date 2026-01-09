use serde::{Deserialize, Serialize};

use crate::telegraph::types::Node;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatePageRequest {
    pub access_token: String,
    pub title: String,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub content: Vec<Node>,
    pub return_content: bool
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditPageRequest {
    pub access_token: String,
    pub path: String,
    pub title: String,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub content: Vec<Node>,
    pub return_content: bool
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetPageRequest {
    pub path: String,
    pub return_content: bool
}