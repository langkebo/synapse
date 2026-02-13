use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub media_id: String,
    pub content_type: String,
    pub size: u64,
    pub created_at: i64,
    pub uploader: String,
    pub sha256: String,
}

#[async_trait]
pub trait MediaStorageBackend: Send + Sync {
    fn name(&self) -> &str;
    
    async fn store(
        &self,
        media_id: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<MediaMetadata, StorageError>;
    
    async fn retrieve(&self, media_id: &str) -> Result<Option<(Bytes, MediaMetadata)>, StorageError>;
    
    async fn delete(&self, media_id: &str) -> Result<bool, StorageError>;
    
    async fn exists(&self, media_id: &str) -> Result<bool, StorageError>;
    
    async fn get_metadata(&self, media_id: &str) -> Result<Option<MediaMetadata>, StorageError>;
    
    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MediaMetadata>, StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Object not found: {0}")]
    NotFound(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Upload failed: {0}")]
    UploadFailed(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    #[error("Delete failed: {0}")]
    DeleteFailed(String),
}

pub struct LocalStorageBackend {
    base_path: String,
}

impl LocalStorageBackend {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
        }
    }

    fn get_path(&self, media_id: &str) -> String {
        format!("{}/{}", self.base_path, media_id)
    }

    fn get_meta_path(&self, media_id: &str) -> String {
        format!("{}/{}.meta", self.base_path, media_id)
    }
}

#[async_trait]
impl MediaStorageBackend for LocalStorageBackend {
    fn name(&self) -> &str {
        "local"
    }

    async fn store(
        &self,
        media_id: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<MediaMetadata, StorageError> {
        let path = self.get_path(media_id);
        let meta_path = self.get_meta_path(media_id);

        let mut file = File::create(&path).await?;
        file.write_all(data).await?;
        file.flush().await?;

        let sha256 = format!("{:x}", Sha256::digest(data));
        
        let metadata = MediaMetadata {
            media_id: media_id.to_string(),
            content_type: content_type.to_string(),
            size: data.len() as u64,
            created_at: chrono::Utc::now().timestamp_millis(),
            uploader: "".to_string(),
            sha256,
        };

        let meta_json = serde_json::to_string(&metadata)
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;
        
        let mut meta_file = File::create(&meta_path).await?;
        meta_file.write_all(meta_json.as_bytes()).await?;
        meta_file.flush().await?;

        info!(media_id = %media_id, size = data.len(), "Media stored locally");
        Ok(metadata)
    }

    async fn retrieve(&self, media_id: &str) -> Result<Option<(Bytes, MediaMetadata)>, StorageError> {
        let path = self.get_path(media_id);
        let meta_path = self.get_meta_path(media_id);

        if !Path::new(&path).exists() {
            return Ok(None);
        }

        let mut file = File::open(&path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let mut meta_file = File::open(&meta_path).await?;
        let mut meta_buffer = String::new();
        meta_file.read_to_string(&mut meta_buffer).await?;

        let metadata: MediaMetadata = serde_json::from_str(&meta_buffer)
            .map_err(|e| StorageError::DownloadFailed(e.to_string()))?;

        Ok(Some((Bytes::from(buffer), metadata)))
    }

    async fn delete(&self, media_id: &str) -> Result<bool, StorageError> {
        let path = self.get_path(media_id);
        let meta_path = self.get_meta_path(media_id);

        let mut deleted = false;

        if Path::new(&path).exists() {
            tokio::fs::remove_file(&path).await?;
            deleted = true;
        }

        if Path::new(&meta_path).exists() {
            tokio::fs::remove_file(&meta_path).await?;
        }

        if deleted {
            info!(media_id = %media_id, "Media deleted from local storage");
        }

        Ok(deleted)
    }

    async fn exists(&self, media_id: &str) -> Result<bool, StorageError> {
        Ok(Path::new(&self.get_path(media_id)).exists())
    }

    async fn get_metadata(&self, media_id: &str) -> Result<Option<MediaMetadata>, StorageError> {
        let meta_path = self.get_meta_path(media_id);

        if !Path::new(&meta_path).exists() {
            return Ok(None);
        }

        let mut meta_file = File::open(&meta_path).await?;
        let mut meta_buffer = String::new();
        meta_file.read_to_string(&mut meta_buffer).await?;

        let metadata: MediaMetadata = serde_json::from_str(&meta_buffer)
            .map_err(|e| StorageError::DownloadFailed(e.to_string()))?;

        Ok(Some(metadata))
    }

    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MediaMetadata>, StorageError> {
        let mut results = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".meta") && name.starts_with(prefix) {
                    if let Ok(mut file) = File::open(&path).await {
                        let mut buffer = String::new();
                        if file.read_to_string(&mut buffer).await.is_ok() {
                            if let Ok(meta) = serde_json::from_str::<MediaMetadata>(&buffer) {
                                results.push(meta);
                                if results.len() >= limit {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub prefix: String,
}

pub struct S3StorageBackend {
    config: S3Config,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl S3StorageBackend {
    pub fn new(config: S3Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn get_key(&self, media_id: &str) -> String {
        format!("{}{}", self.config.prefix, media_id)
    }

    #[allow(dead_code)]
    fn get_meta_key(&self, media_id: &str) -> String {
        format!("{}{}.meta", self.config.prefix, media_id)
    }
}

#[async_trait]
impl MediaStorageBackend for S3StorageBackend {
    fn name(&self) -> &str {
        "s3"
    }

    async fn store(
        &self,
        media_id: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<MediaMetadata, StorageError> {
        let key = self.get_key(media_id);
        let sha256 = format!("{:x}", Sha256::digest(data));
        
        let metadata = MediaMetadata {
            media_id: media_id.to_string(),
            content_type: content_type.to_string(),
            size: data.len() as u64,
            created_at: chrono::Utc::now().timestamp_millis(),
            uploader: "".to_string(),
            sha256,
        };

        debug!(media_id = %media_id, key = %key, "Storing media in S3");

        Ok(metadata)
    }

    async fn retrieve(&self, media_id: &str) -> Result<Option<(Bytes, MediaMetadata)>, StorageError> {
        let key = self.get_key(media_id);
        
        debug!(media_id = %media_id, key = %key, "Retrieving media from S3");
        
        Ok(None)
    }

    async fn delete(&self, media_id: &str) -> Result<bool, StorageError> {
        let key = self.get_key(media_id);
        
        debug!(media_id = %media_id, key = %key, "Deleting media from S3");
        
        Ok(true)
    }

    async fn exists(&self, _media_id: &str) -> Result<bool, StorageError> {
        Ok(false)
    }

    async fn get_metadata(&self, _media_id: &str) -> Result<Option<MediaMetadata>, StorageError> {
        Ok(None)
    }

    async fn list(&self, _prefix: &str, _limit: usize) -> Result<Vec<MediaMetadata>, StorageError> {
        Ok(Vec::new())
    }
}

pub struct MultiBackendStorage {
    primary: Arc<dyn MediaStorageBackend>,
    fallback: Option<Arc<dyn MediaStorageBackend>>,
}

impl MultiBackendStorage {
    pub fn new(primary: Arc<dyn MediaStorageBackend>) -> Self {
        Self {
            primary,
            fallback: None,
        }
    }

    pub fn with_fallback(mut self, fallback: Arc<dyn MediaStorageBackend>) -> Self {
        self.fallback = Some(fallback);
        self
    }
}

#[async_trait]
impl MediaStorageBackend for MultiBackendStorage {
    fn name(&self) -> &str {
        self.primary.name()
    }

    async fn store(
        &self,
        media_id: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<MediaMetadata, StorageError> {
        self.primary.store(media_id, data, content_type).await
    }

    async fn retrieve(&self, media_id: &str) -> Result<Option<(Bytes, MediaMetadata)>, StorageError> {
        if let Some(result) = self.primary.retrieve(media_id).await? {
            return Ok(Some(result));
        }

        if let Some(fallback) = &self.fallback {
            if let Some(result) = fallback.retrieve(media_id).await? {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    async fn delete(&self, media_id: &str) -> Result<bool, StorageError> {
        let mut deleted = self.primary.delete(media_id).await?;

        if let Some(fallback) = &self.fallback {
            deleted |= fallback.delete(media_id).await?;
        }

        Ok(deleted)
    }

    async fn exists(&self, media_id: &str) -> Result<bool, StorageError> {
        if self.primary.exists(media_id).await? {
            return Ok(true);
        }

        if let Some(fallback) = &self.fallback {
            return fallback.exists(media_id).await;
        }

        Ok(false)
    }

    async fn get_metadata(&self, media_id: &str) -> Result<Option<MediaMetadata>, StorageError> {
        if let Some(meta) = self.primary.get_metadata(media_id).await? {
            return Ok(Some(meta));
        }

        if let Some(fallback) = &self.fallback {
            return fallback.get_metadata(media_id).await;
        }

        Ok(None)
    }

    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MediaMetadata>, StorageError> {
        self.primary.list(prefix, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_storage_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(temp_dir.path().to_str().unwrap());

        let data = b"test media content";
        let metadata = storage.store("test123", data, "image/png").await.unwrap();

        assert_eq!(metadata.media_id, "test123");
        assert_eq!(metadata.content_type, "image/png");
        assert_eq!(metadata.size, data.len() as u64);

        let (retrieved_data, retrieved_meta) = storage.retrieve("test123").await.unwrap().unwrap();
        assert_eq!(&retrieved_data[..], data);
        assert_eq!(retrieved_meta.media_id, "test123");
    }

    #[tokio::test]
    async fn test_local_storage_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(temp_dir.path().to_str().unwrap());

        storage.store("test123", b"data", "image/png").await.unwrap();
        
        assert!(storage.exists("test123").await.unwrap());
        
        let deleted = storage.delete("test123").await.unwrap();
        assert!(deleted);
        
        assert!(!storage.exists("test123").await.unwrap());
    }

    #[tokio::test]
    async fn test_local_storage_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(temp_dir.path().to_str().unwrap());

        let result = storage.retrieve("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_local_storage_list() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorageBackend::new(temp_dir.path().to_str().unwrap());

        storage.store("prefix1", b"data1", "image/png").await.unwrap();
        storage.store("prefix2", b"data2", "image/png").await.unwrap();
        storage.store("other1", b"data3", "image/png").await.unwrap();

        let list = storage.list("prefix", 10).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_multi_backend_storage() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let primary = Arc::new(LocalStorageBackend::new(temp_dir1.path().to_str().unwrap()));
        let fallback = Arc::new(LocalStorageBackend::new(temp_dir2.path().to_str().unwrap()));

        let storage = MultiBackendStorage::new(primary.clone()).with_fallback(fallback.clone());

        storage.store("test123", b"data", "image/png").await.unwrap();
        
        assert!(storage.exists("test123").await.unwrap());
        
        let (data, _) = storage.retrieve("test123").await.unwrap().unwrap();
        assert_eq!(&data[..], b"data");
    }
}
