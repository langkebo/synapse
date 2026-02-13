use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnockState {
    Pending,
    Approved,
    Rejected,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnockRequest {
    pub knock_id: String,
    pub room_id: String,
    pub user_id: String,
    pub reason: Option<String>,
    pub state: KnockState,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: Option<i64>,
    pub reviewed_by: Option<String>,
    pub rejection_reason: Option<String>,
}

impl KnockRequest {
    pub fn new(room_id: String, user_id: String, reason: Option<String>) -> Self {
        let now = Utc::now().timestamp_millis();
        let expires_at = now + (7 * 24 * 3600 * 1000);

        Self {
            knock_id: uuid::Uuid::new_v4().to_string(),
            room_id,
            user_id,
            reason,
            state: KnockState::Pending,
            created_at: now,
            updated_at: now,
            expires_at: Some(expires_at),
            reviewed_by: None,
            rejection_reason: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            return Utc::now().timestamp_millis() > expires;
        }
        false
    }

    pub fn is_pending(&self) -> bool {
        self.state == KnockState::Pending && !self.is_expired()
    }

    pub fn approve(&mut self, reviewed_by: String) {
        self.state = KnockState::Approved;
        self.reviewed_by = Some(reviewed_by);
        self.updated_at = Utc::now().timestamp_millis();
    }

    pub fn reject(&mut self, reviewed_by: String, reason: Option<String>) {
        self.state = KnockState::Rejected;
        self.reviewed_by = Some(reviewed_by);
        self.rejection_reason = reason;
        self.updated_at = Utc::now().timestamp_millis();
    }

    pub fn cancel(&mut self) {
        self.state = KnockState::Cancelled;
        self.updated_at = Utc::now().timestamp_millis();
    }

    pub fn expire(&mut self) {
        self.state = KnockState::Expired;
        self.updated_at = Utc::now().timestamp_millis();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnockConfig {
    pub enabled: bool,
    pub max_pending_per_room: usize,
    pub max_pending_per_user: usize,
    pub default_expiry_hours: u64,
    pub require_reason: bool,
}

impl Default for KnockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_pending_per_room: 100,
            max_pending_per_user: 10,
            default_expiry_hours: 168,
            require_reason: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnockStats {
    pub total_knocks: u64,
    pub pending_knocks: u64,
    pub approved_knocks: u64,
    pub rejected_knocks: u64,
    pub expired_knocks: u64,
}

pub struct KnockService {
    knocks: Arc<RwLock<HashMap<String, KnockRequest>>>,
    config: KnockConfig,
}

impl KnockService {
    pub fn new(config: KnockConfig) -> Self {
        Self {
            knocks: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn knock(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<KnockRequest, KnockError> {
        if !self.config.enabled {
            return Err(KnockError::KnockDisabled);
        }

        if self.config.require_reason && reason.is_none() {
            return Err(KnockError::ReasonRequired);
        }

        let knocks = self.knocks.read().await;
        let pending_for_room = knocks
            .values()
            .filter(|k| k.room_id == room_id && k.is_pending())
            .count();

        if pending_for_room >= self.config.max_pending_per_room {
            return Err(KnockError::RoomKnockLimitExceeded);
        }

        let pending_for_user = knocks
            .values()
            .filter(|k| k.user_id == user_id && k.is_pending())
            .count();

        if pending_for_user >= self.config.max_pending_per_user {
            return Err(KnockError::UserKnockLimitExceeded);
        }

        let existing = knocks.values().find(|k| {
            k.room_id == room_id && k.user_id == user_id && k.is_pending()
        });

        if existing.is_some() {
            return Err(KnockError::KnockAlreadyExists);
        }
        drop(knocks);

        let knock = KnockRequest::new(room_id.to_string(), user_id.to_string(), reason);

        self.knocks.write().await.insert(knock.knock_id.clone(), knock.clone());

        info!(
            knock_id = %knock.knock_id,
            room_id = %room_id,
            user_id = %user_id,
            "New knock request created"
        );

        Ok(knock)
    }

    pub async fn approve(&self, knock_id: &str, reviewer: &str) -> Result<KnockRequest, KnockError> {
        let mut knocks = self.knocks.write().await;
        
        let knock = knocks
            .get_mut(knock_id)
            .ok_or(KnockError::KnockNotFound)?;

        if !knock.is_pending() {
            return Err(KnockError::InvalidKnockState);
        }

        knock.approve(reviewer.to_string());

        info!(
            knock_id = %knock_id,
            room_id = %knock.room_id,
            user_id = %knock.user_id,
            reviewer = %reviewer,
            "Knock request approved"
        );

        Ok(knock.clone())
    }

    pub async fn reject(
        &self,
        knock_id: &str,
        reviewer: &str,
        reason: Option<String>,
    ) -> Result<KnockRequest, KnockError> {
        let mut knocks = self.knocks.write().await;
        
        let knock = knocks
            .get_mut(knock_id)
            .ok_or(KnockError::KnockNotFound)?;

        if !knock.is_pending() {
            return Err(KnockError::InvalidKnockState);
        }

        knock.reject(reviewer.to_string(), reason);

        info!(
            knock_id = %knock_id,
            room_id = %knock.room_id,
            user_id = %knock.user_id,
            reviewer = %reviewer,
            "Knock request rejected"
        );

        Ok(knock.clone())
    }

    pub async fn cancel(&self, knock_id: &str, user_id: &str) -> Result<(), KnockError> {
        let mut knocks = self.knocks.write().await;
        
        let knock = knocks
            .get_mut(knock_id)
            .ok_or(KnockError::KnockNotFound)?;

        if knock.user_id != user_id {
            return Err(KnockError::NotAuthorized);
        }

        if !knock.is_pending() {
            return Err(KnockError::InvalidKnockState);
        }

        knock.cancel();

        info!(
            knock_id = %knock_id,
            user_id = %user_id,
            "Knock request cancelled"
        );

        Ok(())
    }

    pub async fn get_knock(&self, knock_id: &str) -> Option<KnockRequest> {
        self.knocks.read().await.get(knock_id).cloned()
    }

    pub async fn get_pending_for_room(&self, room_id: &str) -> Vec<KnockRequest> {
        self.knocks
            .read()
            .await
            .values()
            .filter(|k| k.room_id == room_id && k.is_pending())
            .cloned()
            .collect()
    }

    pub async fn get_pending_for_user(&self, user_id: &str) -> Vec<KnockRequest> {
        self.knocks
            .read()
            .await
            .values()
            .filter(|k| k.user_id == user_id && k.is_pending())
            .cloned()
            .collect()
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut knocks = self.knocks.write().await;

        let expired: Vec<String> = knocks
            .iter_mut()
            .filter(|(_, k)| k.state == KnockState::Pending && k.is_expired())
            .map(|(id, k)| {
                k.expire();
                id.clone()
            })
            .collect();

        let count = expired.len();
        if count > 0 {
            debug!(count = count, "Expired knock requests cleaned up");
        }

        count
    }

    pub async fn get_stats(&self) -> KnockStats {
        let knocks = self.knocks.read().await;

        let mut stats = KnockStats {
            total_knocks: knocks.len() as u64,
            pending_knocks: 0,
            approved_knocks: 0,
            rejected_knocks: 0,
            expired_knocks: 0,
        };

        for knock in knocks.values() {
            match knock.state {
                KnockState::Pending => stats.pending_knocks += 1,
                KnockState::Approved => stats.approved_knocks += 1,
                KnockState::Rejected => stats.rejected_knocks += 1,
                KnockState::Expired => stats.expired_knocks += 1,
                KnockState::Cancelled => {}
            }
        }

        stats
    }
}

impl Default for KnockService {
    fn default() -> Self {
        Self::new(KnockConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KnockError {
    #[error("Knock feature is disabled")]
    KnockDisabled,
    #[error("Knock request not found")]
    KnockNotFound,
    #[error("Knock already exists")]
    KnockAlreadyExists,
    #[error("Invalid knock state")]
    InvalidKnockState,
    #[error("Room knock limit exceeded")]
    RoomKnockLimitExceeded,
    #[error("User knock limit exceeded")]
    UserKnockLimitExceeded,
    #[error("Reason is required")]
    ReasonRequired,
    #[error("Not authorized")]
    NotAuthorized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_knock() {
        let service = KnockService::default();

        let knock = service
            .knock("!room:example.com", "@user:example.com", Some("I want to join".to_string()))
            .await
            .unwrap();

        assert_eq!(knock.room_id, "!room:example.com");
        assert_eq!(knock.user_id, "@user:example.com");
        assert_eq!(knock.state, KnockState::Pending);
    }

    #[tokio::test]
    async fn test_approve_knock() {
        let service = KnockService::default();

        let knock = service
            .knock("!room:example.com", "@user:example.com", None)
            .await
            .unwrap();

        let approved = service
            .approve(&knock.knock_id, "@admin:example.com")
            .await
            .unwrap();

        assert_eq!(approved.state, KnockState::Approved);
        assert_eq!(approved.reviewed_by, Some("@admin:example.com".to_string()));
    }

    #[tokio::test]
    async fn test_reject_knock() {
        let service = KnockService::default();

        let knock = service
            .knock("!room:example.com", "@user:example.com", None)
            .await
            .unwrap();

        let rejected = service
            .reject(&knock.knock_id, "@admin:example.com", Some("Not allowed".to_string()))
            .await
            .unwrap();

        assert_eq!(rejected.state, KnockState::Rejected);
        assert_eq!(rejected.rejection_reason, Some("Not allowed".to_string()));
    }

    #[tokio::test]
    async fn test_cancel_knock() {
        let service = KnockService::default();

        let knock = service
            .knock("!room:example.com", "@user:example.com", None)
            .await
            .unwrap();

        service
            .cancel(&knock.knock_id, "@user:example.com")
            .await
            .unwrap();

        let cancelled = service.get_knock(&knock.knock_id).await.unwrap();
        assert_eq!(cancelled.state, KnockState::Cancelled);
    }

    #[tokio::test]
    async fn test_duplicate_knock() {
        let service = KnockService::default();

        service
            .knock("!room:example.com", "@user:example.com", None)
            .await
            .unwrap();

        let result = service
            .knock("!room:example.com", "@user:example.com", None)
            .await;

        assert!(matches!(result, Err(KnockError::KnockAlreadyExists)));
    }

    #[tokio::test]
    async fn test_get_pending_for_room() {
        let service = KnockService::default();

        service
            .knock("!room1:example.com", "@user1:example.com", None)
            .await
            .unwrap();

        service
            .knock("!room1:example.com", "@user2:example.com", None)
            .await
            .unwrap();

        service
            .knock("!room2:example.com", "@user3:example.com", None)
            .await
            .unwrap();

        let pending = service.get_pending_for_room("!room1:example.com").await;
        assert_eq!(pending.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let service = KnockService::default();

        let knock1 = service
            .knock("!room:example.com", "@user1:example.com", None)
            .await
            .unwrap();

        let knock2 = service
            .knock("!room:example.com", "@user2:example.com", None)
            .await
            .unwrap();

        service.approve(&knock1.knock_id, "@admin:example.com").await.unwrap();
        service.reject(&knock2.knock_id, "@admin:example.com", None).await.unwrap();

        let stats = service.get_stats().await;
        assert_eq!(stats.total_knocks, 2);
        assert_eq!(stats.approved_knocks, 1);
        assert_eq!(stats.rejected_knocks, 1);
    }
}
