use shiplog_metrics::*;

#[test]
fn counter_increments() {
    let mut c = MetricsCollector::new();
    c.inc_counter("req", 1.0);
    c.inc_counter("req", 2.0);
    assert_eq!(c.get_counter("req"), Some(3.0));
}

#[test]
fn counter_missing_returns_none() {
    let c = MetricsCollector::new();
    assert_eq!(c.get_counter("missing"), None);
}

#[test]
fn gauge_overwrites() {
    let mut c = MetricsCollector::new();
    c.set_gauge("temp", 72.0);
    c.set_gauge("temp", 73.0);
    assert_eq!(c.get_gauge("temp"), Some(73.0));
}

#[test]
fn histogram_records_values() {
    let mut c = MetricsCollector::new();
    c.record_histogram("lat", 10.0);
    c.record_histogram("lat", 20.0);
    c.record_histogram("lat", 30.0);
    assert_eq!(c.get_histogram("lat").unwrap().len(), 3);
}

#[test]
fn histogram_stats() {
    let mut c = MetricsCollector::new();
    c.record_histogram("lat", 10.0);
    c.record_histogram("lat", 20.0);
    c.record_histogram("lat", 30.0);
    let stats = c.histogram_stats("lat").unwrap();
    assert_eq!(stats.count, 3);
    assert_eq!(stats.sum, 60.0);
    assert!((stats.mean - 20.0).abs() < f64::EPSILON);
    assert_eq!(stats.min, Some(10.0));
    assert_eq!(stats.max, Some(30.0));
}

#[test]
fn histogram_stats_missing() {
    let c = MetricsCollector::new();
    assert!(c.histogram_stats("missing").is_none());
}

#[test]
fn export_all_metrics() {
    let mut c = MetricsCollector::new();
    c.inc_counter("c", 1.0);
    c.set_gauge("g", 2.0);
    c.record_histogram("h", 3.0);
    let exported = c.export();
    assert_eq!(exported.len(), 3);
}

#[test]
fn clear_metrics() {
    let mut c = MetricsCollector::new();
    c.inc_counter("c", 1.0);
    c.set_gauge("g", 2.0);
    c.clear();
    assert_eq!(c.get_counter("c"), None);
    assert_eq!(c.get_gauge("g"), None);
}

#[test]
fn metric_point_with_labels() {
    let p = MetricPoint::new("req", MetricType::Counter, 1.0)
        .with_label("method", "GET")
        .with_label("status", "200");
    assert_eq!(p.labels.get("method"), Some(&"GET".to_string()));
    assert_eq!(p.labels.get("status"), Some(&"200".to_string()));
}

#[test]
fn metrics_report_to_json() {
    let mut c = MetricsCollector::new();
    c.inc_counter("test", 1.0);
    let report = MetricsReport::new(c.export());
    let json = report.to_json();
    assert!(json.contains("test"));
}
