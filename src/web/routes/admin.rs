use serde::{Serialize, Deserialize};
use axum::{routing::{get, post, put, delete}, Router, extract::{State, Json, Path, Query}};
use serde_json::{Value, json};
use std::sync::Arc;
use crate::common::*;
use crate::services::*;
use crate::cache::*;

pub fn create_admin_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/_synapse/admin/v1/register", post(register_user))
        .route("/_synapse/admin/v1/users", get(list_users))
        .route("/_synapse/admin/v1/users/:user_id", get(get_user))
        .route("/_synapse/admin/v1/users/:user_id", put(update_user))
        .route("/_synapse/admin/v1/users/:user_id", delete(delete_user))
        .route("/_synapse/admin/v1/users/:user_id/change_password", post(change_password))
        .route("/_synapse/admin/v1/users/:user_id/deactivate", post(deactivate_user))
        .route("/_synapse/admin/v1/users/:user_id/token", post(generate_token))
        .route("/_synapse/admin/v1/rooms", get(list_rooms))
        .route("/_synapse/admin/v1/rooms/:room_id", get(get_room_details))
        .route("/_synapse/admin/v1/rooms/:room_id", delete(delete_room))
        .route("/_synapse/admin/v1/rooms/:room_id/purge", post(purge_room))
        .route("/_synapse/admin/v1/rooms/:room_id/members", get(get_room_members))
        .route("/_synapse/admin/v1/rooms/:room_id/message", post(send_room_message))
        .route("/_synapse/admin/v1/devices", get(list_all_devices))
        .route("/_synapse/admin/v1/devices/:device_id", get(get_device_details))
        .route("/_synapse/admin/v1/devices/:device_id", delete(delete_device_admin))
        .route("/_synapse/admin/v1/purge_history", post(purge_history))
        .route("/_synapse/admin/v1/status", get(server_status))
        .route("/_synapse/admin/v1/statistics/users", get(user_statistics))
        .route("/_synapse/admin/v1/statistics/rooms", get(room_statistics))
        .route("/_synapse/admin/v1/statistics/database", get(database_statistics))
        .route("/_synapse/admin/v1/statistics/cache", get(cache_statistics))
        .route("/_synapse/admin/v1/background_jobs", get(background_jobs))
        .with_state(state)
}

async fn register_user(State(state): State<AppState>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let username = body.get("username").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("Username required".to_string()))?;
    let password = body.get("password").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("Password required".to_string()))?;
    let admin = body.get("admin").and_then(|v| v.as_bool()).unwrap_or(false);
    let displayname = body.get("displayname").and_then(|v| v.as_str());

    let registration_service = RegistrationService::new(&state.services);
    registration_service.register_user(username, password, admin, displayname).await
}

async fn list_users(State(state): State<AppState>, Query(params): Query<Value>) -> Result<Json<Value>, ApiError> {
    let from = params.get("from").and_then(|v| v.as_i64()).unwrap_or(0);
    let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
    let guest = params.get("guest").and_then(|v| v.as_bool());

    let users = state.services.user_storage.get_user_by_username("").await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(json!({
        "users": users,
        "total": users.len()
    })))
}

async fn get_user(State(state): State<AppState>, Path(user_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let user = state.services.user_storage.get_user_by_id(&user_id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("User not found".to_string()))?;

    let devices = state.services.device_storage.get_user_devices(&user_id).await
        .map_err(|e| ApiError::internal(format!("Failed to get devices: {}", e)))?;

    Ok(Json(json!({
        "name": user.name,
        "admin": user.admin,
        "deactivated": user.deactivated,
        "displayname": user.displayname,
        "avatar_url": user.avatar_url,
        "creation_ts": user.creation_ts.timestamp_millis(),
        "devices": {
            "total": devices.len()
        }
    })))
}

async fn update_user(State(state): State<AppState>, Path(user_id): Path<String>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let admin = body.get("admin").and_then(|v| v.as_bool());

    Ok(Json(json!({})))
}

async fn delete_user(State(state): State<AppState>, Path(user_id): Path<String>) -> Result<Json<Value>, ApiError> {
    state.services.user_storage.deactivate_user(&user_id).await
        .map_err(|e| ApiError::internal(format!("Failed to delete user: {}", e)))?;

    Ok(Json(json!({})))
}

async fn change_password(State(state): State<AppState>, Path(user_id): Path<String>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let new_password = body.get("new_password").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("New password required".to_string()))?;
    let logout_devices = body.get("logout_devices").and_then(|v| v.as_bool()).unwrap_or(true);

    let registration_service = RegistrationService::new(&state.services);
    registration_service.change_password(&user_id, new_password).await?;

    if logout_devices {
        state.services.auth_service.logout_all(&user_id).await?;
    }

    Ok(Json(json!({})))
}

async fn deactivate_user(State(state): State<AppState>, Path(user_id): Path<String>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let erase = body.get("erase").and_then(|v| v.as_bool()).unwrap_or(false);

    let registration_service = RegistrationService::new(&state.services);
    registration_service.deactivate_account(&user_id).await?;

    Ok(Json(json!({})))
}

async fn generate_token(State(state): State<AppState>, Path(user_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let device_id = crate::common::crypto::generate_device_id();
    let access_token = crate::common::crypto::generate_token(64);
    let refresh_token = crate::common::crypto::generate_token(64);

    let expiry_ts = chrono::Utc::now().checked_add_signed(chrono::Duration::hours(24))
        .map(|t| t.timestamp_millis());
    
    state.services.token_storage.create_token(&access_token, &user_id, Some(&device_id), expiry_ts).await
        .map_err(|e| ApiError::internal(format!("Failed to create token: {}", e)))?;

    Ok(Json(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "expires_in": 86400,
        "device_id": device_id
    })))
}

async fn list_rooms(State(state): State<AppState>, Query(params): Query<Value>) -> Result<Json<Value>, ApiError> {
    let from = params.get("from").and_then(|v| v.as_i64()).unwrap_or(0);
    let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);

    let room_count = state.services.room_storage.get_room_count().await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(json!({
        "rooms": [],
        "total": room_count
    })))
}

async fn get_room_details(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let room = state.services.room_storage.get_room(&room_id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Room not found".to_string()))?;

    let member_count = state.services.member_storage.get_member_count(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to get members: {}", e)))?;

    Ok(Json(json!({
        "room_id": room.room_id,
        "name": room.name,
        "topic": room.topic,
        "avatar": room.avatar,
        "canonical_alias": room.canonical_alias,
        "is_public": room.is_public,
        "member_count": member_count,
        "room_version": room.version,
        "creation_ts": room.creation_ts.timestamp_millis()
    })))
}

async fn delete_room(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    state.services.room_storage.delete_room(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to delete room: {}", e)))?;

    Ok(Json(json!({})))
}

async fn purge_room(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    state.services.event_storage.delete_room_events(&room_id).await
        .map_err(|e| ApiError::internal(format!("Failed to purge room: {}", e)))?;

    Ok(Json(json!({})))
}

async fn get_room_members(State(state): State<AppState>, Path(room_id): Path<String>) -> Result<Json<Value>, ApiError> {
    let members = state.services.member_storage.get_room_members(&room_id, Some("join")).await
        .map_err(|e| ApiError::internal(format!("Failed to get members: {}", e)))?;

    let member_list: Vec<Value> = members.iter()
        .map(|m| json!({
            "user_id": m.user_id,
            "display_name": m.display_name,
            "avatar_url": m.avatar_url
        }))
        .collect();

    Ok(Json(json!({
        "members": member_list,
        "total": members.len()
    })))
}

async fn send_room_message(State(state): State<AppState>, Path(room_id): Path<String>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    let msgtype = body.get("msgtype").and_then(|v| v.as_str()).unwrap_or("m.room.message");
    let content = body.get("content").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("Content required".to_string()))?;
    let user_id = body.get("sender").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("Sender required".to_string()))?;

    let room_service = RoomService::new(&state.services);
    room_service.send_message(&room_id, user_id, msgtype, content, None).await
}

async fn list_all_devices(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "devices": []
    })))
}

async fn get_device_details(State(state): State<AppState>, Path(device_id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "device_id": device_id
    })))
}

async fn delete_device_admin(State(state): State<AppState>, Path(device_id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({})))
}

async fn purge_history(State(state): State<AppState>, Json(body): Json<Value>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({})))
}

async fn server_status(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "status": "running",
        "version": "0.1.0",
        "server_name": state.services.server_name
    })))
}

async fn user_statistics(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let total_rooms = state.services.room_storage.get_local_room_count().await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(json!({
        "total_user_count": 1,
        "daily_active_users": 0,
        "monthly_active_users": 0,
        "total_room_count": total_rooms
    })))
}

async fn room_statistics(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let room_count = state.services.room_storage.get_room_count().await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
    let total_messages = state.services.event_storage.get_total_message_count().await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(json!({
        "total_room_count": room_count,
        "total_message_count": total_messages
    })))
}

async fn database_statistics(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "database_size": 0,
        "media_storage_size": 0
    })))
}

async fn cache_statistics(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "cache_size": 0,
        "hit_rate": 0.0
    })))
}

async fn background_jobs(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "running_jobs": [],
        "scheduled_jobs": []
    })))
}
