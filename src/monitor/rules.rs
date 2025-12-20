use frankenstein::types::Message;
use serde::{Deserialize, Serialize};

use crate::helper::message_utils::get_sender_id;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FilterRule {
    pub sender_id: Option<i64>,
    pub chat_id: Option<i64>,
    pub keywords: Vec<String>
}

impl FilterRule {
    pub fn check_message(&self, msg :&Message) -> bool {
        // Sender ID filter
        if let Some(filt_sender_id) = self.sender_id {
            let Some(msg_sender_id) = get_sender_id(msg) else {
                // Ignore message with no sender id, if sender_id is required
                return false;
            };
            if msg_sender_id != filt_sender_id {
                return false;
            }
        }
        // Chat ID filter
        if let Some(filt_chat_id) = self.chat_id {
            if filt_chat_id != msg.chat.id {
                return false;
            }
        }
        // Keyword filter
        if self.keywords.is_empty() {
            // Empty keywords means no keywords required
            return true;
        }
        // Keyword required, check keywords

        // No need to check if the message does not contain any text or caption
        if msg.text.is_none() && msg.caption.is_none() {
            return false;
        }
        // Check keyword in text
        if let Some(text) = msg.text.as_ref() {
            for keyword in self.keywords.iter() {
                if text.contains(keyword) {
                    return true;
                }
            }
        }
        // Check keyword in caption
        if let Some(caption) = msg.caption.as_ref() {
            for keyword in self.keywords.iter() {
                if caption.contains(keyword) {
                    return true;
                }
            }
        }
        return false;
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MonitorRule {
    pub filter: FilterRule,
    /// Chat ID for forwarding
    pub forward_to: i64,
    /// Use label to help memorizing in the data file
    pub user_nickname: Option<String>,
    pub chat_title: Option<String>
}