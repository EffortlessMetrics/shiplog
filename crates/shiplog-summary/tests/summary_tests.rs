use shiplog_summary::*;

#[test]
fn summary_basic_stats() {
    let mut s = Summary::new();
    s.add(1.0);
    s.add(2.0);
    s.add(3.0);
    assert_eq!(s.count(), 3);
    assert_eq!(s.sum(), 6.0);
    assert!((s.mean() - 2.0).abs() < f64::EPSILON);
    assert_eq!(s.min(), 1.0);
    assert_eq!(s.max(), 3.0);
}

#[test]
fn summary_empty() {
    let s = Summary::new();
    assert_eq!(s.count(), 0);
    assert_eq!(s.min(), 0.0);
    assert_eq!(s.max(), 0.0);
}

#[test]
fn summary_single_value() {
    let mut s = Summary::new();
    s.add(42.0);
    assert!((s.mean() - 42.0).abs() < f64::EPSILON);
    assert_eq!(s.variance(), 0.0);
    assert_eq!(s.std_dev(), 0.0);
}

#[test]
fn summary_reset() {
    let mut s = Summary::new();
    s.add(10.0);
    s.reset();
    assert_eq!(s.count(), 0);
}

#[test]
fn summary_default() {
    let s = Summary::default();
    assert_eq!(s.count(), 0);
}

#[test]
fn running_stats_basic() {
    let mut rs = RunningStats::new();
    for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
        rs.push(v);
    }
    assert_eq!(rs.count(), 5);
    assert!((rs.mean() - 3.0).abs() < f64::EPSILON);
    assert_eq!(rs.min(), 1.0);
    assert_eq!(rs.max(), 5.0);
    assert!(rs.variance() > 0.0);
    assert!(rs.std_dev() > 0.0);
}

#[test]
fn running_stats_empty() {
    let rs = RunningStats::new();
    assert_eq!(rs.variance(), 0.0);
    assert_eq!(rs.min(), 0.0);
    assert_eq!(rs.max(), 0.0);
}

#[test]
fn descriptive_stats_median() {
    let mut ds = DescriptiveStats::new();
    ds.extend(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    assert!((ds.median() - 3.0).abs() < f64::EPSILON);

    let mut ds2 = DescriptiveStats::new();
    ds2.extend(vec![1.0, 2.0, 3.0, 4.0]);
    assert!((ds2.median() - 2.5).abs() < f64::EPSILON);
}

#[test]
fn descriptive_stats_variance() {
    let mut ds = DescriptiveStats::new();
    ds.extend(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
    assert!(ds.variance() > 0.0);
    assert!(ds.sample_variance() > ds.variance());
}

#[test]
fn standalone_functions() {
    assert!((mean(&[1.0, 2.0, 3.0]) - 2.0).abs() < f64::EPSILON);
    assert_eq!(mean(&[]), 0.0);
    assert!(variance(&[1.0, 2.0, 3.0]) > 0.0);
    assert!(std_dev(&[1.0, 2.0, 3.0]) > 0.0);
}
