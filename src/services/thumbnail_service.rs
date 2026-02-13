use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Thumbnail size configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailSize {
    pub width: u32,
    pub height: u32,
    pub name: String,
}

impl ThumbnailSize {
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            name: name.to_string(),
        }
    }
}

impl Default for ThumbnailSize {
    fn default() -> Self {
        Self::new("medium", 320, 240)
    }
}

/// Thumbnail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailConfig {
    pub enabled: bool,
    pub sizes: Vec<ThumbnailSize>,
    pub max_source_size: u64,
    pub quality: u8,
    pub allowed_mime_types: Vec<String>,
    pub cache_ttl_seconds: u64,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sizes: vec![
                ThumbnailSize::new("small", 32, 32),
                ThumbnailSize::new("medium", 96, 96),
                ThumbnailSize::new("large", 320, 240),
                ThumbnailSize::new("xlarge", 640, 480),
            ],
            max_source_size: 10 * 1024 * 1024,
            quality: 80,
            allowed_mime_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "image/bmp".to_string(),
            ],
            cache_ttl_seconds: 86400,
        }
    }
}

/// Thumbnail metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailMetadata {
    pub media_id: String,
    pub size_name: String,
    pub width: u32,
    pub height: u32,
    pub content_type: String,
    pub file_size: u64,
    pub created_at: i64,
    pub source_width: u32,
    pub source_height: u32,
}

impl ThumbnailMetadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        media_id: String,
        size_name: String,
        width: u32,
        height: u32,
        content_type: String,
        file_size: u64,
        source_width: u32,
        source_height: u32,
    ) -> Self {
        Self {
            media_id,
            size_name,
            width,
            height,
            content_type,
            file_size,
            created_at: Utc::now().timestamp_millis(),
            source_width,
            source_height,
        }
    }
}

/// Thumbnail generation result
#[derive(Debug, Clone)]
pub struct ThumbnailResult {
    pub data: Vec<u8>,
    pub metadata: ThumbnailMetadata,
}

/// Thumbnail resize method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum ResizeMethod {
    Crop,
    #[default]
    Scale,
    Fit,
}


/// Thumbnail request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailRequest {
    pub media_id: String,
    pub width: u32,
    pub height: u32,
    pub method: ResizeMethod,
    pub animated: bool,
}

impl ThumbnailRequest {
    pub fn new(media_id: String, width: u32, height: u32) -> Self {
        Self {
            media_id,
            width,
            height,
            method: ResizeMethod::default(),
            animated: false,
        }
    }

    pub fn with_method(mut self, method: ResizeMethod) -> Self {
        self.method = method;
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
}

/// Image dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

impl ImageDimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            return 1.0;
        }
        self.width as f32 / self.height as f32
    }

    pub fn scaled_to_fit(&self, max_width: u32, max_height: u32) -> Self {
        let width_ratio = max_width as f32 / self.width as f32;
        let height_ratio = max_height as f32 / self.height as f32;
        let ratio = width_ratio.min(height_ratio);

        Self {
            width: (self.width as f32 * ratio) as u32,
            height: (self.height as f32 * ratio) as u32,
        }
    }

    pub fn scaled_to_cover(&self, min_width: u32, min_height: u32) -> Self {
        let width_ratio = min_width as f32 / self.width as f32;
        let height_ratio = min_height as f32 / self.height as f32;
        let ratio = width_ratio.max(height_ratio);

        Self {
            width: (self.width as f32 * ratio) as u32,
            height: (self.height as f32 * ratio) as u32,
        }
    }
}

/// Thumbnail service
pub struct ThumbnailService {
    config: ThumbnailConfig,
    cache: Arc<RwLock<HashMap<String, ThumbnailMetadata>>>,
}

impl ThumbnailService {
    pub fn new(config: ThumbnailConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn is_mime_type_supported(&self, mime_type: &str) -> bool {
        self.config.allowed_mime_types.iter().any(|t| {
            t == mime_type || mime_type.starts_with(&format!("{}/", t.split('/').next().unwrap_or("")))
        })
    }

    pub fn get_size_by_name(&self, name: &str) -> Option<&ThumbnailSize> {
        self.config.sizes.iter().find(|s| s.name == name)
    }

    pub fn get_closest_size(&self, width: u32, height: u32) -> Option<&ThumbnailSize> {
        self.config
            .sizes
            .iter()
            .min_by_key(|s| {
                let diff_w = (s.width as i32 - width as i32).abs();
                let diff_h = (s.height as i32 - height as i32).abs();
                diff_w + diff_h
            })
    }

    pub async fn generate_thumbnail(
        &self,
        source_data: &[u8],
        source_mime_type: &str,
        request: &ThumbnailRequest,
    ) -> Result<ThumbnailResult, ThumbnailError> {
        if !self.config.enabled {
            return Err(ThumbnailError::Disabled);
        }

        if !self.is_mime_type_supported(source_mime_type) {
            return Err(ThumbnailError::UnsupportedMimeType(source_mime_type.to_string()));
        }

        if source_data.len() as u64 > self.config.max_source_size {
            return Err(ThumbnailError::SourceTooLarge);
        }

        let source_dimensions = self.detect_dimensions(source_data, source_mime_type)?;

        let (target_width, target_height) = match request.method {
            ResizeMethod::Scale => {
                let scaled = source_dimensions.scaled_to_fit(request.width, request.height);
                (scaled.width, scaled.height)
            }
            ResizeMethod::Crop => (request.width, request.height),
            ResizeMethod::Fit => {
                let scaled = source_dimensions.scaled_to_fit(request.width, request.height);
                (scaled.width, scaled.height)
            }
        };

        let thumbnail_data = self.resize_image(
            source_data,
            source_mime_type,
            target_width,
            target_height,
            &request.method,
        )?;

        let output_mime_type = self.get_output_mime_type(source_mime_type);

        let metadata = ThumbnailMetadata::new(
            request.media_id.clone(),
            format!("{}x{}", target_width, target_height),
            target_width,
            target_height,
            output_mime_type.to_string(),
            thumbnail_data.len() as u64,
            source_dimensions.width,
            source_dimensions.height,
        );

        let cache_key = format!("{}:{}:{}:{}", 
            request.media_id, 
            request.width, 
            request.height, 
            request.method.as_str()
        );
        self.cache.write().await.insert(cache_key, metadata.clone());

        info!(
            media_id = %request.media_id,
            width = target_width,
            height = target_height,
            size = thumbnail_data.len(),
            "Thumbnail generated"
        );

        Ok(ThumbnailResult {
            data: thumbnail_data,
            metadata,
        })
    }

    fn detect_dimensions(&self, data: &[u8], mime_type: &str) -> Result<ImageDimensions, ThumbnailError> {
        match mime_type {
            "image/jpeg" | "image/jpg" => self.detect_jpeg_dimensions(data),
            "image/png" => self.detect_png_dimensions(data),
            "image/gif" => self.detect_gif_dimensions(data),
            "image/webp" => self.detect_webp_dimensions(data),
            _ => Err(ThumbnailError::UnsupportedMimeType(mime_type.to_string())),
        }
    }

    fn detect_jpeg_dimensions(&self, data: &[u8]) -> Result<ImageDimensions, ThumbnailError> {
        if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
            return Err(ThumbnailError::InvalidImageData);
        }

        let mut pos = 2;
        while pos < data.len() - 4 {
            if data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = data[pos + 1];
            if marker == 0xC0 || marker == 0xC2 {
                if pos + 9 > data.len() {
                    return Err(ThumbnailError::InvalidImageData);
                }
                let height = u16::from_be_bytes([data[pos + 5], data[pos + 6]]) as u32;
                let width = u16::from_be_bytes([data[pos + 7], data[pos + 8]]) as u32;
                return Ok(ImageDimensions::new(width, height));
            }

            if (0xD0..=0xD9).contains(&marker) {
                pos += 2;
            } else if pos + 4 <= data.len() {
                let length = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
                pos += 2 + length;
            } else {
                break;
            }
        }

        Err(ThumbnailError::InvalidImageData)
    }

    fn detect_png_dimensions(&self, data: &[u8]) -> Result<ImageDimensions, ThumbnailError> {
        if data.len() < 24 {
            return Err(ThumbnailError::InvalidImageData);
        }

        let png_signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        if data[0..8] != png_signature {
            return Err(ThumbnailError::InvalidImageData);
        }

        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        Ok(ImageDimensions::new(width, height))
    }

    fn detect_gif_dimensions(&self, data: &[u8]) -> Result<ImageDimensions, ThumbnailError> {
        if data.len() < 10 {
            return Err(ThumbnailError::InvalidImageData);
        }

        if &data[0..6] != b"GIF87a" && &data[0..6] != b"GIF89a" {
            return Err(ThumbnailError::InvalidImageData);
        }

        let width = u16::from_le_bytes([data[6], data[7]]) as u32;
        let height = u16::from_le_bytes([data[8], data[9]]) as u32;

        Ok(ImageDimensions::new(width, height))
    }

    fn detect_webp_dimensions(&self, data: &[u8]) -> Result<ImageDimensions, ThumbnailError> {
        if data.len() < 30 {
            return Err(ThumbnailError::InvalidImageData);
        }

        if &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
            return Err(ThumbnailError::InvalidImageData);
        }

        let chunk_type = &data[12..16];
        if chunk_type == b"VP8 " {
            if data.len() < 30 {
                return Err(ThumbnailError::InvalidImageData);
            }
            let width = (u16::from_le_bytes([data[26], data[27]]) & 0x3FFF) as u32;
            let height = (u16::from_le_bytes([data[28], data[29]]) & 0x3FFF) as u32;
            return Ok(ImageDimensions::new(width, height));
        } else if chunk_type == b"VP8L" {
            if data.len() < 25 {
                return Err(ThumbnailError::InvalidImageData);
            }
            let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
            let width = (bits & 0x3FFF) + 1;
            let height = ((bits >> 14) & 0x3FFF) + 1;
            return Ok(ImageDimensions::new(width, height));
        }

        Err(ThumbnailError::InvalidImageData)
    }

    fn resize_image(
        &self,
        data: &[u8],
        _mime_type: &str,
        width: u32,
        height: u32,
        _method: &ResizeMethod,
    ) -> Result<Vec<u8>, ThumbnailError> {
        debug!(width, height, "Resizing image (simulated)");

        let mut result = Vec::with_capacity(data.len() / 4);
        let header = format!("THUMBNAIL:{}:{}:", width, height);
        result.extend_from_slice(header.as_bytes());
        result.extend_from_slice(&data[..data.len().min(1024)]);

        Ok(result)
    }

    fn get_output_mime_type(&self, source_mime_type: &str) -> &str {
        match source_mime_type {
            "image/png" => "image/png",
            "image/gif" => "image/gif",
            "image/webp" => "image/webp",
            _ => "image/jpeg",
        }
    }

    pub async fn get_cached_thumbnail(&self, media_id: &str, width: u32, height: u32, method: &ResizeMethod) -> Option<ThumbnailMetadata> {
        let cache_key = format!("{}:{}:{}:{}", media_id, width, height, method.as_str());
        self.cache.read().await.get(&cache_key).cloned()
    }

    pub async fn cleanup_cache(&self) -> usize {
        let mut cache = self.cache.write().await;
        let now = Utc::now().timestamp_millis();
        let ttl_ms = (self.config.cache_ttl_seconds * 1000) as i64;
        
        let before = cache.len();
        cache.retain(|_, m| now - m.created_at < ttl_ms);
        
        let removed = before - cache.len();
        if removed > 0 {
            debug!(count = removed, "Thumbnail cache cleaned up");
        }
        removed
    }

    pub fn get_config(&self) -> &ThumbnailConfig {
        &self.config
    }
}

impl Default for ThumbnailService {
    fn default() -> Self {
        Self::new(ThumbnailConfig::default())
    }
}

impl ResizeMethod {
    pub fn as_str(&self) -> &str {
        match self {
            ResizeMethod::Crop => "crop",
            ResizeMethod::Scale => "scale",
            ResizeMethod::Fit => "fit",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThumbnailError {
    #[error("Thumbnail generation disabled")]
    Disabled,
    #[error("Unsupported MIME type: {0}")]
    UnsupportedMimeType(String),
    #[error("Source image too large")]
    SourceTooLarge,
    #[error("Invalid image data")]
    InvalidImageData,
    #[error("Resize failed: {0}")]
    ResizeFailed(String),
    #[error("IO error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_size() {
        let size = ThumbnailSize::new("medium", 320, 240);
        assert_eq!(size.name, "medium");
        assert_eq!(size.width, 320);
        assert_eq!(size.height, 240);
    }

    #[test]
    fn test_image_dimensions() {
        let dims = ImageDimensions::new(800, 600);
        assert_eq!(dims.width, 800);
        assert_eq!(dims.height, 600);
        assert!((dims.aspect_ratio() - 1.333).abs() < 0.01);
    }

    #[test]
    fn test_scaled_to_fit() {
        let dims = ImageDimensions::new(800, 600);
        let scaled = dims.scaled_to_fit(400, 300);
        assert_eq!(scaled.width, 400);
        assert_eq!(scaled.height, 300);
    }

    #[test]
    fn test_scaled_to_cover() {
        let dims = ImageDimensions::new(800, 600);
        let scaled = dims.scaled_to_cover(400, 400);
        assert!(scaled.width >= 400);
        assert!(scaled.height >= 400);
    }

    #[test]
    fn test_resize_method() {
        assert_eq!(ResizeMethod::Crop.as_str(), "crop");
        assert_eq!(ResizeMethod::Scale.as_str(), "scale");
        assert_eq!(ResizeMethod::Fit.as_str(), "fit");
    }

    #[test]
    fn test_thumbnail_config_default() {
        let config = ThumbnailConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sizes.len(), 4);
        assert!(config.allowed_mime_types.contains(&"image/jpeg".to_string()));
    }

    #[test]
    fn test_is_mime_type_supported() {
        let service = ThumbnailService::default();
        assert!(service.is_mime_type_supported("image/jpeg"));
        assert!(service.is_mime_type_supported("image/png"));
        assert!(!service.is_mime_type_supported("video/mp4"));
    }

    #[test]
    fn test_get_size_by_name() {
        let service = ThumbnailService::default();
        let size = service.get_size_by_name("small");
        assert!(size.is_some());
        assert_eq!(size.unwrap().width, 32);
    }

    #[test]
    fn test_get_closest_size() {
        let service = ThumbnailService::default();
        let size = service.get_closest_size(100, 100);
        assert!(size.is_some());
    }

    #[test]
    fn test_detect_png_dimensions() {
        let service = ThumbnailService::default();
        let mut png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png_data.extend_from_slice(&[0, 0, 0, 13]);
        png_data.extend_from_slice(b"IHDR");
        png_data.extend_from_slice(&[0, 0, 1, 44]);
        png_data.extend_from_slice(&[0, 0, 0, 32]);
        png_data.extend_from_slice(&[0, 0, 0, 0]);

        let dims = service.detect_png_dimensions(&png_data);
        assert!(dims.is_ok());
        let dims = dims.unwrap();
        assert_eq!(dims.width, 300);
        assert_eq!(dims.height, 32);
    }

    #[test]
    fn test_detect_gif_dimensions() {
        let service = ThumbnailService::default();
        let mut gif_data = b"GIF89a".to_vec();
        gif_data.extend_from_slice(&[0x20, 0x01]);
        gif_data.extend_from_slice(&[0xF4, 0x01]);
        gif_data.extend_from_slice(&[0x00, 0x00]);

        let dims = service.detect_gif_dimensions(&gif_data);
        assert!(dims.is_ok());
        let dims = dims.unwrap();
        assert_eq!(dims.width, 288);
        assert_eq!(dims.height, 500);
    }

    #[tokio::test]
    async fn test_thumbnail_request() {
        let request = ThumbnailRequest::new("media123".to_string(), 320, 240)
            .with_method(ResizeMethod::Crop)
            .animated(true);

        assert_eq!(request.media_id, "media123");
        assert_eq!(request.width, 320);
        assert_eq!(request.height, 240);
        assert_eq!(request.method, ResizeMethod::Crop);
        assert!(request.animated);
    }

    #[tokio::test]
    async fn test_generate_thumbnail_disabled() {
        let config = ThumbnailConfig {
            enabled: false,
            ..Default::default()
        };
        let service = ThumbnailService::new(config);

        let request = ThumbnailRequest::new("media123".to_string(), 320, 240);
        let result = service.generate_thumbnail(&[], "image/jpeg", &request).await;

        assert!(matches!(result, Err(ThumbnailError::Disabled)));
    }

    #[tokio::test]
    async fn test_generate_thumbnail_unsupported_mime() {
        let service = ThumbnailService::default();

        let request = ThumbnailRequest::new("media123".to_string(), 320, 240);
        let result = service.generate_thumbnail(&[], "video/mp4", &request).await;

        assert!(matches!(result, Err(ThumbnailError::UnsupportedMimeType(_))));
    }
}
