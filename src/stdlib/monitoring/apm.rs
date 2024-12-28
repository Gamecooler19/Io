use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::HashMap,
};
use parking_lot::RwLock;
use crate::{Result, error::IoError};

#[derive(Debug, Clone)]
pub struct Span {
    id: String,
    parent_id: Option<String>,
    name: String,
    start_time: Instant,
    end_time: Option<Instant>,
    attributes: HashMap<String, String>,
    events: Vec<SpanEvent>,
}

#[derive(Debug, Clone)]
pub struct SpanEvent {
    timestamp: Instant,
    name: String,
    attributes: HashMap<String, String>,
}

pub struct APMTracer {
    spans: Arc<RwLock<HashMap<String, Span>>>,
    active_spans: Arc<RwLock<Vec<String>>>,
    metrics: Arc<RwLock<TracerMetrics>>,
}

#[derive(Debug, Default)]
struct TracerMetrics {
    total_spans: usize,
    active_spans: usize,
    error_count: usize,
    latency_samples: Vec<Duration>,
}

impl APMTracer {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(HashMap::new())),
            active_spans: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(TracerMetrics::default())),
        }
    }

    pub fn start_span(&self, name: &str) -> Result<String> {
        let span_id = generate_span_id();
        let parent_id = self.active_spans.read().last().cloned();

        let span = Span {
            id: span_id.clone(),
            parent_id,
            name: name.to_string(),
            start_time: Instant::now(),
            end_time: None,
            attributes: HashMap::new(),
            events: Vec::new(),
        };

        self.spans.write().insert(span_id.clone(), span);
        self.active_spans.write().push(span_id.clone());

        let mut metrics = self.metrics.write();
        metrics.total_spans += 1;
        metrics.active_spans += 1;

        Ok(span_id)
    }

    pub fn end_span(&self, span_id: &str) -> Result<()> {
        let mut spans = self.spans.write();
        let span = spans.get_mut(span_id).ok_or_else(|| {
            IoError::runtime_error(format!("Span {} not found", span_id))
        })?;

        span.end_time = Some(Instant::now());
        
        // Calculate and record latency
        let duration = span.end_time.unwrap().duration_since(span.start_time);
        let mut metrics = self.metrics.write();
        metrics.latency_samples.push(duration);
        metrics.active_spans -= 1;

        self.active_spans.write().retain(|id| id != span_id);
        Ok(())
    }

    pub fn add_event(&self, span_id: &str, name: &str, attributes: HashMap<String, String>) -> Result<()> {
        let mut spans = self.spans.write();
        let span = spans.get_mut(span_id).ok_or_else(|| {
            IoError::runtime_error(format!("Span {} not found", span_id))
        })?;

        span.events.push(SpanEvent {
            timestamp: Instant::now(),
            name: name.to_string(),
            attributes,
        });

        Ok(())
    }

    pub fn record_error(&self, span_id: &str, error: &IoError) -> Result<()> {
        let mut spans = self.spans.write();
        let span = spans.get_mut(span_id).ok_or_else(|| {
            IoError::runtime_error(format!("Span {} not found", span_id))
        })?;

        span.attributes.insert("error".to_string(), error.to_string());
        self.metrics.write().error_count += 1;

        Ok(())
    }

    pub fn get_trace_metrics(&self) -> TraceSummary {
        let metrics = self.metrics.read();
        let active = self.active_spans.read();

        TraceSummary {
            total_spans: metrics.total_spans,
            active_spans: active.len(),
            error_rate: if metrics.total_spans > 0 {
                metrics.error_count as f64 / metrics.total_spans as f64
            } else {
                0.0
            },
            avg_latency: metrics.latency_samples.iter()
                .map(|d| d.as_secs_f64())
                .sum::<f64>() / metrics.latency_samples.len() as f64,
        }
    }
}

#[derive(Debug)]
pub struct TraceSummary {
    total_spans: usize,
    active_spans: usize,
    error_rate: f64,
    avg_latency: f64,
}

fn generate_span_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}
