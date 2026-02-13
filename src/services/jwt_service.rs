use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// JWT configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Secret key for signing tokens (should be at least 32 characters)
    pub secret: String,
    /// Token issuer identifier
    pub issuer: String,
    /// Token audience identifier
    pub audience: String,
    /// Access token expiry time in seconds
    pub access_token_expiry_seconds: u64,
    /// Refresh token expiry time in seconds
    pub refresh_token_expiry_seconds: u64,
    /// JWT signing algorithm
    pub algorithm: JwtAlgorithm,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                tracing::warn!("JWT_SECRET not set, using insecure default. Set JWT_SECRET environment variable in production!");
                "insecure-default-key-change-in-production".to_string()
            }),
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "synapse-rust".to_string()),
            audience: std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "synapse-rust-users".to_string()),
            access_token_expiry_seconds: 3600,
            refresh_token_expiry_seconds: 604800,
            algorithm: JwtAlgorithm::HS256,
        }
    }
}

impl JwtConfig {
    /// Validates the JWT configuration
    pub fn validate(&self) -> Result<(), JwtError> {
        if self.secret.len() < 16 {
            return Err(JwtError::InsecureSecret);
        }
        if self.issuer.is_empty() {
            return Err(JwtError::InvalidIssuer);
        }
        if self.audience.is_empty() {
            return Err(JwtError::InvalidAudience);
        }
        Ok(())
    }
}

/// Supported JWT signing algorithms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
}

/// JWT claims payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
    /// Token issuer
    pub iss: String,
    /// Token audience
    pub aud: String,
    /// JWT ID (unique identifier)
    pub jti: String,
    /// Optional device ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

impl JwtClaims {
    pub fn new(
        user_id: String,
        issuer: String,
        audience: String,
        expiry_seconds: u64,
    ) -> Self {
        let now = Utc::now().timestamp();
        
        Self {
            sub: user_id,
            exp: now + expiry_seconds as i64,
            iat: now,
            iss: issuer,
            aud: audience,
            jti: uuid::Uuid::new_v4().to_string(),
            device_id: None,
        }
    }

    pub fn with_device(mut self, device_id: String) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    pub fn user_id(&self) -> &str {
        &self.sub
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtToken {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtRefreshToken {
    pub jti: String,
    pub user_id: String,
    pub device_id: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
}

impl JwtRefreshToken {
    pub fn new(user_id: String, device_id: Option<String>, expiry_seconds: u64) -> Self {
        let now = Utc::now().timestamp();
        
        Self {
            jti: uuid::Uuid::new_v4().to_string(),
            user_id,
            device_id,
            created_at: now,
            expires_at: now + expiry_seconds as i64,
            revoked: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired()
    }
}

pub struct JwtService {
    config: JwtConfig,
    refresh_tokens: Arc<RwLock<HashMap<String, JwtRefreshToken>>>,
}

impl JwtService {
    pub fn new(config: JwtConfig) -> Self {
        Self {
            config,
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn generate_token(&self, user_id: &str, device_id: Option<&str>) -> JwtToken {
        let mut claims = JwtClaims::new(
            user_id.to_string(),
            self.config.issuer.clone(),
            self.config.audience.clone(),
            self.config.access_token_expiry_seconds,
        );

        if let Some(device) = device_id {
            claims = claims.with_device(device.to_string());
        }

        let access_token = self.encode_claims(&claims);

        let _refresh_token = JwtRefreshToken::new(
            user_id.to_string(),
            device_id.map(|d| d.to_string()),
            self.config.refresh_token_expiry_seconds,
        );

        let refresh_token_str = self.generate_refresh_token_string();

        info!(
            user_id = %user_id,
            device_id = ?device_id,
            "JWT token generated"
        );

        JwtToken {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_expiry_seconds,
        }
    }

    pub async fn store_refresh_token(&self, token_str: String, token: JwtRefreshToken) {
        self.refresh_tokens.write().await.insert(token_str, token);
    }

    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let claims = self.decode_token(token)?;

        if claims.iss != self.config.issuer {
            return Err(JwtError::InvalidIssuer);
        }

        if claims.aud != self.config.audience {
            return Err(JwtError::InvalidAudience);
        }

        if claims.is_expired() {
            return Err(JwtError::TokenExpired);
        }

        Ok(claims)
    }

    pub async fn refresh_access_token(
        &self,
        refresh_token_str: &str,
    ) -> Result<JwtToken, JwtError> {
        let mut tokens = self.refresh_tokens.write().await;
        let refresh_token = tokens
            .get_mut(refresh_token_str)
            .ok_or(JwtError::InvalidRefreshToken)?;

        if !refresh_token.is_valid() {
            tokens.remove(refresh_token_str);
            return Err(JwtError::RefreshTokenExpired);
        }

        let user_id = refresh_token.user_id.clone();
        let device_id = refresh_token.device_id.clone();

        tokens.remove(refresh_token_str);

        drop(tokens);

        Ok(self.generate_token(&user_id, device_id.as_deref()))
    }

    pub async fn revoke_token(&self, refresh_token_str: &str) -> bool {
        let mut tokens = self.refresh_tokens.write().await;
        if let Some(token) = tokens.get_mut(refresh_token_str) {
            token.revoked = true;
            info!(user_id = %token.user_id, "Refresh token revoked");
            true
        } else {
            false
        }
    }

    pub async fn revoke_all_user_tokens(&self, user_id: &str) -> usize {
        let mut tokens = self.refresh_tokens.write().await;
        let before = tokens.len();
        
        tokens.retain(|_, t| t.user_id != user_id);
        
        let removed = before - tokens.len();
        if removed > 0 {
            info!(user_id = %user_id, count = removed, "All user tokens revoked");
        }
        removed
    }

    fn encode_claims(&self, claims: &JwtClaims) -> String {
        let header = self.create_header();
        let payload = serde_json::to_string(claims).unwrap_or_default();
        
        let header_b64 = base64_encode(header.as_bytes());
        let payload_b64 = base64_encode(payload.as_bytes());
        
        let signature = self.sign(&format!("{}.{}", header_b64, payload_b64));
        
        format!("{}.{}.{}", header_b64, payload_b64, signature)
    }

    fn decode_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let parts: Vec<&str> = token.split('.').collect();
        
        if parts.len() != 3 {
            return Err(JwtError::InvalidTokenFormat);
        }

        let _header = base64_decode_to_string(parts[0])?;
        let payload = base64_decode_to_string(parts[1])?;
        let signature = parts[2];

        let expected_signature = self.sign(&format!("{}.{}", parts[0], parts[1]));
        
        if signature != expected_signature {
            return Err(JwtError::InvalidSignature);
        }

        let claims: JwtClaims = serde_json::from_str(&payload)
            .map_err(|_| JwtError::InvalidClaims)?;

        Ok(claims)
    }

    fn create_header(&self) -> String {
        let alg = match self.config.algorithm {
            JwtAlgorithm::HS256 => "HS256",
            JwtAlgorithm::HS384 => "HS384",
            JwtAlgorithm::HS512 => "HS512",
            JwtAlgorithm::RS256 => "RS256",
            JwtAlgorithm::RS384 => "RS384",
            JwtAlgorithm::RS512 => "RS512",
        };
        
        format!(r#"{{"alg":"{}","typ":"JWT"}}"#, alg)
    }

    fn sign(&self, data: &str) -> String {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.update(self.config.secret.as_bytes());
        
        let result = hasher.finalize();
        base64_encode(format!("{:x}", result).as_bytes())
    }

    fn generate_refresh_token_string(&self) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        
        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub async fn cleanup_expired_tokens(&self) -> usize {
        let mut tokens = self.refresh_tokens.write().await;
        let before = tokens.len();
        
        tokens.retain(|_, t| t.is_valid());
        
        let removed = before - tokens.len();
        if removed > 0 {
            debug!(count = removed, "Expired refresh tokens cleaned up");
        }
        removed
    }
}

impl Default for JwtService {
    fn default() -> Self {
        Self::new(JwtConfig::default())
    }
}

fn base64_encode(input: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD.encode(input)
}

fn base64_decode_to_string(input: &str) -> Result<String, JwtError> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD
        .decode(input)
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .map_err(|_| JwtError::InvalidBase64)
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Invalid token format")]
    InvalidTokenFormat,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid claims")]
    InvalidClaims,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid issuer")]
    InvalidIssuer,
    #[error("Invalid audience")]
    InvalidAudience,
    #[error("Invalid refresh token")]
    InvalidRefreshToken,
    #[error("Refresh token expired")]
    RefreshTokenExpired,
    #[error("Invalid base64 encoding")]
    InvalidBase64,
    #[error("Insecure secret key: must be at least 16 characters")]
    InsecureSecret,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_creation() {
        let claims = JwtClaims::new(
            "@user:example.com".to_string(),
            "synapse".to_string(),
            "users".to_string(),
            3600,
        );

        assert_eq!(claims.sub, "@user:example.com");
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_claims_with_device() {
        let claims = JwtClaims::new(
            "@user:example.com".to_string(),
            "synapse".to_string(),
            "users".to_string(),
            3600,
        )
        .with_device("DEVICE123".to_string());

        assert_eq!(claims.device_id, Some("DEVICE123".to_string()));
    }

    #[tokio::test]
    async fn test_generate_token() {
        let service = JwtService::default();

        let token = service.generate_token("@user:example.com", Some("DEVICE123"));

        assert!(!token.access_token.is_empty());
        assert!(!token.refresh_token.is_empty());
        assert_eq!(token.token_type, "Bearer");
    }

    #[tokio::test]
    async fn test_validate_token() {
        let service = JwtService::default();

        let token = service.generate_token("@user:example.com", None);

        let claims = service.validate_token(&token.access_token).unwrap();

        assert_eq!(claims.sub, "@user:example.com");
    }

    #[tokio::test]
    async fn test_validate_invalid_token() {
        let service = JwtService::default();

        let result = service.validate_token("invalid.token.here");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let service = JwtService::default();

        let token = service.generate_token("@user:example.com", None);
        
        let refresh = JwtRefreshToken::new(
            "@user:example.com".to_string(),
            None,
            3600,
        );
        service.store_refresh_token(token.refresh_token.clone(), refresh).await;

        let new_token = service.refresh_access_token(&token.refresh_token).await;

        assert!(new_token.is_ok());
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let service = JwtService::default();

        let token = service.generate_token("@user:example.com", None);
        
        let refresh = JwtRefreshToken::new(
            "@user:example.com".to_string(),
            None,
            3600,
        );
        service.store_refresh_token(token.refresh_token.clone(), refresh).await;

        let revoked = service.revoke_token(&token.refresh_token).await;
        assert!(revoked);

        let result = service.refresh_access_token(&token.refresh_token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_all_user_tokens() {
        let service = JwtService::default();

        let token1 = service.generate_token("@user1:example.com", None);
        let token2 = service.generate_token("@user1:example.com", None);
        let token3 = service.generate_token("@user2:example.com", None);

        let refresh1 = JwtRefreshToken::new("@user1:example.com".to_string(), None, 3600);
        let refresh2 = JwtRefreshToken::new("@user1:example.com".to_string(), None, 3600);
        let refresh3 = JwtRefreshToken::new("@user2:example.com".to_string(), None, 3600);
        
        service.store_refresh_token(token1.refresh_token, refresh1).await;
        service.store_refresh_token(token2.refresh_token, refresh2).await;
        service.store_refresh_token(token3.refresh_token, refresh3).await;

        let count = service.revoke_all_user_tokens("@user1:example.com").await;
        assert_eq!(count, 2);
    }
}
