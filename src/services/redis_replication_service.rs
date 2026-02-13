use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};

/// Redis replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisReplicationConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub channel_prefix: String,
    pub connection_pool_size: usize,
    pub reconnect_delay_ms: u64,
    pub max_reconnect_attempts: u32,
}

impl Default for RedisReplicationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            channel_prefix: "synapse".to_string(),
            connection_pool_size: 10,
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 10,
        }
    }
}

/// Replication channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplicationChannel {
    Events,
    Presence,
    Typing,
    Receipts,
    DeviceLists,
    Federation,
    Push,
    AccountData,
}

impl ReplicationChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReplicationChannel::Events => "events",
            ReplicationChannel::Presence => "presence",
            ReplicationChannel::Typing => "typing",
            ReplicationChannel::Receipts => "receipts",
            ReplicationChannel::DeviceLists => "device_lists",
            ReplicationChannel::Federation => "federation",
            ReplicationChannel::Push => "push",
            ReplicationChannel::AccountData => "account_data",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "events" => Some(ReplicationChannel::Events),
            "presence" => Some(ReplicationChannel::Presence),
            "typing" => Some(ReplicationChannel::Typing),
            "receipts" => Some(ReplicationChannel::Receipts),
            "device_lists" => Some(ReplicationChannel::DeviceLists),
            "federation" => Some(ReplicationChannel::Federation),
            "push" => Some(ReplicationChannel::Push),
            "account_data" => Some(ReplicationChannel::AccountData),
            _ => None,
        }
    }
}

/// Replication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationMessage {
    pub channel: ReplicationChannel,
    pub instance_name: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
    pub sequence: u64,
}

impl ReplicationMessage {
    pub fn new(channel: ReplicationChannel, instance_name: String, data: serde_json::Value, sequence: u64) -> Self {
        Self {
            channel,
            instance_name,
            timestamp: Utc::now().timestamp_millis(),
            data,
            sequence,
        }
    }
}

/// Replication position tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationPosition {
    pub channel: ReplicationChannel,
    pub position: u64,
    pub instance_name: String,
    pub updated_at: i64,
}

impl ReplicationPosition {
    pub fn new(channel: ReplicationChannel, instance_name: String) -> Self {
        Self {
            channel,
            position: 0,
            instance_name,
            updated_at: Utc::now().timestamp_millis(),
        }
    }

    pub fn advance(&mut self) -> u64 {
        self.position += 1;
        self.updated_at = Utc::now().timestamp_millis();
        self.position
    }

    pub fn update(&mut self, position: u64) {
        self.position = position;
        self.updated_at = Utc::now().timestamp_millis();
    }
}

/// Redis replication statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicationStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub reconnect_count: u64,
    pub last_connected_at: Option<i64>,
    pub last_error: Option<String>,
    pub is_connected: bool,
}

/// Redis replication service
pub struct RedisReplicationService {
    config: RedisReplicationConfig,
    instance_name: String,
    positions: Arc<RwLock<HashMap<ReplicationChannel, ReplicationPosition>>>,
    stats: Arc<RwLock<ReplicationStats>>,
    sender: broadcast::Sender<ReplicationMessage>,
    _receiver: broadcast::Receiver<ReplicationMessage>,
}

impl RedisReplicationService {
    pub fn new(config: RedisReplicationConfig, instance_name: String) -> Self {
        let (sender, receiver) = broadcast::channel(1000);
        
        let mut positions = HashMap::new();
        for channel in [
            ReplicationChannel::Events,
            ReplicationChannel::Presence,
            ReplicationChannel::Typing,
            ReplicationChannel::Receipts,
            ReplicationChannel::DeviceLists,
            ReplicationChannel::Federation,
            ReplicationChannel::Push,
            ReplicationChannel::AccountData,
        ] {
            positions.insert(channel, ReplicationPosition::new(channel, instance_name.clone()));
        }
        
        Self {
            config,
            instance_name,
            positions: Arc::new(RwLock::new(positions)),
            stats: Arc::new(RwLock::new(ReplicationStats::default())),
            sender,
            _receiver: receiver,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn publish(&self, channel: ReplicationChannel, data: serde_json::Value) -> Result<(), ReplicationError> {
        if !self.config.enabled {
            return Ok(());
        }

        let sequence = {
            let mut positions = self.positions.write().await;
            let position = positions.get_mut(&channel).ok_or(ReplicationError::InvalidChannel)?;
            position.advance()
        };

        let message = ReplicationMessage::new(
            channel,
            self.instance_name.clone(),
            data,
            sequence,
        );

        if let Err(e) = self.sender.send(message.clone()) {
            warn!(
                channel = ?channel,
                error = %e,
                "Failed to broadcast replication message"
            );
            
            let mut stats = self.stats.write().await;
            stats.messages_dropped += 1;
            
            return Err(ReplicationError::SendError(e.to_string()));
        }

        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;

        debug!(
            channel = ?channel,
            sequence = sequence,
            "Replication message published"
        );

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ReplicationMessage> {
        self.sender.subscribe()
    }

    pub async fn process_incoming(&self, message: ReplicationMessage) -> Result<(), ReplicationError> {
        if message.instance_name == self.instance_name {
            return Ok(());
        }

        {
            let mut positions = self.positions.write().await;
            if let Some(position) = positions.get_mut(&message.channel) {
                if message.sequence <= position.position {
                    debug!(
                        channel = ?message.channel,
                        seq = message.sequence,
                        current = position.position,
                        "Skipping old replication message"
                    );
                    return Ok(());
                }
                position.update(message.sequence);
            }
        }

        if let Err(e) = self.sender.send(message.clone()) {
            warn!(
                channel = ?message.channel,
                error = %e,
                "Failed to forward replication message"
            );
        }

        let mut stats = self.stats.write().await;
        stats.messages_received += 1;

        Ok(())
    }

    pub async fn get_position(&self, channel: ReplicationChannel) -> u64 {
        self.positions
            .read()
            .await
            .get(&channel)
            .map(|p| p.position)
            .unwrap_or(0)
    }

    pub async fn get_all_positions(&self) -> HashMap<ReplicationChannel, u64> {
        self.positions
            .read()
            .await
            .iter()
            .map(|(k, v)| (*k, v.position))
            .collect()
    }

    pub async fn get_stats(&self) -> ReplicationStats {
        self.stats.read().await.clone()
    }

    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ReplicationStats::default();
    }

    pub fn get_channel_name(&self, channel: ReplicationChannel) -> String {
        format!("{}:{}", self.config.channel_prefix, channel.as_str())
    }

    pub async fn mark_connected(&self) {
        let mut stats = self.stats.write().await;
        stats.is_connected = true;
        stats.last_connected_at = Some(Utc::now().timestamp_millis());
        stats.last_error = None;
    }

    pub async fn mark_disconnected(&self, error: Option<String>) {
        let mut stats = self.stats.write().await;
        stats.is_connected = false;
        stats.last_error = error;
        stats.reconnect_count += 1;
    }

    pub fn get_config(&self) -> &RedisReplicationConfig {
        &self.config
    }
}

/// Redis connection manager (simplified for demonstration)
pub struct RedisConnectionManager {
    config: RedisReplicationConfig,
    connected: Arc<RwLock<bool>>,
}

impl RedisConnectionManager {
    pub fn new(config: RedisReplicationConfig) -> Self {
        Self {
            config,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn connect(&self) -> Result<(), ReplicationError> {
        debug!(
            host = %self.config.host,
            port = self.config.port,
            "Connecting to Redis"
        );
        
        *self.connected.write().await = true;
        
        info!("Redis connection established");
        Ok(())
    }

    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
        info!("Redis connection closed");
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn reconnect(&self) -> Result<(), ReplicationError> {
        self.disconnect().await;
        
        for attempt in 1..=self.config.max_reconnect_attempts {
            debug!(attempt = attempt, "Attempting to reconnect to Redis");
            
            if self.connect().await.is_ok() {
                return Ok(());
            }
            
            tokio::time::sleep(std::time::Duration::from_millis(self.config.reconnect_delay_ms)).await;
        }
        
        Err(ReplicationError::ConnectionFailed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("Redis connection failed")]
    ConnectionFailed,
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Invalid channel")]
    InvalidChannel,
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Max reconnection attempts reached")]
    MaxReconnectAttempts,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_replication_channel() {
        assert_eq!(ReplicationChannel::Events.as_str(), "events");
        assert_eq!(ReplicationChannel::from_str("events"), Some(ReplicationChannel::Events));
        assert_eq!(ReplicationChannel::from_str("invalid"), None);
    }

    #[test]
    fn test_replication_message() {
        let msg = ReplicationMessage::new(
            ReplicationChannel::Events,
            "worker1".to_string(),
            json!({"event": "test"}),
            1,
        );
        
        assert_eq!(msg.channel, ReplicationChannel::Events);
        assert_eq!(msg.instance_name, "worker1");
        assert_eq!(msg.sequence, 1);
    }

    #[test]
    fn test_replication_position() {
        let mut pos = ReplicationPosition::new(ReplicationChannel::Events, "master".to_string());
        
        assert_eq!(pos.position, 0);
        
        let seq1 = pos.advance();
        assert_eq!(seq1, 1);
        
        let seq2 = pos.advance();
        assert_eq!(seq2, 2);
        
        pos.update(100);
        assert_eq!(pos.position, 100);
    }

    #[tokio::test]
    async fn test_replication_service_creation() {
        let config = RedisReplicationConfig::default();
        let service = RedisReplicationService::new(config, "master".to_string());
        
        assert!(!service.is_enabled());
    }

    #[tokio::test]
    async fn test_replication_service_publish_disabled() {
        let config = RedisReplicationConfig {
            enabled: false,
            ..Default::default()
        };
        let service = RedisReplicationService::new(config, "master".to_string());
        
        let result = service.publish(ReplicationChannel::Events, json!({"test": "data"})).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replication_service_publish_enabled() {
        let config = RedisReplicationConfig {
            enabled: true,
            ..Default::default()
        };
        let service = RedisReplicationService::new(config, "master".to_string());
        
        let result = service.publish(ReplicationChannel::Events, json!({"test": "data"})).await;
        assert!(result.is_ok());
        
        let position = service.get_position(ReplicationChannel::Events).await;
        assert_eq!(position, 1);
    }

    #[tokio::test]
    async fn test_replication_service_subscribe() {
        let config = RedisReplicationConfig {
            enabled: true,
            ..Default::default()
        };
        let service = RedisReplicationService::new(config, "master".to_string());
        
        let mut receiver = service.subscribe();
        
        service.publish(ReplicationChannel::Events, json!({"test": "data"})).await.unwrap();
        
        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.channel, ReplicationChannel::Events);
    }

    #[tokio::test]
    async fn test_replication_service_stats() {
        let config = RedisReplicationConfig {
            enabled: true,
            ..Default::default()
        };
        let service = RedisReplicationService::new(config, "master".to_string());
        
        service.publish(ReplicationChannel::Events, json!({"test": "data"})).await.unwrap();
        
        let stats = service.get_stats().await;
        assert_eq!(stats.messages_sent, 1);
    }

    #[tokio::test]
    async fn test_redis_connection_manager() {
        let config = RedisReplicationConfig::default();
        let manager = RedisConnectionManager::new(config);
        
        assert!(!manager.is_connected().await);
        
        manager.connect().await.unwrap();
        assert!(manager.is_connected().await);
        
        manager.disconnect().await;
        assert!(!manager.is_connected().await);
    }

    #[test]
    fn test_get_channel_name() {
        let config = RedisReplicationConfig {
            channel_prefix: "synapse".to_string(),
            ..Default::default()
        };
        let service = RedisReplicationService::new(config, "master".to_string());
        
        assert_eq!(service.get_channel_name(ReplicationChannel::Events), "synapse:events");
    }
}
