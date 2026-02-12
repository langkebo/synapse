use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::common::*;
use crate::storage::{
    CreateExternalServiceParams, ExternalServiceStorage,
};
use crate::web::routes::{AppState, AuthenticatedUser};

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveServiceConfigRequest {
    pub endpoint: String,
    pub api_key: String,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceConfigResponse {
    pub endpoint: String,
    pub has_api_key: bool,
    pub config: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialsResponse {
    pub endpoint: String,
    pub api_key: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceListResponse {
    pub services: Vec<ServiceInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub service_type: String,
    pub endpoint: String,
    pub has_api_key: bool,
    pub status: String,
}

pub fn create_external_services_router() -> Router<AppState> {
    Router::new()
        .route(
            "/_matrix/client/v3/users/me/external-services",
            get(list_services),
        )
        .route(
            "/_matrix/client/v3/users/me/external-services/{service_type}",
            get(get_service_config).put(save_service_config).delete(delete_service_config),
        )
        .route(
            "/_matrix/client/v3/users/me/external-services/{service_type}/credentials",
            get(get_credentials),
        )
        .route(
            "/_matrix/client/v3/users/me/external-services/{service_type}/status",
            put(set_service_status),
        )
}

async fn list_services(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<ServiceListResponse>, ApiError> {
    let storage = ExternalServiceStorage::new(&state.services.user_storage.pool);
    
    let services = storage
        .list_user_services(&auth_user.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list services: {}", e)))?;
    
    let service_list: Vec<ServiceInfo> = services
        .into_iter()
        .map(|s| ServiceInfo {
            service_type: s.service_type,
            endpoint: s.endpoint,
            has_api_key: s.api_key_encrypted.is_some(),
            status: s.status,
        })
        .collect();
    
    Ok(Json(ServiceListResponse {
        services: service_list,
    }))
}

async fn get_service_config(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(service_type): Path<String>,
) -> Result<Json<ServiceConfigResponse>, ApiError> {
    validate_service_type(&service_type)?;
    
    let storage = ExternalServiceStorage::new(&state.services.user_storage.pool);
    
    let config = storage
        .get_service_config(&auth_user.user_id, &service_type)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get service config: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Service '{}' not configured", service_type)))?;
    
    Ok(Json(ServiceConfigResponse {
        endpoint: config.endpoint,
        has_api_key: config.has_api_key,
        config: config.config,
        status: config.status,
    }))
}

async fn save_service_config(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(service_type): Path<String>,
    Json(body): Json<SaveServiceConfigRequest>,
) -> Result<StatusCode, ApiError> {
    validate_service_type(&service_type)?;
    validate_endpoint(&body.endpoint)?;
    
    if body.api_key.is_empty() {
        return Err(ApiError::bad_request("API key is required".to_string()));
    }
    
    if body.api_key.len() > 1024 {
        return Err(ApiError::bad_request("API key too long (max 1024 characters)".to_string()));
    }
    
    let encrypted_key = encrypt_api_key(&body.api_key, &state.services.config)
        .map_err(|e| ApiError::internal(format!("Failed to encrypt API key: {}", e)))?;
    
    let storage = ExternalServiceStorage::new(&state.services.user_storage.pool);
    
    storage
        .upsert_service(CreateExternalServiceParams {
            user_id: auth_user.user_id.clone(),
            service_type: service_type.clone(),
            endpoint: body.endpoint.clone(),
            api_key_encrypted: Some(encrypted_key),
            config: body.config,
        })
        .await
        .map_err(|e| ApiError::internal(format!("Failed to save service config: {}", e)))?;
    
    Ok(StatusCode::OK)
}

async fn delete_service_config(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(service_type): Path<String>,
) -> Result<StatusCode, ApiError> {
    validate_service_type(&service_type)?;
    
    let storage = ExternalServiceStorage::new(&state.services.user_storage.pool);
    
    let deleted = storage
        .delete_service(&auth_user.user_id, &service_type)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete service config: {}", e)))?;
    
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!("Service '{}' not found", service_type)))
    }
}

async fn get_credentials(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(service_type): Path<String>,
) -> Result<Json<CredentialsResponse>, ApiError> {
    validate_service_type(&service_type)?;
    
    let storage = ExternalServiceStorage::new(&state.services.user_storage.pool);
    
    let credentials = storage
        .get_service_credentials(&auth_user.user_id, &service_type)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get credentials: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Service '{}' not configured or inactive", service_type)))?;
    
    let decrypted_key = decrypt_api_key(&credentials.api_key, &state.services.config)
        .map_err(|e| ApiError::internal(format!("Failed to decrypt API key: {}", e)))?;
    
    storage
        .update_last_used(&auth_user.user_id, &service_type)
        .await
        .ok();
    
    Ok(Json(CredentialsResponse {
        endpoint: credentials.endpoint,
        api_key: decrypted_key,
        config: credentials.config,
    }))
}

#[derive(Debug, Deserialize)]
struct SetStatusRequest {
    status: String,
}

async fn set_service_status(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(service_type): Path<String>,
    Json(body): Json<SetStatusRequest>,
) -> Result<StatusCode, ApiError> {
    validate_service_type(&service_type)?;
    
    if body.status != "active" && body.status != "inactive" {
        return Err(ApiError::bad_request("Status must be 'active' or 'inactive'".to_string()));
    }
    
    let storage = ExternalServiceStorage::new(&state.services.user_storage.pool);
    
    let updated = storage
        .set_service_status(&auth_user.user_id, &service_type, &body.status)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update status: {}", e)))?;
    
    if updated {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::not_found(format!("Service '{}' not found", service_type)))
    }
}

fn validate_service_type(service_type: &str) -> Result<(), ApiError> {
    if service_type.is_empty() {
        return Err(ApiError::bad_request("Service type is required".to_string()));
    }
    
    if service_type.len() > 50 {
        return Err(ApiError::bad_request("Service type too long (max 50 characters)".to_string()));
    }
    
    let valid_types = [
        "trendradar",
        "openclaw",
        "openai",
        "claude",
        "deepseek",
        "anthropic",
        "gemini",
        "custom",
    ];
    
    let is_valid = valid_types.contains(&service_type.to_lowercase().as_str())
        || service_type.starts_with("custom_");
    
    if !is_valid {
        return Err(ApiError::bad_request(format!(
            "Invalid service type. Valid types: {}",
            valid_types.join(", ")
        )));
    }
    
    Ok(())
}

fn validate_endpoint(endpoint: &str) -> Result<(), ApiError> {
    if endpoint.is_empty() {
        return Err(ApiError::bad_request("Endpoint is required".to_string()));
    }
    
    if endpoint.len() > 500 {
        return Err(ApiError::bad_request("Endpoint URL too long (max 500 characters)".to_string()));
    }
    
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        return Err(ApiError::bad_request(
            "Endpoint must be a valid URL starting with http:// or https://".to_string(),
        ));
    }
    
    Ok(())
}

fn encrypt_api_key(key: &str, config: &crate::common::config::Config) -> Result<String, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use sha2::{Digest, Sha256};
    
    let encryption_key = config.security.secret.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(encryption_key);
    let key_bytes = hasher.finalize();
    
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;
    
    let _nonce_bytes = chrono::Utc::now().timestamp().to_le_bytes();
    let nonce = Nonce::from_slice(&[0u8; 12]);
    
    let encrypted = cipher
        .encrypt(nonce, key.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    Ok(BASE64.encode(&encrypted))
}

fn decrypt_api_key(encrypted: &str, config: &crate::common::config::Config) -> Result<String, String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use sha2::{Digest, Sha256};
    
    let encryption_key = config.security.secret.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(encryption_key);
    let key_bytes = hasher.finalize();
    
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;
    
    let encrypted_bytes = BASE64
        .decode(encrypted)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;
    
    let nonce = Nonce::from_slice(&[0u8; 12]);
    
    let decrypted = cipher
        .decrypt(nonce, encrypted_bytes.as_slice())
        .map_err(|e| format!("Decryption failed: {}", e))?;
    
    String::from_utf8(decrypted).map_err(|e| format!("Invalid UTF-8: {}", e))
}
