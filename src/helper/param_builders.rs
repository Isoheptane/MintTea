use frankenstein::types::ReplyParameters;

pub fn reply_parameters(message_id: i32, chat_id: Option<i64>) -> ReplyParameters {
    ReplyParameters::builder()
        .message_id(message_id)
        .maybe_chat_id(chat_id)
        .build()
}