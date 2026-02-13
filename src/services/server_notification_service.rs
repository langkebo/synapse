use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationType {
    ServerNotice,
    Warning,
    Maintenance,
    Update,
    PolicyChange,
    Custom(String),
}

impl NotificationType {
    pub fn as_str(&self) -> &str {
        match self {
            NotificationType::ServerNotice => "server_notice",
            NotificationType::Warning => "warning",
            NotificationType::Maintenance => "maintenance",
            NotificationType::Update => "update",
            NotificationType::PolicyChange => "policy_change",
            NotificationType::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerNotification {
    pub id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub priority: u8,
    pub target_users: Option<Vec<String>>,
    pub target_rooms: Option<Vec<String>>,
    pub is_global: bool,
    pub read_by: Vec<String>,
    pub dismissed_by: Vec<String>,
}

impl ServerNotification {
    pub fn new(
        notification_type: NotificationType,
        title: String,
        content: String,
        priority: u8,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type,
            title,
            content,
            created_at: Utc::now().timestamp_millis(),
            expires_at: None,
            priority,
            target_users: None,
            target_rooms: None,
            is_global: true,
            read_by: Vec::new(),
            dismissed_by: Vec::new(),
        }
    }

    pub fn with_expiry(mut self, hours: u64) -> Self {
        self.expires_at = Some(self.created_at + (hours * 3600 * 1000) as i64);
        self
    }

    pub fn for_users(mut self, users: Vec<String>) -> Self {
        self.target_users = Some(users);
        self.is_global = false;
        self
    }

    pub fn for_rooms(mut self, rooms: Vec<String>) -> Self {
        self.target_rooms = Some(rooms);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            return Utc::now().timestamp_millis() > expires;
        }
        false
    }

    pub fn is_targeted_at(&self, user_id: &str) -> bool {
        if self.is_global {
            return true;
        }

        if let Some(ref users) = self.target_users {
            if users.contains(&user_id.to_string()) {
                return true;
            }
        }

        false
    }

    pub fn mark_read(&mut self, user_id: &str) {
        if !self.read_by.contains(&user_id.to_string()) {
            self.read_by.push(user_id.to_string());
        }
    }

    pub fn mark_dismissed(&mut self, user_id: &str) {
        if !self.dismissed_by.contains(&user_id.to_string()) {
            self.dismissed_by.push(user_id.to_string());
        }
    }

    pub fn is_read_by(&self, user_id: &str) -> bool {
        self.read_by.contains(&user_id.to_string())
    }

    pub fn is_dismissed_by(&self, user_id: &str) -> bool {
        self.dismissed_by.contains(&user_id.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub max_active_notifications: usize,
    pub default_expiry_hours: u64,
    pub max_content_length: usize,
    pub allow_html: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            max_active_notifications: 100,
            default_expiry_hours: 168,
            max_content_length: 10000,
            allow_html: false,
        }
    }
}

pub struct ServerNotificationService {
    notifications: Arc<RwLock<HashMap<String, ServerNotification>>>,
    config: NotificationConfig,
}

impl ServerNotificationService {
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            notifications: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn create_notification(
        &self,
        notification_type: NotificationType,
        title: String,
        content: String,
        priority: u8,
    ) -> Result<ServerNotification, NotificationError> {
        if content.len() > self.config.max_content_length {
            return Err(NotificationError::ContentTooLong);
        }

        let notification = ServerNotification::new(notification_type, title, content, priority)
            .with_expiry(self.config.default_expiry_hours);

        self.notifications
            .write()
            .await
            .insert(notification.id.clone(), notification.clone());

        info!(
            notification_id = %notification.id,
            notification_type = %notification.notification_type.as_str(),
            title = %notification.title,
            "Server notification created"
        );

        Ok(notification)
    }

    pub async fn create_targeted_notification(
        &self,
        notification_type: NotificationType,
        title: String,
        content: String,
        priority: u8,
        target_users: Vec<String>,
    ) -> Result<ServerNotification, NotificationError> {
        let mut notification = ServerNotification::new(notification_type, title, content, priority)
            .with_expiry(self.config.default_expiry_hours);

        notification.target_users = Some(target_users);
        notification.is_global = false;

        self.notifications
            .write()
            .await
            .insert(notification.id.clone(), notification.clone());

        info!(
            notification_id = %notification.id,
            "Targeted notification created"
        );

        Ok(notification)
    }

    pub async fn get_notifications_for_user(
        &self,
        user_id: &str,
        include_dismissed: bool,
    ) -> Vec<ServerNotification> {
        self.notifications
            .read()
            .await
            .values()
            .filter(|n| {
                !n.is_expired()
                    && n.is_targeted_at(user_id)
                    && (include_dismissed || !n.is_dismissed_by(user_id))
            })
            .cloned()
            .collect()
    }

    pub async fn get_unread_count(&self, user_id: &str) -> usize {
        self.notifications
            .read()
            .await
            .values()
            .filter(|n| {
                !n.is_expired()
                    && n.is_targeted_at(user_id)
                    && !n.is_read_by(user_id)
                    && !n.is_dismissed_by(user_id)
            })
            .count()
    }

    pub async fn mark_as_read(&self, notification_id: &str, user_id: &str) -> Result<(), NotificationError> {
        let mut notifications = self.notifications.write().await;
        let notification = notifications
            .get_mut(notification_id)
            .ok_or(NotificationError::NotFound)?;

        notification.mark_read(user_id);

        debug!(
            notification_id = %notification_id,
            user_id = %user_id,
            "Notification marked as read"
        );

        Ok(())
    }

    pub async fn mark_as_dismissed(
        &self,
        notification_id: &str,
        user_id: &str,
    ) -> Result<(), NotificationError> {
        let mut notifications = self.notifications.write().await;
        let notification = notifications
            .get_mut(notification_id)
            .ok_or(NotificationError::NotFound)?;

        notification.mark_dismissed(user_id);

        debug!(
            notification_id = %notification_id,
            user_id = %user_id,
            "Notification dismissed"
        );

        Ok(())
    }

    pub async fn delete_notification(&self, notification_id: &str) -> Result<(), NotificationError> {
        if self
            .notifications
            .write()
            .await
            .remove(notification_id)
            .is_some()
        {
            info!(notification_id = %notification_id, "Notification deleted");
            Ok(())
        } else {
            Err(NotificationError::NotFound)
        }
    }

    pub async fn get_notification(&self, notification_id: &str) -> Option<ServerNotification> {
        self.notifications.read().await.get(notification_id).cloned()
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut notifications = self.notifications.write().await;
        let before = notifications.len();

        notifications.retain(|_, n| !n.is_expired());

        let removed = before - notifications.len();
        if removed > 0 {
            debug!(count = removed, "Expired notifications cleaned up");
        }

        removed
    }

    pub async fn get_active_count(&self) -> usize {
        self.notifications
            .read()
            .await
            .values()
            .filter(|n| !n.is_expired())
            .count()
    }

    pub async fn broadcast_to_all(&self, title: String, content: String) -> Result<ServerNotification, NotificationError> {
        self.create_notification(
            NotificationType::ServerNotice,
            title,
            content,
            5,
        )
        .await
    }
}

impl Default for ServerNotificationService {
    fn default() -> Self {
        Self::new(NotificationConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Notification not found")]
    NotFound,
    #[error("Content too long")]
    ContentTooLong,
    #[error("Too many active notifications")]
    TooManyActive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_notification() {
        let service = ServerNotificationService::default();

        let notification = service
            .create_notification(
                NotificationType::ServerNotice,
                "Test Title".to_string(),
                "Test content".to_string(),
                5,
            )
            .await
            .unwrap();

        assert_eq!(notification.title, "Test Title");
        assert!(notification.is_global);
    }

    #[tokio::test]
    async fn test_targeted_notification() {
        let service = ServerNotificationService::default();

        let notification = service
            .create_targeted_notification(
                NotificationType::Warning,
                "Warning".to_string(),
                "Warning content".to_string(),
                8,
                vec!["@user1:example.com".to_string()],
            )
            .await
            .unwrap();

        assert!(notification.is_targeted_at("@user1:example.com"));
        assert!(!notification.is_targeted_at("@user2:example.com"));
    }

    #[tokio::test]
    async fn test_get_notifications_for_user() {
        let service = ServerNotificationService::default();

        service
            .create_notification(
                NotificationType::ServerNotice,
                "Global Notice".to_string(),
                "Global content".to_string(),
                5,
            )
            .await
            .unwrap();

        service
            .create_targeted_notification(
                NotificationType::Warning,
                "User Warning".to_string(),
                "User content".to_string(),
                8,
                vec!["@user1:example.com".to_string()],
            )
            .await
            .unwrap();

        let notifications = service
            .get_notifications_for_user("@user1:example.com", false)
            .await;

        assert_eq!(notifications.len(), 2);

        let notifications = service
            .get_notifications_for_user("@user2:example.com", false)
            .await;

        assert_eq!(notifications.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_as_read() {
        let service = ServerNotificationService::default();

        let notification = service
            .create_notification(
                NotificationType::ServerNotice,
                "Test".to_string(),
                "Content".to_string(),
                5,
            )
            .await
            .unwrap();

        assert_eq!(service.get_unread_count("@user:example.com").await, 1);

        service
            .mark_as_read(&notification.id, "@user:example.com")
            .await
            .unwrap();

        assert_eq!(service.get_unread_count("@user:example.com").await, 0);
    }

    #[tokio::test]
    async fn test_dismiss_notification() {
        let service = ServerNotificationService::default();

        let notification = service
            .create_notification(
                NotificationType::ServerNotice,
                "Test".to_string(),
                "Content".to_string(),
                5,
            )
            .await
            .unwrap();

        service
            .mark_as_dismissed(&notification.id, "@user:example.com")
            .await
            .unwrap();

        let notifications = service
            .get_notifications_for_user("@user:example.com", false)
            .await;

        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_notification() {
        let service = ServerNotificationService::default();

        let notification = service
            .create_notification(
                NotificationType::ServerNotice,
                "Test".to_string(),
                "Content".to_string(),
                5,
            )
            .await
            .unwrap();

        service.delete_notification(&notification.id).await.unwrap();

        assert!(service.get_notification(&notification.id).await.is_none());
    }
}
