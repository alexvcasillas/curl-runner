//! Latency statistics for profile mode (`utils/stats.ts`).

use crate::output::format::{stringify, to_fixed};
use curl_runner_core::model::ProfileStats;
use serde_json::json;

/// Percentile of an ascending `sorted` slice (linear interpolation).
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    match sorted.len() {
        0 => 0.0,
        1 => sorted[0],
        n => {
            let index = (p / 100.0) * (n - 1) as f64;
            let lower = index.floor();
            let upper = index.ceil();
            let fraction = index - lower;
            let (l, u) = (lower as usize, upper as usize);
            if l == u {
                sorted[l]
            } else {
                sorted[l] * (1.0 - fraction) + sorted[u] * fraction
            }
        }
    }
}

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population standard deviation.
pub fn std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    var.sqrt()
}

/// JS `Math.round(x * 100) / 100` (Math.round rounds .5 up).
fn round2(x: f64) -> f64 {
    (x * 100.0 + 0.5).floor() / 100.0
}

/// `Math.round(failures / total * 10000) / 100` (a percentage, 0 when `total` is 0).
pub fn failure_rate(failures: u64, total: u64) -> f64 {
    if total > 0 {
        (failures as f64 / total as f64 * 10000.0 + 0.5).floor() / 100.0
    } else {
        0.0
    }
}

/// `calculateProfileStats`: the first `warmup` timings are excluded.
pub fn profile_stats(timings: &[f64], warmup: u64, failures: u64) -> ProfileStats {
    let effective: Vec<f64> = timings.iter().skip(warmup as usize).copied().collect();
    let mut sorted = effective.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let m = mean(&sorted);
    let total = timings.len();
    ProfileStats {
        iterations: effective.len() as u64,
        warmup,
        min: sorted.first().copied().unwrap_or(0.0),
        max: sorted.last().copied().unwrap_or(0.0),
        mean: round2(m),
        median: round2(percentile(&sorted, 50.0)),
        p50: round2(percentile(&sorted, 50.0)),
        p95: round2(percentile(&sorted, 95.0)),
        p99: round2(percentile(&sorted, 99.0)),
        std_dev: round2(std_dev(&sorted, m)),
        failures,
        failure_rate: failure_rate(failures, total as u64),
        timings: effective,
    }
}

/// `generateHistogram`: ASCII latency distribution.
pub fn histogram(timings: &[f64], buckets: usize, width: usize) -> Vec<String> {
    if timings.is_empty() || buckets == 0 {
        return vec!["No data".into()];
    }
    let min = timings.iter().copied().fold(f64::INFINITY, f64::min);
    let max = timings.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = if max - min == 0.0 { 1.0 } else { max - min };
    let size = range / buckets as f64;
    let mut counts = vec![0usize; buckets];
    for t in timings {
        let b = (((t - min) / size).floor() as usize).min(buckets - 1);
        counts[b] += 1;
    }
    let max_count = counts.iter().copied().max().unwrap_or(0);
    (0..buckets)
        .map(|i| {
            let lo = min + i as f64 * size;
            let hi = min + (i + 1) as f64 * size;
            let bar = if max_count > 0 {
                ((counts[i] as f64 / max_count as f64) * width as f64 + 0.5).floor() as usize
            } else {
                0
            };
            format!(
                "{:>6}ms - {:>6}ms │{} {}",
                to_fixed(lo, 0),
                to_fixed(hi, 0),
                "█".repeat(bar),
                counts[i]
            )
        })
        .collect()
}

/// `exportToCSV`.
pub fn export_csv(stats: &ProfileStats) -> String {
    let mut lines = vec!["iteration,latency_ms".to_string()];
    lines.extend(
        stats
            .timings
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{},{}", i + 1, curl_runner_core::jsval::num_to_string(*t))),
    );
    lines.join("\n")
}

/// `exportToJSON`.
pub fn export_json(stats: &ProfileStats, request_name: &str) -> String {
    let v = json!({
        "request": request_name,
        "summary": {
            "iterations": stats.iterations,
            "warmup": stats.warmup,
            "failures": stats.failures,
            "failureRate": stats.failure_rate,
            "min": stats.min,
            "max": stats.max,
            "mean": stats.mean,
            "median": stats.median,
            "p50": stats.p50,
            "p95": stats.p95,
            "p99": stats.p99,
            "stdDev": stats.std_dev,
        },
        "timings": stats.timings,
    });
    stringify(&v, Some(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, digits: i32) -> bool {
        (a - b).abs() < 10f64.powi(-digits) / 2.0
    }

    // stats.test.ts
    #[test]
    fn percentile_cases() {
        assert_eq!(percentile(&[], 50.0), 0.0);
        assert_eq!(percentile(&[100.0], 50.0), 100.0);
        assert_eq!(percentile(&[100.0], 99.0), 100.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0], 50.0), 3.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 50.0), 2.5);
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert!(close(percentile(&v, 95.0), 95.05, 1));
        assert!(close(percentile(&v, 99.0), 99.01, 1));
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0, 50.0], 50.0), 30.0);
    }

    #[test]
    fn mean_and_std_dev() {
        assert_eq!(mean(&[]), 0.0);
        assert_eq!(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
        assert_eq!(mean(&[10.0, 20.0, 30.0]), 20.0);
        assert_eq!(mean(&[100.0]), 100.0);
        assert_eq!(std_dev(&[], 0.0), 0.0);
        assert_eq!(std_dev(&[100.0], 100.0), 0.0);
        let v = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!(close(std_dev(&v, mean(&v)), 2.0, 0));
    }

    #[test]
    fn profile_stats_cases() {
        let s = profile_stats(&[10.0, 20.0, 30.0, 40.0, 50.0], 0, 0);
        assert_eq!((s.iterations, s.warmup), (5, 0));
        assert_eq!((s.min, s.max, s.mean), (10.0, 50.0, 30.0));
        assert_eq!((s.failures, s.failure_rate), (0, 0.0));

        let s = profile_stats(&[100.0, 10.0, 20.0, 30.0, 40.0], 1, 0);
        assert_eq!((s.iterations, s.warmup), (4, 1));
        assert_eq!((s.min, s.max, s.mean), (10.0, 40.0, 25.0));

        let s = profile_stats(&[10.0, 20.0, 30.0], 0, 2);
        assert_eq!(s.failures, 2);
        assert!(close(s.failure_rate, 66.67, 1));

        let s = profile_stats(&[], 0, 0);
        assert_eq!((s.iterations, s.min, s.max, s.mean), (0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn histogram_cases() {
        assert_eq!(histogram(&[], 10, 40), vec!["No data"]);
        let t: Vec<f64> = (0..100).map(f64::from).collect();
        assert_eq!(histogram(&t, 5, 20).len(), 5);
        let h = histogram(&[10.0, 20.0, 30.0, 40.0, 50.0], 2, 10);
        assert!(h[0].contains("ms -") && h[0].contains("ms │"));
        assert_eq!(h[0], "    10ms -     30ms │███████ 2");
    }

    #[test]
    fn exports() {
        let s = profile_stats(&[10.0, 20.0, 30.0], 0, 0);
        let csv = export_csv(&s);
        assert!(csv.contains("iteration,latency_ms"));
        assert!(csv.contains("1,10") && csv.contains("2,20") && csv.contains("3,30"));
        let j: serde_json::Value = serde_json::from_str(&export_json(&s, "Test Request")).unwrap();
        assert_eq!(j["request"], "Test Request");
        assert_eq!(j["summary"]["iterations"], 3);
        assert_eq!(j["summary"]["min"], 10);
        assert_eq!(j["summary"]["max"], 30);
        assert_eq!(j["timings"], serde_json::json!([10, 20, 30]));
    }
}
