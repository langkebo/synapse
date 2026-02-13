use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("Histogram value comparison error: {0}")]
    ValueComparisonError(String),
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub timestamp: Instant,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Counter {
    name: String,
    value: Arc<AtomicU64>,
    labels: HashMap<String, String>,
}

impl Counter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            value: Arc::new(AtomicU64::new(0)),
            labels: HashMap::new(),
        }
    }

    pub fn with_labels(name: String, labels: HashMap<String, String>) -> Self {
        Self {
            name,
            value: Arc::new(AtomicU64::new(0)),
            labels,
        }
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_by(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct Gauge {
    name: String,
    value: Arc<AtomicI64>,
    labels: HashMap<String, String>,
}

impl Gauge {
    pub fn new(name: String) -> Self {
        Self {
            name,
            value: Arc::new(AtomicI64::new(0)),
            labels: HashMap::new(),
        }
    }

    pub fn with_labels(name: String, labels: HashMap<String, String>) -> Self {
        Self {
            name,
            value: Arc::new(AtomicI64::new(0)),
            labels,
        }
    }

    pub fn set(&self, value: f64) {
        self.value.store(value as i64, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn add(&self, delta: f64) {
        self.value.fetch_add(delta as i64, Ordering::Relaxed);
    }

    pub fn sub(&self, delta: f64) {
        self.value.fetch_sub(delta as i64, Ordering::Relaxed);
    }

    pub fn get(&self) -> f64 {
        self.value.load(Ordering::Relaxed) as f64
    }
}

#[derive(Debug, Clone)]
pub struct Histogram {
    name: String,
    values: Arc<parking_lot::Mutex<Vec<f64>>>,
    labels: HashMap<String, String>,
}

impl Histogram {
    pub fn new(name: String) -> Self {
        Self {
            name,
            values: Arc::new(parking_lot::Mutex::new(Vec::new())),
            labels: HashMap::new(),
        }
    }

    pub fn with_labels(name: String, labels: HashMap<String, String>) -> Self {
        Self {
            name,
            values: Arc::new(parking_lot::Mutex::new(Vec::new())),
            labels,
        }
    }

    pub fn observe(&self, value: f64) {
        let mut values = self.values.lock();
        values.push(value);
    }

    pub fn get_values(&self) -> Vec<f64> {
        let values = self.values.lock();
        values.clone()
    }

    pub fn get_count(&self) -> usize {
        let values = self.values.lock();
        values.len()
    }

    pub fn get_sum(&self) -> f64 {
        let values = self.values.lock();
        values.iter().sum()
    }

    pub fn get_avg(&self) -> f64 {
        let values = self.values.lock();
        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        }
    }

    pub fn get_percentile(&self, percentile: f64) -> Result<f64, MetricsError> {
        let mut values = self.values.lock();
        if values.is_empty() {
            return Ok(0.0);
        }
        values.sort_by(|a, b| {
            a.partial_cmp(b)
                .ok_or_else(|| {
                    MetricsError::ValueComparisonError(
                        "Cannot compare histogram values".to_string(),
                    )
                })
                .expect("This should never happen after ok_or_else")
        });
        let index = ((percentile / 100.0) * (values.len() - 1) as f64).floor() as usize;
        Ok(values[index.min(values.len() - 1)])
    }

    pub fn reset(&self) {
        let mut values = self.values.lock();
        values.clear();
    }
}

pub struct MetricsCollector {
    counters: Arc<parking_lot::Mutex<HashMap<String, Counter>>>,
    gauges: Arc<parking_lot::Mutex<HashMap<String, Gauge>>>,
    histograms: Arc<parking_lot::Mutex<HashMap<String, Histogram>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            gauges: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            histograms: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }

    pub fn register_counter(&self, name: String) -> Counter {
        let counter = Counter::new(name.clone());
        let mut counters = self.counters.lock();
        counters.insert(name, counter.clone());
        counter
    }

    pub fn register_counter_with_labels(
        &self,
        name: String,
        labels: HashMap<String, String>,
    ) -> Counter {
        let counter = Counter::with_labels(name.clone(), labels);
        let mut counters = self.counters.lock();
        counters.insert(name, counter.clone());
        counter
    }

    pub fn register_gauge(&self, name: String) -> Gauge {
        let gauge = Gauge::new(name.clone());
        let mut gauges = self.gauges.lock();
        gauges.insert(name, gauge.clone());
        gauge
    }

    pub fn register_gauge_with_labels(
        &self,
        name: String,
        labels: HashMap<String, String>,
    ) -> Gauge {
        let gauge = Gauge::with_labels(name.clone(), labels);
        let mut gauges = self.gauges.lock();
        gauges.insert(name, gauge.clone());
        gauge
    }

    pub fn register_histogram(&self, name: String) -> Histogram {
        let histogram = Histogram::new(name.clone());
        let mut histograms = self.histograms.lock();
        histograms.insert(name, histogram.clone());
        histogram
    }

    pub fn register_histogram_with_labels(
        &self,
        name: String,
        labels: HashMap<String, String>,
    ) -> Histogram {
        let histogram = Histogram::with_labels(name.clone(), labels);
        let mut histograms = self.histograms.lock();
        histograms.insert(name, histogram.clone());
        histogram
    }

    pub fn get_counter(&self, name: &str) -> Option<Counter> {
        let counters = self.counters.lock();
        counters.get(name).cloned()
    }

    pub fn get_gauge(&self, name: &str) -> Option<Gauge> {
        let gauges = self.gauges.lock();
        gauges.get(name).cloned()
    }

    pub fn get_histogram(&self, name: &str) -> Option<Histogram> {
        let histograms = self.histograms.lock();
        histograms.get(name).cloned()
    }

    pub fn collect_metrics(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();

        let counters = self.counters.lock();
        for counter in counters.values() {
            metrics.push(Metric {
                name: counter.name.clone(),
                value: counter.get() as f64,
                timestamp: Instant::now(),
                labels: counter.labels.clone(),
            });
        }

        let gauges = self.gauges.lock();
        for gauge in gauges.values() {
            metrics.push(Metric {
                name: gauge.name.clone(),
                value: gauge.get(),
                timestamp: Instant::now(),
                labels: gauge.labels.clone(),
            });
        }

        let histograms = self.histograms.lock();
        for histogram in histograms.values() {
            metrics.push(Metric {
                name: format!("{}_count", histogram.name),
                value: histogram.get_count() as f64,
                timestamp: Instant::now(),
                labels: histogram.labels.clone(),
            });
            metrics.push(Metric {
                name: format!("{}_sum", histogram.name),
                value: histogram.get_sum(),
                timestamp: Instant::now(),
                labels: histogram.labels.clone(),
            });
            metrics.push(Metric {
                name: format!("{}_avg", histogram.name),
                value: histogram.get_avg(),
                timestamp: Instant::now(),
                labels: histogram.labels.clone(),
            });
        }

        metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new("test_counter".to_string());
        assert_eq!(counter.get(), 0);
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_with_labels() {
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        let counter = Counter::with_labels("test_counter".to_string(), labels);
        counter.inc();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new("test_gauge".to_string());
        assert_eq!(gauge.get(), 0.0);
        gauge.set(42.0);
        assert_eq!(gauge.get(), 42.0);
        gauge.inc();
        assert_eq!(gauge.get(), 43.0);
        gauge.dec();
        assert_eq!(gauge.get(), 42.0);
        gauge.add(10.0);
        assert_eq!(gauge.get(), 52.0);
        gauge.sub(2.0);
        assert_eq!(gauge.get(), 50.0);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new("test_histogram".to_string());
        histogram.observe(1.0);
        histogram.observe(2.0);
        histogram.observe(3.0);
        assert_eq!(histogram.get_count(), 3);
        assert_eq!(histogram.get_sum(), 6.0);
        assert_eq!(histogram.get_avg(), 2.0);
        assert_eq!(histogram.get_percentile(50.0).unwrap(), 2.0);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        let counter = collector.register_counter("test_counter".to_string());
        counter.inc();
        assert_eq!(collector.get_counter("test_counter").unwrap().get(), 1);

        let gauge = collector.register_gauge("test_gauge".to_string());
        gauge.set(42.0);
        assert_eq!(collector.get_gauge("test_gauge").unwrap().get(), 42.0);

        let histogram = collector.register_histogram("test_histogram".to_string());
        histogram.observe(1.0);
        assert_eq!(
            collector
                .get_histogram("test_histogram")
                .unwrap()
                .get_count(),
            1
        );

        let metrics = collector.collect_metrics();
        assert!(!metrics.is_empty());
    }
}

pub struct MatrixMetrics {
    pub active_users: Gauge,
    pub registered_users: Counter,
    pub rooms: Gauge,
    pub messages_total: Counter,
    pub federation_requests_total: Counter,
    pub federation_request_duration: Histogram,
    pub sync_requests_total: Counter,
    pub sync_request_duration: Histogram,
    pub event_persist_duration: Histogram,
    pub push_notifications_total: Counter,
    pub push_notification_duration: Histogram,
    pub media_upload_total: Counter,
    pub media_upload_bytes: Counter,
    pub database_query_duration: Histogram,
    pub database_connections: Gauge,
    pub cache_hits: Counter,
    pub cache_misses: Counter,
}

impl MatrixMetrics {
    pub fn new(collector: &MetricsCollector) -> Self {
        Self {
            active_users: collector.register_gauge("synapse_active_users".to_string()),
            registered_users: collector.register_counter("synapse_registered_users_total".to_string()),
            rooms: collector.register_gauge("synapse_rooms".to_string()),
            messages_total: collector.register_counter("synapse_messages_total".to_string()),
            federation_requests_total: collector.register_counter("synapse_federation_requests_total".to_string()),
            federation_request_duration: collector.register_histogram("synapse_federation_request_duration_seconds".to_string()),
            sync_requests_total: collector.register_counter("synapse_sync_requests_total".to_string()),
            sync_request_duration: collector.register_histogram("synapse_sync_request_duration_seconds".to_string()),
            event_persist_duration: collector.register_histogram("synapse_event_persist_duration_seconds".to_string()),
            push_notifications_total: collector.register_counter("synapse_push_notifications_total".to_string()),
            push_notification_duration: collector.register_histogram("synapse_push_notification_duration_seconds".to_string()),
            media_upload_total: collector.register_counter("synapse_media_uploads_total".to_string()),
            media_upload_bytes: collector.register_counter("synapse_media_upload_bytes_total".to_string()),
            database_query_duration: collector.register_histogram("synapse_database_query_duration_seconds".to_string()),
            database_connections: collector.register_gauge("synapse_database_connections".to_string()),
            cache_hits: collector.register_counter("synapse_cache_hits_total".to_string()),
            cache_misses: collector.register_counter("synapse_cache_misses_total".to_string()),
        }
    }
}

pub struct PrometheusExporter {
    collector: Arc<MetricsCollector>,
    namespace: String,
}

impl PrometheusExporter {
    pub fn new(collector: Arc<MetricsCollector>, namespace: Option<&str>) -> Self {
        Self {
            collector,
            namespace: namespace.unwrap_or("synapse").to_string(),
        }
    }

    pub fn export(&self) -> String {
        let metrics = self.collector.collect_metrics();
        let mut output = String::with_capacity(metrics.len() * 100);

        let mut grouped: HashMap<String, Vec<&Metric>> = HashMap::new();
        for metric in &metrics {
            grouped
                .entry(metric.name.clone())
                .or_default()
                .push(metric);
        }

        for (name, metric_group) in grouped {
            if let Some(_first) = metric_group.first() {
                let metric_type = if name.ends_with("_total") || name.ends_with("_count") {
                    "counter"
                } else {
                    "gauge"
                };

                output.push_str(&format!("# HELP {}_{} Metric\n", self.namespace, name));
                output.push_str(&format!("# TYPE {}_{} {}\n", self.namespace, name, metric_type));

                for metric in metric_group {
                    let labels_str = if metric.labels.is_empty() {
                        String::new()
                    } else {
                        let labels: Vec<String> = metric
                            .labels
                            .iter()
                            .map(|(k, v)| format!("{}=\"{}\"", k, v))
                            .collect();
                        format!("{{{}}}", labels.join(","))
                    };

                    output.push_str(&format!(
                        "{}_{}{} {}\n",
                        self.namespace,
                        name,
                        labels_str,
                        metric.value
                    ));
                }
                output.push('\n');
            }
        }

        output
    }

    pub fn export_with_help(&self) -> String {
        let mut output = String::new();

        output.push_str("# Synapse Rust Prometheus Metrics Export\n");
        output.push_str("# Format: Prometheus Text Format\n\n");

        output.push_str(&self.export());

        output
    }
}

pub struct MetricsBuilder {
    collector: Arc<MetricsCollector>,
}

impl MetricsBuilder {
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    pub fn counter(&self, name: &str) -> Counter {
        self.collector.get_counter(name).unwrap_or_else(|| {
            self.collector.register_counter(name.to_string())
        })
    }

    pub fn counter_with_labels(&self, name: &str, labels: HashMap<String, String>) -> Counter {
        let key = format!("{}:{:?}", name, labels);
        self.collector.get_counter(&key).unwrap_or_else(|| {
            self.collector.register_counter_with_labels(name.to_string(), labels)
        })
    }

    pub fn gauge(&self, name: &str) -> Gauge {
        self.collector.get_gauge(name).unwrap_or_else(|| {
            self.collector.register_gauge(name.to_string())
        })
    }

    pub fn gauge_with_labels(&self, name: &str, labels: HashMap<String, String>) -> Gauge {
        let key = format!("{}:{:?}", name, labels);
        self.collector.get_gauge(&key).unwrap_or_else(|| {
            self.collector.register_gauge_with_labels(name.to_string(), labels)
        })
    }

    pub fn histogram(&self, name: &str) -> Histogram {
        self.collector.get_histogram(name).unwrap_or_else(|| {
            self.collector.register_histogram(name.to_string())
        })
    }

    pub fn histogram_with_labels(&self, name: &str, labels: HashMap<String, String>) -> Histogram {
        let key = format!("{}:{:?}", name, labels);
        self.collector.get_histogram(&key).unwrap_or_else(|| {
            self.collector.register_histogram_with_labels(name.to_string(), labels)
        })
    }

    pub fn time<F, T>(&self, histogram: &Histogram, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed().as_secs_f64();
        histogram.observe(duration);
        result
    }

    pub async fn time_async<F, Fut, T>(&self, histogram: &Histogram, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed().as_secs_f64();
        histogram.observe(duration);
        result
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn test_matrix_metrics() {
        let collector = MetricsCollector::new();
        let metrics = MatrixMetrics::new(&collector);

        metrics.active_users.set(100.0);
        assert_eq!(metrics.active_users.get(), 100.0);

        metrics.registered_users.inc();
        assert_eq!(metrics.registered_users.get(), 1);

        metrics.messages_total.inc_by(10);
        assert_eq!(metrics.messages_total.get(), 10);
    }

    #[test]
    fn test_prometheus_exporter() {
        let collector = Arc::new(MetricsCollector::new());
        let counter = collector.register_counter("test_counter".to_string());
        counter.inc_by(5);

        let gauge = collector.register_gauge("test_gauge".to_string());
        gauge.set(42.0);

        let exporter = PrometheusExporter::new(collector, Some("test"));
        let output = exporter.export();

        assert!(output.contains("test_test_counter"));
        assert!(output.contains("test_test_gauge"));
        assert!(output.contains("5"));
        assert!(output.contains("42"));
    }

    #[test]
    fn test_metrics_builder() {
        let collector = Arc::new(MetricsCollector::new());
        let builder = MetricsBuilder::new(collector.clone());

        let counter = builder.counter("my_counter");
        counter.inc();
        assert_eq!(counter.get(), 1);

        let gauge = builder.gauge("my_gauge");
        gauge.set(10.0);
        assert_eq!(gauge.get(), 10.0);
    }

    #[test]
    fn test_time_macro() {
        let collector = Arc::new(MetricsCollector::new());
        let builder = MetricsBuilder::new(collector);
        let histogram = builder.histogram("timing_test");

        let result = builder.time(&histogram, || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
        assert!(histogram.get_count() >= 1);
    }
}
