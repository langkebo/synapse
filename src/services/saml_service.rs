use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    pub enabled: bool,
    pub idp_metadata_url: String,
    pub sp_entity_id: String,
    pub acs_url: String,
    pub slo_url: String,
    pub name_id_format: String,
    pub attribute_mapping: HashMap<String, String>,
    pub want_assertions_signed: bool,
    pub want_responses_signed: bool,
}

impl Default for SamlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            idp_metadata_url: String::new(),
            sp_entity_id: String::new(),
            acs_url: String::new(),
            slo_url: String::new(),
            name_id_format: "urn:oasis:names:tc:SAML:2.0:nameid-format:transient".to_string(),
            attribute_mapping: HashMap::new(),
            want_assertions_signed: true,
            want_responses_signed: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAuthnRequest {
    pub id: String,
    pub issue_instant: i64,
    pub destination: String,
    pub issuer: String,
    pub assertion_consumer_service_url: String,
    pub name_id_policy_format: String,
}

impl SamlAuthnRequest {
    pub fn new(config: &SamlConfig) -> Self {
        Self {
            id: format!("id_{}", uuid::Uuid::new_v4()),
            issue_instant: Utc::now().timestamp_millis(),
            destination: config.idp_metadata_url.clone(),
            issuer: config.sp_entity_id.clone(),
            assertion_consumer_service_url: config.acs_url.clone(),
            name_id_policy_format: config.name_id_format.clone(),
        }
    }

    pub fn to_xml(&self) -> String {
        format!(
            r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    ID="{}"
    Version="2.0"
    IssueInstant="{}"
    Destination="{}"
    ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
    <saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">{}</saml:Issuer>
    <samlp:NameIDPolicy Format="{}" AllowCreate="true"/>
</samlp:AuthnRequest>"#,
            self.id,
            self.issue_instant,
            self.destination,
            self.issuer,
            self.name_id_policy_format
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAssertion {
    pub id: String,
    pub issue_instant: i64,
    pub issuer: String,
    pub subject: SamlSubject,
    pub attributes: HashMap<String, Vec<String>>,
    pub session_index: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlSubject {
    pub name_id: String,
    pub name_id_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlResponse {
    pub id: String,
    pub in_response_to: Option<String>,
    pub issue_instant: i64,
    pub destination: String,
    pub issuer: String,
    pub status: SamlStatus,
    pub assertion: Option<SamlAssertion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlStatus {
    pub status_code: String,
    pub status_message: Option<String>,
}

impl SamlStatus {
    pub fn success() -> Self {
        Self {
            status_code: "urn:oasis:names:tc:SAML:2.0:status:Success".to_string(),
            status_message: None,
        }
    }

    pub fn failure(message: &str) -> Self {
        Self {
            status_code: "urn:oasis:names:tc:SAML:2.0:status:Requester".to_string(),
            status_message: Some(message.to_string()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status_code == "urn:oasis:names:tc:SAML:2.0:status:Success"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlSession {
    pub session_id: String,
    pub name_id: String,
    pub session_index: Option<String>,
    pub attributes: HashMap<String, String>,
    pub created_at: i64,
    pub expires_at: i64,
}

impl SamlSession {
    pub fn new(assertion: &SamlAssertion, ttl_seconds: u64) -> Self {
        let now = Utc::now().timestamp_millis();
        
        let attributes: HashMap<String, String> = assertion
            .attributes
            .iter()
            .filter_map(|(k, v)| v.first().map(|s| (k.clone(), s.clone())))
            .collect();

        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            name_id: assertion.subject.name_id.clone(),
            session_index: assertion.session_index.clone(),
            attributes,
            created_at: now,
            expires_at: now + (ttl_seconds * 1000) as i64,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }
}

pub struct SamlService {
    config: SamlConfig,
    sessions: Arc<RwLock<HashMap<String, SamlSession>>>,
    authn_requests: Arc<RwLock<HashMap<String, SamlAuthnRequest>>>,
}

impl SamlService {
    pub fn new(config: SamlConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            authn_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_authn_request(&self) -> SamlAuthnRequest {
        let request = SamlAuthnRequest::new(&self.config);
        
        self.authn_requests
            .write()
            .await
            .insert(request.id.clone(), request.clone());

        info!(request_id = %request.id, "SAML AuthnRequest created");

        request
    }

    pub async fn get_authn_request(&self, id: &str) -> Option<SamlAuthnRequest> {
        self.authn_requests.read().await.get(id).cloned()
    }

    pub async fn process_response(&self, response: &SamlResponse) -> Result<SamlSession, SamlError> {
        if !self.config.enabled {
            return Err(SamlError::NotEnabled);
        }

        if !response.status.is_success() {
            return Err(SamlError::AuthenticationFailed(
                response.status.status_message.clone().unwrap_or_default(),
            ));
        }

        let assertion = response
            .assertion
            .as_ref()
            .ok_or(SamlError::MissingAssertion)?;

        let session = SamlSession::new(assertion, 3600);

        self.sessions
            .write()
            .await
            .insert(session.session_id.clone(), session.clone());

        info!(
            session_id = %session.session_id,
            name_id = %session.name_id,
            "SAML session created"
        );

        Ok(session)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<SamlSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn validate_session(&self, session_id: &str) -> bool {
        self.sessions
            .read()
            .await
            .get(session_id)
            .map(|s| s.is_valid())
            .unwrap_or(false)
    }

    pub async fn logout(&self, session_id: &str) -> Result<(), SamlError> {
        let mut sessions = self.sessions.write().await;
        
        if sessions.remove(session_id).is_some() {
            info!(session_id = %session_id, "SAML session terminated");
            Ok(())
        } else {
            Err(SamlError::SessionNotFound)
        }
    }

    pub fn create_logout_request(&self, session: &SamlSession) -> String {
        format!(
            r#"<samlp:LogoutRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
    ID="id_{}"
    Version="2.0"
    IssueInstant="{}">
    <saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">{}</saml:Issuer>
    <saml:NameID xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" Format="{}">{}</saml:NameID>
    <samlp:SessionIndex>{}</samlp:SessionIndex>
</samlp:LogoutRequest>"#,
            uuid::Uuid::new_v4(),
            Utc::now().timestamp_millis(),
            self.config.sp_entity_id,
            session.attributes.get("name_id_format").unwrap_or(&"".to_string()),
            session.name_id,
            session.session_index.as_deref().unwrap_or("")
        )
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        
        sessions.retain(|_, s| s.is_valid());
        
        let removed = before - sessions.len();
        if removed > 0 {
            debug!(count = removed, "Expired SAML sessions cleaned up");
        }
        removed
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn get_config(&self) -> &SamlConfig {
        &self.config
    }
}

impl Default for SamlService {
    fn default() -> Self {
        Self::new(SamlConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SamlError {
    #[error("SAML not enabled")]
    NotEnabled,
    #[error("Missing assertion")]
    MissingAssertion,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Session not found")]
    SessionNotFound,
    #[error("Invalid response")]
    InvalidResponse,
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saml_status() {
        let success = SamlStatus::success();
        assert!(success.is_success());

        let failure = SamlStatus::failure("Test error");
        assert!(!failure.is_success());
    }

    #[test]
    fn test_saml_authn_request() {
        let config = SamlConfig {
            enabled: true,
            sp_entity_id: "https://sp.example.com".to_string(),
            acs_url: "https://sp.example.com/acs".to_string(),
            ..Default::default()
        };

        let request = SamlAuthnRequest::new(&config);

        assert!(!request.id.is_empty());
        assert!(request.to_xml().contains("AuthnRequest"));
    }

    #[test]
    fn test_saml_session() {
        let assertion = SamlAssertion {
            id: "assertion_123".to_string(),
            issue_instant: Utc::now().timestamp_millis(),
            issuer: "https://idp.example.com".to_string(),
            subject: SamlSubject {
                name_id: "user@example.com".to_string(),
                name_id_format: "urn:oasis:names:tc:SAML:2.0:nameid-format:emailAddress".to_string(),
            },
            attributes: HashMap::from([
                ("email".to_string(), vec!["user@example.com".to_string()]),
                ("name".to_string(), vec!["Test User".to_string()]),
            ]),
            session_index: Some("session_123".to_string()),
        };

        let session = SamlSession::new(&assertion, 3600);

        assert!(!session.session_id.is_empty());
        assert_eq!(session.name_id, "user@example.com");
        assert!(session.is_valid());
        assert_eq!(session.get_attribute("email"), Some(&"user@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_create_authn_request() {
        let service = SamlService::new(SamlConfig {
            enabled: true,
            ..Default::default()
        });

        let request = service.create_authn_request().await;

        let retrieved = service.get_authn_request(&request.id).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_process_response_disabled() {
        let service = SamlService::default();

        let response = SamlResponse {
            id: "response_123".to_string(),
            in_response_to: None,
            issue_instant: Utc::now().timestamp_millis(),
            destination: "https://sp.example.com/acs".to_string(),
            issuer: "https://idp.example.com".to_string(),
            status: SamlStatus::success(),
            assertion: None,
        };

        let result = service.process_response(&response).await;
        assert!(matches!(result, Err(SamlError::NotEnabled)));
    }

    #[tokio::test]
    async fn test_process_response_failure() {
        let service = SamlService::new(SamlConfig {
            enabled: true,
            ..Default::default()
        });

        let response = SamlResponse {
            id: "response_123".to_string(),
            in_response_to: None,
            issue_instant: Utc::now().timestamp_millis(),
            destination: "https://sp.example.com/acs".to_string(),
            issuer: "https://idp.example.com".to_string(),
            status: SamlStatus::failure("Auth failed"),
            assertion: None,
        };

        let result = service.process_response(&response).await;
        assert!(matches!(result, Err(SamlError::AuthenticationFailed(_))));
    }

    #[tokio::test]
    async fn test_process_response_missing_assertion() {
        let service = SamlService::new(SamlConfig {
            enabled: true,
            ..Default::default()
        });

        let response = SamlResponse {
            id: "response_123".to_string(),
            in_response_to: None,
            issue_instant: Utc::now().timestamp_millis(),
            destination: "https://sp.example.com/acs".to_string(),
            issuer: "https://idp.example.com".to_string(),
            status: SamlStatus::success(),
            assertion: None,
        };

        let result = service.process_response(&response).await;
        assert!(matches!(result, Err(SamlError::MissingAssertion)));
    }

    #[tokio::test]
    async fn test_process_response_success() {
        let service = SamlService::new(SamlConfig {
            enabled: true,
            ..Default::default()
        });

        let response = SamlResponse {
            id: "response_123".to_string(),
            in_response_to: None,
            issue_instant: Utc::now().timestamp_millis(),
            destination: "https://sp.example.com/acs".to_string(),
            issuer: "https://idp.example.com".to_string(),
            status: SamlStatus::success(),
            assertion: Some(SamlAssertion {
                id: "assertion_123".to_string(),
                issue_instant: Utc::now().timestamp_millis(),
                issuer: "https://idp.example.com".to_string(),
                subject: SamlSubject {
                    name_id: "user@example.com".to_string(),
                    name_id_format: "urn:oasis:names:tc:SAML:2.0:nameid-format:emailAddress".to_string(),
                },
                attributes: HashMap::new(),
                session_index: None,
            }),
        };

        let session = service.process_response(&response).await.unwrap();
        assert!(service.validate_session(&session.session_id).await);
    }

    #[tokio::test]
    async fn test_logout() {
        let service = SamlService::new(SamlConfig {
            enabled: true,
            ..Default::default()
        });

        let response = SamlResponse {
            id: "response_123".to_string(),
            in_response_to: None,
            issue_instant: Utc::now().timestamp_millis(),
            destination: "https://sp.example.com/acs".to_string(),
            issuer: "https://idp.example.com".to_string(),
            status: SamlStatus::success(),
            assertion: Some(SamlAssertion {
                id: "assertion_123".to_string(),
                issue_instant: Utc::now().timestamp_millis(),
                issuer: "https://idp.example.com".to_string(),
                subject: SamlSubject {
                    name_id: "user@example.com".to_string(),
                    name_id_format: "urn:oasis:names:tc:SAML:2.0:nameid-format:emailAddress".to_string(),
                },
                attributes: HashMap::new(),
                session_index: None,
            }),
        };

        let session = service.process_response(&response).await.unwrap();
        
        service.logout(&session.session_id).await.unwrap();
        assert!(!service.validate_session(&session.session_id).await);
    }
}
