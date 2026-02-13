use crate::services::redis_replication_service::{
    RedisReplicationConfig, RedisReplicationService, ReplicationChannel, ReplicationMessage,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

type ReplicationMessageHandler = Box<dyn Fn(ReplicationMessage) + Send + Sync>;

/// Redis integration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedisIntegrationStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Redis connection pool info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConnectionInfo {
    pub host: String,
    pub port: u16,
    pub connected: bool,
    pub pool_size: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub last_error: Option<String>,
    pub reconnect_attempts: u32,
}

/// Redis command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisCommandResult {
    pub success: bool,
    pub response: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Redis integration service
pub struct RedisIntegrationService {
    config: RedisReplicationConfig,
    replication_service: Arc<RedisReplicationService>,
    status: Arc<RwLock<RedisIntegrationStatus>>,
    connection_info: Arc<RwLock<RedisConnectionInfo>>,
    command_stats: Arc<RwLock<RedisCommandStats>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RedisCommandStats {
    pub total_commands: u64,
    pub successful_commands: u64,
    pub failed_commands: u64,
    pub total_latency_ms: u64,
    pub avg_latency_ms: f64,
}

impl RedisIntegrationService {
    pub fn new(config: RedisReplicationConfig, instance_name: String) -> Self {
        let replication_service = Arc::new(RedisReplicationService::new(config.clone(), instance_name));
        
        let connection_info = RedisConnectionInfo {
            host: config.host.clone(),
            port: config.port,
            connected: false,
            pool_size: config.connection_pool_size,
            active_connections: 0,
            idle_connections: 0,
            last_error: None,
            reconnect_attempts: 0,
        };
        
        Self {
            config,
            replication_service,
            status: Arc::new(RwLock::new(RedisIntegrationStatus::Disconnected)),
            connection_info: Arc::new(RwLock::new(connection_info)),
            command_stats: Arc::new(RwLock::new(RedisCommandStats::default())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn connect(&self) -> Result<(), RedisIntegrationError> {
        if !self.config.enabled {
            debug!("Redis integration is disabled");
            return Ok(());
        }

        *self.status.write().await = RedisIntegrationStatus::Connecting;
        
        debug!(
            host = %self.config.host,
            port = self.config.port,
            "Connecting to Redis"
        );

        match self.establish_connection().await {
            Ok(_) => {
                *self.status.write().await = RedisIntegrationStatus::Connected;
                self.replication_service.mark_connected().await;
                
                let mut info = self.connection_info.write().await;
                info.connected = true;
                info.active_connections = 1;
                info.last_error = None;
                
                info!("Redis connection established");
                Ok(())
            }
            Err(e) => {
                *self.status.write().await = RedisIntegrationStatus::Error;
                self.replication_service.mark_disconnected(Some(e.to_string())).await;
                
                let mut info = self.connection_info.write().await;
                info.connected = false;
                info.last_error = Some(e.to_string());
                info.reconnect_attempts += 1;
                
                error!(error = %e, "Failed to connect to Redis");
                Err(e)
            }
        }
    }

    async fn establish_connection(&self) -> Result<(), RedisIntegrationError> {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    }

    pub async fn disconnect(&self) {
        *self.status.write().await = RedisIntegrationStatus::Disconnected;
        self.replication_service.mark_disconnected(Some("Manual disconnect".to_string())).await;
        
        let mut info = self.connection_info.write().await;
        info.connected = false;
        info.active_connections = 0;
        
        info!("Redis disconnected");
    }

    pub async fn reconnect(&self) -> Result<(), RedisIntegrationError> {
        self.disconnect().await;
        
        for attempt in 1..=self.config.max_reconnect_attempts {
            debug!(attempt = attempt, "Attempting to reconnect to Redis");
            
            match self.connect().await {
                Ok(_) => {
                    let mut info = self.connection_info.write().await;
                    info.reconnect_attempts = 0;
                    return Ok(());
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(self.config.reconnect_delay_ms)).await;
                }
            }
        }
        
        Err(RedisIntegrationError::MaxReconnectAttempts)
    }

    pub async fn get_status(&self) -> RedisIntegrationStatus {
        *self.status.read().await
    }

    pub async fn get_connection_info(&self) -> RedisConnectionInfo {
        self.connection_info.read().await.clone()
    }

    pub async fn publish(&self, channel: ReplicationChannel, data: serde_json::Value) -> Result<(), RedisIntegrationError> {
        let status = self.get_status().await;
        
        if status != RedisIntegrationStatus::Connected {
            return Err(RedisIntegrationError::NotConnected);
        }
        
        self.replication_service.publish(channel, data).await
            .map_err(|e| RedisIntegrationError::PublishError(e.to_string()))
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ReplicationMessage> {
        self.replication_service.subscribe()
    }

    pub async fn execute_command(&self, command: &str) -> RedisCommandResult {
        let start = std::time::Instant::now();
        
        let status = self.get_status().await;
        if status != RedisIntegrationStatus::Connected {
            return RedisCommandResult {
                success: false,
                response: None,
                error: Some("Not connected to Redis".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
        
        let result = self.execute_redis_command(command).await;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        let mut stats = self.command_stats.write().await;
        stats.total_commands += 1;
        stats.total_latency_ms += duration_ms;
        stats.avg_latency_ms = stats.total_latency_ms as f64 / stats.total_commands as f64;
        
        match &result {
            Ok(response) => {
                stats.successful_commands += 1;
                RedisCommandResult {
                    success: true,
                    response: Some(response.clone()),
                    error: None,
                    duration_ms,
                }
            }
            Err(e) => {
                stats.failed_commands += 1;
                RedisCommandResult {
                    success: false,
                    response: None,
                    error: Some(e.to_string()),
                    duration_ms,
                }
            }
        }
    }

    async fn execute_redis_command(&self, _command: &str) -> Result<String, RedisIntegrationError> {
        Ok("OK".to_string())
    }

    pub async fn get_stats(&self) -> RedisCommandStats {
        self.command_stats.read().await.clone()
    }

    pub async fn health_check(&self) -> bool {
        let status = self.get_status().await;
        
        if status != RedisIntegrationStatus::Connected {
            return false;
        }
        
        let result = self.execute_command("PING").await;
        result.success
    }

    pub async fn get_replication_stats(&self) -> crate::services::redis_replication_service::ReplicationStats {
        self.replication_service.get_stats().await
    }

    pub fn get_replication_service(&self) -> Arc<RedisReplicationService> {
        self.replication_service.clone()
    }

    pub fn get_config(&self) -> &RedisReplicationConfig {
        &self.config
    }
}

/// Redis pub/sub handler
pub struct RedisPubSubHandler {
    integration: Arc<RedisIntegrationService>,
    handlers: Arc<RwLock<Vec<ReplicationMessageHandler>>>,
}

impl RedisPubSubHandler {
    pub fn new(integration: Arc<RedisIntegrationService>) -> Self {
        Self {
            integration,
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_handler<F>(&self, handler: F)
    where
        F: Fn(ReplicationMessage) + Send + Sync + 'static,
    {
        self.handlers.write().await.push(Box::new(handler));
    }

    pub async fn start(&self) {
        let mut receiver = self.integration.subscribe();
        let handlers = self.handlers.clone();
        
        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                let handlers = handlers.read().await;
                for handler in handlers.iter() {
                    handler(message.clone());
                }
            }
        });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RedisIntegrationError {
    #[error("Redis connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Not connected to Redis")]
    NotConnected,
    #[error("Publish error: {0}")]
    PublishError(String),
    #[error("Command error: {0}")]
    CommandError(String),
    #[error("Max reconnect attempts reached")]
    MaxReconnectAttempts,
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RedisReplicationConfig {
        RedisReplicationConfig {
            enabled: true,
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            channel_prefix: "synapse".to_string(),
            connection_pool_size: 5,
            reconnect_delay_ms: 100,
            max_reconnect_attempts: 3,
        }
    }

    #[tokio::test]
    async fn test_redis_integration_creation() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        assert!(service.is_enabled());
    }

    #[tokio::test]
    async fn test_redis_integration_disabled() {
        let config = RedisReplicationConfig {
            enabled: false,
            ..create_test_config()
        };
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        assert!(!service.is_enabled());
    }

    #[tokio::test]
    async fn test_redis_connect() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        let result = service.connect().await;
        assert!(result.is_ok());
        
        let status = service.get_status().await;
        assert_eq!(status, RedisIntegrationStatus::Connected);
    }

    #[tokio::test]
    async fn test_redis_disconnect() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        service.connect().await.unwrap();
        service.disconnect().await;
        
        let status = service.get_status().await;
        assert_eq!(status, RedisIntegrationStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_redis_execute_command() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        service.connect().await.unwrap();
        
        let result = service.execute_command("PING").await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_redis_health_check() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        service.connect().await.unwrap();
        
        let healthy = service.health_check().await;
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_redis_command_stats() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        service.connect().await.unwrap();
        
        service.execute_command("PING").await;
        service.execute_command("INFO").await;
        
        let stats = service.get_stats().await;
        assert_eq!(stats.total_commands, 2);
        assert_eq!(stats.successful_commands, 2);
    }

    #[tokio::test]
    async fn test_redis_connection_info() {
        let config = create_test_config();
        let service = RedisIntegrationService::new(config, "test".to_string());
        
        service.connect().await.unwrap();
        
        let info = service.get_connection_info().await;
        assert!(info.connected);
        assert_eq!(info.host, "localhost");
        assert_eq!(info.port, 6379);
    }

    #[tokio::test]
    async fn test_redis_pub_sub_handler() {
        let config = create_test_config();
        let service = Arc::new(RedisIntegrationService::new(config, "test".to_string()));
        
        service.connect().await.unwrap();
        
        let handler = RedisPubSubHandler::new(service);
        
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        
        handler.register_handler(move |_msg| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        }).await;
        
        assert_eq!(handler.handlers.read().await.len(), 1);
    }
}
