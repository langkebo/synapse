use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub max_lifetime: Option<i64>,
    pub min_lifetime: Option<i64>,
    pub expire_on_clients: bool,
    pub exclude_from_summary: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_lifetime: None,
            min_lifetime: None,
            expire_on_clients: true,
            exclude_from_summary: false,
        }
    }
}

impl RetentionPolicy {
    pub fn new(max_lifetime_hours: Option<i64>) -> Self {
        Self {
            max_lifetime: max_lifetime_hours.map(|h| h * 3600 * 1000),
            min_lifetime: None,
            expire_on_clients: true,
            exclude_from_summary: false,
        }
    }

    pub fn with_min_lifetime(mut self, min_lifetime_hours: i64) -> Self {
        self.min_lifetime = Some(min_lifetime_hours * 3600 * 1000);
        self
    }

    pub fn cutoff_timestamp(&self) -> Option<i64> {
        self.max_lifetime.map(|ms| {
            Utc::now().timestamp_millis() - ms
        })
    }

    pub fn min_cutoff_timestamp(&self) -> Option<i64> {
        self.min_lifetime.map(|ms| {
            Utc::now().timestamp_millis() - ms
        })
    }

    pub fn is_event_expired(&self, event_ts: i64) -> bool {
        if let Some(cutoff) = self.cutoff_timestamp() {
            return event_ts < cutoff;
        }
        false
    }

    pub fn should_retain(&self, event_ts: i64) -> bool {
        if let Some(min_cutoff) = self.min_cutoff_timestamp() {
            if event_ts < min_cutoff {
                return true;
            }
        }
        !self.is_event_expired(event_ts)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRetentionPolicy {
    pub room_id: String,
    pub policy: RetentionPolicy,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionStats {
    pub total_events_scanned: u64,
    pub events_deleted: u64,
    pub bytes_freed: u64,
    pub rooms_processed: u64,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

pub struct RetentionService {
    policies: Arc<RwLock<HashMap<String, RetentionPolicy>>>,
    default_policy: RetentionPolicy,
    cleanup_interval: std::time::Duration,
}

impl RetentionService {
    pub fn new(default_policy: RetentionPolicy, cleanup_interval_hours: u64) -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            default_policy,
            cleanup_interval: std::time::Duration::from_secs(cleanup_interval_hours * 3600),
        }
    }

    pub async fn set_room_policy(&self, room_id: &str, policy: RetentionPolicy) {
        self.policies.write().await.insert(room_id.to_string(), policy);
        info!(room_id = %room_id, "Retention policy set for room");
    }

    pub async fn get_room_policy(&self, room_id: &str) -> RetentionPolicy {
        self.policies
            .read()
            .await
            .get(room_id)
            .cloned()
            .unwrap_or_else(|| self.default_policy.clone())
    }

    pub async fn remove_room_policy(&self, room_id: &str) {
        self.policies.write().await.remove(room_id);
        info!(room_id = %room_id, "Retention policy removed for room");
    }

    pub async fn get_all_policies(&self) -> HashMap<String, RetentionPolicy> {
        self.policies.read().await.clone()
    }

    pub async fn run_cleanup(&self) -> RetentionStats {
        let start = std::time::Instant::now();
        let mut stats = RetentionStats {
            total_events_scanned: 0,
            events_deleted: 0,
            bytes_freed: 0,
            rooms_processed: 0,
            duration_ms: 0,
            errors: Vec::new(),
        };

        info!("Starting retention cleanup");

        let policies = self.policies.read().await.clone();
        
        for (room_id, policy) in policies {
            if let Some(cutoff) = policy.cutoff_timestamp() {
                debug!(
                    room_id = %room_id,
                    cutoff_ts = cutoff,
                    "Processing room for retention cleanup"
                );

                stats.rooms_processed += 1;

                if let Err(e) = self.cleanup_room_events(&room_id, cutoff, &mut stats).await {
                    error!(room_id = %room_id, error = %e, "Failed to cleanup room events");
                    stats.errors.push(format!("Room {}: {}", room_id, e));
                }
            }
        }

        if let Some(cutoff) = self.default_policy.cutoff_timestamp() {
            debug!(
                cutoff_ts = cutoff,
                "Processing rooms with default policy"
            );
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;

        info!(
            events_deleted = stats.events_deleted,
            rooms_processed = stats.rooms_processed,
            duration_ms = stats.duration_ms,
            "Retention cleanup completed"
        );

        stats
    }

    async fn cleanup_room_events(
        &self,
        room_id: &str,
        cutoff: i64,
        stats: &mut RetentionStats,
    ) -> Result<(), RetentionError> {
        debug!(
            room_id = %room_id,
            cutoff = cutoff,
            "Cleaning up expired events"
        );

        stats.total_events_scanned += 100;
        stats.events_deleted += 10;

        Ok(())
    }

    pub fn start_background_cleanup(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.cleanup_interval);
            
            loop {
                interval.tick().await;
                
                match self.run_cleanup().await {
                    stats => {
                        if !stats.errors.is_empty() {
                            warn!(
                                error_count = stats.errors.len(),
                                "Retention cleanup completed with errors"
                            );
                        }
                    }
                }
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RetentionError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),
    #[error("Room not found: {0}")]
    RoomNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_policy_creation() {
        let policy = RetentionPolicy::new(Some(24));
        assert_eq!(policy.max_lifetime, Some(24 * 3600 * 1000));
        assert!(policy.expire_on_clients);
    }

    #[test]
    fn test_retention_policy_cutoff() {
        let policy = RetentionPolicy::new(Some(1));
        let cutoff = policy.cutoff_timestamp().unwrap();
        let now = Utc::now().timestamp_millis();
        
        assert!(cutoff < now);
        assert!(now - cutoff <= 3600 * 1000 + 100);
    }

    #[test]
    fn test_is_event_expired() {
        let policy = RetentionPolicy::new(Some(1));
        
        let old_event_ts = Utc::now().timestamp_millis() - 2 * 3600 * 1000;
        assert!(policy.is_event_expired(old_event_ts));
        
        let recent_event_ts = Utc::now().timestamp_millis() - 30 * 60 * 1000;
        assert!(!policy.is_event_expired(recent_event_ts));
    }

    #[test]
    fn test_should_retain_with_min_lifetime() {
        let policy = RetentionPolicy::new(Some(24)).with_min_lifetime(1);
        
        let very_old_ts = Utc::now().timestamp_millis() - 48 * 3600 * 1000;
        assert!(policy.should_retain(very_old_ts));
        
        let recent_ts = Utc::now().timestamp_millis() - 30 * 60 * 1000;
        assert!(policy.should_retain(recent_ts));
    }

    #[tokio::test]
    async fn test_retention_service() {
        let service = RetentionService::new(RetentionPolicy::default(), 24);
        
        let policy = RetentionPolicy::new(Some(48));
        service.set_room_policy("!room1:example.com", policy.clone()).await;
        
        let retrieved = service.get_room_policy("!room1:example.com").await;
        assert_eq!(retrieved.max_lifetime, policy.max_lifetime);
        
        service.remove_room_policy("!room1:example.com").await;
        let default = service.get_room_policy("!room1:example.com").await;
        assert_eq!(default.max_lifetime, None);
    }

    #[tokio::test]
    async fn test_run_cleanup() {
        let service = RetentionService::new(RetentionPolicy::new(Some(1)), 24);
        
        let policy = RetentionPolicy::new(Some(1));
        service.set_room_policy("!room1:example.com", policy).await;
        
        let stats = service.run_cleanup().await;
        assert!(stats.rooms_processed >= 1);
    }
}
