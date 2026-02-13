use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppServiceConfig {
    pub id: String,
    pub url: String,
    pub as_token: String,
    pub hs_token: String,
    pub sender_localpart: String,
    pub rate_limited: bool,
    pub protocols: Vec<String>,
    pub namespaces: Namespaces,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespaces {
    pub users: Vec<Namespace>,
    pub aliases: Vec<Namespace>,
    pub rooms: Vec<Namespace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub exclusive: bool,
    pub regex: String,
    #[serde(default)]
    pub group_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledNamespace {
    pub exclusive: bool,
    pub regex: Regex,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppService {
    pub config: AppServiceConfig,
    pub compiled_user_namespaces: Vec<CompiledNamespace>,
    pub compiled_alias_namespaces: Vec<CompiledNamespace>,
    pub compiled_room_namespaces: Vec<CompiledNamespace>,
    http_client: reqwest::Client,
}

impl AppService {
    pub fn new(config: AppServiceConfig) -> Result<Self, regex::Error> {
        let compiled_user_namespaces = config.namespaces
            .users
            .iter()
            .map(Self::compile_namespace)
            .collect::<Result<Vec<_>, _>>()?;

        let compiled_alias_namespaces = config.namespaces
            .aliases
            .iter()
            .map(Self::compile_namespace)
            .collect::<Result<Vec<_>, _>>()?;

        let compiled_room_namespaces = config.namespaces
            .rooms
            .iter()
            .map(Self::compile_namespace)
            .collect::<Result<Vec<_>, _>>()?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Ok(Self {
            config,
            compiled_user_namespaces,
            compiled_alias_namespaces,
            compiled_room_namespaces,
            http_client,
        })
    }

    fn compile_namespace(ns: &Namespace) -> Result<CompiledNamespace, regex::Error> {
        Ok(CompiledNamespace {
            exclusive: ns.exclusive,
            regex: Regex::new(&ns.regex)?,
            group_id: ns.group_id.clone(),
        })
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn sender_localpart(&self) -> &str {
        &self.config.sender_localpart
    }

    pub fn as_token(&self) -> &str {
        &self.config.as_token
    }

    pub fn hs_token(&self) -> &str {
        &self.config.hs_token
    }

    pub fn is_user_in_namespace(&self, user_id: &str) -> bool {
        self.compiled_user_namespaces
            .iter()
            .any(|ns| ns.regex.is_match(user_id))
    }

    pub fn is_alias_in_namespace(&self, alias: &str) -> bool {
        self.compiled_alias_namespaces
            .iter()
            .any(|ns| ns.regex.is_match(alias))
    }

    pub fn is_room_in_namespace(&self, room_id: &str) -> bool {
        self.compiled_room_namespaces
            .iter()
            .any(|ns| ns.regex.is_match(room_id))
    }

    pub fn has_exclusive_user_namespace(&self, user_id: &str) -> bool {
        self.compiled_user_namespaces
            .iter()
            .any(|ns| ns.exclusive && ns.regex.is_match(user_id))
    }

    pub fn has_exclusive_alias_namespace(&self, alias: &str) -> bool {
        self.compiled_alias_namespaces
            .iter()
            .any(|ns| ns.exclusive && ns.regex.is_match(alias))
    }

    pub fn has_exclusive_room_namespace(&self, room_id: &str) -> bool {
        self.compiled_room_namespaces
            .iter()
            .any(|ns| ns.exclusive && ns.regex.is_match(room_id))
    }

    pub fn supports_protocol(&self, protocol: &str) -> bool {
        self.config.protocols.contains(&protocol.to_string())
    }

    pub async fn send_event(&self, event: &AppServiceEvent) -> Result<(), AppServiceError> {
        let url = format!("{}/_matrix/app/v1/transactions/{}", self.config.url, event.txn_id);
        
        debug!(
            appservice_id = %self.config.id,
            txn_id = %event.txn_id,
            "Sending transaction to application service"
        );

        let response = self.http_client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.hs_token))
            .json(&event)
            .send()
            .await
            .map_err(|e| AppServiceError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(
                appservice_id = %self.config.id,
                status = %status,
                body = %body,
                "Application service returned error"
            );
            return Err(AppServiceError::HttpError(status.to_string()));
        }

        info!(
            appservice_id = %self.config.id,
            txn_id = %event.txn_id,
            "Transaction sent successfully"
        );

        Ok(())
    }

    pub async fn query_user(&self, user_id: &str) -> Result<Option<QueryResponse>, AppServiceError> {
        let url = format!("{}/_matrix/app/v1/users/{}", self.config.url, user_id);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.hs_token))
            .send()
            .await
            .map_err(|e| AppServiceError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(AppServiceError::HttpError(response.status().to_string()));
        }

        let query_response: QueryResponse = response.json().await
            .map_err(|e| AppServiceError::ParseError(e.to_string()))?;

        Ok(Some(query_response))
    }

    pub async fn query_alias(&self, alias: &str) -> Result<Option<QueryResponse>, AppServiceError> {
        let url = format!("{}/_matrix/app/v1/rooms/{}", self.config.url, alias);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.hs_token))
            .send()
            .await
            .map_err(|e| AppServiceError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(AppServiceError::HttpError(response.status().to_string()));
        }

        let query_response: QueryResponse = response.json().await
            .map_err(|e| AppServiceError::ParseError(e.to_string()))?;

        Ok(Some(query_response))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppServiceEvent {
    pub txn_id: String,
    pub events: Vec<serde_json::Value>,
    pub ephemeral: Option<Vec<serde_json::Value>>,
    pub device_lists: Option<DeviceLists>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLists {
    pub changed: Vec<String>,
    pub left: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    #[serde(default)]
    pub rooms: Vec<RoomInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub canonical_alias: Option<String>,
    pub join_rules: Option<String>,
    pub world_readable: Option<bool>,
    pub guest_can_join: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppServiceError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid regex: {0}")]
    RegexError(#[from] regex::Error),
}

pub struct AppServiceRegistry {
    services: Arc<RwLock<HashMap<String, Arc<AppService>>>>,
    token_to_service: Arc<RwLock<HashMap<String, String>>>,
}

impl AppServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            token_to_service: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, config: AppServiceConfig) -> Result<(), AppServiceError> {
        let service = AppService::new(config.clone())?;
        let id = config.id.clone();
        let as_token = config.as_token.clone();

        self.services.write().await.insert(id.clone(), Arc::new(service));
        self.token_to_service.write().await.insert(as_token, id.clone());

        info!(appservice_id = %id, "Application service registered");
        Ok(())
    }

    pub async fn unregister(&self, id: &str) {
        if let Some(service) = self.services.write().await.remove(id) {
            self.token_to_service.write().await.remove(service.as_token());
            info!(appservice_id = %id, "Application service unregistered");
        }
    }

    pub async fn get(&self, id: &str) -> Option<Arc<AppService>> {
        self.services.read().await.get(id).cloned()
    }

    pub async fn get_by_token(&self, token: &str) -> Option<Arc<AppService>> {
        let token_map = self.token_to_service.read().await;
        let id = token_map.get(token)?;
        self.services.read().await.get(id).cloned()
    }

    pub async fn get_all(&self) -> Vec<Arc<AppService>> {
        self.services.read().await.values().cloned().collect()
    }

    pub async fn find_service_for_user(&self, user_id: &str) -> Option<Arc<AppService>> {
        let services = self.services.read().await;
        services
            .values()
            .find(|s| s.is_user_in_namespace(user_id))
            .cloned()
    }

    pub async fn find_service_for_alias(&self, alias: &str) -> Option<Arc<AppService>> {
        let services = self.services.read().await;
        services
            .values()
            .find(|s| s.is_alias_in_namespace(alias))
            .cloned()
    }

    pub async fn find_service_for_room(&self, room_id: &str) -> Option<Arc<AppService>> {
        let services = self.services.read().await;
        services
            .values()
            .find(|s| s.is_room_in_namespace(room_id))
            .cloned()
    }

    pub async fn has_exclusive_user_namespace(&self, user_id: &str) -> bool {
        let services = self.services.read().await;
        services.values().any(|s| s.has_exclusive_user_namespace(user_id))
    }

    pub async fn has_exclusive_alias_namespace(&self, alias: &str) -> bool {
        let services = self.services.read().await;
        services.values().any(|s| s.has_exclusive_alias_namespace(alias))
    }

    pub async fn has_exclusive_room_namespace(&self, room_id: &str) -> bool {
        let services = self.services.read().await;
        services.values().any(|s| s.has_exclusive_room_namespace(room_id))
    }

    pub async fn get_services_for_protocol(&self, protocol: &str) -> Vec<Arc<AppService>> {
        let services = self.services.read().await;
        services
            .values()
            .filter(|s| s.supports_protocol(protocol))
            .cloned()
            .collect()
    }
}

impl Default for AppServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AppServiceConfig {
        AppServiceConfig {
            id: "test-appservice".to_string(),
            url: "http://localhost:9999".to_string(),
            as_token: "test_as_token".to_string(),
            hs_token: "test_hs_token".to_string(),
            sender_localpart: "test-bot".to_string(),
            rate_limited: false,
            protocols: vec!["irc".to_string(), "slack".to_string()],
            namespaces: Namespaces {
                users: vec![
                    Namespace {
                        exclusive: true,
                        regex: "@irc_.*:example\\.com".to_string(),
                        group_id: None,
                    },
                ],
                aliases: vec![
                    Namespace {
                        exclusive: false,
                        regex: "#irc_.*:example\\.com".to_string(),
                        group_id: None,
                    },
                ],
                rooms: vec![],
            },
        }
    }

    #[test]
    fn test_appservice_creation() {
        let config = create_test_config();
        let service = AppService::new(config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_user_namespace_matching() {
        let config = create_test_config();
        let service = AppService::new(config).unwrap();

        assert!(service.is_user_in_namespace("@irc_user:example.com"));
        assert!(!service.is_user_in_namespace("@regular_user:example.com"));
    }

    #[test]
    fn test_alias_namespace_matching() {
        let config = create_test_config();
        let service = AppService::new(config).unwrap();

        assert!(service.is_alias_in_namespace("#irc_channel:example.com"));
        assert!(!service.is_alias_in_namespace("#regular_channel:example.com"));
    }

    #[test]
    fn test_exclusive_namespace() {
        let config = create_test_config();
        let service = AppService::new(config).unwrap();

        assert!(service.has_exclusive_user_namespace("@irc_user:example.com"));
        assert!(!service.has_exclusive_alias_namespace("#irc_channel:example.com"));
    }

    #[test]
    fn test_protocol_support() {
        let config = create_test_config();
        let service = AppService::new(config).unwrap();

        assert!(service.supports_protocol("irc"));
        assert!(service.supports_protocol("slack"));
        assert!(!service.supports_protocol("discord"));
    }

    #[tokio::test]
    async fn test_registry_register() {
        let registry = AppServiceRegistry::new();
        let config = create_test_config();

        let result = registry.register(config).await;
        assert!(result.is_ok());

        let service = registry.get("test-appservice").await;
        assert!(service.is_some());
    }

    #[tokio::test]
    async fn test_registry_find_service_for_user() {
        let registry = AppServiceRegistry::new();
        let config = create_test_config();
        registry.register(config).await.unwrap();

        let service = registry.find_service_for_user("@irc_user:example.com").await;
        assert!(service.is_some());

        let service = registry.find_service_for_user("@regular_user:example.com").await;
        assert!(service.is_none());
    }

    #[tokio::test]
    async fn test_registry_get_by_token() {
        let registry = AppServiceRegistry::new();
        let config = create_test_config();
        registry.register(config).await.unwrap();

        let service = registry.get_by_token("test_as_token").await;
        assert!(service.is_some());

        let service = registry.get_by_token("invalid_token").await;
        assert!(service.is_none());
    }
}
