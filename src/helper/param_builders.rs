use frankenstein::types::{ReplyKeyboardRemove, ReplyMarkup, ReplyParameters};

#[allow(unused)]
pub fn reply_parameters(message_id: i32, chat_id: Option<i64>) -> ReplyParameters {
    ReplyParameters::builder()
        .message_id(message_id)
        .maybe_chat_id(chat_id)
        .build()
}

#[allow(unused)]
pub fn reply_keyboard_remove() -> ReplyMarkup {
    ReplyMarkup::ReplyKeyboardRemove(ReplyKeyboardRemove::builder().remove_keyboard(true).build())
}