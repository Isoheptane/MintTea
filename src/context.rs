

use std::collections::HashMap;

use frankenstein::client_reqwest::Bot;
use tokio::sync::RwLock;

use crate::config::BotConfig;
use crate::sticker::StickerModalState;
use crate::types::ChatSender;

#[derive(Debug, PartialEq, Clone)]
pub enum ModalState {
    Sticker(StickerModalState)
}

#[derive(Debug)]
pub struct ModalStateStorage {
    map: RwLock<HashMap<ChatSender, ModalState>>
}

impl ModalStateStorage {
    pub async fn set_state<T: Into<ChatSender>>(&self, chat_sender: T, state: ModalState) {
        let mut guard = self.map.write().await;
        guard.insert(chat_sender.into(), state);
    }

    pub async fn get_state<T: Into<ChatSender>>(&self, chat_sender: T) -> Option<ModalState> {
        let guard = self.map.read().await;
        guard.get(&chat_sender.into()).cloned()
    }

    pub async fn release_state<T: Into<ChatSender>>(&self, chat_sender: T) -> Option<ModalState> {
        let mut guard = self.map.write().await;
        guard.remove(&chat_sender.into())
    }
}

impl Default for ModalStateStorage {
    fn default() -> Self {
        ModalStateStorage {
            map: RwLock::new(HashMap::new())    
        }
    }
}

#[derive(Debug)]
pub struct Context {
    pub bot: Bot,
    pub config: BotConfig,
    pub modal_states: ModalStateStorage
}

impl Context {
    pub fn new(bot: Bot, config: BotConfig) -> Context {
        Context {
            bot,
            config,
            modal_states: ModalStateStorage::default()
        }
    }
}