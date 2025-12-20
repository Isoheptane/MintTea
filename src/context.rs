use std::path::PathBuf;

use dashmap::DashMap;
use frankenstein::client_reqwest::Bot;

use crate::config::BotConfig;
use crate::monitor::MonitorModalState;
use crate::monitor::context::MonitorContext;
use crate::pixiv::context::PixivContext;
use crate::sticker::StickerModalState;
use crate::types::ChatSender;

#[derive(Debug, PartialEq, Clone)]
pub enum ModalState {
    Sticker(StickerModalState),
    Monitor(MonitorModalState),
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
    pub temp_root_path: PathBuf,
    pub data_root_path: PathBuf,
    pub modal_states: ModalStateStorage,

    pub pixiv: PixivContext,
    pub monitor: MonitorContext,
}

impl Context {
    pub fn new(bot: Bot, config: BotConfig, temp_root_path: PathBuf, data_root_path: PathBuf) -> Context {
        let pixiv =  PixivContext::from_config(&config.pixiv).expect("Failed to create Pixiv Context");
        let monitor = MonitorContext::default();
        Context {
            bot,
            config,
            temp_root_path,
            data_root_path,
            modal_states: ModalStateStorage::default(),
            pixiv,
            monitor
        }
    }
}