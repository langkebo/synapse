use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub room_id: String,
    pub sender: String,
    pub event_type: String,
    pub content: serde_json::Value,
    pub origin_server_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    pub room_id: String,
    pub room_name: Option<String>,
    pub room_members: Vec<String>,
    pub sender_display_name: Option<String>,
    pub sender_avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpamCheckResult {
    Allow,
    ShadowBan,
    Reject { reason: String },
    NotSpam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventAction {
    Allow,
    Reject { reason: String },
    Modify { new_content: serde_json::Value },
}

#[async_trait]
pub trait SpamChecker: Send + Sync {
    fn name(&self) -> &str;
    
    async fn check_event_for_spam(
        &self,
        event: &Event,
        context: &EventContext,
    ) -> Result<SpamCheckResult, ModuleError>;
}

#[async_trait]
pub trait ThirdPartyRules: Send + Sync {
    fn name(&self) -> &str;
    
    async fn check_event_allowed(
        &self,
        event: &Event,
        context: &EventContext,
    ) -> Result<EventAction, ModuleError>;
    
    async fn on_new_event(
        &self,
        event: &Event,
        context: &EventContext,
    ) -> Result<(), ModuleError>;
}

#[async_trait]
pub trait PresenceRouter: Send + Sync {
    fn name(&self) -> &str;
    
    async fn get_users_for_presence(
        &self,
        user_ids: &[String],
    ) -> Result<Vec<String>, ModuleError>;
}

#[async_trait]
pub trait AccountValidity: Send + Sync {
    fn name(&self) -> &str;
    
    async fn is_user_valid(&self, user_id: &str) -> Result<bool, ModuleError>;
    
    async fn on_user_registration(&self, user_id: &str) -> Result<(), ModuleError>;
    
    async fn on_user_login(&self, user_id: &str) -> Result<(), ModuleError>;
}

#[async_trait]
pub trait PasswordAuthProvider: Send + Sync {
    fn name(&self) -> &str;
    
    async fn check_password(
        &self,
        user_id: &str,
        password: &str,
    ) -> Result<bool, ModuleError>;
    
    async fn on_password_change(
        &self,
        user_id: &str,
        new_password: &str,
    ) -> Result<(), ModuleError>;
}

#[async_trait]
pub trait MediaRepositoryCallback: Send + Sync {
    fn name(&self) -> &str;
    
    async fn on_media_upload(
        &self,
        media_id: &str,
        user_id: &str,
        content_type: &str,
        size: u64,
    ) -> Result<(), ModuleError>;
    
    async fn on_media_download(
        &self,
        media_id: &str,
        user_id: &str,
    ) -> Result<(), ModuleError>;
    
    async fn on_media_delete(
        &self,
        media_id: &str,
    ) -> Result<(), ModuleError>;
}

#[async_trait]
pub trait RateLimitCallback: Send + Sync {
    fn name(&self) -> &str;
    
    async fn check_rate_limit(
        &self,
        key: &str,
        action: &str,
    ) -> Result<bool, ModuleError>;
    
    async fn on_rate_limit_exceeded(
        &self,
        key: &str,
        action: &str,
    ) -> Result<(), ModuleError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("Module error: {0}")]
    Error(String),
    #[error("Module not found: {0}")]
    NotFound(String),
    #[error("Module initialization failed: {0}")]
    InitFailed(String),
}

pub struct ModuleRegistry {
    spam_checkers: RwLock<Vec<Arc<dyn SpamChecker>>>,
    third_party_rules: RwLock<Vec<Arc<dyn ThirdPartyRules>>>,
    presence_routers: RwLock<Vec<Arc<dyn PresenceRouter>>>,
    account_validity: RwLock<Vec<Arc<dyn AccountValidity>>>,
    password_providers: RwLock<Vec<Arc<dyn PasswordAuthProvider>>>,
    media_callbacks: RwLock<Vec<Arc<dyn MediaRepositoryCallback>>>,
    rate_limit_callbacks: RwLock<Vec<Arc<dyn RateLimitCallback>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            spam_checkers: RwLock::new(Vec::new()),
            third_party_rules: RwLock::new(Vec::new()),
            presence_routers: RwLock::new(Vec::new()),
            account_validity: RwLock::new(Vec::new()),
            password_providers: RwLock::new(Vec::new()),
            media_callbacks: RwLock::new(Vec::new()),
            rate_limit_callbacks: RwLock::new(Vec::new()),
        }
    }

    pub async fn register_spam_checker(&self, checker: Arc<dyn SpamChecker>) {
        info!(name = %checker.name(), "Registering spam checker");
        self.spam_checkers.write().await.push(checker);
    }

    pub async fn register_third_party_rules(&self, rules: Arc<dyn ThirdPartyRules>) {
        info!(name = %rules.name(), "Registering third party rules");
        self.third_party_rules.write().await.push(rules);
    }

    pub async fn register_presence_router(&self, router: Arc<dyn PresenceRouter>) {
        info!(name = %router.name(), "Registering presence router");
        self.presence_routers.write().await.push(router);
    }

    pub async fn register_account_validity(&self, validity: Arc<dyn AccountValidity>) {
        info!(name = %validity.name(), "Registering account validity module");
        self.account_validity.write().await.push(validity);
    }

    pub async fn register_password_provider(&self, provider: Arc<dyn PasswordAuthProvider>) {
        info!(name = %provider.name(), "Registering password provider");
        self.password_providers.write().await.push(provider);
    }

    pub async fn register_media_callback(&self, callback: Arc<dyn MediaRepositoryCallback>) {
        info!(name = %callback.name(), "Registering media callback");
        self.media_callbacks.write().await.push(callback);
    }

    pub async fn register_rate_limit_callback(&self, callback: Arc<dyn RateLimitCallback>) {
        info!(name = %callback.name(), "Registering rate limit callback");
        self.rate_limit_callbacks.write().await.push(callback);
    }

    pub async fn check_spam(
        &self,
        event: &Event,
        context: &EventContext,
    ) -> Result<SpamCheckResult, ModuleError> {
        let checkers = self.spam_checkers.read().await;
        
        for checker in checkers.iter() {
            match checker.check_event_for_spam(event, context).await {
                Ok(SpamCheckResult::Allow) => continue,
                Ok(result) => {
                    debug!(
                        checker = %checker.name(),
                        result = ?result,
                        "Spam check result"
                    );
                    return Ok(result);
                }
                Err(e) => {
                    error!(checker = %checker.name(), error = %e, "Spam check failed");
                }
            }
        }
        
        Ok(SpamCheckResult::NotSpam)
    }

    pub async fn check_event_allowed(
        &self,
        event: &Event,
        context: &EventContext,
    ) -> Result<EventAction, ModuleError> {
        let rules = self.third_party_rules.read().await;
        
        for rule in rules.iter() {
            match rule.check_event_allowed(event, context).await {
                Ok(EventAction::Allow) => continue,
                Ok(action) => {
                    debug!(
                        rule = %rule.name(),
                        action = ?action,
                        "Third party rule action"
                    );
                    return Ok(action);
                }
                Err(e) => {
                    error!(rule = %rule.name(), error = %e, "Rule check failed");
                }
            }
        }
        
        Ok(EventAction::Allow)
    }

    pub async fn on_new_event(
        &self,
        event: &Event,
        context: &EventContext,
    ) -> Result<(), ModuleError> {
        let rules = self.third_party_rules.read().await;
        
        for rule in rules.iter() {
            if let Err(e) = rule.on_new_event(event, context).await {
                warn!(rule = %rule.name(), error = %e, "on_new_event callback failed");
            }
        }
        
        Ok(())
    }

    pub async fn is_user_valid(&self, user_id: &str) -> Result<bool, ModuleError> {
        let validity_modules = self.account_validity.read().await;
        
        for validity in validity_modules.iter() {
            match validity.is_user_valid(user_id).await {
                Ok(false) => return Ok(false),
                Ok(true) => continue,
                Err(e) => {
                    warn!(module = %validity.name(), error = %e, "Account validity check failed");
                }
            }
        }
        
        Ok(true)
    }

    pub async fn check_password(
        &self,
        user_id: &str,
        password: &str,
    ) -> Result<bool, ModuleError> {
        let providers = self.password_providers.read().await;
        
        if providers.is_empty() {
            return Ok(false);
        }
        
        for provider in providers.iter() {
            match provider.check_password(user_id, password).await {
                Ok(true) => return Ok(true),
                Ok(false) => continue,
                Err(e) => {
                    warn!(provider = %provider.name(), error = %e, "Password check failed");
                }
            }
        }
        
        Ok(false)
    }

    pub async fn on_media_upload(
        &self,
        media_id: &str,
        user_id: &str,
        content_type: &str,
        size: u64,
    ) -> Result<(), ModuleError> {
        let callbacks = self.media_callbacks.read().await;
        
        for callback in callbacks.iter() {
            if let Err(e) = callback.on_media_upload(media_id, user_id, content_type, size).await {
                warn!(callback = %callback.name(), error = %e, "Media upload callback failed");
            }
        }
        
        Ok(())
    }

    pub async fn check_rate_limit(
        &self,
        key: &str,
        action: &str,
    ) -> Result<bool, ModuleError> {
        let callbacks = self.rate_limit_callbacks.read().await;
        
        for callback in callbacks.iter() {
            match callback.check_rate_limit(key, action).await {
                Ok(false) => return Ok(false),
                Ok(true) => continue,
                Err(e) => {
                    warn!(callback = %callback.name(), error = %e, "Rate limit check failed");
                }
            }
        }
        
        Ok(true)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SimpleSpamChecker {
    blocked_words: Vec<String>,
}

impl SimpleSpamChecker {
    pub fn new(blocked_words: Vec<String>) -> Self {
        Self { blocked_words }
    }
}

#[async_trait]
impl SpamChecker for SimpleSpamChecker {
    fn name(&self) -> &str {
        "simple_spam_checker"
    }
    
    async fn check_event_for_spam(
        &self,
        event: &Event,
        _context: &EventContext,
    ) -> Result<SpamCheckResult, ModuleError> {
        if event.event_type != "m.room.message" {
            return Ok(SpamCheckResult::NotSpam);
        }
        
        if let Some(body) = event.content.get("body").and_then(|b| b.as_str()) {
            let body_lower = body.to_lowercase();
            for word in &self.blocked_words {
                if body_lower.contains(&word.to_lowercase()) {
                    return Ok(SpamCheckResult::Reject {
                        reason: format!("Contains blocked word: {}", word),
                    });
                }
            }
        }
        
        Ok(SpamCheckResult::NotSpam)
    }
}

pub struct SimpleThirdPartyRules {
    max_message_length: usize,
}

impl SimpleThirdPartyRules {
    pub fn new(max_message_length: usize) -> Self {
        Self { max_message_length }
    }
}

#[async_trait]
impl ThirdPartyRules for SimpleThirdPartyRules {
    fn name(&self) -> &str {
        "simple_third_party_rules"
    }
    
    async fn check_event_allowed(
        &self,
        event: &Event,
        _context: &EventContext,
    ) -> Result<EventAction, ModuleError> {
        if event.event_type == "m.room.message" {
            if let Some(body) = event.content.get("body").and_then(|b| b.as_str()) {
                if body.len() > self.max_message_length {
                    return Ok(EventAction::Reject {
                        reason: format!("Message too long: {} > {}", body.len(), self.max_message_length),
                    });
                }
            }
        }
        
        Ok(EventAction::Allow)
    }
    
    async fn on_new_event(
        &self,
        _event: &Event,
        _context: &EventContext,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_module_registry_spam_check() {
        let registry = Arc::new(ModuleRegistry::new());
        
        let checker = Arc::new(SimpleSpamChecker::new(vec!["spam".to_string()]));
        registry.register_spam_checker(checker).await;
        
        let event = Event {
            event_id: "$event1".to_string(),
            room_id: "!room1:example.com".to_string(),
            sender: "@user:example.com".to_string(),
            event_type: "m.room.message".to_string(),
            content: serde_json::json!({"body": "This is spam content"}),
            origin_server_ts: 0,
        };
        
        let context = EventContext {
            room_id: "!room1:example.com".to_string(),
            room_name: None,
            room_members: vec![],
            sender_display_name: None,
            sender_avatar_url: None,
        };
        
        let result = registry.check_spam(&event, &context).await.unwrap();
        assert!(matches!(result, SpamCheckResult::Reject { .. }));
    }

    #[tokio::test]
    async fn test_module_registry_third_party_rules() {
        let registry = Arc::new(ModuleRegistry::new());
        
        let rules = Arc::new(SimpleThirdPartyRules::new(100));
        registry.register_third_party_rules(rules).await;
        
        let event = Event {
            event_id: "$event1".to_string(),
            room_id: "!room1:example.com".to_string(),
            sender: "@user:example.com".to_string(),
            event_type: "m.room.message".to_string(),
            content: serde_json::json!({"body": "x".repeat(150)}),
            origin_server_ts: 0,
        };
        
        let context = EventContext {
            room_id: "!room1:example.com".to_string(),
            room_name: None,
            room_members: vec![],
            sender_display_name: None,
            sender_avatar_url: None,
        };
        
        let action = registry.check_event_allowed(&event, &context).await.unwrap();
        assert!(matches!(action, EventAction::Reject { .. }));
    }

    #[tokio::test]
    async fn test_module_registry_allow_event() {
        let registry = Arc::new(ModuleRegistry::new());
        
        let checker = Arc::new(SimpleSpamChecker::new(vec!["spam".to_string()]));
        registry.register_spam_checker(checker).await;
        
        let event = Event {
            event_id: "$event1".to_string(),
            room_id: "!room1:example.com".to_string(),
            sender: "@user:example.com".to_string(),
            event_type: "m.room.message".to_string(),
            content: serde_json::json!({"body": "Hello world"}),
            origin_server_ts: 0,
        };
        
        let context = EventContext {
            room_id: "!room1:example.com".to_string(),
            room_name: None,
            room_members: vec![],
            sender_display_name: None,
            sender_avatar_url: None,
        };
        
        let result = registry.check_spam(&event, &context).await.unwrap();
        assert!(matches!(result, SpamCheckResult::NotSpam));
    }
}
