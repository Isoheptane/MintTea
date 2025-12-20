use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::monitor::rules::MonitorRule;

#[derive(Debug)]
pub struct MonitorContext {
    // May be very slow
    rules: RwLock<Vec<MonitorRule>>,
    
    rules_by_sender: DashMap<i64, Vec<(usize, MonitorRule)>>,

    rules_by_chat: DashMap<i64, Vec<(usize, MonitorRule)>>,

}

impl Default for MonitorContext {
    fn default() -> Self {
        let rules: Vec<MonitorRule> = vec![];
        let sender_map = DashMap::new();
        let chat_map = DashMap::new();
        MonitorContext {
            rules: RwLock::new(rules),
            rules_by_sender: sender_map, 
            rules_by_chat: chat_map
        }
    }
}

impl MonitorContext {
    pub async fn add_rule(&self, rule :MonitorRule) {
        let index: usize = {
            let mut guard = self.rules.write().await;
            guard.push(rule.clone());
            guard.len()
        };
        if let Some(sender_id) = rule.filter.sender_id {
            let mut list = self.rules_by_sender.entry(sender_id).or_insert(vec![]);
            list.push((index, rule.clone())); 
        }
        if let Some(chat_id) = rule.filter.chat_id {
            let mut list = self.rules_by_chat.entry(chat_id).or_insert(vec![]);
            list.push((index, rule.clone())); 
        }
    }
}