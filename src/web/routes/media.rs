use serde::{Serialize, Deserialize};
use axum::{routing::{get, post, delete}, Router, extract::{State, Json, Path, Query, Multipart}, response::IntoResponse};
use serde_json::{Value, json};
use std::sync::Arc;
use crate::common::*;
use crate::services::*;
use crate::cache::*;

pub fn create_media_router(state: Arc<AppState>, media_path: std::path::PathBuf) -> Router {
    Router::new()
        .route("/_matrix/media/r0/upload", post(upload_media))
        .route("/_matrix/media/r0/download/:server_name/:media_id", get(download_media))
        .route("/_matrix/media/r0/preview_url", get(preview_url))
        .route("/_matrix/media/r0/thumbnail/:server_name/:media_id", get(get_thumbnail))
        .route("/_matrix/media/v1/config", get(media_config))
        .route("/_matrix/media/r1/upload", post(upload_media_v1))
        .route("/_matrix/media/r1/download/:server_name/:media_id", get(download_media_v1))
        .with_state((state, media_path))
}

async fn upload_media(State(state): State<AppState>, multipart: Multipart) -> Result<Json<Value>, ApiError> {
    let token = extract_token_from_multipart(&multipart).await
        .ok_or_else(|| ApiError::unauthorized("Missing access token".to_string()))?;
    
    let (user_id, _, _) = state.services.auth_service.validate_token(&token).await?;

    let mut content: Vec<u8> = Vec::new();
    let mut content_type = "application/octet-stream".to_string();
    let mut filename: Option<&str> = None;

    while let Some(field) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.unwrap_or(&[]);

        if name == "file" {
            content = data.to_vec();
            if let Some(content_type_header) = field.headers().get("content-type") {
                if let Ok(ct) = content_type_header.to_str() {
                    content_type = ct.to_string();
                }
            }
        } else if name == "filename" {
            if let Ok(s) = std::str::from_utf8(data) {
                filename = Some(s);
            }
        }
    }

    if content.is_empty() {
        return Err(ApiError::bad_request("No file provided".to_string()));
    }

    let media_service = MediaService::new(&state.services, std::path::PathBuf::from("media"));
    media_service.upload_media(&user_id, &content, &content_type, filename).await
}

async fn download_media(State(state): State<AppState>, Path((server_name, media_id)): Path<(String, String)>) -> impl IntoResponse {
    let media_service = MediaService::new(&state.services, std::path::PathBuf::from("media"));
    
    match media_service.download_media(&server_name, &media_id).await {
        Ok(content) => {
            let content_type = guess_content_type(&media_id);
            ([("Content-Type", content_type), ("Content-Length", content.len().to_string())], content)
        }
        Err(e) => {
            ([("Content-Type", "application/json")], serde_json::to_vec(&json!({
                "errcode": e.code,
                "error": e.message
            })).unwrap())
        }
    }
}

async fn preview_url(State(state): State<AppState>, Query(params): Query<Value>) -> Result<Json<Value>, ApiError> {
    let url = params.get("url").and_then(|v| v.as_str()).ok_or_else(|| ApiError::bad_request("URL required".to_string()))?;
    
    Ok(Json(json!({
        "url": url,
        "title": "Preview",
        "description": "URL preview"
    })))
}

async fn get_thumbnail(State(state): State<AppState>, Path((server_name, media_id)): Path<(String, String)>, Query(params): Query<Value>) -> impl IntoResponse {
    let width = params.get("width").and_then(|v| v.as_u64()).unwrap_or(800);
    let height = params.get("height").and_then(|v| v.as_u64()).unwrap_or(600);
    let method = params.get("method").and_then(|v| v.as_str()).unwrap_or("scale");

    let media_service = MediaService::new(&state.services, std::path::PathBuf::from("media"));
    
    match media_service.get_thumbnail(&server_name, &media_id, width as u32, height as u32, method).await {
        Ok(content) => {
            let content_type = guess_content_type(&media_id);
            ([("Content-Type", content_type), ("Content-Length", content.len().to_string())], content)
        }
        Err(e) => {
            ([("Content-Type", "application/json")], serde_json::to_vec(&json!({
                "errcode": e.code,
                "error": e.message
            })).unwrap())
        }
    }
}

async fn media_config(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({
        "m.upload.size": 53687091200
    })))
}

async fn upload_media_v1(State(state): State<AppState>, multipart: Multipart) -> Result<Json<Value>, ApiError> {
    upload_media(State(state), multipart).await
}

async fn download_media_v1(State(state): State<AppState>, Path((server_name, media_id)): Path<(String, String)>) -> impl IntoResponse {
    download_media(State(state), Path((server_name, media_id))).await
}

fn extract_token_from_multipart(multipart: &Multipart) -> impl std::future::Future<Output = Option<String>> + Send {
    async {
        let mut token = None;
        let mut fields = std::collections::HashMap::new();
        
        let mut field = multipart.next_field().await;
        while let Some(f) = &mut field {
            let name = f.name().unwrap_or("").to_string();
            let data = f.bytes().await.unwrap_or(&[]);
            fields.insert(name, data.to_vec());
            field = multipart.next_field().await;
        }

        if let Some(token_data) = fields.get("access_token") {
            if let Ok(s) = std::str::from_utf8(token_data) {
                token = Some(s.to_string());
            }
        }

        token
    }
}

fn guess_content_type(media_id: &str) -> String {
    let ext = media_id.split('.').last().unwrap_or("");
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }.to_string()
}
