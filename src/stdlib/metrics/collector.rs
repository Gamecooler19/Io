use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use tokio::time::interval;

#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Summary { sum: f64, count: u64 },
}

#[derive(Debug)]
pub struct MetricCollector {
    metrics: Arc<RwLock<HashMap<String, MetricValue>>>,
    labels: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    aggregation_interval: Duration,
}

impl MetricCollector {
    pub fn new(aggregation_interval: Duration) -> Self {
        let collector = Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            labels: Arc::new(RwLock::new(HashMap::new())),
            aggregation_interval,
        };

        // Start background aggregation task
        collector.start_aggregation();
        collector
    }

    pub fn increment_counter(&self, name: &str, value: u64) {
        let mut metrics = self.metrics.write();
        match metrics.get_mut(name) {
            Some(MetricValue::Counter(count)) => *count += value,
            _ => {
                metrics.insert(name.to_string(), MetricValue::Counter(value));
            }
        }
    }

    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write();
        metrics.insert(name.to_string(), MetricValue::Gauge(value));
    }

    pub fn observe_histogram(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write();
        match metrics.get_mut(name) {
            Some(MetricValue::Histogram(values)) => values.push(value),
            _ => {
                metrics.insert(name.to_string(), MetricValue::Histogram(vec![value]));
            }
        }
    }

    pub fn add_label(&self, metric_name: &str, key: &str, value: &str) {
        let mut labels = self.labels.write();
        labels
            .entry(metric_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
    }

    fn start_aggregation(&self) {
        let metrics = self.metrics.clone();
        let interval_duration = self.aggregation_interval;

        tokio::spawn(async move {
            let mut interval = interval(interval_duration);
            loop {
                interval.tick().await;
                Self::aggregate_metrics(&metrics);
            }
        });
    }

    fn aggregate_metrics(metrics: &Arc<RwLock<HashMap<String, MetricValue>>>) {
        let mut metrics = metrics.write();
        
        for value in metrics.values_mut() {
            match value {
                MetricValue::Histogram(values) => {
                    if !values.is_empty() {
                        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    }
                }
                MetricValue::Summary { sum, count } => {
                    if *count > 0 {
                        *sum /= *count as f64;
                        *count = 0;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn export_prometheus(&self) -> String {
        let metrics = self.metrics.read();
        let labels = self.labels.read();
        let mut output = String::new();

        for (name, value) in metrics.iter() {
            let label_str = if let Some(metric_labels) = labels.get(name) {
                let labels = metric_labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{}}}", labels)
            } else {
                String::new()
            };

            match value {
                MetricValue::Counter(count) => {
                    output.push_str(&format!("{}{} {}\n", name, label_str, count));
                }
                MetricValue::Gauge(value) => {
                    output.push_str(&format!("{}{} {}\n", name, label_str, value));
                }
                MetricValue::Histogram(values) => {
                    if !values.is_empty() {
                        output.push_str(&format!("{}_count{} {}\n", name, label_str, values.len()));
                        output.push_str(&format!("{}_sum{} {}\n", name, label_str, values.iter().sum::<f64>()));
                    }
                }
                MetricValue::Summary { sum, count } => {
                    output.push_str(&format!("{}_sum{} {}\n", name, label_str, sum));
                    output.push_str(&format!("{}_count{} {}\n", name, label_str, count));
                }
            }
        }

        output
    }
}
