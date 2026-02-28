use chrono::Utc;
use shiplog_watermark::*;

#[test]
fn watermark_creation() {
    let wm = Watermark::new(1000);
    assert_eq!(wm.timestamp_millis(), 1000);
}

#[test]
fn watermark_default_is_min() {
    let wm = Watermark::default();
    assert_eq!(wm.timestamp_millis(), i64::MIN);
}

#[test]
fn watermark_from_datetime() {
    let dt = Utc::now();
    let wm = Watermark::from_datetime(&dt);
    assert!(wm.timestamp_millis() > 0);
}

#[test]
fn watermark_to_datetime() {
    let wm = Watermark::new(1_700_000_000_000);
    let dt = wm.to_datetime();
    assert_eq!(dt.timestamp_millis(), 1_700_000_000_000);
}

#[test]
fn watermark_comparison() {
    let wm = Watermark::new(1000);
    assert!(wm.is_before(2000));
    assert!(!wm.is_before(500));
    assert!(wm.is_after(500));
    assert!(!wm.is_after(2000));
    assert!(!wm.is_before(1000));
    assert!(!wm.is_after(1000));
}

#[test]
fn watermark_equality() {
    assert_eq!(Watermark::new(100), Watermark::new(100));
    assert_ne!(Watermark::new(100), Watermark::new(200));
}

#[test]
fn periodic_watermark_generator() {
    let mut pwg = PeriodicWatermarkGenerator::new(100);
    let wm1 = pwg.update(1000);
    assert_eq!(wm1.timestamp_millis(), 900);
    let wm2 = pwg.update(2000);
    assert_eq!(wm2.timestamp_millis(), 1900);
    // Lower timestamp doesn't advance
    let wm3 = pwg.update(1500);
    assert_eq!(wm3.timestamp_millis(), 1900);
}

#[test]
fn periodic_watermark_current() {
    let pwg = PeriodicWatermarkGenerator::new(100);
    let wm = pwg.current();
    assert_eq!(wm.timestamp_millis(), i64::MIN);
}

#[test]
fn tumbling_watermark_generator() {
    let mut twg = TumblingWatermarkGenerator::new(1000);
    let wm = twg.current();
    assert!(wm.timestamp_millis() > 0);
}

#[test]
fn watermark_tracker_basic() {
    let mut tracker = WatermarkTracker::new(3);
    tracker.add(Watermark::new(100));
    tracker.add(Watermark::new(200));
    tracker.add(Watermark::new(300));
    assert_eq!(tracker.earliest().unwrap().timestamp_millis(), 100);
    assert_eq!(tracker.latest().unwrap().timestamp_millis(), 300);
}

#[test]
fn watermark_tracker_evicts_oldest() {
    let mut tracker = WatermarkTracker::new(2);
    tracker.add(Watermark::new(1));
    tracker.add(Watermark::new(2));
    tracker.add(Watermark::new(3));
    assert_eq!(tracker.earliest().unwrap().timestamp_millis(), 2);
    assert_eq!(tracker.latest().unwrap().timestamp_millis(), 3);
    assert_eq!(tracker.watermarks().len(), 2);
}

#[test]
fn watermark_tracker_empty() {
    let tracker = WatermarkTracker::default();
    assert!(tracker.latest().is_none());
    assert!(tracker.earliest().is_none());
}
