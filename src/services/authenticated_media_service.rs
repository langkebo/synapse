use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticatedMediaType {
    Avatar,
    Profile,
    RoomAvatar,
    RoomBanner,
    Sticker,
    Custom(String),
}

impl AuthenticatedMediaType {
    pub fn as_str(&self) -> &str {
        match self {
            AuthenticatedMediaType::Avatar => "avatar",
            AuthenticatedMediaType::Profile => "profile",
            AuthenticatedMediaType::RoomAvatar => "room_avatar",
            AuthenticatedMediaType::RoomBanner => "room_banner",
            AuthenticatedMediaType::Sticker => "sticker",
            AuthenticatedMediaType::Custom(s) => s,
        }
    }

    pub fn parse_from_str(s: &str) -> Self {
        match s {
            "avatar" => AuthenticatedMediaType::Avatar,
            "profile" => AuthenticatedMediaType::Profile,
            "room_avatar" => AuthenticatedMediaType::RoomAvatar,
            "room_banner" => AuthenticatedMediaType::RoomBanner,
            "sticker" => AuthenticatedMediaType::Sticker,
            other => AuthenticatedMediaType::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedMediaConfig {
    pub enabled: bool,
    pub require_auth_for_all: bool,
    pub allowed_types: Vec<AuthenticatedMediaType>,
    pub max_file_size: u64,
    pub allowed_mime_types: Vec<String>,
    pub token_expiry_seconds: u64,
}

impl Default for AuthenticatedMediaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_auth_for_all: false,
            allowed_types: vec![
                AuthenticatedMediaType::Avatar,
                AuthenticatedMediaType::Profile,
                AuthenticatedMediaType::RoomAvatar,
            ],
            max_file_size: 10 * 1024 * 1024,
            allowed_mime_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
            ],
            token_expiry_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAccessToken {
    pub token: String,
    pub media_id: String,
    pub user_id: String,
    pub media_type: AuthenticatedMediaType,
    pub created_at: i64,
    pub expires_at: i64,
    pub download_count: u32,
    pub max_downloads: Option<u32>,
}

impl MediaAccessToken {
    pub fn new(
        media_id: String,
        user_id: String,
        media_type: AuthenticatedMediaType,
        expiry_seconds: u64,
    ) -> Self {
        let now = Utc::now().timestamp_millis();
        let token = Self::generate_token();

        Self {
            token,
            media_id,
            user_id,
            media_type,
            created_at: now,
            expires_at: now + (expiry_seconds * 1000) as i64,
            download_count: 0,
            max_downloads: None,
        }
    }

    fn generate_token() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        
        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        if self.is_expired() {
            return false;
        }

        if let Some(max) = self.max_downloads {
            if self.download_count >= max {
                return false;
            }
        }

        true
    }

    pub fn increment_download(&mut self) {
        self.download_count += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedMediaRecord {
    pub media_id: String,
    pub media_type: AuthenticatedMediaType,
    pub owner_id: String,
    pub content_type: String,
    pub size: u64,
    pub sha256: String,
    pub is_public: bool,
    pub created_at: i64,
    pub access_count: u64,
}

pub struct AuthenticatedMediaService {
    config: AuthenticatedMediaConfig,
    tokens: Arc<RwLock<HashMap<String, MediaAccessToken>>>,
    media_records: Arc<RwLock<HashMap<String, AuthenticatedMediaRecord>>>,
}

impl AuthenticatedMediaService {
    pub fn new(config: AuthenticatedMediaConfig) -> Self {
        Self {
            config,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            media_records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn register_media(
        &self,
        media_id: &str,
        media_type: AuthenticatedMediaType,
        owner_id: &str,
        content_type: &str,
        size: u64,
        sha256: &str,
        is_public: bool,
    ) -> Result<AuthenticatedMediaRecord, AuthMediaError> {
        if !self.config.allowed_types.iter().any(|t| t == &media_type) {
            return Err(AuthMediaError::MediaTypeNotAllowed);
        }

        if !self.config.allowed_mime_types.contains(&content_type.to_string()) {
            return Err(AuthMediaError::MimeTypeNotAllowed);
        }

        if size > self.config.max_file_size {
            return Err(AuthMediaError::FileTooLarge);
        }

        let record = AuthenticatedMediaRecord {
            media_id: media_id.to_string(),
            media_type,
            owner_id: owner_id.to_string(),
            content_type: content_type.to_string(),
            size,
            sha256: sha256.to_string(),
            is_public,
            created_at: Utc::now().timestamp_millis(),
            access_count: 0,
        };

        self.media_records.write().await.insert(media_id.to_string(), record.clone());

        info!(
            media_id = %media_id,
            media_type = %record.media_type.as_str(),
            owner = %owner_id,
            "Authenticated media registered"
        );

        Ok(record)
    }

    pub async fn generate_access_token(
        &self,
        media_id: &str,
        user_id: &str,
        media_type: AuthenticatedMediaType,
        max_downloads: Option<u32>,
    ) -> Result<MediaAccessToken, AuthMediaError> {
        let records = self.media_records.read().await;
        let record = records
            .get(media_id)
            .ok_or(AuthMediaError::MediaNotFound)?;

        if !record.is_public && record.owner_id != user_id {
            return Err(AuthMediaError::AccessDenied);
        }
        drop(records);

        let mut token = MediaAccessToken::new(
            media_id.to_string(),
            user_id.to_string(),
            media_type,
            self.config.token_expiry_seconds,
        );
        token.max_downloads = max_downloads;

        self.tokens.write().await.insert(token.token.clone(), token.clone());

        debug!(
            media_id = %media_id,
            user_id = %user_id,
            expires_in = self.config.token_expiry_seconds,
            "Access token generated"
        );

        Ok(token)
    }

    pub async fn validate_access(
        &self,
        token_str: &str,
        user_id: &str,
    ) -> Result<AuthenticatedMediaRecord, AuthMediaError> {
        let mut tokens = self.tokens.write().await;
        let token = tokens
            .get_mut(token_str)
            .ok_or(AuthMediaError::InvalidToken)?;

        if !token.is_valid() {
            tokens.remove(token_str);
            return Err(AuthMediaError::TokenExpired);
        }

        if token.user_id != user_id {
            return Err(AuthMediaError::AccessDenied);
        }

        let media_id = token.media_id.clone();
        token.increment_download();

        let records = self.media_records.read().await;
        let record = records
            .get(&media_id)
            .ok_or(AuthMediaError::MediaNotFound)?
            .clone();

        Ok(record)
    }

    pub async fn check_public_access(
        &self,
        media_id: &str,
    ) -> Result<AuthenticatedMediaRecord, AuthMediaError> {
        let records = self.media_records.read().await;
        let record = records
            .get(media_id)
            .ok_or(AuthMediaError::MediaNotFound)?;

        if !record.is_public && self.config.require_auth_for_all {
            return Err(AuthMediaError::AuthenticationRequired);
        }

        Ok(record.clone())
    }

    pub async fn revoke_token(&self, token_str: &str) -> bool {
        if self.tokens.write().await.remove(token_str).is_some() {
            info!(token = %token_str, "Access token revoked");
            true
        } else {
            false
        }
    }

    pub async fn revoke_all_tokens_for_media(&self, media_id: &str) -> usize {
        let mut tokens = self.tokens.write().await;
        let before = tokens.len();
        
        tokens.retain(|_, t| t.media_id != media_id);
        
        before - tokens.len()
    }

    pub async fn delete_media(&self, media_id: &str) -> Result<(), AuthMediaError> {
        self.media_records.write().await.remove(media_id);
        self.revoke_all_tokens_for_media(media_id).await;

        info!(media_id = %media_id, "Authenticated media deleted");

        Ok(())
    }

    pub async fn cleanup_expired_tokens(&self) -> usize {
        let mut tokens = self.tokens.write().await;
        let before = tokens.len();
        
        tokens.retain(|_, t| !t.is_expired());
        
        let removed = before - tokens.len();
        if removed > 0 {
            debug!(count = removed, "Expired tokens cleaned up");
        }

        removed
    }

    pub fn config(&self) -> &AuthenticatedMediaConfig {
        &self.config
    }
}

impl Default for AuthenticatedMediaService {
    fn default() -> Self {
        Self::new(AuthenticatedMediaConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthMediaError {
    #[error("Media not found")]
    MediaNotFound,
    #[error("Invalid access token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Access denied")]
    AccessDenied,
    #[error("Authentication required")]
    AuthenticationRequired,
    #[error("Media type not allowed")]
    MediaTypeNotAllowed,
    #[error("MIME type not allowed")]
    MimeTypeNotAllowed,
    #[error("File too large")]
    FileTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_media() {
        let service = AuthenticatedMediaService::default();

        let record = service
            .register_media(
                "media123",
                AuthenticatedMediaType::Avatar,
                "@user:example.com",
                "image/png",
                1024,
                "abc123",
                false,
            )
            .await
            .unwrap();

        assert_eq!(record.media_id, "media123");
        assert_eq!(record.media_type, AuthenticatedMediaType::Avatar);
    }

    #[tokio::test]
    async fn test_generate_access_token() {
        let service = AuthenticatedMediaService::default();

        service
            .register_media(
                "media123",
                AuthenticatedMediaType::Avatar,
                "@user:example.com",
                "image/png",
                1024,
                "abc123",
                false,
            )
            .await
            .unwrap();

        let token = service
            .generate_access_token(
                "media123",
                "@user:example.com",
                AuthenticatedMediaType::Avatar,
                None,
            )
            .await
            .unwrap();

        assert!(!token.token.is_empty());
        assert!(token.is_valid());
    }

    #[tokio::test]
    async fn test_validate_access() {
        let service = AuthenticatedMediaService::default();

        service
            .register_media(
                "media123",
                AuthenticatedMediaType::Avatar,
                "@user:example.com",
                "image/png",
                1024,
                "abc123",
                false,
            )
            .await
            .unwrap();

        let token = service
            .generate_access_token(
                "media123",
                "@user:example.com",
                AuthenticatedMediaType::Avatar,
                None,
            )
            .await
            .unwrap();

        let record = service
            .validate_access(&token.token, "@user:example.com")
            .await
            .unwrap();

        assert_eq!(record.media_id, "media123");
    }

    #[tokio::test]
    async fn test_access_denied_for_different_user() {
        let service = AuthenticatedMediaService::default();

        service
            .register_media(
                "media123",
                AuthenticatedMediaType::Avatar,
                "@user1:example.com",
                "image/png",
                1024,
                "abc123",
                false,
            )
            .await
            .unwrap();

        let result = service
            .generate_access_token(
                "media123",
                "@user2:example.com",
                AuthenticatedMediaType::Avatar,
                None,
            )
            .await;

        assert!(matches!(result, Err(AuthMediaError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_max_downloads() {
        let service = AuthenticatedMediaService::default();

        service
            .register_media(
                "media123",
                AuthenticatedMediaType::Avatar,
                "@user:example.com",
                "image/png",
                1024,
                "abc123",
                false,
            )
            .await
            .unwrap();

        let token = service
            .generate_access_token(
                "media123",
                "@user:example.com",
                AuthenticatedMediaType::Avatar,
                Some(1),
            )
            .await
            .unwrap();

        service
            .validate_access(&token.token, "@user:example.com")
            .await
            .unwrap();

        let result = service
            .validate_access(&token.token, "@user:example.com")
            .await;

        assert!(matches!(result, Err(AuthMediaError::TokenExpired) | Err(AuthMediaError::InvalidToken)));
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let service = AuthenticatedMediaService::default();

        service
            .register_media(
                "media123",
                AuthenticatedMediaType::Avatar,
                "@user:example.com",
                "image/png",
                1024,
                "abc123",
                false,
            )
            .await
            .unwrap();

        let token = service
            .generate_access_token(
                "media123",
                "@user:example.com",
                AuthenticatedMediaType::Avatar,
                None,
            )
            .await
            .unwrap();

        assert!(service.revoke_token(&token.token).await);

        let result = service
            .validate_access(&token.token, "@user:example.com")
            .await;

        assert!(matches!(result, Err(AuthMediaError::InvalidToken)));
    }
}
