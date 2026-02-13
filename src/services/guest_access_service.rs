use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Guest access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestAccessConfig {
    pub enabled: bool,
    pub max_guest_accounts: u64,
    pub guest_access_ttl_seconds: u64,
    pub allow_join_rooms: bool,
    pub allow_send_messages: bool,
    pub allow_upload_media: bool,
    pub max_rooms_per_guest: u32,
    pub rate_limit_per_minute: u32,
}

impl Default for GuestAccessConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_guest_accounts: 1000,
            guest_access_ttl_seconds: 86400,
            allow_join_rooms: true,
            allow_send_messages: true,
            allow_upload_media: false,
            max_rooms_per_guest: 5,
            rate_limit_per_minute: 10,
        }
    }
}

/// Guest user account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestAccount {
    pub user_id: String,
    pub device_id: String,
    pub access_token: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub last_active_at: i64,
    pub joined_rooms: Vec<String>,
    pub is_active: bool,
}

impl GuestAccount {
    pub fn new(server_name: &str) -> Self {
        let now = Utc::now().timestamp_millis();
        let guest_id = format!("@guest_{}:{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap(), server_name);
        let device_id = format!("GUEST_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
        let access_token = generate_guest_token();
        
        Self {
            user_id: guest_id,
            device_id,
            access_token,
            created_at: now,
            expires_at: now + (86400 * 1000),
            last_active_at: now,
            joined_rooms: Vec::new(),
            is_active: true,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() > self.expires_at
    }

    pub fn touch(&mut self) {
        self.last_active_at = Utc::now().timestamp_millis();
    }

    pub fn join_room(&mut self, room_id: String) {
        if !self.joined_rooms.contains(&room_id) {
            self.joined_rooms.push(room_id);
        }
    }

    pub fn leave_room(&mut self, room_id: &str) {
        self.joined_rooms.retain(|r| r != room_id);
    }

    pub fn can_join_more_rooms(&self, max_rooms: u32) -> bool {
        self.joined_rooms.len() < max_rooms as usize
    }
}

fn generate_guest_token() -> String {
    format!("guest_{}", uuid::Uuid::new_v4())
}

/// Guest permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestPermissions {
    pub can_join_rooms: bool,
    pub can_send_messages: bool,
    pub can_upload_media: bool,
    pub can_invite: bool,
    pub can_redact: bool,
    pub max_message_length: usize,
    pub allowed_event_types: Vec<String>,
}

impl Default for GuestPermissions {
    fn default() -> Self {
        Self {
            can_join_rooms: true,
            can_send_messages: true,
            can_upload_media: false,
            can_invite: false,
            can_redact: false,
            max_message_length: 1000,
            allowed_event_types: vec![
                "m.room.message".to_string(),
                "m.reaction".to_string(),
            ],
        }
    }
}

impl GuestPermissions {
    pub fn from_config(config: &GuestAccessConfig) -> Self {
        Self {
            can_join_rooms: config.allow_join_rooms,
            can_send_messages: config.allow_send_messages,
            can_upload_media: config.allow_upload_media,
            can_invite: false,
            can_redact: false,
            max_message_length: 1000,
            allowed_event_types: vec![
                "m.room.message".to_string(),
                "m.reaction".to_string(),
            ],
        }
    }

    pub fn can_send_event(&self, event_type: &str) -> bool {
        self.can_send_messages && self.allowed_event_types.contains(&event_type.to_string())
    }
}

/// Guest rate limit tracker
#[derive(Debug, Clone, Default)]
pub struct GuestRateLimit {
    pub request_count: u32,
    pub window_start: i64,
}

impl GuestRateLimit {
    pub fn check_and_increment(&mut self, limit: u32, window_ms: i64) -> bool {
        let now = Utc::now().timestamp_millis();
        
        if now - self.window_start > window_ms {
            self.request_count = 0;
            self.window_start = now;
        }
        
        if self.request_count >= limit {
            return false;
        }
        
        self.request_count += 1;
        true
    }
}

/// Guest access service
pub struct GuestAccessService {
    config: GuestAccessConfig,
    guests: Arc<RwLock<HashMap<String, GuestAccount>>>,
    tokens: Arc<RwLock<HashMap<String, String>>>,
    rate_limits: Arc<RwLock<HashMap<String, GuestRateLimit>>>,
    permissions: GuestPermissions,
    server_name: String,
}

impl GuestAccessService {
    pub fn new(config: GuestAccessConfig, server_name: String) -> Self {
        let permissions = GuestPermissions::from_config(&config);
        
        Self {
            config,
            guests: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            permissions,
            server_name,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn create_guest(&self) -> Result<GuestAccount, GuestAccessError> {
        if !self.config.enabled {
            return Err(GuestAccessError::Disabled);
        }

        let guests = self.guests.read().await;
        if guests.len() >= self.config.max_guest_accounts as usize {
            return Err(GuestAccessError::MaxGuestsReached);
        }
        drop(guests);

        let guest = GuestAccount::new(&self.server_name);
        let token = guest.access_token.clone();
        let user_id = guest.user_id.clone();

        self.guests.write().await.insert(user_id.clone(), guest.clone());
        self.tokens.write().await.insert(token, user_id);

        info!(
            user_id = %guest.user_id,
            device_id = %guest.device_id,
            "Guest account created"
        );

        Ok(guest)
    }

    pub async fn get_guest(&self, user_id: &str) -> Option<GuestAccount> {
        self.guests.read().await.get(user_id).cloned()
    }

    pub async fn get_guest_by_token(&self, token: &str) -> Option<GuestAccount> {
        let tokens = self.tokens.read().await;
        let user_id = tokens.get(token)?;
        self.guests.read().await.get(user_id).cloned()
    }

    pub async fn validate_guest(&self, user_id: &str) -> Result<GuestAccount, GuestAccessError> {
        let mut guests = self.guests.write().await;
        
        if let Some(guest) = guests.get_mut(user_id) {
            if guest.is_expired() {
                guest.is_active = false;
                return Err(GuestAccessError::GuestExpired);
            }
            
            guest.touch();
            return Ok(guest.clone());
        }
        
        Err(GuestAccessError::GuestNotFound)
    }

    pub async fn join_room(&self, user_id: &str, room_id: &str) -> Result<(), GuestAccessError> {
        let mut guests = self.guests.write().await;
        
        if let Some(guest) = guests.get_mut(user_id) {
            if !guest.can_join_more_rooms(self.config.max_rooms_per_guest) {
                return Err(GuestAccessError::MaxRoomsReached);
            }
            
            guest.join_room(room_id.to_string());
            
            debug!(
                user_id = %user_id,
                room_id = %room_id,
                rooms_count = guest.joined_rooms.len(),
                "Guest joined room"
            );
            
            return Ok(());
        }
        
        Err(GuestAccessError::GuestNotFound)
    }

    pub async fn leave_room(&self, user_id: &str, room_id: &str) -> Result<(), GuestAccessError> {
        let mut guests = self.guests.write().await;
        
        if let Some(guest) = guests.get_mut(user_id) {
            guest.leave_room(room_id);
            
            debug!(
                user_id = %user_id,
                room_id = %room_id,
                "Guest left room"
            );
            
            return Ok(());
        }
        
        Err(GuestAccessError::GuestNotFound)
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> bool {
        let mut limits = self.rate_limits.write().await;
        let limit = limits.entry(user_id.to_string()).or_default();
        
        limit.check_and_increment(
            self.config.rate_limit_per_minute,
            60000,
        )
    }

    pub fn get_permissions(&self) -> &GuestPermissions {
        &self.permissions
    }

    pub async fn can_perform_action(&self, user_id: &str, action: GuestAction) -> Result<bool, GuestAccessError> {
        let guest = self.validate_guest(user_id).await?;
        
        if !self.check_rate_limit(user_id).await {
            return Err(GuestAccessError::RateLimited);
        }

        let allowed = match action {
            GuestAction::JoinRoom => self.permissions.can_join_rooms && guest.can_join_more_rooms(self.config.max_rooms_per_guest),
            GuestAction::SendMessage(event_type) => self.permissions.can_send_event(&event_type),
            GuestAction::UploadMedia => self.permissions.can_upload_media,
            GuestAction::Invite => self.permissions.can_invite,
            GuestAction::Redact => self.permissions.can_redact,
        };

        Ok(allowed)
    }

    pub async fn deactivate_guest(&self, user_id: &str) -> Result<(), GuestAccessError> {
        let mut guests = self.guests.write().await;
        
        if let Some(guest) = guests.get_mut(user_id) {
            guest.is_active = false;
            
            info!(user_id = %user_id, "Guest account deactivated");
            
            return Ok(());
        }
        
        Err(GuestAccessError::GuestNotFound)
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut guests = self.guests.write().await;
        let mut tokens = self.tokens.write().await;
        
        let before = guests.len();
        
        guests.retain(|_, g| !g.is_expired() && g.is_active);
        
        tokens.retain(|_, user_id| guests.contains_key(user_id));
        
        let removed = before - guests.len();
        
        if removed > 0 {
            info!(count = removed, "Expired guest accounts cleaned up");
        }
        
        removed
    }

    pub async fn get_guest_count(&self) -> usize {
        self.guests.read().await.len()
    }

    pub async fn get_active_guest_count(&self) -> usize {
        self.guests
            .read()
            .await
            .values()
            .filter(|g| g.is_active && !g.is_expired())
            .count()
    }

    pub fn get_config(&self) -> &GuestAccessConfig {
        &self.config
    }
}

/// Guest action types for permission checking
#[derive(Debug, Clone)]
pub enum GuestAction {
    JoinRoom,
    SendMessage(String),
    UploadMedia,
    Invite,
    Redact,
}

#[derive(Debug, thiserror::Error)]
pub enum GuestAccessError {
    #[error("Guest access is disabled")]
    Disabled,
    #[error("Maximum number of guest accounts reached")]
    MaxGuestsReached,
    #[error("Guest account not found")]
    GuestNotFound,
    #[error("Guest account has expired")]
    GuestExpired,
    #[error("Maximum rooms per guest reached")]
    MaxRoomsReached,
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Permission denied")]
    PermissionDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_account_creation() {
        let guest = GuestAccount::new("example.com");
        
        assert!(guest.user_id.starts_with("@guest_"));
        assert!(guest.user_id.ends_with(":example.com"));
        assert!(guest.device_id.starts_with("GUEST_"));
        assert!(guest.access_token.starts_with("guest_"));
        assert!(guest.is_active);
        assert!(!guest.is_expired());
    }

    #[test]
    fn test_guest_account_rooms() {
        let mut guest = GuestAccount::new("example.com");
        
        assert!(guest.can_join_more_rooms(5));
        
        guest.join_room("!room1:example.com".to_string());
        assert_eq!(guest.joined_rooms.len(), 1);
        
        guest.join_room("!room1:example.com".to_string());
        assert_eq!(guest.joined_rooms.len(), 1);
        
        guest.join_room("!room2:example.com".to_string());
        assert_eq!(guest.joined_rooms.len(), 2);
        
        guest.leave_room("!room1:example.com");
        assert_eq!(guest.joined_rooms.len(), 1);
    }

    #[test]
    fn test_guest_permissions() {
        let perms = GuestPermissions::default();
        
        assert!(perms.can_join_rooms);
        assert!(perms.can_send_messages);
        assert!(!perms.can_upload_media);
        assert!(!perms.can_invite);
        
        assert!(perms.can_send_event("m.room.message"));
        assert!(!perms.can_send_event("m.room.member"));
    }

    #[test]
    fn test_guest_rate_limit() {
        let mut limit = GuestRateLimit::default();
        
        for _ in 0..10 {
            assert!(limit.check_and_increment(10, 60000));
        }
        
        assert!(!limit.check_and_increment(10, 60000));
    }

    #[tokio::test]
    async fn test_create_guest() {
        let config = GuestAccessConfig::default();
        let service = GuestAccessService::new(config, "example.com".to_string());
        
        let guest = service.create_guest().await.unwrap();
        
        assert!(guest.user_id.starts_with("@guest_"));
        assert_eq!(service.get_guest_count().await, 1);
    }

    #[tokio::test]
    async fn test_guest_disabled() {
        let config = GuestAccessConfig {
            enabled: false,
            ..Default::default()
        };
        let service = GuestAccessService::new(config, "example.com".to_string());
        
        let result = service.create_guest().await;
        assert!(matches!(result, Err(GuestAccessError::Disabled)));
    }

    #[tokio::test]
    async fn test_max_guests() {
        let config = GuestAccessConfig {
            max_guest_accounts: 2,
            ..Default::default()
        };
        let service = GuestAccessService::new(config, "example.com".to_string());
        
        service.create_guest().await.unwrap();
        service.create_guest().await.unwrap();
        
        let result = service.create_guest().await;
        assert!(matches!(result, Err(GuestAccessError::MaxGuestsReached)));
    }

    #[tokio::test]
    async fn test_guest_join_room() {
        let service = GuestAccessService::new(GuestAccessConfig::default(), "example.com".to_string());
        
        let guest = service.create_guest().await.unwrap();
        
        service.join_room(&guest.user_id, "!room1:example.com").await.unwrap();
        
        let updated = service.get_guest(&guest.user_id).await.unwrap();
        assert_eq!(updated.joined_rooms.len(), 1);
    }

    #[tokio::test]
    async fn test_max_rooms_per_guest() {
        let config = GuestAccessConfig {
            max_rooms_per_guest: 2,
            ..Default::default()
        };
        let service = GuestAccessService::new(config, "example.com".to_string());
        
        let guest = service.create_guest().await.unwrap();
        
        service.join_room(&guest.user_id, "!room1:example.com").await.unwrap();
        service.join_room(&guest.user_id, "!room2:example.com").await.unwrap();
        
        let result = service.join_room(&guest.user_id, "!room3:example.com").await;
        assert!(matches!(result, Err(GuestAccessError::MaxRoomsReached)));
    }

    #[tokio::test]
    async fn test_can_perform_action() {
        let service = GuestAccessService::new(GuestAccessConfig::default(), "example.com".to_string());
        
        let guest = service.create_guest().await.unwrap();
        
        let can_join = service.can_perform_action(&guest.user_id, GuestAction::JoinRoom).await.unwrap();
        assert!(can_join);
        
        let can_send = service.can_perform_action(&guest.user_id, GuestAction::SendMessage("m.room.message".to_string())).await.unwrap();
        assert!(can_send);
        
        let can_upload = service.can_perform_action(&guest.user_id, GuestAction::UploadMedia).await.unwrap();
        assert!(!can_upload);
    }
}
