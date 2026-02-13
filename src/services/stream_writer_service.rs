use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::debug;

/// Stream types for replication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamType {
    Events,
    AccountData,
    Receipts,
    Typing,
    Presence,
    ToDevice,
    DeviceLists,
    Federation,
    PushRules,
    TagAccountData,
    Backfill,
}

impl StreamType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StreamType::Events => "events",
            StreamType::AccountData => "account_data",
            StreamType::Receipts => "receipts",
            StreamType::Typing => "typing",
            StreamType::Presence => "presence",
            StreamType::ToDevice => "to_device",
            StreamType::DeviceLists => "device_lists",
            StreamType::Federation => "federation",
            StreamType::PushRules => "push_rules",
            StreamType::TagAccountData => "tag_account_data",
            StreamType::Backfill => "backfill",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "events" => Some(StreamType::Events),
            "account_data" => Some(StreamType::AccountData),
            "receipts" => Some(StreamType::Receipts),
            "typing" => Some(StreamType::Typing),
            "presence" => Some(StreamType::Presence),
            "to_device" => Some(StreamType::ToDevice),
            "device_lists" => Some(StreamType::DeviceLists),
            "federation" => Some(StreamType::Federation),
            "push_rules" => Some(StreamType::PushRules),
            "tag_account_data" => Some(StreamType::TagAccountData),
            "backfill" => Some(StreamType::Backfill),
            _ => None,
        }
    }
}

/// Stream position/token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPosition {
    pub stream_type: StreamType,
    pub position: u64,
    pub instance_name: String,
    pub timestamp: i64,
}

impl StreamPosition {
    pub fn new(stream_type: StreamType, position: u64, instance_name: String) -> Self {
        Self {
            stream_type,
            position,
            instance_name,
            timestamp: Utc::now().timestamp_millis(),
        }
    }

    pub fn advance(&mut self) {
        self.position += 1;
        self.timestamp = Utc::now().timestamp_millis();
    }
}

/// Stream row for replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRow {
    pub stream_type: StreamType,
    pub position: u64,
    pub data: serde_json::Value,
    pub instance_name: String,
}

/// Stream batch for efficient replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamBatch {
    pub stream_type: StreamType,
    pub rows: Vec<StreamRow>,
    pub start_position: u64,
    pub end_position: u64,
}

impl StreamBatch {
    pub fn new(stream_type: StreamType) -> Self {
        Self {
            stream_type,
            rows: Vec::new(),
            start_position: 0,
            end_position: 0,
        }
    }

    pub fn add_row(&mut self, position: u64, data: serde_json::Value, instance_name: String) {
        if self.rows.is_empty() {
            self.start_position = position;
        }
        self.end_position = position;
        self.rows.push(StreamRow {
            stream_type: self.stream_type,
            position,
            data,
            instance_name,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

/// Stream writer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamWriterConfig {
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub max_pending_rows: usize,
    pub enable_replication: bool,
}

impl Default for StreamWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval_ms: 100,
            max_pending_rows: 10000,
            enable_replication: true,
        }
    }
}

/// Stream writer for a single stream type
pub struct StreamWriter {
    stream_type: StreamType,
    config: StreamWriterConfig,
    position: Arc<RwLock<StreamPosition>>,
    pending_rows: Arc<RwLock<Vec<StreamRow>>>,
    sender: mpsc::Sender<StreamBatch>,
    instance_name: String,
}

impl StreamWriter {
    pub fn new(
        stream_type: StreamType,
        config: StreamWriterConfig,
        sender: mpsc::Sender<StreamBatch>,
        instance_name: String,
    ) -> Self {
        let position = StreamPosition::new(stream_type, 0, instance_name.clone());
        
        Self {
            stream_type,
            config,
            position: Arc::new(RwLock::new(position)),
            pending_rows: Arc::new(RwLock::new(Vec::new())),
            sender,
            instance_name,
        }
    }

    pub async fn write(&self, data: serde_json::Value) -> Result<u64, StreamWriterError> {
        let mut position = self.position.write().await;
        position.advance();
        
        let row = StreamRow {
            stream_type: self.stream_type,
            position: position.position,
            data,
            instance_name: self.instance_name.clone(),
        };

        let mut pending = self.pending_rows.write().await;
        pending.push(row);

        if pending.len() >= self.config.batch_size {
            self.flush_pending(&mut pending).await?;
        }

        Ok(position.position)
    }

    pub async fn write_batch(&self, items: Vec<serde_json::Value>) -> Result<Vec<u64>, StreamWriterError> {
        let mut positions = Vec::with_capacity(items.len());
        
        for data in items {
            let pos = self.write(data).await?;
            positions.push(pos);
        }
        
        Ok(positions)
    }

    async fn flush_pending(&self, pending: &mut Vec<StreamRow>) -> Result<(), StreamWriterError> {
        if pending.is_empty() {
            return Ok(());
        }

        let mut batch = StreamBatch::new(self.stream_type);
        
        for row in pending.drain(..) {
            batch.add_row(row.position, row.data, row.instance_name.clone());
        }

        if self.config.enable_replication {
            let batch_clone = batch.clone();
            self.sender.send(batch_clone).await
                .map_err(|e| StreamWriterError::SendError(e.to_string()))?;
        }

        debug!(
            stream_type = ?self.stream_type,
            count = batch.len(),
            "Flushed stream batch"
        );

        Ok(())
    }

    pub async fn flush(&self) -> Result<(), StreamWriterError> {
        let mut pending = self.pending_rows.write().await;
        self.flush_pending(&mut pending).await
    }

    pub async fn get_position(&self) -> u64 {
        self.position.read().await.position
    }

    pub async fn get_pending_count(&self) -> usize {
        self.pending_rows.read().await.len()
    }
}

/// Multi-stream writer manager
pub struct StreamWriterManager {
    writers: Arc<RwLock<HashMap<StreamType, Arc<StreamWriter>>>>,
    config: StreamWriterConfig,
    batch_sender: mpsc::Sender<StreamBatch>,
    instance_name: String,
}

impl StreamWriterManager {
    pub fn new(config: StreamWriterConfig, instance_name: String) -> (Self, mpsc::Receiver<StreamBatch>) {
        let (sender, receiver) = mpsc::channel(1000);
        
        (
            Self {
                writers: Arc::new(RwLock::new(HashMap::new())),
                config,
                batch_sender: sender,
                instance_name,
            },
            receiver,
        )
    }

    pub async fn get_writer(&self, stream_type: StreamType) -> Arc<StreamWriter> {
        let mut writers = self.writers.write().await;
        
        if !writers.contains_key(&stream_type) {
            let writer = StreamWriter::new(
                stream_type,
                self.config.clone(),
                self.batch_sender.clone(),
                self.instance_name.clone(),
            );
            writers.insert(stream_type, Arc::new(writer));
        }
        
        writers.get(&stream_type).unwrap().clone()
    }

    pub async fn write(&self, stream_type: StreamType, data: serde_json::Value) -> Result<u64, StreamWriterError> {
        let writer = self.get_writer(stream_type).await;
        writer.write(data).await
    }

    pub async fn flush_all(&self) -> Result<(), StreamWriterError> {
        let writers = self.writers.read().await;
        
        for writer in writers.values() {
            writer.flush().await?;
        }
        
        Ok(())
    }

    pub async fn get_positions(&self) -> HashMap<StreamType, u64> {
        let writers = self.writers.read().await;
        let mut positions = HashMap::new();
        
        for (stream_type, writer) in writers.iter() {
            positions.insert(*stream_type, writer.get_position().await);
        }
        
        positions
    }

    pub async fn get_pending_counts(&self) -> HashMap<StreamType, usize> {
        let writers = self.writers.read().await;
        let mut counts = HashMap::new();
        
        for (stream_type, writer) in writers.iter() {
            counts.insert(*stream_type, writer.get_pending_count().await);
        }
        
        counts
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamWriterError {
    #[error("Send error: {0}")]
    SendError(String),
    #[error("Flush error: {0}")]
    FlushError(String),
    #[error("Invalid stream type")]
    InvalidStreamType,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stream_type() {
        assert_eq!(StreamType::Events.as_str(), "events");
        assert_eq!(StreamType::from_str("events"), Some(StreamType::Events));
        assert_eq!(StreamType::from_str("invalid"), None);
    }

    #[test]
    fn test_stream_position() {
        let mut pos = StreamPosition::new(StreamType::Events, 100, "master".to_string());
        assert_eq!(pos.position, 100);
        
        pos.advance();
        assert_eq!(pos.position, 101);
    }

    #[test]
    fn test_stream_batch() {
        let mut batch = StreamBatch::new(StreamType::Events);
        assert!(batch.is_empty());
        
        batch.add_row(1, json!({"test": "data"}), "master".to_string());
        batch.add_row(2, json!({"test": "data2"}), "master".to_string());
        
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.start_position, 1);
        assert_eq!(batch.end_position, 2);
    }

    #[tokio::test]
    async fn test_stream_writer() {
        let config = StreamWriterConfig::default();
        let (sender, mut receiver) = mpsc::channel(100);
        
        let writer = StreamWriter::new(
            StreamType::Events,
            config,
            sender,
            "master".to_string(),
        );

        let pos = writer.write(json!({"event": "test"})).await.unwrap();
        assert_eq!(pos, 1);
        
        writer.flush().await.unwrap();
        
        let batch = receiver.recv().await.unwrap();
        assert_eq!(batch.len(), 1);
    }

    #[tokio::test]
    async fn test_stream_writer_manager() {
        let config = StreamWriterConfig {
            batch_size: 2,
            ..Default::default()
        };
        
        let (manager, mut receiver) = StreamWriterManager::new(config, "master".to_string());
        
        let pos1 = manager.write(StreamType::Events, json!({"event": 1})).await.unwrap();
        let pos2 = manager.write(StreamType::Events, json!({"event": 2})).await.unwrap();
        
        assert_eq!(pos1, 1);
        assert_eq!(pos2, 2);
        
        let batch = receiver.recv().await.unwrap();
        assert_eq!(batch.len(), 2);
    }

    #[tokio::test]
    async fn test_get_positions() {
        let config = StreamWriterConfig::default();
        let (manager, _) = StreamWriterManager::new(config, "master".to_string());
        
        manager.write(StreamType::Events, json!({"event": 1})).await.unwrap();
        manager.write(StreamType::AccountData, json!({"data": 1})).await.unwrap();
        
        let positions = manager.get_positions().await;
        
        assert_eq!(positions.get(&StreamType::Events), Some(&1));
        assert_eq!(positions.get(&StreamType::AccountData), Some(&1));
    }
}
