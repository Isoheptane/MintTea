use std::collections::HashMap;

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Node {
    String(String),
    NodeElement(NodeElement)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeElement {
    pub tag: String,
    pub attrs: Option<HashMap<String, String>>,
    pub children: Option<Vec<Node>>
}

impl NodeElement {
    pub fn paragraph_text(text: &str) -> NodeElement {
        NodeElement {
            tag: "p".to_string(),
            attrs: None,
            children: Some(vec![Node::String(text.to_string())])
        }
    }
    pub fn paragraph(children: Vec<Node>) -> NodeElement {
        NodeElement {
            tag: "p".to_string(),
            attrs: None,
            children: Some(children)
        }
    }
    #[allow(unused)]
    pub fn h3(text: &str) -> NodeElement {
        NodeElement {
            tag: "h3".to_string(),
            attrs: None,
            children: Some(vec![Node::String(text.to_string())])
        }
    }
    pub fn h4(text: &str) -> NodeElement {
        NodeElement {
            tag: "h4".to_string(),
            attrs: None,
            children: Some(vec![Node::String(text.to_string())])
        }
    }
    pub fn link(text: &str, href: Option<&str>) -> NodeElement {
        NodeElement {
            tag: "a".to_string(),
            attrs: href.map(|href| hashmap! { "href".to_string() => href.to_string() }),
            children: Some(vec![Node::String(text.to_string())])
        }
    }
    pub fn image(src: &str) -> NodeElement {
        NodeElement {
            tag: "img".to_string(),
            attrs: Some(hashmap! { "src".to_string() => src.to_string() }),
            children: None
        }
    }
    pub fn video(src: &str) -> NodeElement {
        NodeElement {
            tag: "video".to_string(),
            attrs: Some(hashmap! { "src".to_string() => src.to_string() }),
            children: None
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Page {
    pub path: String,
    pub url: String,
    pub title: String,
    pub description: String,
    pub author_name: Option<String>,
    pub image_url: Option<String>,
    pub content: Option<Vec<Node>>,
    pub views: u64,
    pub can_edit: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegraphResponse {
    pub ok: bool,
    pub error: Option<Value>,
    pub result: Option<Value>,
}