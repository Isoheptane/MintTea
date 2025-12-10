use dashmap::DashMap;
use frankenstein::client_reqwest::Bot;

use crate::config::BotConfig;
use crate::sticker::StickerModalState;
use crate::types::ChatSender;

#[derive(Debug, PartialEq, Clone)]
pub enum ModalState {
    Sticker(StickerModalState)
}

#[derive(Debug)]
pub struct ModalStateStorage {
    map: DashMap<ChatSender, ModalState>,
}

impl ModalStateStorage {
    pub async fn set_state<T: Into<ChatSender>>(&self, chat_sender: T, state: ModalState) {
        self.map.insert(chat_sender.into(), state);
    }

    pub async fn get_state<T: Into<ChatSender>>(&self, chat_sender: T) -> Option<ModalState> {
        self.map.get(&chat_sender.into()).map(|lock| lock.value().clone())
    }

    pub async fn release_state<T: Into<ChatSender>>(&self, chat_sender: T) -> Option<ModalState> {
        self.map.remove(&chat_sender.into()).map(|lock| lock.1)
    }
}

impl Default for ModalStateStorage {
    fn default() -> Self {
        ModalStateStorage {
            map: DashMap::new()
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