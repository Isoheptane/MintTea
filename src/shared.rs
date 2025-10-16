

use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::config::BotConfig;
use crate::sticker::ChatStickerState;
use crate::types::ChatSender;

#[derive(Debug, PartialEq, Clone)]
pub enum ChatState {
    Sticker(ChatStickerState)
}

#[derive(Debug)]
pub struct ChatStateStorage {
    map: RwLock<HashMap<ChatSender, ChatState>>
}

impl ChatStateStorage {
    pub async fn set_state<T: Into<ChatSender>>(&self, chat_sender: T, state: ChatState) {
        let mut guard = self.map.write().await;
        guard.insert(chat_sender.into(), state);
    }

    pub async fn get_state<T: Into<ChatSender>>(&self, chat_sender: T) -> Option<ChatState> {
        let guard = self.map.read().await;
        guard.get(&chat_sender.into()).cloned()
    }

    pub async fn release_state<T: Into<ChatSender>>(&self, chat_sender: T) -> Option<ChatState> {
        let mut guard = self.map.write().await;
        guard.remove(&chat_sender.into())
    }
}

impl Default for ChatStateStorage {
    fn default() -> Self {
        ChatStateStorage {
            map: RwLock::new(HashMap::new())    
        }
    }
}

#[derive(Debug)]
pub struct SharedData {
    pub config: BotConfig,
    pub chat_state_storage: ChatStateStorage
}
