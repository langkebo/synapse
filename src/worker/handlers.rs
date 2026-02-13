use async_trait::async_trait;
use serde_json::Value;
use std::time::Instant;

#[async_trait]
pub trait TaskHandler: Send + Sync {
    fn name(&self) -> &str;
    async fn handle(&self, payload: Value) -> Result<Value, TaskHandlerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TaskHandlerError {
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("Processing error: {0}")]
    ProcessingError(String),
    #[error("Timeout")]
    Timeout,
    #[error("Retryable error: {0}")]
    Retryable(String),
}

pub struct TaskHandlerContext {
    pub task_id: String,
    pub worker_id: String,
    pub started_at: Instant,
    pub retry_count: usize,
}

pub mod event_handlers {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PersistEventPayload {
        pub event_id: String,
        pub room_id: String,
        pub event_type: String,
        pub content: Value,
        pub sender: String,
    }

    pub struct PersistEventHandler;

    #[async_trait]
    impl TaskHandler for PersistEventHandler {
        fn name(&self) -> &str {
            "persist_event"
        }

        async fn handle(&self, payload: Value) -> Result<Value, TaskHandlerError> {
            let event: PersistEventPayload = serde_json::from_value(payload)
                .map_err(|e| TaskHandlerError::InvalidPayload(e.to_string()))?;

            tracing::info!(
                event_id = %event.event_id,
                room_id = %event.room_id,
                "Persisting event"
            );

            Ok(serde_json::json!({
                "event_id": event.event_id,
                "persisted": true
            }))
        }
    }
}

pub mod federation_handlers {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SendFederationPayload {
        pub destination: String,
        pub txn_id: String,
        pub pdus: Vec<Value>,
        pub edus: Vec<Value>,
    }

    pub struct SendFederationHandler;

    #[async_trait]
    impl TaskHandler for SendFederationHandler {
        fn name(&self) -> &str {
            "send_federation"
        }

        async fn handle(&self, payload: Value) -> Result<Value, TaskHandlerError> {
            let fed: SendFederationPayload = serde_json::from_value(payload)
                .map_err(|e| TaskHandlerError::InvalidPayload(e.to_string()))?;

            tracing::info!(
                destination = %fed.destination,
                txn_id = %fed.txn_id,
                pdu_count = fed.pdus.len(),
                "Sending federation transaction"
            );

            Ok(serde_json::json!({
                "destination": fed.destination,
                "txn_id": fed.txn_id,
                "sent": true
            }))
        }
    }
}

pub mod push_handlers {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SendPushPayload {
        pub device_pushkey: String,
        pub app_id: String,
        pub notification: Value,
    }

    pub struct SendPushHandler;

    #[async_trait]
    impl TaskHandler for SendPushHandler {
        fn name(&self) -> &str {
            "send_push"
        }

        async fn handle(&self, payload: Value) -> Result<Value, TaskHandlerError> {
            let push: SendPushPayload = serde_json::from_value(payload)
                .map_err(|e| TaskHandlerError::InvalidPayload(e.to_string()))?;

            tracing::info!(
                pushkey = %push.device_pushkey,
                app_id = %push.app_id,
                "Sending push notification"
            );

            Ok(serde_json::json!({
                "pushkey": push.device_pushkey,
                "sent": true
            }))
        }
    }
}

pub mod media_handlers {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ProcessMediaPayload {
        pub media_id: String,
        pub media_type: String,
        pub operations: Vec<String>,
    }

    pub struct ProcessMediaHandler;

    #[async_trait]
    impl TaskHandler for ProcessMediaHandler {
        fn name(&self) -> &str {
            "process_media"
        }

        async fn handle(&self, payload: Value) -> Result<Value, TaskHandlerError> {
            let media: ProcessMediaPayload = serde_json::from_value(payload)
                .map_err(|e| TaskHandlerError::InvalidPayload(e.to_string()))?;

            tracing::info!(
                media_id = %media.media_id,
                media_type = %media.media_type,
                operations = ?media.operations,
                "Processing media"
            );

            Ok(serde_json::json!({
                "media_id": media.media_id,
                "processed": true,
                "thumbnails_generated": media.operations.contains(&"thumbnail".to_string())
            }))
        }
    }
}
