use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStatus {
    Pending,
    Running,
    Completed,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundUpdate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub status: UpdateStatus,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub progress: f32,
    pub total_items: u64,
    pub processed_items: u64,
    pub error_message: Option<String>,
    pub depends_on: Vec<String>,
    pub can_rollback: bool,
}

impl BackgroundUpdate {
    pub fn new(name: String, description: String, version: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            version,
            status: UpdateStatus::Pending,
            created_at: Utc::now().timestamp_millis(),
            started_at: None,
            completed_at: None,
            progress: 0.0,
            total_items: 0,
            processed_items: 0,
            error_message: None,
            depends_on: Vec::new(),
            can_rollback: false,
        }
    }

    pub fn with_total_items(mut self, total: u64) -> Self {
        self.total_items = total;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn with_rollback(mut self, can_rollback: bool) -> Self {
        self.can_rollback = can_rollback;
        self
    }

    pub fn start(&mut self) {
        self.status = UpdateStatus::Running;
        self.started_at = Some(Utc::now().timestamp_millis());
    }

    pub fn update_progress(&mut self, processed: u64) {
        self.processed_items = processed;
        if self.total_items > 0 {
            self.progress = (processed as f64 / self.total_items as f64) as f32;
        }
    }

    pub fn complete(&mut self) {
        self.status = UpdateStatus::Completed;
        self.completed_at = Some(Utc::now().timestamp_millis());
        self.progress = 100.0;
        self.processed_items = self.total_items;
    }

    pub fn fail(&mut self, error: String) {
        self.status = UpdateStatus::Failed;
        self.error_message = Some(error);
    }

    pub fn rollback(&mut self) {
        self.status = UpdateStatus::RolledBack;
        self.completed_at = Some(Utc::now().timestamp_millis());
    }

    pub fn is_ready(&self, completed_updates: &[String]) -> bool {
        self.status == UpdateStatus::Pending
            && self
                .depends_on
                .iter()
                .all(|dep| completed_updates.contains(dep))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub max_concurrent_updates: usize,
    pub batch_size: usize,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            max_concurrent_updates: 3,
            batch_size: 100,
            retry_count: 3,
            retry_delay_ms: 1000,
        }
    }
}

pub struct BackgroundUpdateService {
    updates: Arc<RwLock<HashMap<String, BackgroundUpdate>>>,
    config: UpdateConfig,
}

impl BackgroundUpdateService {
    pub fn new(config: UpdateConfig) -> Self {
        Self {
            updates: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn register_update(&self, update: BackgroundUpdate) -> Result<String, UpdateError> {
        let id = update.id.clone();
        
        self.updates.write().await.insert(id.clone(), update.clone());

        info!(
            update_id = %id,
            name = %update.name,
            version = %update.version,
            "Background update registered"
        );

        Ok(id)
    }

    pub async fn get_pending_updates(&self) -> Vec<BackgroundUpdate> {
        let updates = self.updates.read().await;
        let completed: Vec<String> = updates
            .values()
            .filter(|u| u.status == UpdateStatus::Completed)
            .map(|u| u.name.clone())
            .collect();

        updates
            .values()
            .filter(|u| u.is_ready(&completed))
            .cloned()
            .collect()
    }

    pub async fn get_running_updates(&self) -> Vec<BackgroundUpdate> {
        self.updates
            .read()
            .await
            .values()
            .filter(|u| u.status == UpdateStatus::Running)
            .cloned()
            .collect()
    }

    pub async fn start_update(&self, update_id: &str) -> Result<(), UpdateError> {
        let completed: Vec<String> = {
            let updates = self.updates.read().await;
            updates
                .values()
                .filter(|u| u.status == UpdateStatus::Completed)
                .map(|u| u.name.clone())
                .collect()
        };

        let mut updates = self.updates.write().await;
        let update = updates
            .get_mut(update_id)
            .ok_or(UpdateError::NotFound)?;

        if update.status != UpdateStatus::Pending {
            return Err(UpdateError::InvalidStatus);
        }

        if !update.is_ready(&completed) {
            return Err(UpdateError::DependenciesNotMet);
        }

        update.start();

        info!(update_id = %update_id, name = %update.name, "Background update started");

        Ok(())
    }

    pub async fn update_progress(
        &self,
        update_id: &str,
        processed: u64,
    ) -> Result<(), UpdateError> {
        let mut updates = self.updates.write().await;
        let update = updates
            .get_mut(update_id)
            .ok_or(UpdateError::NotFound)?;

        if update.status != UpdateStatus::Running {
            return Err(UpdateError::InvalidStatus);
        }

        update.update_progress(processed);

        debug!(
            update_id = %update_id,
            progress = %update.progress,
            processed = processed,
            total = update.total_items,
            "Update progress"
        );

        Ok(())
    }

    pub async fn complete_update(&self, update_id: &str) -> Result<(), UpdateError> {
        let mut updates = self.updates.write().await;
        let update = updates
            .get_mut(update_id)
            .ok_or(UpdateError::NotFound)?;

        if update.status != UpdateStatus::Running {
            return Err(UpdateError::InvalidStatus);
        }

        update.complete();

        info!(
            update_id = %update_id,
            name = %update.name,
            duration_ms = update.completed_at.unwrap_or(0) - update.started_at.unwrap_or(0),
            "Background update completed"
        );

        Ok(())
    }

    pub async fn fail_update(&self, update_id: &str, error: String) -> Result<(), UpdateError> {
        let mut updates = self.updates.write().await;
        let update = updates
            .get_mut(update_id)
            .ok_or(UpdateError::NotFound)?;

        update.fail(error.clone());

        error!(
            update_id = %update_id,
            name = %update.name,
            error = %error,
            "Background update failed"
        );

        Ok(())
    }

    pub async fn rollback_update(&self, update_id: &str) -> Result<(), UpdateError> {
        let mut updates = self.updates.write().await;
        let update = updates
            .get_mut(update_id)
            .ok_or(UpdateError::NotFound)?;

        if !update.can_rollback {
            return Err(UpdateError::RollbackNotSupported);
        }

        if update.status != UpdateStatus::Completed && update.status != UpdateStatus::Failed {
            return Err(UpdateError::InvalidStatus);
        }

        update.rollback();

        warn!(
            update_id = %update_id,
            name = %update.name,
            "Background update rolled back"
        );

        Ok(())
    }

    pub async fn get_update(&self, update_id: &str) -> Option<BackgroundUpdate> {
        self.updates.read().await.get(update_id).cloned()
    }

    pub async fn get_all_updates(&self) -> Vec<BackgroundUpdate> {
        self.updates.read().await.values().cloned().collect()
    }

    pub async fn get_stats(&self) -> UpdateStats {
        let updates = self.updates.read().await;

        let mut stats = UpdateStats::default();

        for update in updates.values() {
            match update.status {
                UpdateStatus::Pending => stats.pending += 1,
                UpdateStatus::Running => stats.running += 1,
                UpdateStatus::Completed => stats.completed += 1,
                UpdateStatus::Failed => stats.failed += 1,
                UpdateStatus::RolledBack => stats.rolled_back += 1,
            }
        }

        stats.total = updates.len() as u64;
        stats
    }

    pub async fn can_run_more(&self) -> bool {
        let running = self.get_running_updates().await;
        running.len() < self.config.max_concurrent_updates
    }
}

impl Default for BackgroundUpdateService {
    fn default() -> Self {
        Self::new(UpdateConfig::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateStats {
    pub total: u64,
    pub pending: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
    pub rolled_back: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Update not found")]
    NotFound,
    #[error("Invalid update status")]
    InvalidStatus,
    #[error("Dependencies not met")]
    DependenciesNotMet,
    #[error("Rollback not supported")]
    RollbackNotSupported,
    #[error("Update already exists")]
    AlreadyExists,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_update() {
        let service = BackgroundUpdateService::default();

        let update = BackgroundUpdate::new(
            "test_update".to_string(),
            "Test update description".to_string(),
            "1.0.0".to_string(),
        );

        let id = service.register_update(update).await.unwrap();

        let retrieved = service.get_update(&id).await.unwrap();
        assert_eq!(retrieved.name, "test_update");
        assert_eq!(retrieved.status, UpdateStatus::Pending);
    }

    #[tokio::test]
    async fn test_start_update() {
        let service = BackgroundUpdateService::default();

        let update = BackgroundUpdate::new(
            "test_update".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        );

        let id = service.register_update(update).await.unwrap();

        service.start_update(&id).await.unwrap();

        let started = service.get_update(&id).await.unwrap();
        assert_eq!(started.status, UpdateStatus::Running);
        assert!(started.started_at.is_some());
    }

    #[tokio::test]
    async fn test_update_progress() {
        let service = BackgroundUpdateService::default();

        let update = BackgroundUpdate::new(
            "test_update".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        )
        .with_total_items(100);

        let id = service.register_update(update).await.unwrap();
        service.start_update(&id).await.unwrap();

        service.update_progress(&id, 50).await.unwrap();

        let updated = service.get_update(&id).await.unwrap();
        assert_eq!(updated.processed_items, 50);
        assert!((updated.progress - 0.5).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_complete_update() {
        let service = BackgroundUpdateService::default();

        let update = BackgroundUpdate::new(
            "test_update".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        );

        let id = service.register_update(update).await.unwrap();
        service.start_update(&id).await.unwrap();
        service.complete_update(&id).await.unwrap();

        let completed = service.get_update(&id).await.unwrap();
        assert_eq!(completed.status, UpdateStatus::Completed);
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_fail_update() {
        let service = BackgroundUpdateService::default();

        let update = BackgroundUpdate::new(
            "test_update".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        );

        let id = service.register_update(update).await.unwrap();
        service.start_update(&id).await.unwrap();
        service
            .fail_update(&id, "Something went wrong".to_string())
            .await
            .unwrap();

        let failed = service.get_update(&id).await.unwrap();
        assert_eq!(failed.status, UpdateStatus::Failed);
        assert_eq!(failed.error_message, Some("Something went wrong".to_string()));
    }

    #[tokio::test]
    async fn test_rollback_update() {
        let service = BackgroundUpdateService::default();

        let update = BackgroundUpdate::new(
            "test_update".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        )
        .with_rollback(true);

        let id = service.register_update(update).await.unwrap();
        service.start_update(&id).await.unwrap();
        service.complete_update(&id).await.unwrap();
        service.rollback_update(&id).await.unwrap();

        let rolled_back = service.get_update(&id).await.unwrap();
        assert_eq!(rolled_back.status, UpdateStatus::RolledBack);
    }

    #[tokio::test]
    async fn test_dependencies() {
        let service = BackgroundUpdateService::default();

        let update1 = BackgroundUpdate::new(
            "update1".to_string(),
            "First update".to_string(),
            "1.0.0".to_string(),
        );

        let update2 = BackgroundUpdate::new(
            "update2".to_string(),
            "Second update".to_string(),
            "1.0.0".to_string(),
        )
        .with_dependencies(vec!["update1".to_string()]);

        let id1 = service.register_update(update1).await.unwrap();
        let _id2 = service.register_update(update2).await.unwrap();

        let pending = service.get_pending_updates().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name, "update1");

        service.start_update(&id1).await.unwrap();
        service.complete_update(&id1).await.unwrap();

        let pending = service.get_pending_updates().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name, "update2");
    }

    #[tokio::test]
    async fn test_stats() {
        let service = BackgroundUpdateService::default();

        for i in 0..5 {
            let update = BackgroundUpdate::new(
                format!("update_{}", i),
                "Test".to_string(),
                "1.0.0".to_string(),
            );
            service.register_update(update).await.unwrap();
        }

        let stats = service.get_stats().await;
        assert_eq!(stats.total, 5);
        assert_eq!(stats.pending, 5);
    }
}
