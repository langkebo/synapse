use super::{AppState, AuthenticatedUser};
use crate::ApiError;
use crate::web::routes::MatrixJson;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde_json::Value;

pub fn create_e2ee_router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/_matrix/client/r0/keys/upload", post(upload_keys))
        .route("/_matrix/client/r0/keys/query", post(query_keys))
        .route("/_matrix/client/r0/keys/claim", post(claim_keys))
        .route("/_matrix/client/r0/keys/changes", get(key_changes))
        .route(
            "/_matrix/client/r0/rooms/{room_id}/keys/distribution",
            get(room_key_distribution),
        )
        .route(
            "/_matrix/client/r0/sendToDevice/{event_type}/{transaction_id}",
            put(send_to_device),
        )
        .route(
            "/_matrix/client/unstable/keys/signatures/upload",
            post(upload_signatures),
        )
        .route(
            "/_matrix/client/r0/keys/device_signing/upload",
            post(upload_device_signing_keys),
        )
}

#[axum::debug_handler]
async fn upload_keys(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    MatrixJson(body): MatrixJson<Value>,
) -> Result<Json<Value>, ApiError> {
    let device_id = auth_user
        .device_id
        .clone()
        .ok_or_else(|| ApiError::bad_request("Device ID required".to_string()))?;

    let request = crate::e2ee::device_keys::KeyUploadRequest {
        device_keys: if body.get("device_keys").is_some() {
            Some(crate::e2ee::device_keys::DeviceKeys {
                user_id: auth_user.user_id.clone(),
                device_id,
                algorithms: vec!["m.olm.v1.curve25519-aes-sha2".to_string()],
                keys: body["device_keys"]["keys"].clone(),
                signatures: body["device_keys"]["signatures"].clone(),
                unsigned: body["device_keys"]["unsigned"]
                    .as_object()
                    .map(|v| v.clone().into()),
            })
        } else {
            None
        },
        one_time_keys: body.get("one_time_keys").cloned(),
    };

    let response = state
        .services
        .device_keys_service
        .upload_keys(request)
        .await?;

    Ok(Json(serde_json::json!({
        "one_time_key_counts": response.one_time_key_counts
    })))
}

#[axum::debug_handler]
async fn query_keys(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    MatrixJson(body): MatrixJson<Value>,
) -> Result<Json<Value>, ApiError> {
    let request: crate::e2ee::device_keys::KeyQueryRequest = serde_json::from_value(body)
        .map_err(|e| crate::error::ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let response = state
        .services
        .device_keys_service
        .query_keys(request)
        .await?;

    Ok(Json(serde_json::json!({
        "device_keys": response.device_keys,
        "failures": response.failures
    })))
}

#[axum::debug_handler]
async fn claim_keys(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    MatrixJson(body): MatrixJson<Value>,
) -> Result<Json<Value>, crate::error::ApiError> {
    let request: crate::e2ee::device_keys::KeyClaimRequest = serde_json::from_value(body)
        .map_err(|e| crate::error::ApiError::bad_request(format!("Invalid request: {}", e)))?;

    let response = state
        .services
        .device_keys_service
        .claim_keys(request)
        .await?;

    Ok(Json(serde_json::json!({
        "one_time_keys": response.one_time_keys,
        "failures": response.failures
    })))
}

#[axum::debug_handler]
async fn key_changes(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Query(params): Query<Value>,
) -> Result<Json<Value>, crate::error::ApiError> {
    let from = params.get("from").and_then(|v| v.as_str()).unwrap_or("0");
    let to = params.get("to").and_then(|v| v.as_str()).unwrap_or("");

    let (changed, left) = state
        .services
        .device_keys_service
        .get_key_changes(from, to)
        .await?;

    Ok(Json(serde_json::json!({
        "changed": changed,
        "left": left
    })))
}

#[axum::debug_handler]
async fn room_key_distribution(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, crate::error::ApiError> {
    let session = state
        .services
        .megolm_service
        .get_outbound_session(&room_id)
        .await?;

    match session {
        Some(s) => Ok(Json(serde_json::json!({
            "room_id": room_id,
            "algorithm": "m.megolm.v1.aes-sha2",
            "session_id": s.session_id,
            "session_key": s.session_key
        }))),
        _ => Ok(Json(serde_json::json!({
            "room_id": room_id,
            "algorithm": "m.megolm.v1.aes-sha2",
            "session_id": "",
            "session_key": ""
        }))),
    }
}

#[axum::debug_handler]
async fn send_to_device(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path((_event_type, _transaction_id)): Path<(String, String)>,
    MatrixJson(body): MatrixJson<Value>,
) -> Result<Json<Value>, crate::error::ApiError> {
    let messages = body.get("messages").ok_or_else(|| {
        crate::error::ApiError::bad_request("Missing 'messages' field".to_string())
    })?;

    state
        .services
        .to_device_service
        .send_messages(&auth_user.user_id, messages)
        .await?;

    Ok(Json(serde_json::json!({})))
}

#[axum::debug_handler]
async fn upload_signatures(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    MatrixJson(body): MatrixJson<Value>,
) -> Result<Json<Value>, crate::error::ApiError> {
    let signatures = body.get("signatures").cloned().unwrap_or(body.clone());
    
    if let Some(sigs) = signatures.as_object() {
        for (user_id, user_sigs) in sigs {
            if let Some(device_sigs) = user_sigs.as_object() {
                for (device_id, sig_data) in device_sigs {
                    if let Some(sig_obj) = sig_data.as_object() {
                        for (key_id, signature) in sig_obj {
                            sqlx::query(
                                r#"
                                INSERT INTO device_key_signatures 
                                (user_id, device_id, key_id, signature, created_at)
                                VALUES ($1, $2, $3, $4, $5)
                                ON CONFLICT (user_id, device_id, key_id) 
                                DO UPDATE SET signature = $4
                                "#,
                            )
                            .bind(user_id)
                            .bind(device_id)
                            .bind(key_id)
                            .bind(signature.to_string())
                            .bind(chrono::Utc::now().timestamp())
                            .execute(&*state.services.user_storage.pool)
                            .await
                            .ok();
                        }
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({})))
}

#[axum::debug_handler]
async fn upload_device_signing_keys(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    MatrixJson(body): MatrixJson<Value>,
) -> Result<Json<Value>, crate::error::ApiError> {
    let master_key = body.get("master_key").cloned();
    let self_signing_key = body.get("self_signing_key").cloned();
    let user_signing_key = body.get("user_signing_key").cloned();

    let now = chrono::Utc::now().timestamp();

    if let Some(master) = master_key {
        sqlx::query(
            r#"
            INSERT INTO cross_signing_keys 
            (user_id, key_type, key_data, created_at)
            VALUES ($1, 'master', $2, $3)
            ON CONFLICT (user_id, key_type) DO UPDATE SET key_data = $2
            "#,
        )
        .bind(&auth_user.user_id)
        .bind(master.to_string())
        .bind(now)
        .execute(&*state.services.user_storage.pool)
        .await
        .ok();
    }

    if let Some(self_signing) = self_signing_key {
        sqlx::query(
            r#"
            INSERT INTO cross_signing_keys 
            (user_id, key_type, key_data, created_at)
            VALUES ($1, 'self_signing', $2, $3)
            ON CONFLICT (user_id, key_type) DO UPDATE SET key_data = $2
            "#,
        )
        .bind(&auth_user.user_id)
        .bind(self_signing.to_string())
        .bind(now)
        .execute(&*state.services.user_storage.pool)
        .await
        .ok();
    }

    if let Some(user_signing) = user_signing_key {
        sqlx::query(
            r#"
            INSERT INTO cross_signing_keys 
            (user_id, key_type, key_data, created_at)
            VALUES ($1, 'user_signing', $2, $3)
            ON CONFLICT (user_id, key_type) DO UPDATE SET key_data = $2
            "#,
        )
        .bind(&auth_user.user_id)
        .bind(user_signing.to_string())
        .bind(now)
        .execute(&*state.services.user_storage.pool)
        .await
        .ok();
    }

    Ok(Json(serde_json::json!({})))
}
