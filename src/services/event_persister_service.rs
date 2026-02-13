use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedEvent {
    pub event_id: String,
    pub room_id: String,
    pub sender: String,
    pub event_type: String,
    pub content: serde_json::Value,
    pub depth: u64,
    pub prev_events: Vec<String>,
    pub auth_events: Vec<String>,
    pub origin_server_ts: i64,
    pub state_key: Option<String>,
    pub processed_at: i64,
}

impl PersistedEvent {
    pub fn new(
        event_id: String,
        room_id: String,
        sender: String,
        event_type: String,
        content: serde_json::Value,
        depth: u64,
    ) -> Self {
        Self {
            event_id,
            room_id,
            sender,
            event_type,
            content,
            depth,
            prev_events: Vec::new(),
            auth_events: Vec::new(),
            origin_server_ts: Utc::now().timestamp_millis(),
            state_key: None,
            processed_at: 0,
        }
    }

    pub fn with_prev_events(mut self, events: Vec<String>) -> Self {
        self.prev_events = events;
        self
    }

    pub fn with_auth_events(mut self, events: Vec<String>) -> Self {
        self.auth_events = events;
        self
    }

    pub fn with_state_key(mut self, key: String) -> Self {
        self.state_key = Some(key);
        self
    }

    pub fn is_state_event(&self) -> bool {
        self.state_key.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBatch {
    pub batch_id: String,
    pub events: Vec<PersistedEvent>,
    pub created_at: i64,
    pub processed: bool,
}

impl EventBatch {
    pub fn new(events: Vec<PersistedEvent>) -> Self {
        Self {
            batch_id: uuid::Uuid::new_v4().to_string(),
            events,
            created_at: Utc::now().timestamp_millis(),
            processed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersisterConfig {
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub max_queue_size: usize,
}

impl Default for PersisterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval_ms: 1000,
            max_retries: 3,
            retry_delay_ms: 100,
            max_queue_size: 10000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersisterStats {
    pub total_events: u64,
    pub persisted_events: u64,
    pub failed_events: u64,
    pub batches_processed: u64,
    pub queue_size: usize,
}

pub struct EventPersister {
    config: PersisterConfig,
    events: Arc<RwLock<Vec<PersistedEvent>>>,
    batches: Arc<RwLock<HashMap<String, EventBatch>>>,
    stats: Arc<RwLock<PersisterStats>>,
}

impl EventPersister {
    pub fn new(config: PersisterConfig) -> Self {
        Self {
            config,
            events: Arc::new(RwLock::new(Vec::new())),
            batches: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PersisterStats::default())),
        }
    }

    pub async fn persist_event(&self, event: PersistedEvent) -> Result<(), PersisterError> {
        let mut events = self.events.write().await;
        
        if events.len() >= self.config.max_queue_size {
            return Err(PersisterError::QueueFull);
        }

        events.push(event);
        
        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        stats.queue_size = events.len();

        Ok(())
    }

    pub async fn persist_batch(&self, events: Vec<PersistedEvent>) -> Result<String, PersisterError> {
        if events.is_empty() {
            return Err(PersisterError::EmptyBatch);
        }

        let batch = EventBatch::new(events);
        let batch_id = batch.batch_id.clone();

        self.batches.write().await.insert(batch_id.clone(), batch);

        info!(batch_id = %batch_id, "Event batch created");

        Ok(batch_id)
    }

    pub async fn flush(&self) -> Result<usize, PersisterError> {
        let mut events = self.events.write().await;
        
        if events.is_empty() {
            return Ok(0);
        }

        let to_persist: Vec<PersistedEvent> = events.drain(..).collect();
        let count = to_persist.len();

        let batch = EventBatch::new(to_persist);
        let batch_id = batch.batch_id.clone();
        
        self.batches.write().await.insert(batch_id.clone(), batch);

        let mut stats = self.stats.write().await;
        stats.persisted_events += count as u64;
        stats.batches_processed += 1;
        stats.queue_size = 0;

        info!(batch_id = %batch_id, count = count, "Events flushed to persistence");

        Ok(count)
    }

    pub async fn process_batch(&self, batch_id: &str) -> Result<(), PersisterError> {
        let mut batches = self.batches.write().await;
        let batch = batches
            .get_mut(batch_id)
            .ok_or(PersisterError::BatchNotFound)?;

        if batch.processed {
            return Err(PersisterError::AlreadyProcessed);
        }

        for event in &mut batch.events {
            event.processed_at = Utc::now().timestamp_millis();
        }

        batch.processed = true;

        debug!(batch_id = %batch_id, event_count = batch.events.len(), "Batch processed");

        Ok(())
    }

    pub async fn get_event(&self, event_id: &str) -> Option<PersistedEvent> {
        let events = self.events.read().await;
        events.iter().find(|e| e.event_id == event_id).cloned()
    }

    pub async fn get_events_for_room(&self, room_id: &str) -> Vec<PersistedEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.room_id == room_id)
            .cloned()
            .collect()
    }

    pub async fn get_pending_count(&self) -> usize {
        self.events.read().await.len()
    }

    pub async fn get_stats(&self) -> PersisterStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.queue_size = self.events.read().await.len();
        result
    }

    pub async fn clear_processed_batches(&self) -> usize {
        let mut batches = self.batches.write().await;
        let before = batches.len();
        
        batches.retain(|_, b| !b.processed);
        
        before - batches.len()
    }

    pub async fn retry_failed(&self) -> Result<usize, PersisterError> {
        let batches = self.batches.read().await;
        let failed_count = batches
            .values()
            .filter(|b| !b.processed)
            .count();

        if failed_count > 0 {
            info!(count = failed_count, "Retrying failed batches");
        }

        Ok(failed_count)
    }
}

impl Default for EventPersister {
    fn default() -> Self {
        Self::new(PersisterConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PersisterError {
    #[error("Queue is full")]
    QueueFull,
    #[error("Empty batch")]
    EmptyBatch,
    #[error("Batch not found")]
    BatchNotFound,
    #[error("Already processed")]
    AlreadyProcessed,
    #[error("Persistence failed: {0}")]
    PersistenceFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_event(event_id: &str, room_id: &str) -> PersistedEvent {
        PersistedEvent::new(
            event_id.to_string(),
            room_id.to_string(),
            "@user:example.com".to_string(),
            "m.room.message".to_string(),
            json!({"body": "test"}),
            1,
        )
    }

    #[tokio::test]
    async fn test_persist_event() {
        let persister = EventPersister::default();

        let event = create_test_event("$event1", "!room1:example.com");
        
        persister.persist_event(event).await.unwrap();

        assert_eq!(persister.get_pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_persist_batch() {
        let persister = EventPersister::default();

        let events = vec![
            create_test_event("$event1", "!room1:example.com"),
            create_test_event("$event2", "!room1:example.com"),
        ];

        let batch_id = persister.persist_batch(events).await.unwrap();

        let batches = persister.batches.read().await;
        assert!(batches.contains_key(&batch_id));
        assert_eq!(batches.get(&batch_id).unwrap().events.len(), 2);
    }

    #[tokio::test]
    async fn test_flush() {
        let persister = EventPersister::default();

        persister.persist_event(create_test_event("$event1", "!room1")).await.unwrap();
        persister.persist_event(create_test_event("$event2", "!room1")).await.unwrap();

        let count = persister.flush().await.unwrap();
        assert_eq!(count, 2);
        assert_eq!(persister.get_pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_process_batch() {
        let persister = EventPersister::default();

        let events = vec![create_test_event("$event1", "!room1")];
        let batch_id = persister.persist_batch(events).await.unwrap();

        persister.process_batch(&batch_id).await.unwrap();

        let batches = persister.batches.read().await;
        let batch = batches.get(&batch_id).unwrap();
        assert!(batch.processed);
        assert!(batch.events[0].processed_at > 0);
    }

    #[tokio::test]
    async fn test_get_events_for_room() {
        let persister = EventPersister::default();

        persister.persist_event(create_test_event("$event1", "!room1")).await.unwrap();
        persister.persist_event(create_test_event("$event2", "!room1")).await.unwrap();
        persister.persist_event(create_test_event("$event3", "!room2")).await.unwrap();

        let room1_events = persister.get_events_for_room("!room1").await;
        assert_eq!(room1_events.len(), 2);

        let room2_events = persister.get_events_for_room("!room2").await;
        assert_eq!(room2_events.len(), 1);
    }

    #[tokio::test]
    async fn test_queue_full() {
        let config = PersisterConfig {
            max_queue_size: 2,
            ..Default::default()
        };
        let persister = EventPersister::new(config);

        persister.persist_event(create_test_event("$event1", "!room1")).await.unwrap();
        persister.persist_event(create_test_event("$event2", "!room1")).await.unwrap();

        let result = persister.persist_event(create_test_event("$event3", "!room1")).await;
        assert!(matches!(result, Err(PersisterError::QueueFull)));
    }

    #[tokio::test]
    async fn test_stats() {
        let persister = EventPersister::default();

        persister.persist_event(create_test_event("$event1", "!room1")).await.unwrap();
        persister.persist_event(create_test_event("$event2", "!room1")).await.unwrap();

        let stats = persister.get_stats().await;
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.queue_size, 2);
    }
}
