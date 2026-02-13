use crate::worker::{TaskPriority, TaskRegistry, WorkerCoordinator, WorkerTask};
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::error;

pub struct TaskScheduler {
    registry: Arc<TaskRegistry>,
    coordinator: Arc<WorkerCoordinator>,
    pending_tasks: Arc<RwLock<BinaryHeap<ScheduledTask>>>,
    max_queue_size: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ScheduledTask {
    task: WorkerTask,
    scheduled_at: Instant,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.task
            .priority
            .cmp(&other.task.priority)
            .then_with(|| other.scheduled_at.cmp(&self.scheduled_at))
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TaskScheduler {
    pub fn new(
        registry: Arc<TaskRegistry>,
        coordinator: Arc<WorkerCoordinator>,
        max_queue_size: usize,
    ) -> Self {
        Self {
            registry,
            coordinator,
            pending_tasks: Arc::new(RwLock::new(BinaryHeap::new())),
            max_queue_size,
        }
    }

    pub async fn schedule(&self, mut task: WorkerTask) -> Result<(), String> {
        let pending = self.pending_tasks.read().await;
        if pending.len() >= self.max_queue_size {
            return Err("Task queue is full".to_string());
        }
        drop(pending);

        if let Some(definition) = self.registry.get(&task.task_type) {
            if task.priority < definition.default_priority {
                task.priority = definition.default_priority;
            }
        }

        let scheduled = ScheduledTask {
            task,
            scheduled_at: Instant::now(),
        };

        self.pending_tasks.write().await.push(scheduled);
        Ok(())
    }

    pub async fn dispatch_next(&self) -> Option<(WorkerTask, String)> {
        let mut pending = self.pending_tasks.write().await;
        let scheduled = pending.pop()?;

        let worker_types = self.registry.get_worker_types(&scheduled.task.task_type);
        if worker_types.is_empty() {
            error!(
                task_type = %scheduled.task.task_type,
                "No worker types registered for task"
            );
            return None;
        }

        for worker_type in worker_types {
            if let Some(worker_id) = self.coordinator.select_worker(worker_type).await {
                return Some((scheduled.task, worker_id));
            }
        }

        pending.push(scheduled);
        None
    }

    pub async fn get_queue_depth(&self) -> usize {
        self.pending_tasks.read().await.len()
    }

    pub async fn get_pending_by_priority(&self) -> HashMap<TaskPriority, usize> {
        let pending = self.pending_tasks.read().await;
        let mut counts = HashMap::new();
        for scheduled in pending.iter() {
            *counts.entry(scheduled.task.priority).or_insert(0) += 1;
        }
        counts
    }
}

use std::collections::HashMap;

pub struct RetryPolicy {
    max_retries: usize,
    base_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
}

impl RetryPolicy {
    pub fn new(max_retries: usize, base_delay: Duration, max_delay: Duration, multiplier: f64) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            multiplier,
        }
    }

    pub fn should_retry(&self, retry_count: usize) -> bool {
        retry_count < self.max_retries
    }

    pub fn get_delay(&self, retry_count: usize) -> Duration {
        let delay_ms = self.base_delay.as_millis() as f64
            * self.multiplier.powi(retry_count as i32);
        let delay = Duration::from_millis(delay_ms as u64);
        delay.min(self.max_delay)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, Duration::from_millis(100), Duration::from_secs(60), 2.0)
    }
}

pub struct LoadBalancer {
    strategy: LoadBalanceStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastLoaded,
    Random,
    Weighted,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self { strategy }
    }

    pub fn select_worker<'a>(&self, workers: &'a [super::WorkerInfo]) -> Option<&'a super::WorkerInfo> {
        if workers.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::LeastLoaded => workers.iter().min_by_key(|w| w.current_tasks),
            LoadBalanceStrategy::RoundRobin => workers.first(),
            LoadBalanceStrategy::Random => {
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as usize)
                    % workers.len();
                Some(&workers[idx])
            }
            LoadBalanceStrategy::Weighted => {
                workers
                    .iter()
                    .filter(|w| w.status == super::WorkerStatus::Running)
                    .min_by_key(|w| w.current_tasks)
            }
        }
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(LoadBalanceStrategy::LeastLoaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));

        let d1 = policy.get_delay(0);
        let d2 = policy.get_delay(1);
        assert!(d2 > d1);
    }

    #[test]
    fn test_scheduled_task_ordering() {
        let t1 = ScheduledTask {
            task: WorkerTask {
                id: "1".to_string(),
                task_type: "test".to_string(),
                priority: TaskPriority::Normal,
                payload: serde_json::json!({}),
                created_at: Instant::now(),
                retry_count: 0,
                assigned_worker: None,
            },
            scheduled_at: Instant::now(),
        };

        let t2 = ScheduledTask {
            task: WorkerTask {
                id: "2".to_string(),
                task_type: "test".to_string(),
                priority: TaskPriority::High,
                payload: serde_json::json!({}),
                created_at: Instant::now(),
                retry_count: 0,
                assigned_worker: None,
            },
            scheduled_at: Instant::now(),
        };

        assert!(t2 > t1);
    }
}
