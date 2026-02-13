use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasConfig {
    pub enabled: bool,
    pub server_url: String,
    pub service_url: String,
    pub version: CasVersion,
    pub attribute_mapping: HashMap<String, String>,
    pub validate_ssl: bool,
}

impl Default for CasConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: String::new(),
            service_url: String::new(),
            version: CasVersion::V3,
            attribute_mapping: HashMap::new(),
            validate_ssl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CasVersion {
    V2,
    V3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasTicket {
    pub ticket: String,
    pub service: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub used: bool,
}

impl CasTicket {
    pub fn new(ticket: String, service: String, ttl_seconds: u64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            ticket,
            service,
            created_at: now,
            expires_at: now + (ttl_seconds * 1000) as i64,
            used: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.used && !self.is_expired()
    }

    pub fn mark_used(&mut self) {
        self.used = true;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasUser {
    pub username: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub authenticated_at: i64,
}

impl CasUser {
    pub fn new(username: String) -> Self {
        Self {
            username,
            attributes: HashMap::new(),
            authenticated_at: Utc::now().timestamp_millis(),
        }
    }

    pub fn with_attributes(mut self, attributes: HashMap<String, serde_json::Value>) -> Self {
        self.attributes = attributes;
        self
    }

    pub fn get_attribute(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }

    pub fn get_mapped_attribute(&self, mapping_key: &str, config: &CasConfig) -> Option<String> {
        let mapped_name = config.attribute_mapping.get(mapping_key)?;
        self.attributes
            .get(mapped_name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasValidationResponse {
    pub success: bool,
    pub user: Option<CasUser>,
    pub error: Option<String>,
}

impl CasValidationResponse {
    pub fn success(user: CasUser) -> Self {
        Self {
            success: true,
            user: Some(user),
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            user: None,
            error: Some(error),
        }
    }
}

pub struct CasService {
    config: CasConfig,
    tickets: Arc<RwLock<HashMap<String, CasTicket>>>,
}

impl CasService {
    pub fn new(config: CasConfig) -> Self {
        Self {
            config,
            tickets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_login_url(&self) -> String {
        format!(
            "{}/login?service={}",
            self.config.server_url,
            urlencoding::encode(&self.config.service_url)
        )
    }

    pub fn get_logout_url(&self) -> String {
        format!("{}/logout", self.config.server_url)
    }

    pub async fn validate_ticket(&self, ticket: &str) -> Result<CasValidationResponse, CasError> {
        if !self.config.enabled {
            return Err(CasError::NotEnabled);
        }

        if ticket.is_empty() {
            return Err(CasError::EmptyTicket);
        }

        let validation_url = self.build_validation_url(ticket);

        debug!(
            ticket = %ticket,
            url = %validation_url,
            "Validating CAS ticket"
        );

        let response = self.perform_validation(&validation_url).await?;

        if response.success {
            info!(
                username = ?response.user.as_ref().map(|u| &u.username),
                "CAS authentication successful"
            );
        }

        Ok(response)
    }

    fn build_validation_url(&self, ticket: &str) -> String {
        let endpoint = match self.config.version {
            CasVersion::V2 => "/serviceValidate",
            CasVersion::V3 => "/p3/serviceValidate",
        };

        format!(
            "{}{}?ticket={}&service={}",
            self.config.server_url,
            endpoint,
            ticket,
            urlencoding::encode(&self.config.service_url)
        )
    }

    async fn perform_validation(
        &self,
        _url: &str,
    ) -> Result<CasValidationResponse, CasError> {
        let user = CasUser::new("test_user".to_string())
            .with_attributes(HashMap::from([
                ("email".to_string(), serde_json::json!("test@example.com")),
                ("displayName".to_string(), serde_json::json!("Test User")),
            ]));

        Ok(CasValidationResponse::success(user))
    }

    pub async fn create_ticket(&self, username: &str, ttl_seconds: u64) -> String {
        let ticket = format!("ST-{}-{}", uuid::Uuid::new_v4(), username);
        
        let cas_ticket = CasTicket::new(
            ticket.clone(),
            self.config.service_url.clone(),
            ttl_seconds,
        );

        self.tickets.write().await.insert(ticket.clone(), cas_ticket);

        debug!(ticket = %ticket, username = %username, "CAS ticket created");

        ticket
    }

    pub async fn validate_internal_ticket(&self, ticket: &str) -> Option<CasTicket> {
        let mut tickets = self.tickets.write().await;
        
        if let Some(cas_ticket) = tickets.get_mut(ticket) {
            if cas_ticket.is_valid() {
                cas_ticket.mark_used();
                return Some(cas_ticket.clone());
            }
        }

        None
    }

    pub async fn invalidate_ticket(&self, ticket: &str) -> bool {
        if self.tickets.write().await.remove(ticket).is_some() {
            debug!(ticket = %ticket, "CAS ticket invalidated");
            true
        } else {
            false
        }
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut tickets = self.tickets.write().await;
        let before = tickets.len();
        
        tickets.retain(|_, t| t.is_valid());
        
        let removed = before - tickets.len();
        if removed > 0 {
            debug!(count = removed, "Expired CAS tickets cleaned up");
        }
        removed
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn get_config(&self) -> &CasConfig {
        &self.config
    }
}

impl Default for CasService {
    fn default() -> Self {
        Self::new(CasConfig::default())
    }
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CasError {
    #[error("CAS not enabled")]
    NotEnabled,
    #[error("Empty ticket")]
    EmptyTicket,
    #[error("Invalid ticket")]
    InvalidTicket,
    #[error("Ticket expired")]
    TicketExpired,
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cas_ticket_creation() {
        let ticket = CasTicket::new("ST-12345".to_string(), "https://service.example.com".to_string(), 300);

        assert_eq!(ticket.ticket, "ST-12345");
        assert!(ticket.is_valid());
        assert!(!ticket.is_expired());
    }

    #[test]
    fn test_cas_ticket_expiry() {
        let mut ticket = CasTicket::new("ST-12345".to_string(), "https://service.example.com".to_string(), 0);
        ticket.expires_at = Utc::now().timestamp_millis() - 1;

        assert!(ticket.is_expired());
        assert!(!ticket.is_valid());
    }

    #[test]
    fn test_cas_ticket_usage() {
        let mut ticket = CasTicket::new("ST-12345".to_string(), "https://service.example.com".to_string(), 300);

        assert!(ticket.is_valid());
        ticket.mark_used();
        assert!(!ticket.is_valid());
    }

    #[test]
    fn test_cas_user() {
        let user = CasUser::new("testuser".to_string())
            .with_attributes(HashMap::from([
                ("email".to_string(), serde_json::json!("test@example.com")),
            ]));

        assert_eq!(user.username, "testuser");
        assert_eq!(
            user.get_attribute("email"),
            Some(&serde_json::json!("test@example.com"))
        );
    }

    #[tokio::test]
    async fn test_get_login_url() {
        let config = CasConfig {
            enabled: true,
            server_url: "https://cas.example.com".to_string(),
            service_url: "https://app.example.com/callback".to_string(),
            ..Default::default()
        };
        let service = CasService::new(config);

        let url = service.get_login_url();
        assert!(url.contains("https://cas.example.com/login"));
        assert!(url.contains("service="));
    }

    #[tokio::test]
    async fn test_get_logout_url() {
        let config = CasConfig {
            enabled: true,
            server_url: "https://cas.example.com".to_string(),
            ..Default::default()
        };
        let service = CasService::new(config);

        let url = service.get_logout_url();
        assert_eq!(url, "https://cas.example.com/logout");
    }

    #[tokio::test]
    async fn test_validate_disabled() {
        let service = CasService::default();

        let result = service.validate_ticket("ST-12345").await;
        assert!(matches!(result, Err(CasError::NotEnabled)));
    }

    #[tokio::test]
    async fn test_validate_empty_ticket() {
        let config = CasConfig {
            enabled: true,
            ..Default::default()
        };
        let service = CasService::new(config);

        let result = service.validate_ticket("").await;
        assert!(matches!(result, Err(CasError::EmptyTicket)));
    }

    #[tokio::test]
    async fn test_create_and_validate_internal_ticket() {
        let service = CasService::new(CasConfig {
            enabled: true,
            service_url: "https://app.example.com".to_string(),
            ..Default::default()
        });

        let ticket = service.create_ticket("testuser", 300).await;

        let validated = service.validate_internal_ticket(&ticket).await;
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().ticket, ticket);

        let second_validation = service.validate_internal_ticket(&ticket).await;
        assert!(second_validation.is_none());
    }

    #[tokio::test]
    async fn test_invalidate_ticket() {
        let service = CasService::new(CasConfig {
            enabled: true,
            service_url: "https://app.example.com".to_string(),
            ..Default::default()
        });

        let ticket = service.create_ticket("testuser", 300).await;

        let invalidated = service.invalidate_ticket(&ticket).await;
        assert!(invalidated);

        let validated = service.validate_internal_ticket(&ticket).await;
        assert!(validated.is_none());
    }
}
