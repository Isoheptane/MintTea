use std::sync::Arc;

use frankenstein::types::Message;
use serde::Deserialize;

use crate::context::Context;
use crate::helper::bot_actions;
use crate::helper::log::LogOp;
use crate::kemono::post::KemonoPost;
use crate::telegraph::request::CreatePageRequest;
use crate::telegraph::types::{Node, NodeElement, Page, TelegraphResponse};
use crate::types::FileName;

const KEMONO_PREFIX: &'static str = "https://kemono.cr";

enum FileType {
    Image,
    Video,
    Other
}

fn check_file_type(file_name: &str) -> FileType {
    const IMAGE_EXT: &[&'static str] = &["png", "jpg", "gif", "tiff", "tga", "webp"];
    const VIDEO_EXT: &[&'static str] = &["mp4",  "mov", "mkv",];


    let file_name = FileName::from(file_name);
    let ext = file_name.extension_str();
    if IMAGE_EXT.iter().any(|image_ext| ext.to_lowercase() == *image_ext) {
        FileType::Image
    } else if VIDEO_EXT.iter().any(|video_ext| ext.to_lowercase() == *video_ext) {
        FileType::Video
    } else {
        FileType::Other
    }
}

pub async fn send_telegraph_preview(
    ctx: Arc<Context>, 
    msg: Arc<Message>,
    kemono_post: &KemonoPost
) -> anyhow::Result<()> {

    let original_url = format!("https://kemono.cr/{}/user/{}/posts/{}", kemono_post.service, kemono_post.user, kemono_post.id);

    let mut content: Vec<Node> = vec![];
    content.push(Node::NodeElement(NodeElement::image(&format!("{}{}", KEMONO_PREFIX, kemono_post.file.path))));
    content.push(Node::NodeElement(NodeElement::h4("Preview")));
    let mut have_preview = false;
    for file in &kemono_post.attachments {
        match check_file_type(&file.name) {
            FileType::Image => {
                have_preview = true;
                content.push(Node::NodeElement(NodeElement::image(&format!("{}{}", KEMONO_PREFIX, file.path))));
            },
            FileType::Video => {
                have_preview = true;
                content.push(Node::NodeElement(NodeElement::video(&format!("{}{}", KEMONO_PREFIX, file.path))));
            },
            _ => {}
        }
    }
    if !have_preview {
        content.push(Node::NodeElement(NodeElement::paragraph_text("No previewable media")));
    }
    content.push(Node::NodeElement(NodeElement::h4("Attachments")));
    let mut have_attachment = false;
    for file in &kemono_post.attachments {
        match check_file_type(&file.name) {
            FileType::Other => {
                have_attachment = true;
                content.push(Node::NodeElement(NodeElement::paragraph(vec![
                    Node::NodeElement(NodeElement::link(&file.name, Some(&format!("{}{}", KEMONO_PREFIX, file.path))))
                ])));
            },
            _ => {}
        }
    }
    if !have_attachment {
        content.push(Node::NodeElement(NodeElement::paragraph_text("No attachments")));
    }
    content.push(Node::NodeElement(NodeElement::paragraph(vec![
        Node::NodeElement(NodeElement::link(&original_url, Some(&original_url)))
    ])));


    let chat_name = if let Some(title) = msg.chat.title.as_ref() {
        Some(title.to_string())
    } else if let Some(first_name) = msg.chat.first_name.as_ref() {
        if let Some(last_name) = msg.chat.last_name.as_ref() {
            Some(format!("{} {}", first_name, last_name))
        } else {
            Some(first_name.to_string())
        }
    } else {
        None
    };

    let create_page_req = CreatePageRequest {
        access_token: ctx.config.telegraph.access_token.clone(),
        title: kemono_post.title.clone(),
        author_name: chat_name,
        author_url: None,
        content: content,
        return_content: false,
    };

    let client = reqwest::Client::new();
    let response: TelegraphResponse = client.post("https://api.telegra.ph/createPage")
        .json(&create_page_req)
        .send()
        .await?
        .json()
        .await?;

    let Some(page) = response.result else {
        log::warn!(
            target: "kemono_telegraph",
            "{} Create telegraph page failed",
            LogOp(&msg)
        );
        return Ok(())
    };

    let page: Page = Page::deserialize(page)?;

    bot_actions::send_html_message(&ctx.bot, msg.chat.id, format!(
        "Telegraph 預覽: <a href=\"{}\">{}</a>", page.url, page.title
    )).await?;

    Ok(())
}