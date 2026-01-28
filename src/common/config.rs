use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub logging: LoggingConfig,
    pub federation: FederationConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub registration_shared_secret: Option<String>,
    pub admin_contact: Option<String>,
    pub max_upload_size: u64,
    pub max_image_resolution: u32,
    pub enable_registration: bool,
    pub enable_registration_captcha: bool,
    pub background_tasks_interval: u64,
    pub expire_access_token: bool,
    pub expire_access_token_lifetime: i64,
    pub refresh_token_lifetime: i64,
    pub refresh_token_sliding_window_size: i64,
    pub session_duration: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub name: String,
    pub pool_size: u32,
    pub max_size: u32,
    pub min_idle: Option<u32>,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub key_prefix: String,
    pub pool_size: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub log_file: Option<String>,
    pub log_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FederationConfig {
    pub enabled: bool,
    pub allow_ingress: bool,
    pub server_name: String,
    pub federation_port: u16,
    pub connection_pool_size: u32,
    pub max_transaction_payload: u64,
    pub ca_file: Option<PathBuf>,
    pub client_ca_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub secret: String,
    pub expiry_time: i64,
    pub refresh_token_expiry: i64,
    pub bcrypt_rounds: u32,
}

impl Config {
    pub async fn load() -> Result<Self, config::ConfigError> {
        let mut config = config::Config::new();
        config.merge(config::File::with_name("config.yaml"))?;
        config.merge(config::Environment::with_prefix("SYNAPSE"))?;
        config.try_into()
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.username,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.name
        )
    }

    pub fn redis_url(&self) -> String {
        format!("redis://{}:{}", self.redis.host, self.redis.port)
    }
}
