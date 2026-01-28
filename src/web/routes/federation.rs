use serde::{Serialize, Deserialize};
use axum::{routing::{get, post, put}, Router, extract::{State, Json, Path, Query, RawBody}};
use serde_json::{Value, json};
use std::sync::Arc;
use crate::common::*;
use crate::services::*;
use crate::cache::*;
use bytes::Bytes;

pub fn create_federation_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/_matrix/federation/v1/version", get(federation_version))
        .route("/_matrix/federation/v1", get(federation_discovery))
        .route("/_matrix/federation/v1/send/:txn_id", put(send_transaction))
        .route("/_matrix/federation/v1/make_join/:room_id/:user_id", get(make_join))
        .route("/_matrix/federation/v1/make_leave/:room_id/:user_id", get(make_leave))
        .route("/_matrix/federation/v1/send_join/:room_id/:event_id", put(send_join))
        .route("/_matrix/federation/v1/send_leave/:room_id/:event_id", put(send_leave))
        .route("/_matrix/federation/v1/invite/:room_id/:event_id", put(invite))
        .route("/_matrix/federation/v1/get_missing_events/:room_id", post(get_missing_events))
        .route("/_matrix/federation/v1/get_event_auth/:room_id/:event_id", get(get_event_auth))
        .route("/_matrix/federation/v1/state/:room_id", get(get_state))
        .route("/_matrix/federation/v1/state_ids/:room_id", get(get_state_ids))
        .route("/_matrix/federation/v1/query/directory/room/:room_id", get(room_directory_query))
        .route("/_matrix/federation/v1/query/profile/:user_id", get(profile_query))
        .route("/_matrix/federation/v1/backfill/:room_id", get(backfill))
        .route("/_matrix/federation/v1/keys/claim", post(keys_claim))
        .route("/_matrix/federation/v1/keys/upload", post(keys_upload))
        .route("/_matrix/federation/v2/server", get(server_key))
        .route("/_matrix/federation/v2/query/:server_name/:key_id", get(key_query))
        .route("/_matrix/federation/v2/key/clone", post(key_clone))
        .route("/_matrix/federation/v2/user/keys/query", post(user_keys_query))
        .with_state(state)
}

async fn federation_version() -> Json<Value> {
    Json(json!({
        "version": "0.1.0"
    }))
}

async fn federation_discovery() -> Json<Value> {
    Json(json!({
        "version": "0.1.0",
        "name": "Synapse Rust",
        "capabilities": {
            "m.change_password": true,
            "m.room_versions": {
                "1": {
                    "status": "stable"
                }
            }
        }
    }))
}

async fn send_transaction(State(state): State<AppState>, Path(txn_id): Path<String>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let origin = body.get("origin").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("Origin required".to_string()))?;
    let pdus = body.get("pdu").and_then(|v| v.as_array()).ok_or_else(|| ApiError::bad_request("PDU required".to_string()))?;

    let mut results = Vec::new();
    
    for pdu in pdus {
        let event_id = pdu.get("event_id").and_then(|v| v.as_str()).unwrap_or("unknown");
        results.push(json!({
            "event_id": event_id,
            "success": true
        }));
    }

    tracing::info!("Received transaction {} from {} with {} PDUs", txn_id, origin, pdus.len());

    Ok(Json(json!({
        "txn_id": txn_id,
        "results": results
    })))
}

async fn make_join(State(state): State<AppState>, Path((room_id, user_id)): Path<(String, String)>) -> Result<Json<Value>, ApiError> {
    let auth_events = state.services.event_storage.get_state_events(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to get auth events: {}", e)))?;

    let auth_events_json: Vec<Value> = auth_events.iter()
        .map(|e| json!({
            "event_id": e.event_id,
            "type": e.event_type,
            "state_key": e.state_key
        }))
        .collect();

    Ok(Json(json!({
        "room_version": "1",
        "auth_events": auth_events_json,
        "event": {
            "type": "m.room.member",
            "content": {
                "membership": "join"
            },
            "sender": user_id,
            "state_key": user_id
        }
    })))
}

async fn make_leave(State(state): State<AppState>, Path((room_id, user_id)): Path<(String, String)>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "room_version": "1",
        "event": {
            "type": "m.room.member",
            "content": {
                "membership": "leave"
            },
            "sender": user_id,
            "state_key": user_id
        }
    })))
}

async fn send_join(State(state): State<AppState>, Path((room_id, event_id)): Path<(String, String)>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let origin = body.get("origin").and_then(|v| v.as_str()).unwrap_or("unknown");
    
    tracing::info!("Processing join event {} for room {} from {}", event_id, room_id, origin);

    Ok(Json(json!({
        "room_id": room_id,
        "state": []
    })))
}

async fn send_leave(State(state): State<AppState>, Path((room_id, event_id)): Path<(String, String)>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let origin = body.get("origin").and_then(|v| v.as_str()).unwrap_or("unknown");
    
    tracing::info!("Processing leave event {} for room {} from {}", event_id, room_id, origin);

    Ok(Json(json!({
        "room_id": room_id,
        "event_id": event_id
    })))
}

async fn invite(State(state): State<AppState>, Path((room_id, event_id)): Path<(String, String)>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    tracing::info!("Processing invite for room {} event {}", room_id, event_id);

    Ok(Json(json!({
        "room_id": room_id,
        "event": body
    })))
}

async fn get_missing_events(State(state): State<AppState>, Path(room_id): Path<String>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let earliest_events = body.get("earliest_events").and_then(|v| v.as_array()).map(|v| v.len()).unwrap_or(0);
    let latest_events = body.get("latest_events").and_then(|v| v.as_array()).and_then(|v| v.first().and_then(|e| e.as_str())).unwrap_or("");
    let limit = body.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);

    let events = state.services.event_storage.get_room_events(&room_id, limit as i64, None).await
        .map_err(|e| ApiError::internal(format!("Failed to get events: {}", e)))?;

    let events_json: Vec<Value> = events.iter()
        .map(|e| json!({
            "event_id": e.event_id,
            "type": e.event_type,
            "sender": e.user_id,
            "content": serde_json::from_str(&e.content).unwrap_or(json!({})),
            "origin_server_ts": e.origin_server_ts
        }))
        .collect();

    Ok(Json(json!({
        "events": events_json
    })))
}

async fn get_event_auth(State(state): State<AppState>, Path((room_id, event_id)): Path<(String, String)>) -> Result<Json<Value>, ApiError> {
    let auth_events = state.services.event_storage.get_state_events(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to get auth events: {}", e)))?;

    let auth_events_json: Vec<Value> = auth_events.iter()
        .map(|e| json!({
            "event_id": e.event_id,
            "type": e.event_type,
            "sender": e.user_id,
            "content": serde_json::from_str(&e.content).unwrap_or(json!({}))
        }))
        .collect();

    Ok(Json(json!({
        "auth_events": auth_events_json,
        "room_state": auth_events_json
    })))
}

async fn get_state(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let state_events = state.services.event_storage.get_state_events(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to get state: {}", e)))?;

    let state_json: Vec<Value> = state_events.iter()
        .map(|e| json!({
            "event_id": e.event_id,
            "type": e.event_type,
            "sender": e.user_id,
            "content": serde_json::from_str(&e.content).unwrap_or(json!({})),
            "state_key": e.state_key
        }))
        .collect();

    Ok(Json(json!({
        "state": state_json
    })))
}

async fn get_state_ids(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let state_events = state.services.event_storage.get_state_events(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to get state: {}", e)))?;

    let mut state_ids = serde_json::Map::new();
    for e in &state_events {
        let key = format!("{}:{}", e.event_type, e.state_key.clone().unwrap_or_default());
        state_ids.insert(key, json!(e.event_id));
    }

    Ok(Json(json!({
        "state_ids": state_ids
    })))
}

async fn room_directory_query(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let room = state.services.room_storage.get_room(&room_id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    match room {
        Some(r) => Ok(Json(json!({
            "room_id": r.room_id,
            "aliases": r.canonical_alias.map(|a| vec![a])
        }))),
        None => Err(ApiError::not_found("Room not found".to_string()))
    }
}

async fn profile_query(State(state): State<AppState>, Path(user_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let user = state.services.user_storage.get_user_by_id(&user_id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    match user {
        Some(u) => Ok(Json(json!({
            "user_id": u.name,
            "displayname": u.displayname,
            "avatar_url": u.avatar_url
        }))),
        None => Err(ApiError::not_found("User not found".to_string()))
    }
}

async fn backfill(State(state): State<AppState>, Path(room_id): Path<String>, Query(params): Query<Value>) -> Result<Json<Value>, ApiError> {
    let v = params.get("v").and_then(|v| v.as_array()).map(|v| v.len()).unwrap_or(0);
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);

    let events = state.services.event_storage.get_room_events(&room_id, limit as i64, None).await
        .map_err(|e| ApiError::internal(format!("Failed to get events: {}", e)))?;

    let events_json: Vec<Value> = events.iter()
        .map(|e| json!({
            "event_id": e.event_id,
            "type": e.event_type,
            "sender": e.user_id,
            "content": serde_json::from_str(&e.content).unwrap_or(json!({})),
            "origin_server_ts": e.origin_server_ts,
            "depth": e.depth
        }))
        .collect();

    Ok(Json(json!({
        "events": events_json
    })))
}

async fn keys_claim(State(state): State<AppState>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let one_time_keys = body.get("one_time_keys").and_then(|v| v.as_object()).unwrap_or(&serde_json::Map::new());
    
    let mut claimed_keys = serde_json::Map::new();
    
    for (user_id, keys) in one_time_keys {
        let mut user_keys = serde_json::Map::new();
        for key_id in keys.as_array().unwrap_or(&vec![]) {
            if let Some(kid) = key_id.as_str() {
                let key_value = format!("{}{}", kid, user_id);
                user_keys.insert(kid.to_string(), json!(key_value));
            }
        }
        if !user_keys.is_empty() {
            claimed_keys.insert(user_id.clone(), json!(user_keys));
        }
    }

    Ok(Json(json!({
        "one_time_keys": claimed_keys,
        "failures": json!({})
    })))
}

async fn keys_upload(State(state): State<AppState>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let one_time_key_count = body.get("one_time_keys").and_then(|v| v.as_object()).map(|m| m.len() as i64).unwrap_or(0);

    Ok(Json(json!({
        "one_time_key_count": one_time_key_count,
        "unused_fallback_key_types": []
    })))
}

async fn server_key(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "server_name": state.services.server_name,
        "valid_until_ts": chrono::Utc::now().checked_add_signed(chrono::Duration::hours(24))
            .unwrap_or(chrono::Utc::now()).timestamp_millis() + 86400000,
        "verify_keys": {
            "ed25519:a_XUL": "a_XULcertificate"
        }
    })))
}

async fn key_query(State(state): State<AppState>, Path((server_name, key_id)): Path<(String, String)>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "server_name": server_name,
        "valid_until_ts": chrono::Utc::now().checked_add_signed(chrono::Duration::hours(24))
            .unwrap_or(chrono::Utc::now()).timestamp_millis() + 86400000,
        "verify_keys": {
            key_id.clone(): "mock_key_value"
        }
    })))
}

async fn key_clone(State(state): State<AppState>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let origin = body.get("origin").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("origin required".to_string()))?;
    let keys = body.get("keys").and_then(|v| v.as_object()).ok_or_else(|| ApiError::bad_request("keys required".to_string()))?;

    let mut cloned_keys = serde_json::Map::new();
    let mut failures = serde_json::Map::new();

    for (server_name, key_ids) in keys {
        let mut server_keys = serde_json::Map::new();

        for key_id in key_ids.as_array().iter().flatten().filter_map(|v| v.as_str()) {
            server_keys.insert(key_id.to_string(), json!("cloned_key"));
        }

        if !server_keys.is_empty() {
            cloned_keys.insert(server_name.clone(), json!(server_keys));
        }
    }

    tracing::info!("Federation key clone from {}: {} keys cloned, {} failures", origin, cloned_keys.len(), failures.len());

    Ok(Json(json!({
        "origin": state.services.server_name,
        "keys": cloned_keys,
        "failures": failures
    })))
}

async fn user_keys_query(State(state): State<AppState>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let device_keys = body.get("device_keys").and_then(|v| v.as_object()).ok_or_else(|| ApiError::bad_request("device_keys required".to_string()))?;

    let mut user_keys = serde_json::Map::new();
    let mut failures = serde_json::Map::new();

    for (user_id, key_ids) in device_keys {
        let mut user_device_keys = serde_json::Map::new();

        for key_id in key_ids.as_array().iter().flatten().filter_map(|v| v.as_str()) {
            user_device_keys.insert(key_id.to_string(), json!({
                "user_id": user_id,
                "device_id": key_id.split(':').next().unwrap_or(""),
                "key": "mock_device_key"
            }));
        }

        if !user_device_keys.is_empty() {
            user_keys.insert(user_id.clone(), json!(user_device_keys));
        }
    }

    tracing::info!("User keys query: {} users, {} keys retrieved, {} failures", user_keys.len(), user_keys.values().map(|v| v.as_object().map(|o| o.len()).unwrap_or(0)).sum::<usize>(), failures.len());

    Ok(Json(json!({
        "device_keys": user_keys,
        "failures": failures
    })))
}
