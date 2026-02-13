use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Thread relation types (MSC3440)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RelationType {
    Thread,
    InReplyTo,
    Replace,
    Annotation,
    Reference,
}

/// Thread relation for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "rel_type")]
    pub relation_type: RelationType,
    #[serde(rename = "event_id")]
    pub relates_to_event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// Thread summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub thread_id: String,
    pub room_id: String,
    pub root_event_id: String,
    pub latest_event_id: String,
    pub latest_sender: String,
    pub count: u64,
    pub current_user_participated: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ThreadSummary {
    pub fn new(room_id: String, root_event_id: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            thread_id: format!("thread_{}", root_event_id),
            room_id,
            root_event_id: root_event_id.clone(),
            latest_event_id: root_event_id,
            latest_sender: String::new(),
            count: 1,
            current_user_participated: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn increment(&mut self, event_id: String, sender: String) {
        self.count += 1;
        self.latest_event_id = event_id;
        self.latest_sender = sender;
        self.updated_at = Utc::now().timestamp_millis();
    }

    pub fn mark_participated(&mut self) {
        self.current_user_participated = true;
    }
}

/// Thread subscription for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSubscription {
    pub user_id: String,
    pub thread_id: String,
    pub room_id: String,
    pub subscribed_at: i64,
    pub notify: bool,
}

impl ThreadSubscription {
    pub fn new(user_id: String, thread_id: String, room_id: String) -> Self {
        Self {
            user_id,
            thread_id,
            room_id,
            subscribed_at: Utc::now().timestamp_millis(),
            notify: true,
        }
    }
}

/// Thread event with relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadEvent {
    pub event_id: String,
    pub room_id: String,
    pub sender: String,
    pub event_type: String,
    pub content: serde_json::Value,
    pub thread_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub created_at: i64,
}

impl ThreadEvent {
    pub fn new(
        event_id: String,
        room_id: String,
        sender: String,
        event_type: String,
        content: serde_json::Value,
    ) -> Self {
        let (thread_id, in_reply_to) = Self::extract_relations(&content);
        
        Self {
            event_id,
            room_id,
            sender,
            event_type,
            content,
            thread_id,
            in_reply_to,
            created_at: Utc::now().timestamp_millis(),
        }
    }

    fn extract_relations(content: &serde_json::Value) -> (Option<String>, Option<String>) {
        let relates_to = content.get("m.relates_to");
        
        if let Some(rel) = relates_to {
            let rel_type = rel.get("rel_type").and_then(|v| v.as_str());
            
            match rel_type {
                Some("m.thread") => {
                    let event_id = rel.get("event_id").and_then(|v| v.as_str());
                    (event_id.map(|_| "thread".to_string()), event_id.map(|s| s.to_string()))
                }
                Some("m.in_reply_to") => {
                    let event_id = rel.get("event_id").and_then(|v| v.as_str());
                    (None, event_id.map(|s| s.to_string()))
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        }
    }

    pub fn is_thread_reply(&self) -> bool {
        self.thread_id.is_some()
    }

    pub fn get_root_event_id(&self) -> Option<&str> {
        self.in_reply_to.as_deref()
    }
}

/// Thread service for managing threads
pub struct ThreadService {
    threads: Arc<RwLock<HashMap<String, ThreadSummary>>>,
    subscriptions: Arc<RwLock<HashMap<String, Vec<ThreadSubscription>>>>,
    events: Arc<RwLock<HashMap<String, Vec<ThreadEvent>>>>,
}

impl ThreadService {
    pub fn new() -> Self {
        Self {
            threads: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_thread(
        &self,
        room_id: &str,
        root_event_id: &str,
    ) -> ThreadSummary {
        let thread = ThreadSummary::new(room_id.to_string(), root_event_id.to_string());
        let thread_id = thread.thread_id.clone();
        
        self.threads.write().await.insert(thread_id.clone(), thread.clone());
        
        info!(
            thread_id = %thread_id,
            room_id = %room_id,
            root_event_id = %root_event_id,
            "Thread created"
        );
        
        thread
    }

    pub async fn add_event_to_thread(
        &self,
        _room_id: &str,
        root_event_id: &str,
        event: ThreadEvent,
    ) -> Option<ThreadSummary> {
        let thread_id = format!("thread_{}", root_event_id);
        
        let mut threads = self.threads.write().await;
        
        if let Some(thread) = threads.get_mut(&thread_id) {
            thread.increment(event.event_id.clone(), event.sender.clone());
            
            let mut events = self.events.write().await;
            events
                .entry(thread_id.clone())
                .or_default()
                .push(event);
            
            debug!(
                thread_id = %thread_id,
                count = thread.count,
                "Event added to thread"
            );
            
            return Some(thread.clone());
        }
        
        None
    }

    pub async fn get_thread(&self, thread_id: &str) -> Option<ThreadSummary> {
        self.threads.read().await.get(thread_id).cloned()
    }

    pub async fn get_thread_by_root_event(&self, root_event_id: &str) -> Option<ThreadSummary> {
        let thread_id = format!("thread_{}", root_event_id);
        self.get_thread(&thread_id).await
    }

    pub async fn get_thread_events(&self, thread_id: &str) -> Vec<ThreadEvent> {
        self.events
            .read()
            .await
            .get(thread_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_room_threads(&self, room_id: &str) -> Vec<ThreadSummary> {
        self.threads
            .read()
            .await
            .values()
            .filter(|t| t.room_id == room_id)
            .cloned()
            .collect()
    }

    pub async fn subscribe_to_thread(
        &self,
        user_id: &str,
        thread_id: &str,
        room_id: &str,
    ) -> ThreadSubscription {
        let subscription = ThreadSubscription::new(
            user_id.to_string(),
            thread_id.to_string(),
            room_id.to_string(),
        );
        
        self.subscriptions
            .write()
            .await
            .entry(user_id.to_string())
            .or_default()
            .push(subscription.clone());
        
        debug!(
            user_id = %user_id,
            thread_id = %thread_id,
            "User subscribed to thread"
        );
        
        subscription
    }

    pub async fn unsubscribe_from_thread(&self, user_id: &str, thread_id: &str) -> bool {
        let mut subscriptions = self.subscriptions.write().await;
        
        if let Some(user_subs) = subscriptions.get_mut(user_id) {
            let before = user_subs.len();
            user_subs.retain(|s| s.thread_id != thread_id);
            return user_subs.len() < before;
        }
        
        false
    }

    pub async fn get_user_subscriptions(&self, user_id: &str) -> Vec<ThreadSubscription> {
        self.subscriptions
            .read()
            .await
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_thread_subscribers(&self, thread_id: &str) -> Vec<String> {
        self.subscriptions
            .read()
            .await
            .values()
            .flat_map(|subs| {
                subs.iter()
                    .filter(|s| s.thread_id == thread_id && s.notify)
                    .map(|s| s.user_id.clone())
            })
            .collect()
    }

    pub async fn mark_user_participated(&self, thread_id: &str, user_id: &str) {
        if let Some(thread) = self.threads.write().await.get_mut(thread_id) {
            thread.mark_participated();
            debug!(
                thread_id = %thread_id,
                user_id = %user_id,
                "User marked as participated"
            );
        }
    }

    pub async fn get_thread_count(&self, room_id: &str) -> u64 {
        self.threads
            .read()
            .await
            .values()
            .filter(|t| t.room_id == room_id)
            .count() as u64
    }

    pub async fn delete_thread(&self, thread_id: &str) -> bool {
        let removed = self.threads.write().await.remove(thread_id).is_some();
        
        if removed {
            self.events.write().await.remove(thread_id);
            
            self.subscriptions
                .write()
                .await
                .values_mut()
                .for_each(|subs| subs.retain(|s| s.thread_id != thread_id));
            
            info!(thread_id = %thread_id, "Thread deleted");
        }
        
        removed
    }
}

impl Default for ThreadService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThreadError {
    #[error("Thread not found")]
    ThreadNotFound,
    #[error("Event not found")]
    EventNotFound,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid thread root")]
    InvalidThreadRoot,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_relation_type() {
        let rel = RelationType::Thread;
        assert_eq!(serde_json::to_string(&rel).unwrap(), r#""thread""#);
    }

    #[test]
    fn test_thread_summary() {
        let summary = ThreadSummary::new(
            "!room:example.com".to_string(),
            "$event123".to_string(),
        );
        
        assert_eq!(summary.room_id, "!room:example.com");
        assert_eq!(summary.root_event_id, "$event123");
        assert_eq!(summary.count, 1);
        assert!(!summary.current_user_participated);
    }

    #[test]
    fn test_thread_summary_increment() {
        let mut summary = ThreadSummary::new(
            "!room:example.com".to_string(),
            "$event123".to_string(),
        );
        
        summary.increment("$event456".to_string(), "@alice:example.com".to_string());
        
        assert_eq!(summary.count, 2);
        assert_eq!(summary.latest_event_id, "$event456");
        assert_eq!(summary.latest_sender, "@alice:example.com");
    }

    #[test]
    fn test_thread_event() {
        let content = json!({
            "msgtype": "m.text",
            "body": "Hello",
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": "$root123"
            }
        });
        
        let event = ThreadEvent::new(
            "$event456".to_string(),
            "!room:example.com".to_string(),
            "@alice:example.com".to_string(),
            "m.room.message".to_string(),
            content,
        );
        
        assert!(event.is_thread_reply());
        assert_eq!(event.get_root_event_id(), Some("$root123"));
    }

    #[tokio::test]
    async fn test_create_thread() {
        let service = ThreadService::new();
        
        let thread = service.create_thread(
            "!room:example.com",
            "$root123",
        ).await;
        
        assert_eq!(thread.room_id, "!room:example.com");
        assert_eq!(thread.root_event_id, "$root123");
    }

    #[tokio::test]
    async fn test_add_event_to_thread() {
        let service = ThreadService::new();
        
        service.create_thread("!room:example.com", "$root123").await;
        
        let content = json!({
            "msgtype": "m.text",
            "body": "Reply",
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": "$root123"
            }
        });
        
        let event = ThreadEvent::new(
            "$event456".to_string(),
            "!room:example.com".to_string(),
            "@alice:example.com".to_string(),
            "m.room.message".to_string(),
            content,
        );
        
        let result = service.add_event_to_thread(
            "!room:example.com",
            "$root123",
            event,
        ).await;
        
        assert!(result.is_some());
        let thread = result.unwrap();
        assert_eq!(thread.count, 2);
    }

    #[tokio::test]
    async fn test_subscribe_to_thread() {
        let service = ThreadService::new();
        
        let thread = service.create_thread("!room:example.com", "$root123").await;
        
        let subscription = service.subscribe_to_thread(
            "@alice:example.com",
            &thread.thread_id,
            "!room:example.com",
        ).await;
        
        assert_eq!(subscription.user_id, "@alice:example.com");
        assert!(subscription.notify);
    }

    #[tokio::test]
    async fn test_get_thread_subscribers() {
        let service = ThreadService::new();
        
        let thread = service.create_thread("!room:example.com", "$root123").await;
        
        service.subscribe_to_thread("@alice:example.com", &thread.thread_id, "!room:example.com").await;
        service.subscribe_to_thread("@bob:example.com", &thread.thread_id, "!room:example.com").await;
        
        let subscribers = service.get_thread_subscribers(&thread.thread_id).await;
        
        assert_eq!(subscribers.len(), 2);
    }

    #[tokio::test]
    async fn test_get_room_threads() {
        let service = ThreadService::new();
        
        service.create_thread("!room:example.com", "$root1").await;
        service.create_thread("!room:example.com", "$root2").await;
        service.create_thread("!other:example.com", "$root3").await;
        
        let threads = service.get_room_threads("!room:example.com").await;
        
        assert_eq!(threads.len(), 2);
    }
}
