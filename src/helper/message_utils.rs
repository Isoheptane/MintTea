use frankenstein::types::{Message, MessageEntityType};

use crate::types::ChatSender;

/// Returns the command and the indicated 
pub fn get_command(msg: &Message) -> Option<String> {
    let entity = msg.entities.as_ref()?.get(0)?;
    if entity.type_field != MessageEntityType::BotCommand || entity.offset != 0 {
        return None;
    }
    
    // Remove beginning "/" in the lice
    let command = msg.text.as_ref()?[1..entity.length as usize].split("@").nth(0)?.to_string();
    return Some(command)
}

/// Returns the command and the indicated 
/// #[allow(unused)]
pub fn get_withspace_split(msg: &Message) -> Vec<&str> {
    match msg.text.as_ref() {
        Some(text) => {
            let parts: Vec<&str> = text.split_whitespace().collect();
            return parts;
        }
        None => vec![]
    }
}

/// Return sender's id (User and Sender Chat)
pub fn get_sender_id(msg: &Message) -> Option<i64> {
    if let Some(sender_chat) = msg.sender_chat.as_ref() {
        Some(sender_chat.id)
    } else if let Some(user) = msg.from.as_ref() {
        Some(user.id as i64)
    } else {
        None
    }
}

/// Returns the chat sender combination of the message, sender_id is set to 0 if no sender is specified
pub fn get_chat_sender(msg: &Message) -> ChatSender {
    let chat_id = msg.chat.id;
    // TODO: Need to test whether the chat_id is the same as the fake user's user id
    let sender_id = get_sender_id(&msg);

    return (chat_id, sender_id.unwrap_or(0)).into();
}