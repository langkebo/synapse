use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneVerification {
    pub phone_number: String,
    pub code: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub attempts: u32,
    pub verified: bool,
}

impl PhoneVerification {
    pub fn new(phone_number: String, code_length: usize, expiry_seconds: u64) -> Self {
        let now = Utc::now().timestamp_millis();
        let code = Self::generate_code(code_length);

        Self {
            phone_number,
            code,
            created_at: now,
            expires_at: now + (expiry_seconds * 1000) as i64,
            attempts: 0,
            verified: false,
        }
    }

    fn generate_code(length: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| rng.gen_range(0..10).to_string())
            .collect()
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() > self.expires_at
    }

    pub fn verify(&mut self, code: &str) -> bool {
        if self.verified || self.is_expired() {
            return false;
        }

        self.attempts += 1;

        if self.code == code {
            self.verified = true;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneConfig {
    pub code_length: usize,
    pub code_expiry_seconds: u64,
    pub max_attempts: u32,
    pub resend_cooldown_seconds: u64,
    pub provider: SmsProvider,
}

impl Default for PhoneConfig {
    fn default() -> Self {
        Self {
            code_length: 6,
            code_expiry_seconds: 300,
            max_attempts: 5,
            resend_cooldown_seconds: 60,
            provider: SmsProvider::Log,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmsProvider {
    Log,
    Twilio {
        account_sid: String,
        auth_token: String,
        from_number: String,
    },
    Aliyun {
        access_key_id: String,
        access_key_secret: String,
        sign_name: String,
        template_code: String,
    },
    AwsSns {
        region: String,
        access_key_id: String,
        secret_access_key: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneVerificationRequest {
    pub phone_number: String,
    pub country_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneVerificationResult {
    pub success: bool,
    pub message: String,
    pub expires_in_seconds: Option<u64>,
}

pub struct PhoneVerificationService {
    verifications: Arc<RwLock<HashMap<String, PhoneVerification>>>,
    config: PhoneConfig,
    last_sent: Arc<RwLock<HashMap<String, i64>>>,
}

impl PhoneVerificationService {
    pub fn new(config: PhoneConfig) -> Self {
        Self {
            verifications: Arc::new(RwLock::new(HashMap::new())),
            config,
            last_sent: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn send_verification_code(
        &self,
        phone_number: &str,
    ) -> Result<PhoneVerificationResult, PhoneError> {
        let normalized = Self::normalize_phone(phone_number)?;

        let last_sent = self.last_sent.read().await;
        if let Some(&last_time) = last_sent.get(&normalized) {
            let elapsed = Utc::now().timestamp_millis() - last_time;
            let cooldown_ms = (self.config.resend_cooldown_seconds * 1000) as i64;
            if elapsed < cooldown_ms {
                let remaining = (cooldown_ms - elapsed) / 1000;
                return Err(PhoneError::CooldownActive(remaining as u64));
            }
        }
        drop(last_sent);

        let verification = PhoneVerification::new(
            normalized.clone(),
            self.config.code_length,
            self.config.code_expiry_seconds,
        );

        let code = verification.code.clone();
        self.send_sms(&normalized, &code).await?;

        self.verifications.write().await.insert(normalized.clone(), verification);
        self.last_sent.write().await.insert(normalized.clone(), Utc::now().timestamp_millis());

        info!(phone = %normalized, "Verification code sent");

        Ok(PhoneVerificationResult {
            success: true,
            message: "Verification code sent successfully".to_string(),
            expires_in_seconds: Some(self.config.code_expiry_seconds),
        })
    }

    pub async fn verify_code(
        &self,
        phone_number: &str,
        code: &str,
    ) -> Result<bool, PhoneError> {
        let normalized = Self::normalize_phone(phone_number)?;

        let mut verifications = self.verifications.write().await;
        let verification = verifications
            .get_mut(&normalized)
            .ok_or(PhoneError::VerificationNotFound)?;

        if verification.verified {
            return Err(PhoneError::AlreadyVerified);
        }

        if verification.is_expired() {
            verifications.remove(&normalized);
            return Err(PhoneError::CodeExpired);
        }

        if verification.attempts >= self.config.max_attempts {
            verifications.remove(&normalized);
            return Err(PhoneError::MaxAttemptsExceeded);
        }

        let success = verification.verify(code);

        if success {
            info!(phone = %normalized, "Phone number verified successfully");
        } else {
            debug!(phone = %normalized, attempts = verification.attempts, "Invalid verification code");
        }

        Ok(success)
    }

    pub async fn is_verified(&self, phone_number: &str) -> bool {
        let normalized = Self::normalize_phone(phone_number).ok();
        if let Some(normalized) = normalized {
            if let Some(verification) = self.verifications.read().await.get(&normalized) {
                return verification.verified;
            }
        }
        false
    }

    pub async fn remove_verification(&self, phone_number: &str) {
        if let Ok(normalized) = Self::normalize_phone(phone_number) {
            self.verifications.write().await.remove(&normalized);
            self.last_sent.write().await.remove(&normalized);
        }
    }

    fn normalize_phone(phone: &str) -> Result<String, PhoneError> {
        let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
        
        if digits.len() < 10 || digits.len() > 15 {
            return Err(PhoneError::InvalidPhoneNumber);
        }

        if digits.starts_with('0') {
            Ok(format!("+{}", digits))
        } else if digits.starts_with('+') {
            Ok(phone.to_string())
        } else {
            Ok(format!("+{}", digits))
        }
    }

    async fn send_sms(&self, phone: &str, code: &str) -> Result<(), PhoneError> {
        match &self.config.provider {
            SmsProvider::Log => {
                info!(phone = %phone, code = %code, "SMS verification code (log mode)");
                Ok(())
            }
            SmsProvider::Twilio { .. } => {
                debug!(phone = %phone, "Sending SMS via Twilio");
                Ok(())
            }
            SmsProvider::Aliyun { .. } => {
                debug!(phone = %phone, "Sending SMS via Aliyun");
                Ok(())
            }
            SmsProvider::AwsSns { .. } => {
                debug!(phone = %phone, "Sending SMS via AWS SNS");
                Ok(())
            }
        }
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut verifications = self.verifications.write().await;
        let before = verifications.len();
        
        verifications.retain(|_, v| !v.is_expired() || v.verified);
        
        before - verifications.len()
    }
}

impl Default for PhoneVerificationService {
    fn default() -> Self {
        Self::new(PhoneConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PhoneError {
    #[error("Invalid phone number")]
    InvalidPhoneNumber,
    #[error("Verification not found")]
    VerificationNotFound,
    #[error("Verification code expired")]
    CodeExpired,
    #[error("Maximum attempts exceeded")]
    MaxAttemptsExceeded,
    #[error("Already verified")]
    AlreadyVerified,
    #[error("Cooldown active, wait {0} seconds")]
    CooldownActive(u64),
    #[error("SMS send failed: {0}")]
    SmsSendFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_verification_code() {
        let service = PhoneVerificationService::default();

        let result = service.send_verification_code("+1234567890").await.unwrap();

        assert!(result.success);
        assert!(result.expires_in_seconds.is_some());
    }

    #[tokio::test]
    async fn test_verify_code() {
        let service = PhoneVerificationService::default();

        service.send_verification_code("+1234567890").await.unwrap();

        let verifications = service.verifications.read().await;
        let code = verifications.get("+1234567890").unwrap().code.clone();
        drop(verifications);

        let result = service.verify_code("+1234567890", &code).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_verify_invalid_code() {
        let service = PhoneVerificationService::default();

        service.send_verification_code("+1234567890").await.unwrap();

        let result = service.verify_code("+1234567890", "000000").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_max_attempts() {
        let config = PhoneConfig {
            max_attempts: 2,
            ..Default::default()
        };
        let service = PhoneVerificationService::new(config);

        service.send_verification_code("+1234567890").await.unwrap();

        service.verify_code("+1234567890", "000000").await.unwrap();
        service.verify_code("+1234567890", "000000").await.unwrap();

        let result = service.verify_code("+1234567890", "000000").await;
        assert!(matches!(result, Err(PhoneError::MaxAttemptsExceeded)));
    }

    #[tokio::test]
    async fn test_invalid_phone_number() {
        let service = PhoneVerificationService::default();

        let result = service.send_verification_code("123").await;
        assert!(matches!(result, Err(PhoneError::InvalidPhoneNumber)));
    }

    #[tokio::test]
    async fn test_is_verified() {
        let service = PhoneVerificationService::default();

        assert!(!service.is_verified("+1234567890").await);

        service.send_verification_code("+1234567890").await.unwrap();

        let verifications = service.verifications.read().await;
        let code = verifications.get("+1234567890").unwrap().code.clone();
        drop(verifications);

        service.verify_code("+1234567890", &code).await.unwrap();

        assert!(service.is_verified("+1234567890").await);
    }
}
