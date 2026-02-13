use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationTransaction {
    pub transaction_id: String,
    pub origin: String,
    pub destination: String,
    pub pdus: Vec<serde_json::Value>,
    pub edus: Vec<serde_json::Value>,
    pub created_at: i64,
    pub sent_at: Option<i64>,
    pub retry_count: u32,
    pub status: TransactionStatus,
}

impl FederationTransaction {
    pub fn new(origin: String, destination: String) -> Self {
        Self {
            transaction_id: format!("{}_{}", origin, Utc::now().timestamp_millis()),
            origin,
            destination,
            pdus: Vec::new(),
            edus: Vec::new(),
            created_at: Utc::now().timestamp_millis(),
            sent_at: None,
            retry_count: 0,
            status: TransactionStatus::Pending,
        }
    }

    pub fn with_pdus(mut self, pdus: Vec<serde_json::Value>) -> Self {
        self.pdus = pdus;
        self
    }

    pub fn with_edus(mut self, edus: Vec<serde_json::Value>) -> Self {
        self.edus = edus;
        self
    }

    pub fn mark_sent(&mut self) {
        self.sent_at = Some(Utc::now().timestamp_millis());
        self.status = TransactionStatus::Sent;
    }

    pub fn mark_failed(&mut self) {
        self.retry_count += 1;
        self.status = TransactionStatus::Failed;
    }

    pub fn mark_delivered(&mut self) {
        self.status = TransactionStatus::Delivered;
    }

    pub fn is_empty(&self) -> bool {
        self.pdus.is_empty() && self.edus.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDestination {
    pub server_name: String,
    pub last_successful_send: Option<i64>,
    pub consecutive_failures: u32,
    pub is_active: bool,
    pub retry_delay_ms: u64,
}

impl FederationDestination {
    pub fn new(server_name: String) -> Self {
        Self {
            server_name,
            last_successful_send: None,
            consecutive_failures: 0,
            is_active: true,
            retry_delay_ms: 1000,
        }
    }

    pub fn record_success(&mut self) {
        self.last_successful_send = Some(Utc::now().timestamp_millis());
        self.consecutive_failures = 0;
        self.retry_delay_ms = 1000;
        self.is_active = true;
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.retry_delay_ms = std::cmp::min(self.retry_delay_ms * 2, 300_000);
        
        if self.consecutive_failures >= 5 {
            self.is_active = false;
        }
    }

    pub fn can_send(&self) -> bool {
        if !self.is_active {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSenderConfig {
    pub max_transactions_per_destination: usize,
    pub max_pdu_per_transaction: usize,
    pub max_edu_per_transaction: usize,
    pub transaction_timeout_ms: u64,
    pub max_retry_count: u32,
}

impl Default for FederationSenderConfig {
    fn default() -> Self {
        Self {
            max_transactions_per_destination: 100,
            max_pdu_per_transaction: 50,
            max_edu_per_transaction: 100,
            transaction_timeout_ms: 30000,
            max_retry_count: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FederationStats {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub pending_transactions: u64,
    pub active_destinations: usize,
}

pub struct FederationSenderService {
    config: FederationSenderConfig,
    destinations: Arc<RwLock<HashMap<String, FederationDestination>>>,
    pending_transactions: Arc<RwLock<HashMap<String, VecDeque<FederationTransaction>>>>,
    stats: Arc<RwLock<FederationStats>>,
}

impl FederationSenderService {
    pub fn new(config: FederationSenderConfig) -> Self {
        Self {
            config,
            destinations: Arc::new(RwLock::new(HashMap::new())),
            pending_transactions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(FederationStats::default())),
        }
    }

    pub async fn send_transaction(
        &self,
        destination: &str,
        pdus: Vec<serde_json::Value>,
        edus: Vec<serde_json::Value>,
    ) -> Result<String, FederationError> {
        let mut transaction = FederationTransaction::new(
            "self".to_string(),
            destination.to_string(),
        );
        transaction.pdus = pdus;
        transaction.edus = edus;

        if transaction.is_empty() {
            return Err(FederationError::EmptyTransaction);
        }

        let transaction_id = transaction.transaction_id.clone();

        self.ensure_destination(destination).await;

        let mut pending = self.pending_transactions.write().await;
        let queue = pending.entry(destination.to_string()).or_insert_with(VecDeque::new);
        
        if queue.len() >= self.config.max_transactions_per_destination {
            return Err(FederationError::QueueFull);
        }

        queue.push_back(transaction);

        let mut stats = self.stats.write().await;
        stats.total_transactions += 1;
        stats.pending_transactions += 1;

        info!(
            transaction_id = %transaction_id,
            destination = %destination,
            pdu_count = queue.back().unwrap().pdus.len(),
            edu_count = queue.back().unwrap().edus.len(),
            "Transaction queued for federation"
        );

        Ok(transaction_id)
    }

    async fn ensure_destination(&self, server_name: &str) {
        let mut destinations = self.destinations.write().await;
        if !destinations.contains_key(server_name) {
            destinations.insert(server_name.to_string(), FederationDestination::new(server_name.to_string()));
        }
    }

    pub async fn process_pending(&self) -> Result<usize, FederationError> {
        let mut processed = 0;
        let mut pending = self.pending_transactions.write().await;
        let mut destinations = self.destinations.write().await;

        for (server_name, queue) in pending.iter_mut() {
            let destination = destinations.get_mut(server_name).unwrap();
            
            if !destination.can_send() {
                continue;
            }

            while let Some(mut transaction) = queue.pop_front() {
                if transaction.retry_count >= self.config.max_retry_count {
                    warn!(
                        transaction_id = %transaction.transaction_id,
                        destination = %server_name,
                        "Transaction exceeded max retries"
                    );
                    continue;
                }

                transaction.mark_sent();
                processed += 1;

                destination.record_success();
                transaction.mark_delivered();

                debug!(
                    transaction_id = %transaction.transaction_id,
                    destination = %server_name,
                    "Transaction processed"
                );
            }
        }

        let mut stats = self.stats.write().await;
        stats.successful_transactions += processed as u64;
        stats.pending_transactions = pending.values().map(|q| q.len() as u64).sum();
        stats.active_destinations = destinations.values().filter(|d| d.is_active).count();

        Ok(processed)
    }

    pub async fn get_destination(&self, server_name: &str) -> Option<FederationDestination> {
        self.destinations.read().await.get(server_name).cloned()
    }

    pub async fn get_pending_count(&self, destination: &str) -> usize {
        self.pending_transactions
            .read()
            .await
            .get(destination)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub async fn get_stats(&self) -> FederationStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.pending_transactions = self.pending_transactions
            .read()
            .await
            .values()
            .map(|q| q.len() as u64)
            .sum();
        result.active_destinations = self.destinations
            .read()
            .await
            .values()
            .filter(|d| d.is_active)
            .count();
        result
    }

    pub async fn retry_destination(&self, server_name: &str) -> bool {
        let mut destinations = self.destinations.write().await;
        if let Some(destination) = destinations.get_mut(server_name) {
            destination.is_active = true;
            destination.consecutive_failures = 0;
            destination.retry_delay_ms = 1000;
            info!(server_name = %server_name, "Destination reactivated for retry");
            true
        } else {
            false
        }
    }

    pub async fn clear_destination(&self, server_name: &str) {
        self.pending_transactions.write().await.remove(server_name);
        self.destinations.write().await.remove(server_name);
        info!(server_name = %server_name, "Destination cleared");
    }
}

impl Default for FederationSenderService {
    fn default() -> Self {
        Self::new(FederationSenderConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    #[error("Empty transaction")]
    EmptyTransaction,
    #[error("Queue full for destination")]
    QueueFull,
    #[error("Destination unavailable")]
    DestinationUnavailable,
    #[error("Transaction timeout")]
    Timeout,
    #[error("Send failed: {0}")]
    SendFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_send_transaction() {
        let sender = FederationSenderService::default();

        let pdus = vec![json!({"type": "m.room.message"})];
        let edus = vec![json!({"type": "m.presence"})];

        let tx_id = sender
            .send_transaction("server.example.com", pdus, edus)
            .await
            .unwrap();

        assert!(!tx_id.is_empty());
        assert_eq!(sender.get_pending_count("server.example.com").await, 1);
    }

    #[tokio::test]
    async fn test_empty_transaction() {
        let sender = FederationSenderService::default();

        let result = sender
            .send_transaction("server.example.com", vec![], vec![])
            .await;

        assert!(matches!(result, Err(FederationError::EmptyTransaction)));
    }

    #[tokio::test]
    async fn test_process_pending() {
        let sender = FederationSenderService::default();

        sender
            .send_transaction("server.example.com", vec![json!({"test": 1})], vec![])
            .await
            .unwrap();

        let processed = sender.process_pending().await.unwrap();
        assert_eq!(processed, 1);
        assert_eq!(sender.get_pending_count("server.example.com").await, 0);
    }

    #[tokio::test]
    async fn test_destination_tracking() {
        let sender = FederationSenderService::default();

        sender
            .send_transaction("server.example.com", vec![json!({"test": 1})], vec![])
            .await
            .unwrap();

        let dest = sender.get_destination("server.example.com").await.unwrap();
        assert!(dest.is_active);
        assert_eq!(dest.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_retry_destination() {
        let sender = FederationSenderService::default();

        sender
            .send_transaction("server.example.com", vec![json!({"test": 1})], vec![])
            .await
            .unwrap();

        let result = sender.retry_destination("server.example.com").await;
        assert!(result);

        let result = sender.retry_destination("nonexistent.example.com").await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_stats() {
        let sender = FederationSenderService::default();

        sender
            .send_transaction("server1.example.com", vec![json!({"test": 1})], vec![])
            .await
            .unwrap();

        sender
            .send_transaction("server2.example.com", vec![json!({"test": 2})], vec![])
            .await
            .unwrap();

        let stats = sender.get_stats().await;
        assert_eq!(stats.total_transactions, 2);
        assert_eq!(stats.pending_transactions, 2);
        assert_eq!(stats.active_destinations, 2);
    }

    #[tokio::test]
    async fn test_clear_destination() {
        let sender = FederationSenderService::default();

        sender
            .send_transaction("server.example.com", vec![json!({"test": 1})], vec![])
            .await
            .unwrap();

        sender.clear_destination("server.example.com").await;

        assert_eq!(sender.get_pending_count("server.example.com").await, 0);
        assert!(sender.get_destination("server.example.com").await.is_none());
    }
}
