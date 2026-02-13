use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationToken {
    pub token: String,
    pub uses_allowed: Option<u32>,
    pub pending: u32,
    pub completed: u32,
    pub expiry_time: Option<i64>,
    pub created_at: i64,
    pub created_by: String,
}

impl RegistrationToken {
    pub fn new(
        token: String,
        uses_allowed: Option<u32>,
        expiry_time: Option<i64>,
        created_by: String,
    ) -> Self {
        Self {
            token,
            uses_allowed,
            pending: 0,
            completed: 0,
            expiry_time,
            created_at: Utc::now().timestamp_millis(),
            created_by,
        }
    }

    pub fn is_valid(&self) -> bool {
        if let Some(expiry) = self.expiry_time {
            if Utc::now().timestamp_millis() > expiry {
                return false;
            }
        }

        if let Some(max_uses) = self.uses_allowed {
            if self.completed >= max_uses {
                return false;
            }
        }

        true
    }

    pub fn remaining_uses(&self) -> Option<u32> {
        self.uses_allowed.map(|max| max.saturating_sub(self.completed))
    }

    pub fn can_use(&self) -> bool {
        self.is_valid() && self.remaining_uses().map(|r| r > 0).unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub token: String,
    pub user_id: String,
    pub used_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total_tokens: usize,
    pub active_tokens: usize,
    pub expired_tokens: usize,
    pub exhausted_tokens: usize,
    pub total_registrations: u32,
}

pub struct RegistrationTokenService {
    tokens: Arc<RwLock<HashMap<String, RegistrationToken>>>,
    usage: Arc<RwLock<Vec<TokenUsage>>>,
}

impl RegistrationTokenService {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_token(
        &self,
        length: usize,
        uses_allowed: Option<u32>,
        expiry_time: Option<i64>,
        created_by: &str,
    ) -> Result<RegistrationToken, TokenError> {
        let token = Self::generate_token(length);
        
        let reg_token = RegistrationToken::new(
            token.clone(),
            uses_allowed,
            expiry_time,
            created_by.to_string(),
        );

        self.tokens.write().await.insert(token.clone(), reg_token.clone());

        info!(
            token = %token,
            uses_allowed = ?uses_allowed,
            expiry_time = ?expiry_time,
            created_by = %created_by,
            "Registration token created"
        );

        Ok(reg_token)
    }

    fn generate_token(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
        let mut rng = rand::thread_rng();
        
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub async fn validate_token(&self, token: &str) -> Result<RegistrationToken, TokenError> {
        let tokens = self.tokens.read().await;
        
        let reg_token = tokens
            .get(token)
            .ok_or(TokenError::InvalidToken)?
            .clone();

        if !reg_token.is_valid() {
            if let Some(expiry) = reg_token.expiry_time {
                if Utc::now().timestamp_millis() > expiry {
                    return Err(TokenError::TokenExpired);
                }
            }

            if let Some(max_uses) = reg_token.uses_allowed {
                if reg_token.completed >= max_uses {
                    return Err(TokenError::TokenExhausted);
                }
            }

            return Err(TokenError::InvalidToken);
        }

        Ok(reg_token)
    }

    pub async fn use_token(&self, token: &str, user_id: &str) -> Result<(), TokenError> {
        let mut tokens = self.tokens.write().await;
        
        let reg_token = tokens
            .get_mut(token)
            .ok_or(TokenError::InvalidToken)?;

        if !reg_token.can_use() {
            if let Some(expiry) = reg_token.expiry_time {
                if Utc::now().timestamp_millis() > expiry {
                    return Err(TokenError::TokenExpired);
                }
            }

            return Err(TokenError::TokenExhausted);
        }

        reg_token.pending += 1;
        reg_token.completed += 1;

        let usage = TokenUsage {
            token: token.to_string(),
            user_id: user_id.to_string(),
            used_at: Utc::now().timestamp_millis(),
        };

        self.usage.write().await.push(usage);

        info!(
            token = %token,
            user_id = %user_id,
            remaining = ?reg_token.remaining_uses(),
            "Registration token used"
        );

        Ok(())
    }

    pub async fn delete_token(&self, token: &str) -> Result<(), TokenError> {
        if self.tokens.write().await.remove(token).is_some() {
            info!(token = %token, "Registration token deleted");
            Ok(())
        } else {
            Err(TokenError::InvalidToken)
        }
    }

    pub async fn get_token(&self, token: &str) -> Option<RegistrationToken> {
        self.tokens.read().await.get(token).cloned()
    }

    pub async fn list_tokens(&self) -> Vec<RegistrationToken> {
        self.tokens.read().await.values().cloned().collect()
    }

    pub async fn get_stats(&self) -> TokenStats {
        let tokens = self.tokens.read().await;
        let usage = self.usage.read().await;

        let now = Utc::now().timestamp_millis();
        
        let active = tokens.values().filter(|t| t.is_valid()).count();
        let expired = tokens.values().filter(|t| {
            t.expiry_time.map(|exp| exp < now).unwrap_or(false)
        }).count();
        let exhausted = tokens.values().filter(|t| {
            t.uses_allowed.map(|max| t.completed >= max).unwrap_or(false)
        }).count();

        TokenStats {
            total_tokens: tokens.len(),
            active_tokens: active,
            expired_tokens: expired,
            exhausted_tokens: exhausted,
            total_registrations: usage.len() as u32,
        }
    }

    pub async fn cleanup_expired(&self) -> usize {
        let now = Utc::now().timestamp_millis();
        let mut tokens = self.tokens.write().await;

        let expired: Vec<String> = tokens
            .iter()
            .filter(|(_, t)| {
                t.expiry_time.map(|exp| exp < now).unwrap_or(false)
            })
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired.len();
        for token in expired {
            tokens.remove(&token);
            info!(token = %token, "Expired token removed");
        }

        count
    }

    pub async fn get_usage_for_token(&self, token: &str) -> Vec<TokenUsage> {
        self.usage
            .read()
            .await
            .iter()
            .filter(|u| u.token == token)
            .cloned()
            .collect()
    }
}

impl Default for RegistrationTokenService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token has expired")]
    TokenExpired,
    #[error("Token has been exhausted")]
    TokenExhausted,
    #[error("Token already used")]
    TokenAlreadyUsed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_token() {
        let service = RegistrationTokenService::new();
        
        let token = service.create_token(16, Some(5), None, "@admin:example.com").await.unwrap();
        
        assert_eq!(token.token.len(), 16);
        assert_eq!(token.uses_allowed, Some(5));
        assert!(token.is_valid());
    }

    #[tokio::test]
    async fn test_validate_token() {
        let service = RegistrationTokenService::new();
        
        let token = service.create_token(16, None, None, "@admin:example.com").await.unwrap();
        
        let validated = service.validate_token(&token.token).await.unwrap();
        assert!(validated.is_valid());
    }

    #[tokio::test]
    async fn test_use_token() {
        let service = RegistrationTokenService::new();
        
        let token = service.create_token(16, Some(2), None, "@admin:example.com").await.unwrap();
        
        service.use_token(&token.token, "@user1:example.com").await.unwrap();
        service.use_token(&token.token, "@user2:example.com").await.unwrap();
        
        let result = service.use_token(&token.token, "@user3:example.com").await;
        assert!(matches!(result, Err(TokenError::TokenExhausted)));
    }

    #[tokio::test]
    async fn test_expired_token() {
        let service = RegistrationTokenService::new();
        
        let past_time = Utc::now().timestamp_millis() - 1000;
        let token = service.create_token(16, None, Some(past_time), "@admin:example.com").await.unwrap();
        
        let result = service.validate_token(&token.token).await;
        assert!(matches!(result, Err(TokenError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_delete_token() {
        let service = RegistrationTokenService::new();
        
        let token = service.create_token(16, None, None, "@admin:example.com").await.unwrap();
        
        service.delete_token(&token.token).await.unwrap();
        
        let result = service.validate_token(&token.token).await;
        assert!(matches!(result, Err(TokenError::InvalidToken)));
    }

    #[tokio::test]
    async fn test_stats() {
        let service = RegistrationTokenService::new();
        
        service.create_token(16, Some(1), None, "@admin:example.com").await.unwrap();
        service.create_token(16, None, None, "@admin:example.com").await.unwrap();
        
        let stats = service.get_stats().await;
        assert_eq!(stats.total_tokens, 2);
        assert_eq!(stats.active_tokens, 2);
    }
}
