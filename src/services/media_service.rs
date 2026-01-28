use crate::services::*;
use crate::common::*;
use std::path::PathBuf;
use std::fs;

pub struct MediaService {
    media_path: PathBuf,
}

impl MediaService {
    pub fn new(media_path: &str) -> Self {
        let path = PathBuf::from(media_path);
        if !path.exists() {
            fs::create_dir_all(&path).ok();
        }
        Self { media_path: path }
    }

    pub async fn upload_media(
        &self,
        _user_id: &str,
        content: &[u8],
        content_type: &str,
        _filename: Option<&str>,
    ) -> ApiResult<serde_json::Value> {
        let media_id = generate_token(32);
        let extension = self.get_extension_from_content_type(content_type);
        let file_name = format!("{}.{}", media_id, extension);
        let file_path = self.media_path.join(&file_name);

        fs::write(&file_path, content)
            .map_err(|e| ApiError::internal(format!("Failed to save media: {}", e)))?;

        let media_url = format!("/_matrix/media/v3/download/{}", file_name);

        let json_metadata = serde_json::json!({
            "content_uri": media_url,
            "content_type": content_type,
            "size": content.len(),
            "media_id": media_id
        });

        Ok(json_metadata)
    }

    pub async fn get_media(&self, server_name: &str, media_id: &str) -> Option<Vec<u8>> {
        let file_path = self.media_path.join(format!("{}.*", media_id));
        if let Ok(entries) = fs::read_dir(&self.media_path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with(media_id) {
                        if let Ok(content) = fs::read(entry.path()) {
                            return Some(content);
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn get_media_metadata(&self, server_name: &str, media_id: &str) -> Option<serde_json::Value> {
        if let Ok(entries) = fs::read_dir(&self.media_path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with(media_id) {
                        let metadata = serde_json::json!({
                            "media_id": media_id,
                            "content_uri": format!("/_matrix/media/v3/download/{}", file_name),
                            "filename": file_name
                        });
                        return Some(metadata);
                    }
                }
            }
        }
        None
    }

    fn get_extension_from_content_type(&self, content_type: &str) -> &str {
        if content_type.starts_with("image/png") {
            "png"
        } else if content_type.starts_with("image/jpeg") {
            "jpg"
        } else if content_type.starts_with("image/gif") {
            "gif"
        } else if content_type.starts_with("image/webp") {
            "webp"
        } else if content_type.starts_with("video/mp4") {
            "mp4"
        } else if content_type.starts_with("video/webm") {
            "webm"
        } else if content_type.starts_with("audio/mpeg") {
            "mp3"
        } else if content_type.starts_with("audio/ogg") {
            "ogg"
        } else {
            "bin"
        }
    }
}
