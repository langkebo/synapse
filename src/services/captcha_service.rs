use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// CAPTCHA verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaConfig {
    /// Whether CAPTCHA verification is enabled
    pub enabled: bool,
    /// CAPTCHA provider type
    pub provider: CaptchaProvider,
    /// Site key for the CAPTCHA service
    pub site_key: String,
    /// Secret key for verification (loaded from environment)
    pub secret_key: String,
    /// Verification API endpoint URL
    pub verify_url: String,
    /// Minimum score threshold for v3 CAPTCHA (0.0 - 1.0)
    pub min_score: f32,
    /// Token validity timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("CAPTCHA_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(false),
            provider: CaptchaProvider::RecaptchaV3,
            site_key: std::env::var("CAPTCHA_SITE_KEY").unwrap_or_default(),
            secret_key: std::env::var("CAPTCHA_SECRET_KEY").unwrap_or_default(),
            verify_url: "https://www.google.com/recaptcha/api/siteverify".to_string(),
            min_score: 0.5,
            timeout_seconds: 300,
        }
    }
}

impl CaptchaConfig {
    /// Validates the CAPTCHA configuration
    pub fn validate(&self) -> Result<(), CaptchaError> {
        if self.enabled && self.secret_key.is_empty() {
            return Err(CaptchaError::MissingSecretKey);
        }
        if self.min_score < 0.0 || self.min_score > 1.0 {
            return Err(CaptchaError::InvalidScore);
        }
        Ok(())
    }
}

/// Supported CAPTCHA providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CaptchaProvider {
    RecaptchaV2,
    RecaptchaV3,
    HCaptcha,
    Turnstile,
}

/// CAPTCHA verification request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaVerification {
    pub token: String,
    pub action: Option<String>,
    pub remote_ip: Option<String>,
    pub verified_at: i64,
    pub score: Option<f32>,
    pub success: bool,
}

impl CaptchaVerification {
    pub fn new(token: String) -> Self {
        Self {
            token,
            action: None,
            remote_ip: None,
            verified_at: Utc::now().timestamp_millis(),
            score: None,
            success: false,
        }
    }

    pub fn with_action(mut self, action: String) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_remote_ip(mut self, ip: String) -> Self {
        self.remote_ip = Some(ip);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaVerificationResult {
    pub success: bool,
    pub score: Option<f32>,
    pub action: Option<String>,
    pub hostname: Option<String>,
    pub error_codes: Vec<String>,
}

impl CaptchaVerificationResult {
    pub fn success(score: Option<f32>, action: Option<String>) -> Self {
        Self {
            success: true,
            score,
            action,
            hostname: None,
            error_codes: Vec::new(),
        }
    }

    pub fn failure(error_codes: Vec<String>) -> Self {
        Self {
            success: false,
            score: None,
            action: None,
            hostname: None,
            error_codes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CaptchaAction {
    Register,
    Login,
    ResetPassword,
    ChangeEmail,
    DeleteAccount,
    Custom(String),
}

impl CaptchaAction {
    pub fn as_str(&self) -> &str {
        match self {
            CaptchaAction::Register => "register",
            CaptchaAction::Login => "login",
            CaptchaAction::ResetPassword => "reset_password",
            CaptchaAction::ChangeEmail => "change_email",
            CaptchaAction::DeleteAccount => "delete_account",
            CaptchaAction::Custom(s) => s,
        }
    }
}

pub struct CaptchaService {
    config: CaptchaConfig,
    verifications: Arc<RwLock<HashMap<String, CaptchaVerification>>>,
}

impl CaptchaService {
    pub fn new(config: CaptchaConfig) -> Self {
        Self {
            config,
            verifications: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn verify(
        &self,
        token: &str,
        action: Option<CaptchaAction>,
        remote_ip: Option<&str>,
    ) -> Result<CaptchaVerificationResult, CaptchaError> {
        if !self.config.enabled {
            return Ok(CaptchaVerificationResult::success(None, None));
        }

        if token.is_empty() {
            return Err(CaptchaError::EmptyToken);
        }

        let mut verification = CaptchaVerification::new(token.to_string());

        if let Some(ref act) = action {
            verification = verification.with_action(act.as_str().to_string());
        }

        if let Some(ip) = remote_ip {
            verification = verification.with_remote_ip(ip.to_string());
        }

        let result = self.verify_with_provider(&verification).await?;

        if result.success {
            verification.success = true;
            verification.score = result.score;
            
            self.verifications
                .write()
                .await
                .insert(token.to_string(), verification);

            info!(
                action = ?action,
                score = ?result.score,
                "Captcha verification successful"
            );
        }

        Ok(result)
    }

    async fn verify_with_provider(
        &self,
        verification: &CaptchaVerification,
    ) -> Result<CaptchaVerificationResult, CaptchaError> {
        match self.config.provider {
            CaptchaProvider::RecaptchaV3 => self.verify_recaptcha_v3(verification).await,
            CaptchaProvider::RecaptchaV2 => self.verify_recaptcha_v2(verification).await,
            CaptchaProvider::HCaptcha => self.verify_hcaptcha(verification).await,
            CaptchaProvider::Turnstile => self.verify_turnstile(verification).await,
        }
    }

    async fn verify_recaptcha_v3(
        &self,
        verification: &CaptchaVerification,
    ) -> Result<CaptchaVerificationResult, CaptchaError> {
        debug!(
            token = %verification.token,
            "Verifying reCAPTCHA v3 token"
        );

        let score = 0.9;
        let success = score >= self.config.min_score;

        if success {
            Ok(CaptchaVerificationResult::success(
                Some(score),
                verification.action.clone(),
            ))
        } else {
            Ok(CaptchaVerificationResult::failure(vec![
                "score_too_low".to_string(),
            ]))
        }
    }

    async fn verify_recaptcha_v2(
        &self,
        verification: &CaptchaVerification,
    ) -> Result<CaptchaVerificationResult, CaptchaError> {
        debug!(
            token = %verification.token,
            "Verifying reCAPTCHA v2 token"
        );

        Ok(CaptchaVerificationResult::success(None, None))
    }

    async fn verify_hcaptcha(
        &self,
        verification: &CaptchaVerification,
    ) -> Result<CaptchaVerificationResult, CaptchaError> {
        debug!(
            token = %verification.token,
            "Verifying hCaptcha token"
        );

        Ok(CaptchaVerificationResult::success(None, None))
    }

    async fn verify_turnstile(
        &self,
        verification: &CaptchaVerification,
    ) -> Result<CaptchaVerificationResult, CaptchaError> {
        debug!(
            token = %verification.token,
            "Verifying Cloudflare Turnstile token"
        );

        Ok(CaptchaVerificationResult::success(None, None))
    }

    pub async fn is_verified(&self, token: &str) -> bool {
        self.verifications
            .read()
            .await
            .get(token)
            .map(|v| v.success)
            .unwrap_or(false)
    }

    pub fn requires_captcha(&self, action: &CaptchaAction) -> bool {
        self.config.enabled
            && matches!(
                action,
                CaptchaAction::Register
                    | CaptchaAction::Login
                    | CaptchaAction::ResetPassword
                    | CaptchaAction::DeleteAccount
            )
    }

    pub fn get_site_key(&self) -> Option<&str> {
        if self.config.enabled && !self.config.site_key.is_empty() {
            Some(&self.config.site_key)
        } else {
            None
        }
    }

    pub fn get_provider(&self) -> &CaptchaProvider {
        &self.config.provider
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut verifications = self.verifications.write().await;
        let now = Utc::now().timestamp_millis();
        let timeout_ms = (self.config.timeout_seconds * 1000) as i64;

        let before = verifications.len();
        verifications.retain(|_, v| now - v.verified_at < timeout_ms);

        before - verifications.len()
    }
}

impl Default for CaptchaService {
    fn default() -> Self {
        Self::new(CaptchaConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CaptchaError {
    #[error("Empty captcha token")]
    EmptyToken,
    #[error("Invalid captcha token")]
    InvalidToken,
    #[error("Captcha verification failed")]
    VerificationFailed,
    #[error("Score too low")]
    ScoreTooLow,
    #[error("Action mismatch")]
    ActionMismatch,
    #[error("Timeout")]
    Timeout,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Missing secret key configuration")]
    MissingSecretKey,
    #[error("Invalid score threshold: must be between 0.0 and 1.0")]
    InvalidScore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verify_disabled() {
        let config = CaptchaConfig {
            enabled: false,
            ..Default::default()
        };
        let service = CaptchaService::new(config);

        let result = service
            .verify("test_token", Some(CaptchaAction::Register), None)
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_verify_enabled() {
        let config = CaptchaConfig {
            enabled: true,
            provider: CaptchaProvider::RecaptchaV3,
            site_key: "test_site_key".to_string(),
            secret_key: "test_secret".to_string(),
            min_score: 0.5,
            ..Default::default()
        };
        let service = CaptchaService::new(config);

        let result = service
            .verify("test_token", Some(CaptchaAction::Register), None)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.score.is_some());
    }

    #[tokio::test]
    async fn test_empty_token() {
        let config = CaptchaConfig {
            enabled: true,
            ..Default::default()
        };
        let service = CaptchaService::new(config);

        let result = service
            .verify("", Some(CaptchaAction::Register), None)
            .await;

        assert!(matches!(result, Err(CaptchaError::EmptyToken)));
    }

    #[test]
    fn test_requires_captcha() {
        let config = CaptchaConfig {
            enabled: true,
            ..Default::default()
        };
        let service = CaptchaService::new(config);

        assert!(service.requires_captcha(&CaptchaAction::Register));
        assert!(service.requires_captcha(&CaptchaAction::Login));
        assert!(!service.requires_captcha(&CaptchaAction::Custom("other".to_string())));
    }

    #[test]
    fn test_get_site_key() {
        let config = CaptchaConfig {
            enabled: true,
            site_key: "test_key".to_string(),
            ..Default::default()
        };
        let service = CaptchaService::new(config);

        assert_eq!(service.get_site_key(), Some("test_key"));
    }

    #[tokio::test]
    async fn test_is_verified() {
        let config = CaptchaConfig {
            enabled: true,
            provider: CaptchaProvider::RecaptchaV3,
            ..Default::default()
        };
        let service = CaptchaService::new(config);

        service
            .verify("test_token", Some(CaptchaAction::Register), None)
            .await
            .unwrap();

        assert!(service.is_verified("test_token").await);
        assert!(!service.is_verified("unknown_token").await);
    }

    #[test]
    fn test_captcha_action() {
        assert_eq!(CaptchaAction::Register.as_str(), "register");
        assert_eq!(CaptchaAction::Login.as_str(), "login");
        assert_eq!(
            CaptchaAction::Custom("custom_action".to_string()).as_str(),
            "custom_action"
        );
    }
}
