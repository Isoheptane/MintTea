use frankenstein::response::{MessageOrBool, MethodResponse};
use frankenstein::AsyncTelegramApi;
use frankenstein::types::{ChatAction, Message};
use frankenstein::methods::{DeleteMessageParams, EditMessageTextParams, SendChatActionParams, SendMessageParams};
use frankenstein::client_reqwest::Bot;

// This module is designed to reduce builder codes

pub async fn send_message(bot: &Bot, chat_id: i64, text: impl Into<String>) -> Result<Message, frankenstein::Error> {
    let send_message_param = SendMessageParams::builder()
        .chat_id(chat_id)
        .text(text)
        .build();
    Ok(bot.send_message(&send_message_param).await?.result)
}

pub async fn edit_message_text(bot: &Bot, chat_id: i64, message_id: i32, text: impl Into<String>) -> Result<MessageOrBool, frankenstein::Error> {
    let edit_message_text_param = EditMessageTextParams::builder()
        .chat_id(chat_id)
        .message_id(message_id)
        .text(text)
        .build();
    Ok(bot.edit_message_text(&edit_message_text_param).await?.result)
}

pub async fn delete_message(bot: &Bot, chat_id: i64, message_id: i32) -> Result<MethodResponse<bool>, frankenstein::Error> {
    let param = DeleteMessageParams::builder()
        .chat_id(chat_id)
        .message_id(message_id)
        .build();
    Ok(bot.delete_message(&param).await?)
}

pub async fn sent_chat_action(bot: &Bot, chat_id: i64, action: ChatAction) -> Result<MethodResponse<bool>, frankenstein::Error>  {
    let param = SendChatActionParams::builder()
        .chat_id(chat_id)
        .action(action)
        .build();
    Ok(bot.send_chat_action(&param).await?)
}