use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub percent: u8,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    pub worker_id: String,
    pub timestamp: i64,
    pub status: String,
    pub current_load: f32,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAssignment {
    pub task_id: String,
    pub worker_id: String,
    pub assigned_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueStats {
    pub queue_name: String,
    pub pending: u64,
    pub in_progress: u64,
    pub completed: u64,
    pub failed: u64,
    pub avg_wait_time_ms: f64,
    pub avg_process_time_ms: f64,
}
