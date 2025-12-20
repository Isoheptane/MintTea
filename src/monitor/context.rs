use std::collections::BTreeSet;

use dashmap::DashMap;
use frankenstein::types::Message;
use uuid::Uuid;

use crate::{helper::message_utils::get_sender_id, monitor::rules::MonitorRule};

#[derive(Debug)]
pub struct MonitorRuleSet {
    // May be very slow
    rules: DashMap<Uuid, MonitorRule>,
    
    rules_by_sender: DashMap<i64, BTreeSet<Uuid>>,

    rules_by_chat: DashMap<i64, BTreeSet<Uuid>>,

}

impl Default for MonitorRuleSet {
    fn default() -> Self {
        MonitorRuleSet {
            rules: DashMap::new(),
            rules_by_sender: DashMap::new(), 
            rules_by_chat: DashMap::new()
        }
    }
}

impl MonitorRuleSet {
    /// Add a monitor rule to 
    pub async fn add_rule(&self, rule: MonitorRule, uuid: Uuid) {
        
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
    }
    
    pub async fn check_message(&self, msg: &Message) -> Vec<i64> {
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
