#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChatSender {
    pub chat_id: i64,
    pub sender_id: i64,
}

impl From<(i64, i64)> for ChatSender {
    fn from((chat_id, sender_id): (i64, i64)) -> Self {
        ChatSender { chat_id, sender_id }
    }
}

