use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Manhole configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManholeConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub allowed_origins: Vec<String>,
    pub max_connections: usize,
    pub connection_timeout_seconds: u64,
}

impl Default for ManholeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: "127.0.0.1".to_string(),
            port: 9000,
            username: "admin".to_string(),
            password: "changeme".to_string(),
            allowed_origins: vec!["127.0.0.1".to_string(), "::1".to_string()],
            max_connections: 5,
            connection_timeout_seconds: 300,
        }
    }
}

impl ManholeConfig {
    pub fn validate(&self) -> Result<(), ManholeError> {
        if self.enabled {
            if self.username.is_empty() {
                return Err(ManholeError::InvalidConfiguration("Username cannot be empty".to_string()));
            }
            if self.password.is_empty() || self.password == "changeme" {
                warn!("Manhole is using default or empty password. Set a secure password in production!");
            }
        }
        Ok(())
    }
}

/// Manhole session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManholeSession {
    pub session_id: String,
    pub client_addr: String,
    pub username: String,
    pub connected_at: i64,
    pub last_activity: i64,
    pub commands_executed: u64,
}

impl ManholeSession {
    pub fn new(client_addr: String, username: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            client_addr,
            username,
            connected_at: now,
            last_activity: now,
            commands_executed: 0,
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Utc::now().timestamp_millis();
    }

    pub fn increment_commands(&mut self) {
        self.commands_executed += 1;
        self.touch();
    }

    pub fn is_expired(&self, timeout_seconds: i64) -> bool {
        let now = Utc::now().timestamp_millis();
        (now - self.last_activity) > (timeout_seconds * 1000)
    }
}

/// Manhole command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManholeResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ManholeResult {
    pub fn success(output: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
        }
    }

    pub fn error(error: String, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
            duration_ms,
        }
    }
}

/// Manhole command definition
#[derive(Debug, Clone)]
pub struct ManholeCommand {
    pub name: String,
    pub description: String,
    pub handler: fn(&ManholeContext, &[String]) -> Result<String, ManholeError>,
}

/// Manhole execution context
#[derive(Debug, Clone)]
pub struct ManholeContext {
    pub session: ManholeSession,
    pub server_stats: ServerStats,
}

/// Server statistics for debugging
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerStats {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub active_connections: u64,
    pub memory_used_mb: u64,
    pub cpu_usage_percent: f32,
    pub rooms_count: u64,
    pub users_count: u64,
    pub events_count: u64,
    pub federation_queue_size: u64,
}

/// Manhole service for debugging
pub struct ManholeService {
    config: ManholeConfig,
    sessions: Arc<RwLock<HashMap<String, ManholeSession>>>,
    commands: Arc<RwLock<HashMap<String, ManholeCommand>>>,
    stats: Arc<RwLock<ServerStats>>,
    start_time: i64,
}

impl ManholeService {
    pub fn new(config: ManholeConfig) -> Self {
        config.validate().ok();
        
        let mut commands = HashMap::new();
        
        commands.insert("help".to_string(), ManholeCommand {
            name: "help".to_string(),
            description: "Show available commands".to_string(),
            handler: |_ctx, _args| {
                Ok("Available commands:\n\
                   - help: Show this help\n\
                   - stats: Show server statistics\n\
                   - rooms: List rooms\n\
                   - users: List users\n\
                   - federation: Show federation status\n\
                   - memory: Show memory usage\n\
                   - gc: Trigger garbage collection\n\
                   - version: Show server version\n\
                   - uptime: Show server uptime".to_string())
            },
        });
        
        commands.insert("stats".to_string(), ManholeCommand {
            name: "stats".to_string(),
            description: "Show server statistics".to_string(),
            handler: |ctx, _args| {
                Ok(format!(
                    "Server Statistics:\n\
                     Uptime: {}s\n\
                     Total Requests: {}\n\
                     Active Connections: {}\n\
                     Memory: {}MB\n\
                     CPU: {:.1}%\n\
                     Rooms: {}\n\
                     Users: {}\n\
                     Events: {}",
                    ctx.server_stats.uptime_seconds,
                    ctx.server_stats.total_requests,
                    ctx.server_stats.active_connections,
                    ctx.server_stats.memory_used_mb,
                    ctx.server_stats.cpu_usage_percent,
                    ctx.server_stats.rooms_count,
                    ctx.server_stats.users_count,
                    ctx.server_stats.events_count
                ))
            },
        });
        
        commands.insert("version".to_string(), ManholeCommand {
            name: "version".to_string(),
            description: "Show server version".to_string(),
            handler: |_ctx, _args| {
                Ok("Synapse-Rust v0.1.0\nMatrix homeserver implementation in Rust".to_string())
            },
        });
        
        commands.insert("uptime".to_string(), ManholeCommand {
            name: "uptime".to_string(),
            description: "Show server uptime".to_string(),
            handler: |ctx, _args| {
                let uptime = ctx.server_stats.uptime_seconds;
                let days = uptime / 86400;
                let hours = (uptime % 86400) / 3600;
                let minutes = (uptime % 3600) / 60;
                let seconds = uptime % 60;
                Ok(format!("Uptime: {}d {}h {}m {}s", days, hours, minutes, seconds))
            },
        });
        
        commands.insert("rooms".to_string(), ManholeCommand {
            name: "rooms".to_string(),
            description: "List rooms".to_string(),
            handler: |ctx, _args| {
                Ok(format!("Total rooms: {}\nUse 'room <room_id>' for details", ctx.server_stats.rooms_count))
            },
        });
        
        commands.insert("users".to_string(), ManholeCommand {
            name: "users".to_string(),
            description: "List users".to_string(),
            handler: |ctx, _args| {
                Ok(format!("Total users: {}\nUse 'user <user_id>' for details", ctx.server_stats.users_count))
            },
        });
        
        commands.insert("federation".to_string(), ManholeCommand {
            name: "federation".to_string(),
            description: "Show federation status".to_string(),
            handler: |ctx, _args| {
                Ok(format!(
                    "Federation Status:\n\
                     Queue Size: {}\n\
                     Status: Active",
                    ctx.server_stats.federation_queue_size
                ))
            },
        });
        
        commands.insert("memory".to_string(), ManholeCommand {
            name: "memory".to_string(),
            description: "Show memory usage".to_string(),
            handler: |ctx, _args| {
                Ok(format!(
                    "Memory Usage:\n\
                     Used: {}MB\n\
                     Available: ~{}MB (estimated)",
                    ctx.server_stats.memory_used_mb,
                    1024 - ctx.server_stats.memory_used_mb
                ))
            },
        });
        
        commands.insert("gc".to_string(), ManholeCommand {
            name: "gc".to_string(),
            description: "Trigger garbage collection hint".to_string(),
            handler: |_ctx, _args| {
                std::mem::drop(Vec::<u8>::with_capacity(1024 * 1024));
                Ok("Garbage collection hint sent".to_string())
            },
        });
        
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(commands)),
            stats: Arc::new(RwLock::new(ServerStats::default())),
            start_time: Utc::now().timestamp_millis(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn authenticate(&self, username: &str, password: &str, client_addr: &str) -> Option<ManholeSession> {
        if !self.config.enabled {
            return None;
        }

        if !self.is_origin_allowed(client_addr) {
            warn!(client_addr = %client_addr, "Manhole connection rejected from unauthorized origin");
            return None;
        }

        if username != self.config.username || password != self.config.password {
            warn!(username = %username, "Manhole authentication failed");
            return None;
        }

        let sessions = self.sessions.read().await;
        if sessions.len() >= self.config.max_connections {
            warn!("Manhole max connections reached");
            return None;
        }
        drop(sessions);

        let session = ManholeSession::new(client_addr.to_string(), username.to_string());
        self.sessions.write().await.insert(session.session_id.clone(), session.clone());
        
        info!(
            session_id = %session.session_id,
            client_addr = %client_addr,
            "Manhole session created"
        );
        
        Some(session)
    }

    pub async fn disconnect(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().await.remove(session_id) {
            info!(
                session_id = %session_id,
                commands = session.commands_executed,
                "Manhole session disconnected"
            );
        }
    }

    pub async fn execute(&self, session_id: &str, command: &str) -> ManholeResult {
        let start = std::time::Instant::now();
        
        let mut sessions = self.sessions.write().await;
        let session = match sessions.get_mut(session_id) {
            Some(s) => s,
            None => return ManholeResult::error("Session not found".to_string(), 0),
        };
        
        session.increment_commands();
        let session_clone = session.clone();
        drop(sessions);
        
        let parts: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        if parts.is_empty() {
            return ManholeResult::success("No command provided".to_string(), start.elapsed().as_millis() as u64);
        }
        
        let cmd_name = &parts[0];
        let args = &parts[1..];
        
        let stats = self.stats.read().await.clone();
        let ctx = ManholeContext {
            session: session_clone,
            server_stats: stats,
        };
        
        let commands = self.commands.read().await;
        match commands.get(cmd_name) {
            Some(cmd) => {
                match (cmd.handler)(&ctx, args) {
                    Ok(output) => {
                        debug!(command = %cmd_name, "Manhole command executed");
                        ManholeResult::success(output, start.elapsed().as_millis() as u64)
                    }
                    Err(e) => ManholeResult::error(e.to_string(), start.elapsed().as_millis() as u64),
                }
            }
            None => ManholeResult::error(
                format!("Unknown command: {}. Type 'help' for available commands.", cmd_name),
                start.elapsed().as_millis() as u64,
            ),
        }
    }

    fn is_origin_allowed(&self, addr: &str) -> bool {
        if self.config.allowed_origins.is_empty() {
            return true;
        }
        
        let ip = addr.split(':').next().unwrap_or(addr);
        self.config.allowed_origins.iter().any(|origin| origin == ip || origin == addr)
    }

    pub async fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        
        sessions.retain(|_, s| !s.is_expired(self.config.connection_timeout_seconds as i64));
        
        let removed = before - sessions.len();
        if removed > 0 {
            debug!(count = removed, "Expired manhole sessions cleaned up");
        }
        removed
    }

    pub async fn get_active_sessions(&self) -> Vec<ManholeSession> {
        self.sessions.read().await.values().cloned().collect()
    }

    pub async fn update_stats<F>(&self, f: F) 
    where
        F: FnOnce(&mut ServerStats),
    {
        let mut stats = self.stats.write().await;
        f(&mut stats);
    }

    pub async fn get_stats(&self) -> ServerStats {
        let mut stats = self.stats.read().await.clone();
        stats.uptime_seconds = ((Utc::now().timestamp_millis() - self.start_time) / 1000) as u64;
        stats
    }

    pub fn get_config(&self) -> &ManholeConfig {
        &self.config
    }

    pub async fn register_command(&self, command: ManholeCommand) {
        self.commands.write().await.insert(command.name.clone(), command);
    }
}

impl Default for ManholeService {
    fn default() -> Self {
        Self::new(ManholeConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManholeError {
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Session not found")]
    SessionNotFound,
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    #[error("Max connections reached")]
    MaxConnectionsReached,
    #[error("Origin not allowed")]
    OriginNotAllowed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manhole_config_default() {
        let config = ManholeConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 9000);
    }

    #[test]
    fn test_manhole_config_validate() {
        let config = ManholeConfig {
            enabled: true,
            username: String::new(),
            ..Default::default()
        };
        
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_manhole_session() {
        let session = ManholeSession::new("127.0.0.1:12345".to_string(), "admin".to_string());
        
        assert!(!session.session_id.is_empty());
        assert_eq!(session.username, "admin");
        assert_eq!(session.commands_executed, 0);
    }

    #[test]
    fn test_manhole_session_commands() {
        let mut session = ManholeSession::new("127.0.0.1:12345".to_string(), "admin".to_string());
        
        session.increment_commands();
        assert_eq!(session.commands_executed, 1);
    }

    #[test]
    fn test_manhole_result() {
        let success = ManholeResult::success("output".to_string(), 10);
        assert!(success.success);
        assert_eq!(success.output, "output");
        
        let error = ManholeResult::error("error message".to_string(), 5);
        assert!(!error.success);
        assert_eq!(error.error, Some("error message".to_string()));
    }

    #[tokio::test]
    async fn test_manhole_service_creation() {
        let service = ManholeService::default();
        assert!(!service.is_enabled());
    }

    #[tokio::test]
    async fn test_manhole_authenticate_disabled() {
        let service = ManholeService::default();
        
        let result = service.authenticate("admin", "password", "127.0.0.1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_manhole_authenticate_success() {
        let config = ManholeConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        
        let service = ManholeService::new(config);
        
        let result = service.authenticate("admin", "secret", "127.0.0.1").await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_manhole_authenticate_wrong_password() {
        let config = ManholeConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        
        let service = ManholeService::new(config);
        
        let result = service.authenticate("admin", "wrong", "127.0.0.1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_manhole_execute_help() {
        let config = ManholeConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        
        let service = ManholeService::new(config);
        
        let session = service.authenticate("admin", "secret", "127.0.0.1").await.unwrap();
        let result = service.execute(&session.session_id, "help").await;
        
        assert!(result.success);
        assert!(result.output.contains("Available commands"));
    }

    #[tokio::test]
    async fn test_manhole_execute_stats() {
        let config = ManholeConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        
        let service = ManholeService::new(config);
        
        service.update_stats(|s| {
            s.rooms_count = 100;
            s.users_count = 50;
        }).await;
        
        let session = service.authenticate("admin", "secret", "127.0.0.1").await.unwrap();
        let result = service.execute(&session.session_id, "stats").await;
        
        assert!(result.success);
        assert!(result.output.contains("Rooms: 100"));
        assert!(result.output.contains("Users: 50"));
    }

    #[tokio::test]
    async fn test_manhole_execute_unknown_command() {
        let config = ManholeConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        
        let service = ManholeService::new(config);
        
        let session = service.authenticate("admin", "secret", "127.0.0.1").await.unwrap();
        let result = service.execute(&session.session_id, "unknown").await;
        
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown command"));
    }
}
