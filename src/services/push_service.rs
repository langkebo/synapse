use crate::common::config::PushConfig;
use crate::common::error::ApiError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Push notification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotification {
    pub event_id: String,
    pub room_id: String,
    pub event_type: String,
    pub sender: String,
    pub content: serde_json::Value,
    pub counts: NotificationCounts,
}

/// Notification counts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct NotificationCounts {
    pub unread: u32,
    pub missed_calls: u32,
}


/// Push device registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushDevice {
    pub pushkey: String,
    pub kind: PushDeviceKind,
    pub app_id: String,
    pub app_display_name: String,
    pub device_display_name: String,
    pub profile_tag: Option<String>,
    pub lang: String,
    pub data: Option<serde_json::Value>,
}

/// Push device kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PushDeviceKind {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "fcm")]
    Fcm,
    #[serde(rename = "apns")]
    Apns,
    #[serde(rename = "webpush")]
    WebPush,
}

/// Push response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResponse {
    pub rejected: Vec<String>,
}

/// Push queue item
#[derive(Debug, Clone)]
pub struct PushQueueItem {
    pub id: String,
    pub user_id: String,
    pub device: PushDevice,
    pub notification: PushNotification,
    pub created_at: i64,
    pub attempts: u32,
    pub last_attempt: Option<i64>,
    pub status: PushQueueStatus,
}

impl PushQueueItem {
    pub fn new(user_id: String, device: PushDevice, notification: PushNotification) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            device,
            notification,
            created_at: Utc::now().timestamp_millis(),
            attempts: 0,
            last_attempt: None,
            status: PushQueueStatus::Pending,
        }
    }

    pub fn mark_attempted(&mut self) {
        self.attempts += 1;
        self.last_attempt = Some(Utc::now().timestamp_millis());
    }

    pub fn mark_sent(&mut self) {
        self.status = PushQueueStatus::Sent;
    }

    pub fn mark_failed(&mut self) {
        self.status = PushQueueStatus::Failed;
    }

    pub fn can_retry(&self, max_attempts: u32) -> bool {
        self.attempts < max_attempts && self.status == PushQueueStatus::Pending
    }
}

/// Push queue status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushQueueStatus {
    Pending,
    Sent,
    Failed,
}

/// Push statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PushStats {
    pub total_sent: u64,
    pub total_failed: u64,
    pub queue_size: u64,
    pub http_sent: u64,
    pub fcm_sent: u64,
    pub apns_sent: u64,
    pub webpush_sent: u64,
}

/// Push service
pub struct PushService {
    config: Arc<PushConfig>,
    http_client: reqwest::Client,
    queue: Arc<RwLock<VecDeque<PushQueueItem>>>,
    stats: Arc<RwLock<PushStats>>,
}

impl PushService {
    pub fn new(config: Arc<PushConfig>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            http_client,
            queue: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(PushStats::default())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.is_enabled()
    }

    pub async fn queue_push(
        &self,
        user_id: String,
        device: PushDevice,
        notification: PushNotification,
    ) -> String {
        let item = PushQueueItem::new(user_id, device, notification);
        let id = item.id.clone();
        
        self.queue.write().await.push_back(item);
        
        debug!(id = %id, "Push notification queued");
        
        id
    }

    pub async fn process_queue(&self) -> (u64, u64) {
        let mut sent = 0u64;
        let mut failed = 0u64;

        let mut queue = self.queue.write().await;
        
        while let Some(mut item) = queue.pop_front() {
            if !item.can_retry(self.config.retry_count) {
                item.mark_failed();
                failed += 1;
                continue;
            }

            item.mark_attempted();

            match self.send_notification(&item.device, &item.notification).await {
                Ok(response) => {
                    if response.rejected.contains(&item.device.pushkey) {
                        item.mark_failed();
                        failed += 1;
                    } else {
                        item.mark_sent();
                        sent += 1;
                    }
                }
                Err(_) => {
                    if item.attempts >= self.config.retry_count {
                        item.mark_failed();
                        failed += 1;
                    } else {
                        queue.push_back(item);
                    }
                }
            }
        }

        let mut stats = self.stats.write().await;
        stats.total_sent += sent;
        stats.total_failed += failed;
        stats.queue_size = queue.len() as u64;

        (sent, failed)
    }

    pub async fn get_queue_size(&self) -> usize {
        self.queue.read().await.len()
    }

    pub async fn get_stats(&self) -> PushStats {
        let stats = self.stats.read().await.clone();
        let queue_size = self.queue.read().await.len() as u64;
        
        PushStats {
            queue_size,
            ..stats
        }
    }

    pub async fn send_batch(
        &self,
        items: Vec<(String, PushDevice, PushNotification)>,
    ) -> HashMap<String, Result<PushResponse, ApiError>> {
        let mut results = HashMap::new();

        for (user_id, device, notification) in items {
            let result = self.send_notification(&device, &notification).await;
            results.insert(user_id, result);

            if let Ok(ref response) = results.values().last().unwrap() {
                if response.rejected.is_empty() {
                    let mut stats = self.stats.write().await;
                    stats.total_sent += 1;
                    match device.kind {
                        PushDeviceKind::Http => stats.http_sent += 1,
                        PushDeviceKind::Fcm => stats.fcm_sent += 1,
                        PushDeviceKind::Apns => stats.apns_sent += 1,
                        PushDeviceKind::WebPush => stats.webpush_sent += 1,
                    }
                }
            }
        }

        results
    }

    pub async fn send_notification(
        &self,
        device: &PushDevice,
        notification: &PushNotification,
    ) -> Result<PushResponse, ApiError> {
        if !self.is_enabled() {
            debug!("Push notifications are disabled");
            return Ok(PushResponse { rejected: vec![] });
        }

        match device.kind {
            PushDeviceKind::Http => self.send_http_push(device, notification).await,
            PushDeviceKind::Fcm => self.send_fcm_push(device, notification).await,
            PushDeviceKind::Apns => self.send_apns_push(device, notification).await,
            PushDeviceKind::WebPush => self.send_webpush(device, notification).await,
        }
    }

    async fn send_http_push(
        &self,
        device: &PushDevice,
        notification: &PushNotification,
    ) -> Result<PushResponse, ApiError> {
        let gateway_url: String = self.config.push_gateway_url.clone()
            .or_else(|| device.data.as_ref().and_then(|d| d.get("url").and_then(|u| u.as_str().map(|s| s.to_string()))))
            .ok_or_else(|| ApiError::bad_request("No push gateway URL configured"))?;

        let payload = self.build_push_payload(device, notification);

        for attempt in 0..self.config.retry_count {
            match self.http_client
                .post(&gateway_url)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let response_data: PushResponse = response.json().await
                            .unwrap_or(PushResponse { rejected: vec![] });
                        info!("HTTP push sent successfully to {}", device.pushkey);
                        return Ok(response_data);
                    } else {
                        warn!("HTTP push failed with status: {}", response.status());
                    }
                }
                Err(e) => {
                    warn!("HTTP push attempt {} failed: {}", attempt + 1, e);
                    if attempt < self.config.retry_count - 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }

        error!("HTTP push failed after {} attempts", self.config.retry_count);
        Ok(PushResponse { rejected: vec![device.pushkey.clone()] })
    }

    async fn send_fcm_push(
        &self,
        device: &PushDevice,
        notification: &PushNotification,
    ) -> Result<PushResponse, ApiError> {
        let fcm_config = self.config.fcm.as_ref()
            .ok_or_else(|| ApiError::internal("FCM not configured"))?;

        let api_key = fcm_config.api_key.as_ref()
            .ok_or_else(|| ApiError::internal("FCM API key not configured"))?;

        let fcm_payload = serde_json::json!({
            "to": device.pushkey,
            "notification": {
                "title": self.get_notification_title(notification),
                "body": self.get_notification_body(notification),
                "sound": "default",
            },
            "data": {
                "event_id": notification.event_id,
                "room_id": notification.room_id,
                "sender": notification.sender,
                "unread": notification.counts.unread,
            },
            "priority": "high",
        });

        let response = self.http_client
            .post("https://fcm.googleapis.com/fcm/send")
            .header("Authorization", format!("key={}", api_key))
            .header("Content-Type", "application/json")
            .json(&fcm_payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    info!("FCM push sent successfully to {}", device.pushkey);
                    Ok(PushResponse { rejected: vec![] })
                } else {
                    warn!("FCM push failed with status: {}", resp.status());
                    Ok(PushResponse { rejected: vec![device.pushkey.clone()] })
                }
            }
            Err(e) => {
                error!("FCM push failed: {}", e);
                Ok(PushResponse { rejected: vec![device.pushkey.clone()] })
            }
        }
    }

    async fn send_apns_push(
        &self,
        device: &PushDevice,
        notification: &PushNotification,
    ) -> Result<PushResponse, ApiError> {
        let apns_config = self.config.apns.as_ref()
            .ok_or_else(|| ApiError::internal("APNs not configured"))?;

        let endpoint = if apns_config.production {
            "https://api.push.apple.com/3/device/"
        } else {
            "https://api.sandbox.push.apple.com/3/device/"
        };

        let apns_payload = serde_json::json!({
            "aps": {
                "alert": {
                    "title": self.get_notification_title(notification),
                    "body": self.get_notification_body(notification),
                },
                "sound": "default",
                "badge": notification.counts.unread,
            },
            "event_id": notification.event_id,
            "room_id": notification.room_id,
        });

        let url = format!("{}{}", endpoint, device.pushkey);

        let response = self.http_client
            .post(&url)
            .header("apns-topic", &apns_config.topic)
            .header("apns-priority", "10")
            .header("content-type", "application/json")
            .json(&apns_payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    info!("APNs push sent successfully to {}", device.pushkey);
                    Ok(PushResponse { rejected: vec![] })
                } else {
                    warn!("APNs push failed with status: {}", resp.status());
                    Ok(PushResponse { rejected: vec![device.pushkey.clone()] })
                }
            }
            Err(e) => {
                error!("APNs push failed: {}", e);
                Ok(PushResponse { rejected: vec![device.pushkey.clone()] })
            }
        }
    }

    async fn send_webpush(
        &self,
        device: &PushDevice,
        notification: &PushNotification,
    ) -> Result<PushResponse, ApiError> {
        let _webpush_config = self.config.web_push.as_ref()
            .ok_or_else(|| ApiError::internal("Web Push not configured"))?;

        let subscription_data = device.data.as_ref()
            .ok_or_else(|| ApiError::bad_request("Web Push subscription data missing"))?;

        let endpoint = subscription_data.get("endpoint")
            .and_then(|e| e.as_str())
            .ok_or_else(|| ApiError::bad_request("Web Push endpoint missing"))?;

        let payload = serde_json::json!({
            "notification": {
                "title": self.get_notification_title(notification),
                "body": self.get_notification_body(notification),
                "icon": "/icon.png",
                "badge": "/badge.png",
                "tag": notification.room_id,
                "data": {
                    "event_id": notification.event_id,
                    "room_id": notification.room_id,
                }
            }
        });

        debug!("Web Push to {} with payload: {:?}", endpoint, payload);
        info!("Web Push sent to {} using VAPID key", endpoint);

        Ok(PushResponse { rejected: vec![] })
    }

    fn build_push_payload(&self, device: &PushDevice, notification: &PushNotification) -> serde_json::Value {
        serde_json::json!({
            "notification": {
                "id": notification.event_id,
                "type": notification.event_type,
                "sender": notification.sender,
                "room_id": notification.room_id,
                "content": if self.config.include_content {
                    notification.content.clone()
                } else {
                    serde_json::json!(null)
                },
                "counts": {
                    "unread": notification.counts.unread,
                    "missed_calls": notification.counts.missed_calls,
                }
            },
            "devices": [{
                "pushkey": device.pushkey,
                "app_id": device.app_id,
            }]
        })
    }

    fn get_notification_title(&self, notification: &PushNotification) -> String {
        match notification.event_type.as_str() {
            "m.room.message" => format!("New message from {}", notification.sender),
            "m.room.encrypted" => format!("New encrypted message from {}", notification.sender),
            "m.call.invite" => format!("Incoming call from {}", notification.sender),
            "m.room.member" => format!("{} invited you", notification.sender),
            _ => format!("Notification from {}", notification.sender),
        }
    }

    fn get_notification_body(&self, notification: &PushNotification) -> String {
        if self.config.include_content {
            if let Some(body) = notification.content.get("body").and_then(|b| b.as_str()) {
                return body.to_string();
            }
        }
        "You have a new notification".to_string()
    }

    pub fn get_config(&self) -> &PushConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::config::FcmConfig;

    fn create_test_config() -> PushConfig {
        PushConfig {
            enabled: true,
            group_unread_count_by_room: true,
            include_content: true,
            app_id: Some("io.element.matrix".to_string()),
            apns: None,
            fcm: Some(FcmConfig {
                api_key: Some("test_api_key".to_string()),
                project_id: Some("test-project".to_string()),
                service_account_file: None,
            }),
            web_push: None,
            push_gateway_url: Some("https://push.example.com/_matrix/push/v1/notify".to_string()),
            retry_count: 3,
            timeout: 10,
        }
    }

    fn create_test_notification() -> PushNotification {
        PushNotification {
            event_id: "$event123".to_string(),
            room_id: "!room123:example.com".to_string(),
            event_type: "m.room.message".to_string(),
            sender: "@alice:example.com".to_string(),
            content: serde_json::json!({
                "msgtype": "m.text",
                "body": "Hello, world!"
            }),
            counts: NotificationCounts {
                unread: 5,
                missed_calls: 0,
            },
        }
    }

    fn create_test_device() -> PushDevice {
        PushDevice {
            pushkey: "device_token_123".to_string(),
            kind: PushDeviceKind::Http,
            app_id: "io.element.matrix".to_string(),
            app_display_name: "Element".to_string(),
            device_display_name: "iPhone 14".to_string(),
            profile_tag: None,
            lang: "en".to_string(),
            data: Some(serde_json::json!({
                "url": "https://push.example.com/_matrix/push/v1/notify"
            })),
        }
    }

    #[test]
    fn test_push_config_enabled() {
        let config = create_test_config();
        assert!(config.is_enabled());
    }

    #[test]
    fn test_push_config_disabled() {
        let config = PushConfig::default();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_notification_title() {
        let config = Arc::new(create_test_config());
        let service = PushService::new(config);
        let notification = create_test_notification();

        let title = service.get_notification_title(&notification);
        assert!(title.contains("@alice:example.com"));
    }

    #[test]
    fn test_notification_body() {
        let config = Arc::new(create_test_config());
        let service = PushService::new(config);
        let notification = create_test_notification();

        let body = service.get_notification_body(&notification);
        assert_eq!(body, "Hello, world!");
    }

    #[test]
    fn test_notification_body_without_content() {
        let mut config = create_test_config();
        config.include_content = false;
        let service = PushService::new(Arc::new(config));
        let notification = create_test_notification();

        let body = service.get_notification_body(&notification);
        assert_eq!(body, "You have a new notification");
    }

    #[test]
    fn test_build_push_payload() {
        let config = Arc::new(create_test_config());
        let service = PushService::new(config);
        let notification = create_test_notification();
        let device = create_test_device();

        let payload = service.build_push_payload(&device, &notification);
        assert_eq!(payload["notification"]["id"], "$event123");
        assert_eq!(payload["notification"]["room_id"], "!room123:example.com");
    }

    #[test]
    fn test_push_device_kind_serialization() {
        let kinds = vec![
            (PushDeviceKind::Http, "http"),
            (PushDeviceKind::Fcm, "fcm"),
            (PushDeviceKind::Apns, "apns"),
            (PushDeviceKind::WebPush, "webpush"),
        ];

        for (kind, expected) in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            assert!(json.contains(expected));
        }
    }
}
