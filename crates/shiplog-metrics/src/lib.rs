//! Metrics collection and reporting for shiplog.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metric value type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    /// Counter - monotonically increasing value
    Counter,
    /// Gauge - point-in-time value
    Gauge,
    /// Histogram - distribution of values
    Histogram,
}

/// A single metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: i64,
}

impl MetricPoint {
    /// Create a new metric point
    pub fn new(name: impl Into<String>, metric_type: MetricType, value: f64) -> Self {
        Self {
            name: name.into(),
            metric_type,
            value,
            labels: HashMap::new(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Add a label to the metric
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// Metrics collector
#[derive(Debug, Default)]
pub struct MetricsCollector {
    counters: HashMap<String, f64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, Vec<f64>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment a counter
    pub fn inc_counter(&mut self, name: &str, value: f64) {
        *self.counters.entry(name.to_string()).or_insert(0.0) += value;
    }

    /// Set a gauge value
    pub fn set_gauge(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }

    /// Record a histogram value
    pub fn record_histogram(&mut self, name: &str, value: f64) {
        self.histograms
            .entry(name.to_string())
            .or_default()
            .push(value);
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> Option<f64> {
        self.counters.get(name).copied()
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).copied()
    }

    /// Get histogram values
    pub fn get_histogram(&self, name: &str) -> Option<&[f64]> {
        self.histograms.get(name).map(|v| v.as_slice())
    }

    /// Get histogram statistics
    pub fn histogram_stats(&self, name: &str) -> Option<HistogramStats> {
        self.histograms.get(name).map(|values| {
            let count = values.len() as f64;
            let sum: f64 = values.iter().sum();
            let mean = sum / count;

            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let min = sorted.first().copied();
            let max = sorted.last().copied();
            let median = if count > 0.0 {
                sorted[(count as usize) / 2]
            } else {
                0.0
            };

            HistogramStats {
                count: count as u64,
                sum,
                mean,
                min,
                max,
                median,
            }
        })
    }

    /// Export all metrics as points
    pub fn export(&self) -> Vec<MetricPoint> {
        let mut points = Vec::new();

        for (name, &value) in &self.counters {
            points.push(MetricPoint::new(name, MetricType::Counter, value));
        }

        for (name, &value) in &self.gauges {
            points.push(MetricPoint::new(name, MetricType::Gauge, value));
        }

        for (name, values) in &self.histograms {
            for &value in values {
                points.push(MetricPoint::new(name, MetricType::Histogram, value));
            }
        }

        points
    }

    /// Clear all metrics
    pub fn clear(&mut self) {
        self.counters.clear();
        self.gauges.clear();
        self.histograms.clear();
    }
}

/// Histogram statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramStats {
    pub count: u64,
    pub sum: f64,
    pub mean: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub median: f64,
}

/// Metrics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReport {
    pub timestamp: i64,
    pub metrics: Vec<MetricPoint>,
}

impl MetricsReport {
    /// Create a new metrics report
    pub fn new(metrics: Vec<MetricPoint>) -> Self {
        Self {
            timestamp: chrono::Utc::now().timestamp(),
            metrics,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_metrics() {
        let mut collector = MetricsCollector::new();

        collector.inc_counter("requests", 1.0);
        collector.inc_counter("requests", 2.0);

        assert_eq!(collector.get_counter("requests"), Some(3.0));
    }

    #[test]
    fn gauge_metrics() {
        let mut collector = MetricsCollector::new();

        collector.set_gauge("temperature", 72.5);
        collector.set_gauge("temperature", 73.0);

        assert_eq!(collector.get_gauge("temperature"), Some(73.0));
    }

    #[test]
    fn histogram_metrics() {
        let mut collector = MetricsCollector::new();

        collector.record_histogram("response_time", 10.0);
        collector.record_histogram("response_time", 20.0);
        collector.record_histogram("response_time", 30.0);

        let stats = collector.histogram_stats("response_time").unwrap();

        assert_eq!(stats.count, 3);
        assert_eq!(stats.sum, 60.0);
        assert_eq!(stats.mean, 20.0);
        assert_eq!(stats.min, Some(10.0));
        assert_eq!(stats.max, Some(30.0));
    }

    #[test]
    fn export_metrics() {
        let mut collector = MetricsCollector::new();

        collector.inc_counter("events", 5.0);
        collector.set_gauge("active", 10.0);
        collector.record_histogram("latency", 100.0);

        let exported = collector.export();

        assert_eq!(exported.len(), 3);
    }

    #[test]
    fn metrics_report() {
        let mut collector = MetricsCollector::new();
        collector.inc_counter("test", 1.0);

        let report = MetricsReport::new(collector.export());
        let json = report.to_json();

        assert!(json.contains("test"));
    }

    #[test]
    fn metric_point_with_labels() {
        let point = MetricPoint::new("request", MetricType::Counter, 1.0)
            .with_label("method", "GET")
            .with_label("status", "200");

        assert_eq!(point.labels.get("method"), Some(&"GET".to_string()));
        assert_eq!(point.labels.get("status"), Some(&"200".to_string()));
    }
}
