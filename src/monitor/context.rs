use std::path::Path;
use std::fmt::Display;
use std::collections::BTreeSet;

use dashmap::DashMap;
use frankenstein::types::Message;
use uuid::Uuid;

use crate::monitor::rules::{MonitorRule, SavedMonitorRule};
use crate::helper::message_utils::get_sender_id;

#[derive(Debug)]
pub struct MonitorRuleSet {
    // May be very slow
    rules: DashMap<Uuid, MonitorRule>,
    
    rules_by_sender: DashMap<i64, BTreeSet<Uuid>>,

    rules_by_chat: DashMap<i64, BTreeSet<Uuid>>,

    rules_by_receiver: DashMap<i64, BTreeSet<Uuid>>,

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
    pub fn add_rule(&self, rule: MonitorRule, uuid: Uuid) {
        
        self.rules.insert(uuid, rule.clone());

        if let Some(sender_id) = rule.filter.sender_id {
            let mut list = self.rules_by_sender
                .entry(sender_id)
                .or_insert_with(|| BTreeSet::new());
            list.insert(uuid);
        }
        if let Some(chat_id) = rule.filter.chat_id {
            let mut list = self.rules_by_chat
                .entry(chat_id)
                .or_insert_with(|| BTreeSet::new());
            list.insert(uuid);
        }
        let mut list = self.rules_by_receiver
            .entry(rule.forward_to)
            .or_insert_with(|| BTreeSet::new());
        list.insert(uuid);
    }
    
    pub fn check_message(&self, msg: &Message) -> Vec<i64> {
        let mut matched_uuids: BTreeSet<Uuid> = BTreeSet::new();
        if let Some(sender_id) = get_sender_id(msg) {
            let rules = self.rules_by_sender.get(&sender_id);
            if let Some(rules) = rules {
                matched_uuids.append(&mut rules.clone());
            }
        }
        if let Some(rules) = self.rules_by_chat.get(&msg.chat.id) {
            matched_uuids.append(&mut rules.clone());
        }

        let send_to: Vec<i64> = matched_uuids.iter()
            .filter_map(|uuid| self.rules.get(uuid))
            .filter(|rule| rule.filter.check_message(&msg))
            .map(|rule| rule.forward_to)
            .collect();

        return send_to;
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

    pub fn get_rule(&self, uuid: &Uuid) -> Option<MonitorRule> {
        self.rules.get(uuid).map(|inner| inner.clone())
    }

    pub fn remove_rule(&self, uuid: &Uuid) -> bool{
        let rule = self.rules.get(&uuid);
        let Some(rule) = rule else { return false; };

        let sender_id = rule.filter.sender_id;
        let chat_id = rule.filter.chat_id;
        let receiver_id = rule.forward_to;

        if let Some(id) = sender_id {
            if let Some(mut set) = self.rules_by_sender.get_mut(&id) {
                set.remove(uuid);
            }
        }

        if let Some(id) = chat_id {
            if let Some(mut set) = self.rules_by_chat.get_mut(&id) {
                set.remove(uuid);
            }
        }

        if let Some(mut set) = self.rules_by_receiver.get_mut(&receiver_id) {
            set.remove(uuid);
        }

        return true;
    }

    pub fn get_receiver_rules_uuid(&self, receiver_id :i64) -> Vec<Uuid> {
        if let Some(set) = self.rules_by_receiver.get(&receiver_id) {
            set.iter().map(|u| u.clone()).collect()
        } else {
            vec![]
        }
    }

    pub fn get_receiver_rules(&self, receiver_id :i64) -> Vec<(Uuid, MonitorRule)> {
        let uuid = self.get_receiver_rules_uuid(receiver_id);
        uuid.iter()
            .filter_map(|u|self.rules.get(u))
            .map(|rule| (rule.key().clone(), rule.value().clone()))
            .collect()
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