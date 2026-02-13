use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::info;

pub mod handlers;
pub mod replication;
pub mod scheduler;
pub mod types;

pub use replication::*;
pub use types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerType {
    #[serde(rename = "event_persister")]
    EventPersister,
    #[serde(rename = "federation_sender")]
    FederationSender,
    #[serde(rename = "federation_reader")]
    FederationReader,
    #[serde(rename = "pusher")]
    Pusher,
    #[serde(rename = "media_repository")]
    MediaRepository,
    #[serde(rename = "synchrotron")]
    Synchrotron,
    #[serde(rename = "client_reader")]
    ClientReader,
    #[serde(rename = "client_writer")]
    ClientWriter,
    #[serde(rename = "background_worker")]
    BackgroundWorker,
}

impl WorkerType {
    pub fn all() -> Vec<WorkerType> {
        vec![
            WorkerType::EventPersister,
            WorkerType::FederationSender,
            WorkerType::FederationReader,
            WorkerType::Pusher,
            WorkerType::MediaRepository,
            WorkerType::Synchrotron,
            WorkerType::ClientReader,
            WorkerType::ClientWriter,
            WorkerType::BackgroundWorker,
        ]
    }

    pub fn default_concurrency(&self) -> usize {
        match self {
            WorkerType::EventPersister => 4,
            WorkerType::FederationSender => 8,
            WorkerType::FederationReader => 4,
            WorkerType::Pusher => 4,
            WorkerType::MediaRepository => 2,
            WorkerType::Synchrotron => 4,
            WorkerType::ClientReader => 8,
            WorkerType::ClientWriter => 4,
            WorkerType::BackgroundWorker => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: String,
    pub worker_type: WorkerType,
    pub instance_name: String,
    pub started_at: Instant,
    pub last_heartbeat: Instant,
    pub status: WorkerStatus,
    pub current_tasks: usize,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Starting,
    Running,
    Draining,
    Stopping,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMetrics {
    pub worker_id: String,
    pub worker_type: WorkerType,
    pub status: WorkerStatus,
    pub uptime_seconds: u64,
    pub current_tasks: usize,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub avg_task_duration_ms: f64,
    pub queue_depth: usize,
}

pub struct WorkerCoordinator {
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    task_sender: mpsc::UnboundedSender<WorkerTask>,
    shutdown_tx: broadcast::Sender<()>,
    config: WorkerCoordinatorConfig,
}

#[derive(Debug, Clone)]
pub struct WorkerCoordinatorConfig {
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout: Duration,
    pub max_retries: usize,
    pub task_timeout: Duration,
}

impl Default for WorkerCoordinatorConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            heartbeat_timeout: Duration::from_secs(30),
            max_retries: 3,
            task_timeout: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTask {
    pub id: String,
    pub task_type: String,
    pub priority: TaskPriority,
    pub payload: serde_json::Value,
    pub created_at: Instant,
    pub retry_count: usize,
    pub assigned_worker: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl WorkerCoordinator {
    pub fn new(config: WorkerCoordinatorConfig) -> (Self, mpsc::UnboundedReceiver<WorkerTask>) {
        let (task_sender, task_receiver) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = broadcast::channel(1);

        let coordinator = Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            task_sender,
            shutdown_tx,
            config,
        };

        (coordinator, task_receiver)
    }

    pub async fn register_worker(&self, info: WorkerInfo) {
        let mut workers = self.workers.write().await;
        info!(
            worker_id = %info.id,
            worker_type = ?info.worker_type,
            "Worker registered"
        );
        workers.insert(info.id.clone(), info);
    }

    pub async fn unregister_worker(&self, worker_id: &str) {
        let mut workers = self.workers.write().await;
        if let Some(info) = workers.remove(worker_id) {
            info!(
                worker_id = %worker_id,
                worker_type = ?info.worker_type,
                "Worker unregistered"
            );
        }
    }

    pub async fn update_heartbeat(&self, worker_id: &str) -> bool {
        let mut workers = self.workers.write().await;
        if let Some(info) = workers.get_mut(worker_id) {
            info.last_heartbeat = Instant::now();
            info.status = WorkerStatus::Running;
            true
        } else {
            false
        }
    }

    pub async fn get_available_workers(&self, worker_type: WorkerType) -> Vec<WorkerInfo> {
        let workers = self.workers.read().await;
        workers
            .values()
            .filter(|w| w.worker_type == worker_type && w.status == WorkerStatus::Running)
            .cloned()
            .collect()
    }

    pub async fn select_worker(&self, worker_type: WorkerType) -> Option<String> {
        let workers = self.workers.read().await;
        workers
            .values()
            .filter(|w| w.worker_type == worker_type && w.status == WorkerStatus::Running)
            .min_by_key(|w| w.current_tasks)
            .map(|w| w.id.clone())
    }

    pub async fn submit_task(&self, task: WorkerTask) -> Result<(), String> {
        self.task_sender
            .send(task)
            .map_err(|e| format!("Failed to submit task: {}", e))
    }

    pub async fn get_all_metrics(&self) -> Vec<WorkerMetrics> {
        let workers = self.workers.read().await;
        workers
            .values()
            .map(|w| WorkerMetrics {
                worker_id: w.id.clone(),
                worker_type: w.worker_type,
                status: w.status,
                uptime_seconds: w.started_at.elapsed().as_secs(),
                current_tasks: w.current_tasks,
                completed_tasks: w.completed_tasks,
                failed_tasks: w.failed_tasks,
                avg_task_duration_ms: 0.0,
                queue_depth: 0,
            })
            .collect()
    }

    pub async fn check_health(&self) -> HashMap<String, bool> {
        let workers = self.workers.read().await;
        let now = Instant::now();
        workers
            .values()
            .map(|w| {
                let is_healthy = now.duration_since(w.last_heartbeat) < self.config.heartbeat_timeout
                    && w.status == WorkerStatus::Running;
                (w.id.clone(), is_healthy)
            })
            .collect()
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

pub struct Worker {
    id: String,
    worker_type: WorkerType,
    instance_name: String,
    coordinator: Arc<WorkerCoordinator>,
    status: Arc<RwLock<WorkerStatus>>,
    task_count: Arc<RwLock<(usize, u64, u64)>>,
}

impl Worker {
    pub fn new(
        id: String,
        worker_type: WorkerType,
        instance_name: String,
        coordinator: Arc<WorkerCoordinator>,
    ) -> Self {
        Self {
            id,
            worker_type,
            instance_name,
            coordinator,
            status: Arc::new(RwLock::new(WorkerStatus::Starting)),
            task_count: Arc::new(RwLock::new((0, 0, 0))),
        }
    }

    pub async fn start(&self) {
        let info = WorkerInfo {
            id: self.id.clone(),
            worker_type: self.worker_type,
            instance_name: self.instance_name.clone(),
            started_at: Instant::now(),
            last_heartbeat: Instant::now(),
            status: WorkerStatus::Running,
            current_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
        };

        self.coordinator.register_worker(info).await;
        *self.status.write().await = WorkerStatus::Running;

        info!(
            worker_id = %self.id,
            worker_type = ?self.worker_type,
            "Worker started"
        );
    }

    pub async fn stop(&self) {
        *self.status.write().await = WorkerStatus::Stopping;
        self.coordinator.unregister_worker(&self.id).await;

        info!(
            worker_id = %self.id,
            "Worker stopped"
        );
    }

    pub async fn send_heartbeat(&self) -> bool {
        self.coordinator.update_heartbeat(&self.id).await
    }

    pub async fn increment_task_count(&self, success: bool) {
        let mut count = self.task_count.write().await;
        if success {
            count.1 += 1;
        } else {
            count.2 += 1;
        }
    }

    pub fn get_info(&self) -> WorkerInfo {
        let status = self.status.try_read().map(|s| *s).unwrap_or(WorkerStatus::Error);
        let task_count = self.task_count.try_read().map(|c| *c).unwrap_or((0, 0, 0));

        WorkerInfo {
            id: self.id.clone(),
            worker_type: self.worker_type,
            instance_name: self.instance_name.clone(),
            started_at: Instant::now(),
            last_heartbeat: Instant::now(),
            status,
            current_tasks: task_count.0,
            completed_tasks: task_count.1,
            failed_tasks: task_count.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub name: String,
    pub description: String,
    pub worker_types: Vec<WorkerType>,
    pub default_priority: TaskPriority,
    pub timeout_seconds: u64,
    pub max_retries: usize,
}

pub struct TaskRegistry {
    tasks: HashMap<String, TaskDefinition>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tasks: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.register(TaskDefinition {
            name: "persist_event".to_string(),
            description: "Persist event to database".to_string(),
            worker_types: vec![WorkerType::EventPersister],
            default_priority: TaskPriority::High,
            timeout_seconds: 30,
            max_retries: 3,
        });

        self.register(TaskDefinition {
            name: "send_federation".to_string(),
            description: "Send federation transaction".to_string(),
            worker_types: vec![WorkerType::FederationSender],
            default_priority: TaskPriority::Normal,
            timeout_seconds: 60,
            max_retries: 5,
        });

        self.register(TaskDefinition {
            name: "send_push".to_string(),
            description: "Send push notification".to_string(),
            worker_types: vec![WorkerType::Pusher],
            default_priority: TaskPriority::High,
            timeout_seconds: 30,
            max_retries: 3,
        });

        self.register(TaskDefinition {
            name: "process_media".to_string(),
            description: "Process media file".to_string(),
            worker_types: vec![WorkerType::MediaRepository],
            default_priority: TaskPriority::Normal,
            timeout_seconds: 300,
            max_retries: 2,
        });

        self.register(TaskDefinition {
            name: "generate_sync".to_string(),
            description: "Generate sync response".to_string(),
            worker_types: vec![WorkerType::Synchrotron],
            default_priority: TaskPriority::High,
            timeout_seconds: 30,
            max_retries: 1,
        });
    }

    pub fn register(&mut self, task: TaskDefinition) {
        self.tasks.insert(task.name.clone(), task);
    }

    pub fn get(&self, name: &str) -> Option<&TaskDefinition> {
        self.tasks.get(name)
    }

    pub fn get_worker_types(&self, task_name: &str) -> Vec<WorkerType> {
        self.tasks
            .get(task_name)
            .map(|t| t.worker_types.clone())
            .unwrap_or_default()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
