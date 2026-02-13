use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpanKind {
    Server,
    Client,
    Producer,
    Consumer,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId([u8; 16]);

impl TraceId {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut id = [0u8; 16];
        rng.fill(&mut id);
        Self(id)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        for i in 0..16 {
            bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(Self(bytes))
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId([u8; 8]);

impl SpanId {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut id = [0u8; 8];
        rng.fill(&mut id);
        Self(id)
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 16 {
            return None;
        }
        let mut bytes = [0u8; 8];
        for i in 0..8 {
            bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(Self(bytes))
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SpanContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub trace_flags: u8,
    pub is_remote: bool,
}

impl SpanContext {
    pub fn new(trace_id: TraceId, span_id: SpanId, parent_span_id: Option<SpanId>) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id,
            trace_flags: 1,
            is_remote: false,
        }
    }

    pub fn is_sampled(&self) -> bool {
        self.trace_flags & 1 == 1
    }

    pub fn to_w3c_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id.to_hex(),
            self.span_id.to_hex(),
            self.trace_flags
        )
    }

    pub fn from_w3c_traceparent(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return None;
        }

        let trace_id = TraceId::from_hex(parts[1])?;
        let span_id = SpanId::from_hex(parts[2])?;
        let trace_flags = u8::from_str_radix(parts[3], 16).ok()?;

        Some(Self {
            trace_id,
            span_id,
            parent_span_id: None,
            trace_flags,
            is_remote: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanData {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration_ms: Option<f64>,
    pub attributes: HashMap<String, AttributeValue>,
    pub events: Vec<Event>,
    pub status: SpanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub timestamp: i64,
    pub attributes: HashMap<String, AttributeValue>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpanStatus {
    Ok,
    Error,
    Unset,
}

pub struct Span {
    context: SpanContext,
    name: String,
    kind: SpanKind,
    start_time: Instant,
    attributes: HashMap<String, AttributeValue>,
    events: Vec<Event>,
    status: SpanStatus,
}

impl Span {
    pub fn new(name: &str, kind: SpanKind) -> Self {
        Self {
            context: SpanContext::new(TraceId::new(), SpanId::new(), None),
            name: name.to_string(),
            kind,
            start_time: Instant::now(),
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        }
    }

    pub fn child_of(parent: &SpanContext, name: &str, kind: SpanKind) -> Self {
        Self {
            context: SpanContext::new(
                parent.trace_id.clone(),
                SpanId::new(),
                Some(parent.span_id.clone()),
            ),
            name: name.to_string(),
            kind,
            start_time: Instant::now(),
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        }
    }

    pub fn set_attribute(&mut self, key: &str, value: AttributeValue) {
        self.attributes.insert(key.to_string(), value);
    }

    pub fn add_event(&mut self, name: &str, attributes: HashMap<String, AttributeValue>) {
        self.events.push(Event {
            name: name.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            attributes,
        });
    }

    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    pub fn finish(self) -> SpanData {
        let duration = self.start_time.elapsed();
        SpanData {
            trace_id: self.context.trace_id.to_hex(),
            span_id: self.context.span_id.to_hex(),
            parent_span_id: self.context.parent_span_id.map(|s| s.to_hex()),
            name: self.name,
            kind: match self.kind {
                SpanKind::Server => "server".to_string(),
                SpanKind::Client => "client".to_string(),
                SpanKind::Producer => "producer".to_string(),
                SpanKind::Consumer => "consumer".to_string(),
                SpanKind::Internal => "internal".to_string(),
            },
            start_time: chrono::Utc::now().timestamp_millis() - duration.as_millis() as i64,
            end_time: Some(chrono::Utc::now().timestamp_millis()),
            duration_ms: Some(duration.as_secs_f64() * 1000.0),
            attributes: self.attributes,
            events: self.events,
            status: self.status,
        }
    }
}

pub struct Tracer {
    service_name: String,
    spans: Arc<RwLock<Vec<SpanData>>>,
}

impl Tracer {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn start_span(&self, name: &str, kind: SpanKind) -> Span {
        Span::new(name, kind)
    }

    pub fn start_child_span(&self, parent: &SpanContext, name: &str, kind: SpanKind) -> Span {
        Span::child_of(parent, name, kind)
    }

    pub async fn end_span(&self, span: Span) {
        let span_data = span.finish();
        if span_data.status == SpanStatus::Error {
            error!(
                trace_id = %span_data.trace_id,
                span_id = %span_data.span_id,
                name = %span_data.name,
                duration_ms = ?span_data.duration_ms,
                "Span completed with error"
            );
        } else {
            debug!(
                trace_id = %span_data.trace_id,
                span_id = %span_data.span_id,
                name = %span_data.name,
                duration_ms = ?span_data.duration_ms,
                "Span completed"
            );
        }
        self.spans.write().await.push(span_data);
    }

    pub async fn get_spans(&self) -> Vec<SpanData> {
        self.spans.read().await.clone()
    }

    pub async fn export_to_jaeger(&self) -> JaegerExport {
        let spans = self.spans.read().await;
        JaegerExport {
            data: vec![JaegerProcess {
                service_name: self.service_name.clone(),
                spans: spans.clone(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaegerExport {
    pub data: Vec<JaegerProcess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaegerProcess {
    pub service_name: String,
    pub spans: Vec<SpanData>,
}

pub struct TraceContext {
    current_span: Option<SpanContext>,
}

impl TraceContext {
    pub fn new() -> Self {
        Self { current_span: None }
    }

    pub fn with_span(context: SpanContext) -> Self {
        Self {
            current_span: Some(context),
        }
    }

    pub fn current_span(&self) -> Option<&SpanContext> {
        self.current_span.as_ref()
    }

    pub fn inject_headers(&self, headers: &mut HashMap<String, String>) {
        if let Some(ctx) = &self.current_span {
            headers.insert("traceparent".to_string(), ctx.to_w3c_traceparent());
        }
    }

    pub fn extract_headers(headers: &HashMap<String, String>) -> Option<Self> {
        headers
            .get("traceparent")
            .and_then(|s| SpanContext::from_w3c_traceparent(s))
            .map(Self::with_span)
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MatrixTraceAttributes;

impl MatrixTraceAttributes {
    pub const USER_ID: &str = "matrix.user_id";
    pub const ROOM_ID: &str = "matrix.room_id";
    pub const EVENT_ID: &str = "matrix.event_id";
    pub const EVENT_TYPE: &str = "matrix.event_type";
    pub const DEVICE_ID: &str = "matrix.device_id";
    pub const SERVER_NAME: &str = "matrix.server_name";
    pub const FEDERATION_ORIGIN: &str = "matrix.federation.origin";
    pub const FEDERATION_DESTINATION: &str = "matrix.federation.destination";
    pub const SYNC_TOKEN: &str = "matrix.sync.token";
    pub const MEDIA_ID: &str = "matrix.media_id";
    pub const PUSH_RULE_ID: &str = "matrix.push_rule_id";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_generation() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();
        assert_ne!(id1.to_hex(), id2.to_hex());
        assert_eq!(id1.to_hex().len(), 32);
    }

    #[test]
    fn test_span_id_generation() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();
        assert_ne!(id1.to_hex(), id2.to_hex());
        assert_eq!(id1.to_hex().len(), 16);
    }

    #[test]
    fn test_w3c_traceparent() {
        let ctx = SpanContext::new(TraceId::new(), SpanId::new(), None);
        let traceparent = ctx.to_w3c_traceparent();

        assert!(traceparent.starts_with("00-"));
        assert_eq!(traceparent.len(), 55);

        let parsed = SpanContext::from_w3c_traceparent(&traceparent);
        assert!(parsed.is_some());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.span_id, ctx.span_id);
    }

    #[test]
    fn test_span_creation() {
        let mut span = Span::new("test_operation", SpanKind::Server);
        span.set_attribute("key", AttributeValue::String("value".to_string()));
        span.add_event("checkpoint", HashMap::new());
        span.set_status(SpanStatus::Ok);

        let data = span.finish();
        assert_eq!(data.name, "test_operation");
        assert_eq!(data.kind, "server");
        assert!(data.duration_ms.is_some());
    }

    #[test]
    fn test_child_span() {
        let parent_ctx = SpanContext::new(TraceId::new(), SpanId::new(), None);
        let child = Span::child_of(&parent_ctx, "child_operation", SpanKind::Client);

        assert_eq!(child.context().trace_id, parent_ctx.trace_id);
        assert_ne!(child.context().span_id, parent_ctx.span_id);
        assert_eq!(child.context().parent_span_id, Some(parent_ctx.span_id.clone()));
    }

    #[tokio::test]
    async fn test_tracer() {
        let tracer = Tracer::new("test-service");

        let span = tracer.start_span("operation", SpanKind::Server);
        let ctx = span.context().clone();
        tracer.end_span(span).await;

        let spans = tracer.get_spans().await;
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].trace_id, ctx.trace_id.to_hex());
    }

    #[test]
    fn test_trace_context_headers() {
        let ctx = SpanContext::new(TraceId::new(), SpanId::new(), None);
        let trace_ctx = TraceContext::with_span(ctx);

        let mut headers = HashMap::new();
        trace_ctx.inject_headers(&mut headers);

        assert!(headers.contains_key("traceparent"));

        let extracted = TraceContext::extract_headers(&headers);
        assert!(extracted.is_some());
    }
}
