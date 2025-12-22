use std::path::Path;
use std::fmt::Display;
use std::sync::Arc;

use dashmap::DashMap;
use frankenstein::types::Message;
use uuid::Uuid;

use crate::monitor::rules::{MonitorRule, SavedMonitorRule};
use crate::helper::message_utils::get_sender_id;

#[derive(Debug)]
pub struct MonitorRuleSet {
    // May be very slow
    rules: DashMap<Uuid, Arc<MonitorRule>>,
    // Use Vec<T> as they are expected to be small
    rules_by_sender: DashMap<i64, Vec<Arc<MonitorRule>>>,

    rules_by_chat: DashMap<i64, Vec<Arc<MonitorRule>>>,

    rules_by_receiver: DashMap<i64, Vec<Arc<MonitorRule>>>,

}

impl Default for MonitorRuleSet {
    fn default() -> Self {
        MonitorRuleSet {
            rules: DashMap::new(),
            rules_by_sender: DashMap::new(), 
            rules_by_chat: DashMap::new(),
            rules_by_receiver: DashMap::new(),
        }
    }
}

impl MonitorRuleSet {
    /// Add a monitor rule to 
    pub fn add_rule(&self, rule: Arc<MonitorRule>, uuid: Uuid) {

        // let rule = Arc::new(rule);
        
        self.rules.insert(uuid, rule.clone());

        if let Some(sender_id) = rule.filter.sender_id {
            let mut list = self.rules_by_sender
                .entry(sender_id)
                .or_insert_with(|| Vec::new());
            list.push(rule.clone());
        }
        if let Some(chat_id) = rule.filter.chat_id {
            let mut list = self.rules_by_chat
                .entry(chat_id)
                .or_insert_with(|| Vec::new());
            list.push(rule.clone());
        }
        let mut list = self.rules_by_receiver
            .entry(rule.forward_to)
            .or_insert_with(|| Vec::new());
        list.push(rule.clone());
    }
    
    pub fn check_message(&self, msg: &Message) -> Vec<i64> {
        let mut receivers: Vec<i64> = vec![];
        if let Some(sender_id) = get_sender_id(msg) {
            if let Some(rules) = self.rules_by_sender.get(&sender_id) {
                for rule in rules.iter() {
                    if rule.filter.check_message(msg) {
                        receivers.push(rule.forward_to);
                    }
                }
            }
        }
        if let Some(rules) = self.rules_by_chat.get(&msg.chat.id) {
            for rule in rules.iter() {
                if rule.filter.check_message(msg) {
                    receivers.push(rule.forward_to);
                }
            }
        }
        receivers.sort_unstable();
        receivers.dedup();

        return receivers;
    }

    pub fn write_file(&self, path: impl AsRef<Path>) -> Result<(), SaveFileError> {

        let rules: Vec<SavedMonitorRule> = self.rules.iter()
            .map(|it| {
                SavedMonitorRule {
                    uuid: it.key().clone(),
                    rule: it.value().clone()
                }
            })
            .collect();

        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer(writer, &rules)?;

        Ok(())
    }

    pub fn add_from_file(&self, path: impl AsRef<Path>) -> Result<(), SaveFileError> {
        
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        let saved: Vec<SavedMonitorRule> = serde_json::from_reader(reader)?;

        for rule in saved {
            self.add_rule(rule.rule, rule.uuid);
        }

        Ok(())
    }

    pub fn len(&self) -> usize { self.rules.len() }
    #[allow(unused)]
    pub fn get_rule(&self, uuid: &Uuid) -> Option<Arc<MonitorRule>> {
        self.rules.get(uuid).map(|inner| inner.clone())
    }

    pub fn remove_rule(&self, uuid: &Uuid) -> bool{
        // Don't hold the rule for too long
        let rule = {
            let rule = self.rules.get(&uuid);
            match rule {
                Some(rule) => rule.clone(),
                None => return false,
            }
        };

        if let Some(id) = rule.filter.sender_id {
            if let Some(mut set) = self.rules_by_sender.get_mut(&id) {
                let pos = set.iter().position(|rule| rule.uuid == *uuid);
                if let Some(pos) = pos { set.swap_remove(pos); }
            }
        }
        if let Some(id) = rule.filter.chat_id {
            if let Some(mut set) = self.rules_by_chat.get_mut(&id) {
                let pos = set.iter().position(|rule| rule.uuid == *uuid);
                if let Some(pos) = pos { set.swap_remove(pos); }
            }
        }
        if let Some(mut set) = self.rules_by_receiver.get_mut(&rule.forward_to) {
            let pos = set.iter().position(|rule| rule.forward_to == rule.forward_to);
            if let Some(pos) = pos { set.swap_remove(pos); }
        }

        self.rules.remove(uuid);

        return true;
    }

    pub fn get_receiver_rules(&self, receiver_id :i64) -> Vec<Arc<MonitorRule>> {
        match self.rules_by_receiver.get(&receiver_id) {
            Some(rules) => rules.clone(),
            None => vec![]
        }
    }
}

pub enum SaveFileError {
    IoError(std::io::Error),
    SerdeJsonError(serde_json::Error)
}

impl From<std::io::Error> for SaveFileError {
    fn from(value: std::io::Error) -> Self {
        SaveFileError::IoError(value)
    }
}

impl From<serde_json::Error> for SaveFileError {
    fn from(value: serde_json::Error) -> Self {
        SaveFileError::SerdeJsonError(value)
    }
}

impl Display for SaveFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveFileError::IoError(error) => write!(f, "SaveFileError: {error}"),
            SaveFileError::SerdeJsonError(error) => write!(f, "SaveFileError: {error}"),
        }
    }
}

#[derive(Debug)]
pub struct MonitorContext {
    pub ruleset: MonitorRuleSet
}

impl Default for MonitorContext {
    fn default() -> Self {
        MonitorContext {
            ruleset: MonitorRuleSet::default()
        }
    }
}