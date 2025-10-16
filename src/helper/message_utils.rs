use frankenstein::types::{Message, MessageEntityType};

use crate::types::ChatSender;

/// Returns the command and the indicated bot
pub fn message_command(msg: &Message) -> Option<String> {
    let entity = msg.entities.as_ref()?.get(0)?;
    if entity.type_field != MessageEntityType::BotCommand || entity.offset != 0 {
        return None;
    }
    
    // Remove beginning "/" in the lice
    let command = msg.text.as_ref()?[1..entity.length as usize].split("@").nth(0)?.to_string();
    return Some(command)
}

/// Returns the chat sender combination of the message, sender_id is set to 0 if no sender specified
pub fn message_chat_sender(msg: &Message) -> ChatSender {
    let chat_id = msg.chat.id;
    // TODO: Need to test whether the chat_id is the same as the fake user's user id
    let sender_id = msg.from.as_ref().map(|user| user.id as i64).unwrap_or(0);
    return (chat_id, sender_id as i64).into()
}