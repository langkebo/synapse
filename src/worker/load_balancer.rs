use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{WorkerType, WorkerStatus, WorkerInfo};

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    LeastResponseTime,
    ResourceBased,
    Adaptive,
}

impl Default for LoadBalancingStrategy {
    fn default() -> Self {
        Self::LeastConnections
    }
}

/// Worker load metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerLoadMetrics {
    pub worker_id: String,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub current_tasks: usize,
    pub avg_response_time_ms: f64,
    pub queue_depth: usize,
    pub error_rate: f32,
    pub throughput_per_sec: f64,
    pub last_updated: i64,
}

impl WorkerLoadMetrics {
    pub fn calculate_load_score(&self, weights: &LoadWeightConfig) -> f64 {
        let cpu_score = self.cpu_usage as f64 * weights.cpu_weight as f64;
        let memory_score = self.memory_usage as f64 * weights.memory_weight as f64;
        let task_score = (self.current_tasks as f64 / 100.0) * weights.task_weight as f64;
        let response_score = (self.avg_response_time_ms / 1000.0) * weights.response_time_weight as f64;
        let error_score = self.error_rate as f64 * weights.error_rate_weight as f64;
        
        cpu_score + memory_score + task_score + response_score + error_score
    }
}

/// Load weight configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadWeightConfig {
    pub cpu_weight: f32,
    pub memory_weight: f32,
    pub task_weight: f32,
    pub response_time_weight: f32,
    pub error_rate_weight: f32,
}

impl Default for LoadWeightConfig {
    fn default() -> Self {
        Self {
            cpu_weight: 0.25,
            memory_weight: 0.15,
            task_weight: 0.30,
            response_time_weight: 0.20,
            error_rate_weight: 0.10,
        }
    }
}

/// Scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingConfig {
    pub enabled: bool,
    pub min_workers: usize,
    pub max_workers: usize,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub scale_up_cooldown_seconds: u64,
    pub scale_down_cooldown_seconds: u64,
    pub evaluation_interval_seconds: u64,
    pub target_cpu_utilization: f32,
    pub target_memory_utilization: f32,
    pub target_response_time_ms: f64,
}

impl Default for AutoScalingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_workers: 1,
            max_workers: 10,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            evaluation_interval_seconds: 30,
            target_cpu_utilization: 0.7,
            target_memory_utilization: 0.8,
            target_response_time_ms: 100.0,
        }
    }
}

/// Scaling event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingEvent {
    pub event_id: String,
    pub worker_type: WorkerType,
    pub action: ScalingAction,
    pub reason: String,
    pub from_count: usize,
    pub to_count: usize,
    pub timestamp: i64,
}

impl ScalingEvent {
    pub fn new(worker_type: WorkerType, action: ScalingAction, reason: String, from: usize, to: usize) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            worker_type,
            action,
            reason,
            from_count: from,
            to_count: to,
            timestamp: Utc::now().timestamp_millis(),
        }
    }
}

/// Scaling action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown,
    NoAction,
}

/// Worker pool state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolState {
    pub worker_type: WorkerType,
    pub total_workers: usize,
    pub active_workers: usize,
    pub draining_workers: usize,
    pub total_capacity: usize,
    pub current_load: f64,
    pub avg_response_time_ms: f64,
    pub queue_depth: usize,
    pub last_scaled_at: Option<i64>,
}

/// Dynamic load balancer
pub struct DynamicLoadBalancer {
    strategy: LoadBalancingStrategy,
    weight_config: LoadWeightConfig,
    metrics: Arc<RwLock<HashMap<String, WorkerLoadMetrics>>>,
    round_robin_counters: Arc<RwLock<HashMap<WorkerType, usize>>>,
    worker_weights: Arc<RwLock<HashMap<String, f64>>>,
}

impl DynamicLoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            weight_config: LoadWeightConfig::default(),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counters: Arc::new(RwLock::new(HashMap::new())),
            worker_weights: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_metrics(&self, worker_id: String, metrics: WorkerLoadMetrics) {
        self.metrics.write().await.insert(worker_id, metrics);
    }

    pub async fn update_weight(&self, worker_id: String, weight: f64) {
        self.worker_weights.write().await.insert(worker_id, weight);
    }

    pub async fn select_worker(&self, workers: &[WorkerInfo]) -> Option<String> {
        if workers.is_empty() {
            return None;
        }

        let available_workers: Vec<_> = workers
            .iter()
            .filter(|w| w.status == WorkerStatus::Running)
            .collect();

        if available_workers.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.round_robin_select(&available_workers).await
            }
            LoadBalancingStrategy::LeastConnections => {
                self.least_connections_select(&available_workers).await
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.weighted_round_robin_select(&available_workers).await
            }
            LoadBalancingStrategy::LeastResponseTime => {
                self.least_response_time_select(&available_workers).await
            }
            LoadBalancingStrategy::ResourceBased => {
                self.resource_based_select(&available_workers).await
            }
            LoadBalancingStrategy::Adaptive => {
                self.adaptive_select(&available_workers).await
            }
        }
    }

    async fn round_robin_select(&self, workers: &[&WorkerInfo]) -> Option<String> {
        let worker_type = workers.first()?.worker_type;
        let mut counters = self.round_robin_counters.write().await;
        let counter = counters.entry(worker_type).or_insert(0);
        
        let index = *counter % workers.len();
        *counter = (*counter + 1) % workers.len();
        
        Some(workers[index].id.clone())
    }

    async fn least_connections_select(&self, workers: &[&WorkerInfo]) -> Option<String> {
        workers
            .iter()
            .min_by_key(|w| w.current_tasks)
            .map(|w| w.id.clone())
    }

    async fn weighted_round_robin_select(&self, workers: &[&WorkerInfo]) -> Option<String> {
        let weights = self.worker_weights.read().await;
        
        let total_weight: f64 = workers
            .iter()
            .map(|w| weights.get(&w.id).copied().unwrap_or(1.0))
            .sum();

        let mut random = rand::random::<f64>() * total_weight;
        
        for worker in workers {
            let weight = weights.get(&worker.id).copied().unwrap_or(1.0);
            random -= weight;
            if random <= 0.0 {
                return Some(worker.id.clone());
            }
        }
        
        workers.first().map(|w| w.id.clone())
    }

    async fn least_response_time_select(&self, workers: &[&WorkerInfo]) -> Option<String> {
        let metrics = self.metrics.read().await;
        
        workers
            .iter()
            .min_by(|a, b| {
                let a_time = metrics.get(&a.id).map(|m| m.avg_response_time_ms).unwrap_or(f64::MAX);
                let b_time = metrics.get(&b.id).map(|m| m.avg_response_time_ms).unwrap_or(f64::MAX);
                a_time.partial_cmp(&b_time).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id.clone())
    }

    async fn resource_based_select(&self, workers: &[&WorkerInfo]) -> Option<String> {
        let metrics = self.metrics.read().await;
        
        workers
            .iter()
            .min_by(|a, b| {
                let a_score = metrics.get(&a.id)
                    .map(|m| m.calculate_load_score(&self.weight_config))
                    .unwrap_or(f64::MAX);
                let b_score = metrics.get(&b.id)
                    .map(|m| m.calculate_load_score(&self.weight_config))
                    .unwrap_or(f64::MAX);
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id.clone())
    }

    async fn adaptive_select(&self, workers: &[&WorkerInfo]) -> Option<String> {
        let metrics = self.metrics.read().await;
        
        let now = Utc::now().timestamp_millis();
        
        workers
            .iter()
            .min_by(|a, b| {
                let a_metrics = metrics.get(&a.id);
                let b_metrics = metrics.get(&b.id);
                
                let a_score = self.calculate_adaptive_score(a, a_metrics, now);
                let b_score = self.calculate_adaptive_score(b, b_metrics, now);
                
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id.clone())
    }

    fn calculate_adaptive_score(&self, worker: &WorkerInfo, metrics: Option<&WorkerLoadMetrics>, now: i64) -> f64 {
        let base_score = worker.current_tasks as f64;
        
        if let Some(m) = metrics {
            let recency_factor = if now - m.last_updated < 5000 { 1.0 } else { 0.5 };
            let error_penalty = m.error_rate as f64 * 10.0;
            let response_factor = m.avg_response_time_ms / 100.0;
            
            (base_score + response_factor + error_penalty) * recency_factor
        } else {
            base_score + 1.0
        }
    }

    pub fn get_strategy(&self) -> LoadBalancingStrategy {
        self.strategy
    }

    pub fn set_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
    }
}

/// Auto scaler for dynamic worker scaling
pub struct AutoScaler {
    config: AutoScalingConfig,
    pool_states: Arc<RwLock<HashMap<WorkerType, WorkerPoolState>>>,
    scaling_history: Arc<RwLock<Vec<ScalingEvent>>>,
    last_scale_up: Arc<RwLock<HashMap<WorkerType, i64>>>,
    last_scale_down: Arc<RwLock<HashMap<WorkerType, i64>>>,
}

impl AutoScaler {
    pub fn new(config: AutoScalingConfig) -> Self {
        Self {
            config,
            pool_states: Arc::new(RwLock::new(HashMap::new())),
            scaling_history: Arc::new(RwLock::new(Vec::new())),
            last_scale_up: Arc::new(RwLock::new(HashMap::new())),
            last_scale_down: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn update_pool_state(&self, state: WorkerPoolState) {
        self.pool_states.write().await.insert(state.worker_type, state);
    }

    pub async fn evaluate_scaling(&self, worker_type: WorkerType) -> Option<ScalingAction> {
        if !self.config.enabled {
            return None;
        }

        let pool_states = self.pool_states.read().await;
        let state = pool_states.get(&worker_type)?;
        
        let now = Utc::now().timestamp_millis();
        
        if state.total_workers >= self.config.max_workers && state.current_load > self.config.scale_up_threshold {
            debug!(
                worker_type = ?worker_type,
                load = state.current_load,
                "Cannot scale up: max workers reached"
            );
            return None;
        }
        
        if state.total_workers <= self.config.min_workers && state.current_load < self.config.scale_down_threshold {
            debug!(
                worker_type = ?worker_type,
                load = state.current_load,
                "Cannot scale down: min workers reached"
            );
            return None;
        }

        if state.current_load > self.config.scale_up_threshold {
            let last_scale_up = self.last_scale_up.read().await;
            let last_up = last_scale_up.get(&worker_type).copied().unwrap_or(0);
            
            if now - last_up < (self.config.scale_up_cooldown_seconds as i64 * 1000) {
                debug!(worker_type = ?worker_type, "Scale up cooldown active");
                return None;
            }
            
            return Some(ScalingAction::ScaleUp);
        }
        
        if state.current_load < self.config.scale_down_threshold {
            let last_scale_down = self.last_scale_down.read().await;
            let last_down = last_scale_down.get(&worker_type).copied().unwrap_or(0);
            
            if now - last_down < (self.config.scale_down_cooldown_seconds as i64 * 1000) {
                debug!(worker_type = ?worker_type, "Scale down cooldown active");
                return None;
            }
            
            return Some(ScalingAction::ScaleDown);
        }
        
        Some(ScalingAction::NoAction)
    }

    pub async fn record_scaling_event(&self, event: ScalingEvent) {
        let worker_type = event.worker_type;
        let now = event.timestamp;
        
        match event.action {
            ScalingAction::ScaleUp => {
                self.last_scale_up.write().await.insert(worker_type, now);
            }
            ScalingAction::ScaleDown => {
                self.last_scale_down.write().await.insert(worker_type, now);
            }
            ScalingAction::NoAction => {}
        }
        
        self.scaling_history.write().await.push(event);
    }

    pub async fn get_scaling_history(&self, limit: usize) -> Vec<ScalingEvent> {
        let history = self.scaling_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    pub async fn get_pool_state(&self, worker_type: WorkerType) -> Option<WorkerPoolState> {
        self.pool_states.read().await.get(&worker_type).cloned()
    }

    pub async fn calculate_recommended_workers(&self, worker_type: WorkerType) -> Option<usize> {
        let state = self.get_pool_state(worker_type).await?;
        
        if state.current_load < 0.01 {
            return Some(self.config.min_workers);
        }
        
        let target_load = (self.config.scale_up_threshold + self.config.scale_down_threshold) / 2.0;
        let recommended = (state.total_workers as f64 * state.current_load / target_load).ceil() as usize;
        
        Some(recommended.clamp(self.config.min_workers, self.config.max_workers))
    }

    pub fn get_config(&self) -> &AutoScalingConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: AutoScalingConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_worker_info(id: &str, tasks: usize) -> WorkerInfo {
        WorkerInfo {
            id: id.to_string(),
            worker_type: WorkerType::EventPersister,
            instance_name: "test".to_string(),
            started_at: Instant::now(),
            last_heartbeat: Instant::now(),
            status: WorkerStatus::Running,
            current_tasks: tasks,
            completed_tasks: 0,
            failed_tasks: 0,
        }
    }

    #[test]
    fn test_load_balancing_strategy_default() {
        let strategy = LoadBalancingStrategy::default();
        assert_eq!(strategy, LoadBalancingStrategy::LeastConnections);
    }

    #[test]
    fn test_worker_load_metrics_score() {
        let metrics = WorkerLoadMetrics {
            worker_id: "w1".to_string(),
            cpu_usage: 0.5,
            memory_usage: 0.3,
            current_tasks: 10,
            avg_response_time_ms: 100.0,
            queue_depth: 5,
            error_rate: 0.01,
            throughput_per_sec: 100.0,
            last_updated: Utc::now().timestamp_millis(),
        };
        
        let weights = LoadWeightConfig::default();
        let score = metrics.calculate_load_score(&weights);
        
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_least_connections_select() {
        let balancer = DynamicLoadBalancer::new(LoadBalancingStrategy::LeastConnections);
        
        let workers = vec![
            create_test_worker_info("w1", 5),
            create_test_worker_info("w2", 2),
            create_test_worker_info("w3", 8),
        ];
        
        let selected = balancer.select_worker(&workers).await;
        assert_eq!(selected, Some("w2".to_string()));
    }

    #[tokio::test]
    async fn test_round_robin_select() {
        let balancer = DynamicLoadBalancer::new(LoadBalancingStrategy::RoundRobin);
        
        let workers = vec![
            create_test_worker_info("w1", 0),
            create_test_worker_info("w2", 0),
            create_test_worker_info("w3", 0),
        ];
        
        let first = balancer.select_worker(&workers).await;
        let second = balancer.select_worker(&workers).await;
        let third = balancer.select_worker(&workers).await;
        let fourth = balancer.select_worker(&workers).await;
        
        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_eq!(first, fourth);
    }

    #[tokio::test]
    async fn test_auto_scaler_creation() {
        let config = AutoScalingConfig::default();
        let scaler = AutoScaler::new(config);
        
        assert!(scaler.is_enabled());
    }

    #[tokio::test]
    async fn test_auto_scaler_disabled() {
        let config = AutoScalingConfig {
            enabled: false,
            ..Default::default()
        };
        let scaler = AutoScaler::new(config);
        
        let result = scaler.evaluate_scaling(WorkerType::EventPersister).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_scaling_event() {
        let event = ScalingEvent::new(
            WorkerType::EventPersister,
            ScalingAction::ScaleUp,
            "High load detected".to_string(),
            2,
            3,
        );
        
        assert_eq!(event.action, ScalingAction::ScaleUp);
        assert_eq!(event.from_count, 2);
        assert_eq!(event.to_count, 3);
    }

    #[tokio::test]
    async fn test_pool_state_update() {
        let scaler = AutoScaler::new(AutoScalingConfig::default());
        
        let state = WorkerPoolState {
            worker_type: WorkerType::EventPersister,
            total_workers: 3,
            active_workers: 3,
            draining_workers: 0,
            total_capacity: 30,
            current_load: 0.5,
            avg_response_time_ms: 50.0,
            queue_depth: 10,
            last_scaled_at: None,
        };
        
        scaler.update_pool_state(state).await;
        
        let retrieved = scaler.get_pool_state(WorkerType::EventPersister).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().total_workers, 3);
    }

    #[tokio::test]
    async fn test_recommended_workers() {
        let scaler = AutoScaler::new(AutoScalingConfig::default());
        
        let state = WorkerPoolState {
            worker_type: WorkerType::EventPersister,
            total_workers: 3,
            active_workers: 3,
            draining_workers: 0,
            total_capacity: 30,
            current_load: 0.9,
            avg_response_time_ms: 50.0,
            queue_depth: 10,
            last_scaled_at: None,
        };
        
        scaler.update_pool_state(state).await;
        
        let recommended = scaler.calculate_recommended_workers(WorkerType::EventPersister).await;
        assert!(recommended.is_some());
        assert!(recommended.unwrap() > 3);
    }
}
