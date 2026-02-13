use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: i64,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub action: AuditAction,
    pub resource: String,
    pub resource_id: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub status: AuditStatus,
    pub details: HashMap<String, serde_json::Value>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    UserLogin,
    UserLogout,
    UserRegister,
    UserDelete,
    PasswordChange,
    PasswordReset,
    TokenRefresh,
    TokenRevoke,
    RoomCreate,
    RoomJoin,
    RoomLeave,
    RoomDelete,
    MessageSend,
    MessageDelete,
    MediaUpload,
    MediaDownload,
    MediaDelete,
    FederationSend,
    FederationReceive,
    AdminAction,
    ConfigChange,
    SecurityEvent,
    RateLimitExceeded,
    AuthenticationFailed,
    PermissionDenied,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditStatus {
    Success,
    Failure,
    Pending,
}

impl AuditAction {
    pub fn as_str(&self) -> &str {
        match self {
            AuditAction::UserLogin => "user.login",
            AuditAction::UserLogout => "user.logout",
            AuditAction::UserRegister => "user.register",
            AuditAction::UserDelete => "user.delete",
            AuditAction::PasswordChange => "password.change",
            AuditAction::PasswordReset => "password.reset",
            AuditAction::TokenRefresh => "token.refresh",
            AuditAction::TokenRevoke => "token.revoke",
            AuditAction::RoomCreate => "room.create",
            AuditAction::RoomJoin => "room.join",
            AuditAction::RoomLeave => "room.leave",
            AuditAction::RoomDelete => "room.delete",
            AuditAction::MessageSend => "message.send",
            AuditAction::MessageDelete => "message.delete",
            AuditAction::MediaUpload => "media.upload",
            AuditAction::MediaDownload => "media.download",
            AuditAction::MediaDelete => "media.delete",
            AuditAction::FederationSend => "federation.send",
            AuditAction::FederationReceive => "federation.receive",
            AuditAction::AdminAction => "admin.action",
            AuditAction::ConfigChange => "config.change",
            AuditAction::SecurityEvent => "security.event",
            AuditAction::RateLimitExceeded => "rate_limit.exceeded",
            AuditAction::AuthenticationFailed => "auth.failed",
            AuditAction::PermissionDenied => "permission.denied",
            AuditAction::Custom(s) => s,
        }
    }

    pub fn is_security_sensitive(&self) -> bool {
        matches!(
            self,
            AuditAction::UserLogin
                | AuditAction::UserDelete
                | AuditAction::PasswordChange
                | AuditAction::PasswordReset
                | AuditAction::TokenRevoke
                | AuditAction::AdminAction
                | AuditAction::ConfigChange
                | AuditAction::SecurityEvent
                | AuditAction::AuthenticationFailed
                | AuditAction::PermissionDenied
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    pub user_id: Option<String>,
    pub action: Option<AuditAction>,
    pub resource: Option<String>,
    pub status: Option<AuditStatus>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub ip_address: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            user_id: None,
            action: None,
            resource: None,
            status: None,
            start_time: None,
            end_time: None,
            ip_address: None,
            limit: 100,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: u64,
    pub events_by_action: HashMap<String, u64>,
    pub events_by_status: HashMap<String, u64>,
    pub failed_auth_attempts: u64,
    pub rate_limit_hits: u64,
    pub unique_users: u64,
    pub unique_ips: u64,
}

pub struct AuditLogger {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    max_events: usize,
    retention_days: u64,
}

impl AuditLogger {
    pub fn new(max_events: usize, retention_days: u64) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events,
            retention_days,
        }
    }

    pub async fn log(&self, event: AuditEvent) {
        if event.action.is_security_sensitive() {
            warn!(
                user_id = ?event.user_id,
                action = %event.action.as_str(),
                ip = %event.ip_address,
                status = ?event.status,
                "Security sensitive audit event"
            );
        } else {
            debug!(
                user_id = ?event.user_id,
                action = %event.action.as_str(),
                ip = %event.ip_address,
                "Audit event logged"
            );
        }

        let mut events = self.events.write().await;
        events.push(event);

        if events.len() > self.max_events {
            let excess = events.len() - self.max_events;
            events.drain(0..excess);
        }
    }

    pub async fn log_event(
        &self,
        user_id: Option<String>,
        device_id: Option<String>,
        action: AuditAction,
        resource: String,
        resource_id: Option<String>,
        ip_address: String,
        user_agent: Option<String>,
        status: AuditStatus,
        details: HashMap<String, serde_json::Value>,
        duration_ms: Option<u64>,
    ) {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().timestamp_millis(),
            user_id,
            device_id,
            action,
            resource,
            resource_id,
            ip_address,
            user_agent,
            status,
            details,
            duration_ms,
        };

        self.log(event).await;
    }

    pub async fn query(&self, query: AuditQuery) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        events
            .iter()
            .filter(|e| {
                if let Some(ref user_id) = query.user_id {
                    if e.user_id.as_ref() != Some(user_id) {
                        return false;
                    }
                }
                if let Some(ref action) = query.action {
                    if e.action != *action {
                        return false;
                    }
                }
                if let Some(ref resource) = query.resource {
                    if e.resource != *resource {
                        return false;
                    }
                }
                if let Some(ref status) = query.status {
                    if e.status != *status {
                        return false;
                    }
                }
                if let Some(start_time) = query.start_time {
                    if e.timestamp < start_time {
                        return false;
                    }
                }
                if let Some(end_time) = query.end_time {
                    if e.timestamp > end_time {
                        return false;
                    }
                }
                if let Some(ref ip) = query.ip_address {
                    if e.ip_address != *ip {
                        return false;
                    }
                }
                true
            })
            .skip(query.offset)
            .take(query.limit)
            .cloned()
            .collect()
    }

    pub async fn get_stats(&self) -> AuditStats {
        let events = self.events.read().await;

        let mut events_by_action: HashMap<String, u64> = HashMap::new();
        let mut events_by_status: HashMap<String, u64> = HashMap::new();
        let mut unique_users: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut unique_ips: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut failed_auth_attempts = 0u64;
        let mut rate_limit_hits = 0u64;

        for event in events.iter() {
            *events_by_action.entry(event.action.as_str().to_string()).or_insert(0) += 1;
            *events_by_status.entry(format!("{:?}", event.status)).or_insert(0) += 1;

            if let Some(ref user_id) = event.user_id {
                unique_users.insert(user_id.clone());
            }
            unique_ips.insert(event.ip_address.clone());

            if event.action == AuditAction::AuthenticationFailed {
                failed_auth_attempts += 1;
            }
            if event.action == AuditAction::RateLimitExceeded {
                rate_limit_hits += 1;
            }
        }

        AuditStats {
            total_events: events.len() as u64,
            events_by_action,
            events_by_status,
            failed_auth_attempts,
            rate_limit_hits,
            unique_users: unique_users.len() as u64,
            unique_ips: unique_ips.len() as u64,
        }
    }

    pub async fn cleanup_expired(&self) -> usize {
        let cutoff = Utc::now().timestamp_millis() - (self.retention_days * 24 * 3600 * 1000) as i64;
        let mut events = self.events.write().await;

        let initial_len = events.len();
        events.retain(|e| e.timestamp > cutoff);

        initial_len - events.len()
    }

    pub async fn get_user_activity(&self, user_id: &str) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect()
    }

    pub async fn get_suspicious_activity(&self) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| {
                e.action == AuditAction::AuthenticationFailed
                    || e.action == AuditAction::RateLimitExceeded
                    || e.action == AuditAction::PermissionDenied
                    || e.action == AuditAction::SecurityEvent
            })
            .cloned()
            .collect()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000, 90)
    }
}

pub struct AuditContext {
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub start_time: Instant,
}

impl AuditContext {
    pub fn new(ip_address: String) -> Self {
        Self {
            user_id: None,
            device_id: None,
            ip_address,
            user_agent: None,
            start_time: Instant::now(),
        }
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_device(mut self, device_id: String) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn duration_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logger_log_event() {
        let logger = AuditLogger::new(100, 90);

        logger.log_event(
            Some("@user:example.com".to_string()),
            None,
            AuditAction::UserLogin,
            "user".to_string(),
            Some("@user:example.com".to_string()),
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Success,
            HashMap::new(),
            Some(100),
        ).await;

        let events = logger.query(AuditQuery::default()).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, AuditAction::UserLogin);
    }

    #[tokio::test]
    async fn test_audit_query_filter() {
        let logger = AuditLogger::new(100, 90);

        logger.log_event(
            Some("@user1:example.com".to_string()),
            None,
            AuditAction::UserLogin,
            "user".to_string(),
            None,
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Success,
            HashMap::new(),
            None,
        ).await;

        logger.log_event(
            Some("@user2:example.com".to_string()),
            None,
            AuditAction::UserLogout,
            "user".to_string(),
            None,
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Success,
            HashMap::new(),
            None,
        ).await;

        let query = AuditQuery {
            user_id: Some("@user1:example.com".to_string()),
            ..Default::default()
        };

        let events = logger.query(query).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, Some("@user1:example.com".to_string()));
    }

    #[tokio::test]
    async fn test_audit_stats() {
        let logger = AuditLogger::new(100, 90);

        logger.log_event(
            Some("@user:example.com".to_string()),
            None,
            AuditAction::UserLogin,
            "user".to_string(),
            None,
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Success,
            HashMap::new(),
            None,
        ).await;

        logger.log_event(
            Some("@user:example.com".to_string()),
            None,
            AuditAction::AuthenticationFailed,
            "user".to_string(),
            None,
            "192.168.1.1".to_string(),
            None,
            AuditStatus::Failure,
            HashMap::new(),
            None,
        ).await;

        let stats = logger.get_stats().await;
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.failed_auth_attempts, 1);
        assert_eq!(stats.unique_users, 1);
        assert_eq!(stats.unique_ips, 2);
    }

    #[tokio::test]
    async fn test_security_sensitive_detection() {
        assert!(AuditAction::UserLogin.is_security_sensitive());
        assert!(AuditAction::PasswordChange.is_security_sensitive());
        assert!(AuditAction::AdminAction.is_security_sensitive());
        assert!(!AuditAction::MessageSend.is_security_sensitive());
        assert!(!AuditAction::MediaDownload.is_security_sensitive());
    }

    #[tokio::test]
    async fn test_suspicious_activity() {
        let logger = AuditLogger::new(100, 90);

        logger.log_event(
            None,
            None,
            AuditAction::AuthenticationFailed,
            "user".to_string(),
            None,
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Failure,
            HashMap::new(),
            None,
        ).await;

        logger.log_event(
            None,
            None,
            AuditAction::RateLimitExceeded,
            "api".to_string(),
            None,
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Failure,
            HashMap::new(),
            None,
        ).await;

        logger.log_event(
            Some("@user:example.com".to_string()),
            None,
            AuditAction::MessageSend,
            "room".to_string(),
            None,
            "127.0.0.1".to_string(),
            None,
            AuditStatus::Success,
            HashMap::new(),
            None,
        ).await;

        let suspicious = logger.get_suspicious_activity().await;
        assert_eq!(suspicious.len(), 2);
    }
}
