use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConsentId {
    pub user_id: String,
    pub consent_type: ConsentType,
}

impl ConsentId {
    pub fn new(user_id: String, consent_type: ConsentType) -> Self {
        Self {
            user_id,
            consent_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConsentType {
    TermsOfService,
    PrivacyPolicy,
    EmailMarketing,
    DataProcessing,
    ThirdPartySharing,
    CookieUsage,
    Custom(String),
}

impl ConsentType {
    pub fn as_str(&self) -> &str {
        match self {
            ConsentType::TermsOfService => "terms_of_service",
            ConsentType::PrivacyPolicy => "privacy_policy",
            ConsentType::EmailMarketing => "email_marketing",
            ConsentType::DataProcessing => "data_processing",
            ConsentType::ThirdPartySharing => "third_party_sharing",
            ConsentType::CookieUsage => "cookie_usage",
            ConsentType::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConsent {
    pub id: ConsentId,
    pub version: String,
    pub granted: bool,
    pub granted_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub source: ConsentSource,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UserConsent {
    pub fn new(user_id: String, consent_type: ConsentType, version: String) -> Self {
        Self {
            id: ConsentId::new(user_id, consent_type),
            version,
            granted: false,
            granted_at: None,
            revoked_at: None,
            source: ConsentSource::Web,
            ip_address: None,
            user_agent: None,
            metadata: HashMap::new(),
        }
    }

    pub fn grant(mut self, source: ConsentSource) -> Self {
        self.granted = true;
        self.granted_at = Some(Utc::now().timestamp_millis());
        self.revoked_at = None;
        self.source = source;
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn revoke(&mut self) {
        self.granted = false;
        self.revoked_at = Some(Utc::now().timestamp_millis());
    }

    pub fn is_active(&self) -> bool {
        self.granted && self.revoked_at.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentSource {
    Web,
    Api,
    Admin,
    Migration,
    Implicit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentPolicy {
    pub consent_type: ConsentType,
    pub current_version: String,
    pub required: bool,
    pub description: String,
    pub url: Option<String>,
}

impl ConsentPolicy {
    pub fn new(consent_type: ConsentType, version: String, required: bool) -> Self {
        Self {
            consent_type,
            current_version: version,
            required,
            description: String::new(),
            url: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsentStats {
    pub total_consents: u64,
    pub active_consents: u64,
    pub revoked_consents: u64,
    pub pending_consents: u64,
}

pub struct ConsentService {
    consents: Arc<RwLock<HashMap<ConsentId, UserConsent>>>,
    policies: Arc<RwLock<HashMap<ConsentType, ConsentPolicy>>>,
}

impl ConsentService {
    pub fn new() -> Self {
        Self {
            consents: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_policy(&self, policy: ConsentPolicy) {
        let consent_type = policy.consent_type.clone();
        self.policies.write().await.insert(consent_type, policy);
    }

    pub async fn get_policy(&self, consent_type: &ConsentType) -> Option<ConsentPolicy> {
        self.policies.read().await.get(consent_type).cloned()
    }

    pub async fn get_all_policies(&self) -> Vec<ConsentPolicy> {
        self.policies.read().await.values().cloned().collect()
    }

    pub async fn grant_consent(
        &self,
        user_id: &str,
        consent_type: ConsentType,
        version: String,
        source: ConsentSource,
    ) -> Result<UserConsent, ConsentError> {
        let consent = UserConsent::new(user_id.to_string(), consent_type.clone(), version)
            .grant(source.clone());

        let id = consent.id.clone();
        
        self.consents.write().await.insert(id.clone(), consent.clone());

        info!(
            user_id = %user_id,
            consent_type = %consent_type.as_str(),
            source = ?source,
            "Consent granted"
        );

        Ok(consent)
    }

    pub async fn revoke_consent(
        &self,
        user_id: &str,
        consent_type: &ConsentType,
    ) -> Result<(), ConsentError> {
        let id = ConsentId::new(user_id.to_string(), consent_type.clone());
        
        let mut consents = self.consents.write().await;
        
        if let Some(consent) = consents.get_mut(&id) {
            consent.revoke();
            
            info!(
                user_id = %user_id,
                consent_type = %consent_type.as_str(),
                "Consent revoked"
            );
            
            Ok(())
        } else {
            Err(ConsentError::NotFound)
        }
    }

    pub async fn get_consent(
        &self,
        user_id: &str,
        consent_type: &ConsentType,
    ) -> Option<UserConsent> {
        let id = ConsentId::new(user_id.to_string(), consent_type.clone());
        self.consents.read().await.get(&id).cloned()
    }

    pub async fn get_user_consents(&self, user_id: &str) -> Vec<UserConsent> {
        self.consents
            .read()
            .await
            .values()
            .filter(|c| c.id.user_id == user_id)
            .cloned()
            .collect()
    }

    pub async fn has_consent(&self, user_id: &str, consent_type: &ConsentType) -> bool {
        self.consents
            .read()
            .await
            .values()
            .any(|c| c.id.user_id == user_id 
                && c.id.consent_type == *consent_type 
                && c.is_active())
    }

    pub async fn has_required_consents(&self, user_id: &str) -> bool {
        let policies = self.policies.read().await;
        let consents = self.consents.read().await;

        for (_, policy) in policies.iter() {
            if policy.required {
                let id = ConsentId::new(user_id.to_string(), policy.consent_type.clone());
                
                let has_consent = consents
                    .get(&id)
                    .map(|c| c.is_active() && c.version == policy.current_version)
                    .unwrap_or(false);

                if !has_consent {
                    return false;
                }
            }
        }

        true
    }

    pub async fn get_missing_required_consents(&self, user_id: &str) -> Vec<ConsentType> {
        let policies = self.policies.read().await;
        let consents = self.consents.read().await;

        let mut missing = Vec::new();

        for (_, policy) in policies.iter() {
            if policy.required {
                let id = ConsentId::new(user_id.to_string(), policy.consent_type.clone());
                
                let has_consent = consents
                    .get(&id)
                    .map(|c| c.is_active() && c.version == policy.current_version)
                    .unwrap_or(false);

                if !has_consent {
                    missing.push(policy.consent_type.clone());
                }
            }
        }

        missing
    }

    pub async fn get_stats(&self) -> ConsentStats {
        let consents = self.consents.read().await;

        let mut stats = ConsentStats {
            total_consents: consents.len() as u64,
            ..Default::default()
        };

        for consent in consents.values() {
            if consent.is_active() {
                stats.active_consents += 1;
            } else if consent.revoked_at.is_some() {
                stats.revoked_consents += 1;
            } else {
                stats.pending_consents += 1;
            }
        }

        stats
    }

    pub async fn get_users_with_consent(&self, consent_type: &ConsentType) -> Vec<String> {
        self.consents
            .read()
            .await
            .values()
            .filter(|c| c.id.consent_type == *consent_type && c.is_active())
            .map(|c| c.id.user_id.clone())
            .collect()
    }
}

impl Default for ConsentService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConsentError {
    #[error("Consent not found")]
    NotFound,
    #[error("Consent already granted")]
    AlreadyGranted,
    #[error("Consent required")]
    Required,
    #[error("Invalid version")]
    InvalidVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grant_consent() {
        let service = ConsentService::new();

        let consent = service
            .grant_consent(
                "@user:example.com",
                ConsentType::TermsOfService,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        assert!(consent.is_active());
        assert!(consent.granted);
    }

    #[tokio::test]
    async fn test_revoke_consent() {
        let service = ConsentService::new();

        service
            .grant_consent(
                "@user:example.com",
                ConsentType::PrivacyPolicy,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        service
            .revoke_consent("@user:example.com", &ConsentType::PrivacyPolicy)
            .await
            .unwrap();

        let consent = service
            .get_consent("@user:example.com", &ConsentType::PrivacyPolicy)
            .await
            .unwrap();

        assert!(!consent.is_active());
        assert!(consent.revoked_at.is_some());
    }

    #[tokio::test]
    async fn test_has_consent() {
        let service = ConsentService::new();

        assert!(!service.has_consent("@user:example.com", &ConsentType::TermsOfService).await);

        service
            .grant_consent(
                "@user:example.com",
                ConsentType::TermsOfService,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        assert!(service.has_consent("@user:example.com", &ConsentType::TermsOfService).await);
    }

    #[tokio::test]
    async fn test_required_consents() {
        let service = ConsentService::new();

        service
            .register_policy(
                ConsentPolicy::new(ConsentType::TermsOfService, "1.0.0".to_string(), true)
                    .with_description("Terms of Service".to_string()),
            )
            .await;

        assert!(!service.has_required_consents("@user:example.com").await);

        let missing = service.get_missing_required_consents("@user:example.com").await;
        assert_eq!(missing.len(), 1);

        service
            .grant_consent(
                "@user:example.com",
                ConsentType::TermsOfService,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        assert!(service.has_required_consents("@user:example.com").await);
    }

    #[tokio::test]
    async fn test_get_user_consents() {
        let service = ConsentService::new();

        service
            .grant_consent(
                "@user:example.com",
                ConsentType::TermsOfService,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        service
            .grant_consent(
                "@user:example.com",
                ConsentType::PrivacyPolicy,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        let consents = service.get_user_consents("@user:example.com").await;
        assert_eq!(consents.len(), 2);
    }

    #[tokio::test]
    async fn test_get_users_with_consent() {
        let service = ConsentService::new();

        service
            .grant_consent(
                "@user1:example.com",
                ConsentType::EmailMarketing,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        service
            .grant_consent(
                "@user2:example.com",
                ConsentType::EmailMarketing,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        let users = service.get_users_with_consent(&ConsentType::EmailMarketing).await;
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let service = ConsentService::new();

        service
            .grant_consent(
                "@user1:example.com",
                ConsentType::TermsOfService,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        service
            .grant_consent(
                "@user2:example.com",
                ConsentType::TermsOfService,
                "1.0.0".to_string(),
                ConsentSource::Web,
            )
            .await
            .unwrap();

        service
            .revoke_consent("@user2:example.com", &ConsentType::TermsOfService)
            .await
            .unwrap();

        let stats = service.get_stats().await;
        assert_eq!(stats.total_consents, 2);
        assert_eq!(stats.active_consents, 1);
        assert_eq!(stats.revoked_consents, 1);
    }
}
