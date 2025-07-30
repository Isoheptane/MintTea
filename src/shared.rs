

use std::collections::HashMap;

use teloxide::types::ChatId;
use tokio::sync::RwLock;

use crate::config::BotConfig;
use crate::sticker::ChatStickerState;

#[derive(Debug, PartialEq, Clone)]
pub enum ChatState {
    Sticker(ChatStickerState)
}

#[derive(Debug)]
pub struct ChatStateStorage {
    map: RwLock<HashMap<ChatId, ChatState>>
}

impl ChatStateStorage {
    pub async fn set_state(&self, chat_id: ChatId, state: ChatState) {
        let mut guard = self.map.write().await;
        guard.insert(chat_id, state);
    }

    pub async fn get_state(&self, chat_id: ChatId) -> Option<ChatState> {
        let guard = self.map.read().await;
        guard.get(&chat_id).cloned()
    }

    pub async fn release_state(&self, chat_id: ChatId) -> Option<ChatState> {
        let mut guard = self.map.write().await;
        guard.remove(&chat_id)
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
