//! Profile mode (`executor/profile-executor.ts` + `cli.ts` `executeProfileMode`).
//!
//! Iterations run through the shared [`Executor`] (so transport, redirects
//! and TLS are the same code as a normal run), with everything a normal run
//! adds on top stripped, as TS's "minimal overhead" iteration does: no
//! `expect`, retries, `when`, `store` or snapshots, and never a dry run.
//! `concurrency` 1 runs sequentially, more uses the parallel (`Multi`) path
//! with that many requests live.

use crate::output::{Console, Logger};
use crate::run::CancelSlot;
use crate::stats;
use curl_runner_core::executor::{Executor, NoopSink, PreparedRequest};
use curl_runner_core::model::{GlobalConfig, Json, ProfileConfig, ProfileStats, RequestConfig};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Request fields a profile iteration ignores.
const STRIPPED: [&str; 5] = ["expect", "retry", "when", "store", "snapshot"];

/// Global config the iterations run under: sequential for concurrency 1
/// (or unset), otherwise the parallel path with that many live requests.
/// Never a dry run (TS profiles always send), and a failed iteration never
/// stops the rest.
pub fn profile_global(global: &GlobalConfig, cfg: &ProfileConfig) -> GlobalConfig {
    let mut g = global.clone();
    // TS `concurrency === 1` is sequential; 0 would loop forever there, here
    // it is treated as 1.
    let concurrency = cfg.concurrency.unwrap_or(1).max(1);
    if concurrency == 1 {
        g.execution = Some("sequential".into());
        g.max_concurrency = None;
    } else {
        g.execution = Some("parallel".into());
        g.max_concurrency = Some(concurrency);
    }
    g.dry_run = None;
    g.continue_on_error = Some(true);
    g
}

/// The request as a profile iteration sends it: transport only.
pub fn iteration_request(p: &PreparedRequest) -> PreparedRequest {
    let mut raw = p.raw.clone();
    if let Some(m) = raw.as_object_mut() {
        for k in STRIPPED {
            m.remove(k);
        }
    }
    PreparedRequest { raw }
}

/// One finished iteration: only what the stats need.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub ok: bool,
    pub ms: f64,
}

/// A profiled request (what the logger and the exports read).
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileOutcome {
    pub request: RequestConfig,
    pub stats: ProfileStats,
    /// Iterations actually run, warmup included (less than asked after Ctrl-C).
    pub attempted: u64,
}

/// Stats of the finished iterations, split by position: the first `warmup`
/// are warmup (run, then dropped), the rest are measured. Timings are the
/// measured successes; `failureRate` is the measured failures over the
/// measured count (TS divided by the number of successes, which could
/// exceed 100%); `failures` counts every failed iteration, warmup included.
pub fn collect_stats(samples: &[Sample], warmup: u64) -> ProfileStats {
    let warmup = usize::try_from(warmup)
        .unwrap_or(usize::MAX)
        .min(samples.len());
    let (_, measured) = samples.split_at(warmup);
    let timings: Vec<f64> = measured.iter().filter(|s| s.ok).map(|s| s.ms).collect();
    let measured_failures = (measured.len() - timings.len()) as u64;
    let total_failures = samples.iter().filter(|s| !s.ok).count() as u64;
    let mut s = stats::profile_stats(&timings, 0, total_failures);
    s.warmup = warmup as u64;
    s.failure_rate = stats::failure_rate(measured_failures, measured.len() as u64);
    s
}

/// The typed request for display and export (name, url, method).
fn typed_request(raw: &Json) -> RequestConfig {
    serde_json::from_value(raw.clone()).unwrap_or_else(|_| {
        let s = |k: &str| raw.get(k).and_then(Json::as_str).map(String::from);
        RequestConfig {
            name: s("name"),
            url: s("url").unwrap_or_default(),
            method: s("method"),
            ..Default::default()
        }
    })
}

/// Requests handed to the executor at once (rounded up to a multiple of
/// `concurrency`), so memory never depends on the iteration count.
const WINDOW: u64 = 256;

/// Size of one scheduling window.
pub fn window_size(concurrency: u64) -> u64 {
    let c = concurrency.max(1);
    WINDOW.div_ceil(c).max(1) * c
}

/// Run `iterations` copies of `one` in bounded windows, keeping only
/// (success, duration) per iteration. Stops early when `ex` is cancelled.
pub fn run_iterations(
    ex: &Executor,
    one: &PreparedRequest,
    iterations: u64,
    concurrency: u64,
) -> Vec<Sample> {
    let window = window_size(concurrency);
    let mut samples = Vec::with_capacity(iterations.min(1 << 16) as usize);
    let cancel = ex.cancel_handle();
    let mut left = iterations;
    while left > 0 && !cancel.load(Ordering::Relaxed) {
        let n = left.min(window);
        let summary = ex.run(vec![one.clone(); n as usize], &mut NoopSink);
        samples.extend(
            summary
                .results
                .iter()
                .map(|r| match (&r.metrics, r.success) {
                    (Some(m), true) => Sample {
                        ok: true,
                        ms: m.duration,
                    },
                    _ => Sample { ok: false, ms: 0.0 },
                }),
        );
        left -= n;
    }
    samples
}

/// Run one request `cfg.iterations` times (warmup included: they are run
/// and then dropped from the stats).
pub fn profile_request(ex: &Executor, p: &PreparedRequest, cfg: &ProfileConfig) -> ProfileOutcome {
    let one = iteration_request(p);
    let samples = run_iterations(ex, &one, cfg.iterations, cfg.concurrency.unwrap_or(1));
    ProfileOutcome {
        request: typed_request(&one.raw),
        stats: collect_stats(&samples, cfg.warmup.unwrap_or(1)),
        attempted: samples.len() as u64,
    }
}

/// Exit code: 1 when more than half of the iterations failed, warmup
/// included (`cli.ts`: failures / (iterations + warmup) > 0.5). The total is
/// the iterations actually run; the `total > 0` guard only matters for a run
/// cancelled before its first iteration (no failures either), where TS's
/// divisor could be 0 (Infinity > 0.5).
pub fn exit_code(results: &[ProfileOutcome]) -> i32 {
    let failures: u64 = results.iter().map(|r| r.stats.failures).sum();
    let total: u64 = results.iter().map(|r| r.attempted).sum();
    i32::from(total > 0 && failures as f64 / total as f64 > 0.5)
}

/// File content of `--profile-export` (CSV when the name ends in `.csv`).
pub fn export_content(results: &[ProfileOutcome], file: &str) -> String {
    let csv = file.ends_with(".csv");
    let parts: Vec<String> = results
        .iter()
        .map(|r| {
            let name = r.request.name.clone().unwrap_or_default();
            let name = if name.is_empty() {
                &r.request.url
            } else {
                &name
            };
            if csv {
                stats::export_csv(&r.stats)
            } else {
                stats::export_json(&r.stats, name)
            }
        })
        .collect();
    if csv {
        parts.join("\n\n")
    } else {
        format!("[{}]", parts.join(",\n"))
    }
}

/// Profile every request, print, export; returns the exit code.
#[allow(clippy::too_many_arguments)]
pub fn run(
    requests: &[PreparedRequest],
    global: &GlobalConfig,
    cfg: &ProfileConfig,
    cwd: &Path,
    log: &Logger,
    console: &mut Console,
    interrupted: &AtomicBool,
    slot: &CancelSlot,
) -> i32 {
    let ex = Executor::new(profile_global(global, cfg)).with_cwd(cwd);
    let cancel = ex.cancel_handle();
    if let Ok(mut s) = slot.lock() {
        *s = Some(cancel.clone());
    }
    let (warmup, concurrency) = (cfg.warmup.unwrap_or(1), cfg.concurrency.unwrap_or(1));
    let mut results = Vec::new();
    for (i, p) in requests.iter().enumerate() {
        if interrupted.load(Ordering::SeqCst) {
            cancel.store(true, Ordering::SeqCst);
            break;
        }
        let req = typed_request(&p.raw);
        let name = req.name.clone().filter(|n| !n.is_empty());
        log.log_profile_start(
            console,
            &name.unwrap_or_else(|| format!("Request {}", i + 1)),
            cfg.iterations,
            warmup,
            concurrency,
        );
        let r = profile_request(&ex, p, cfg);
        if r.attempted == 0 && interrupted.load(Ordering::SeqCst) {
            break;
        }
        log.log_profile_result(console, &r, cfg.histogram.unwrap_or(false));
        results.push(r);
    }
    if let Ok(mut s) = slot.lock() {
        *s = None;
    }
    log.log_profile_summary(console, &results);

    let mut code = exit_code(&results);
    if let Some(file) = cfg.export_file.as_deref().filter(|f| !f.is_empty()) {
        match std::fs::write(cwd.join(file), export_content(&results, file)) {
            Ok(()) => log.log_info(console, &format!("Profile results exported to {file}")),
            Err(e) => {
                log.log_error(console, &format!("Cannot export profile results: {e}"));
                code = 1;
            }
        }
    }
    if interrupted.load(Ordering::SeqCst) {
        log.log_error(console, "Interrupted");
        code = crate::run::EXIT_INTERRUPTED;
    }
    console.flush();
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::config::{self, CliOptions, ExecutionMode};
    use serde_json::json;
    use testkit::TestServer;

    fn ok(ms: f64) -> Sample {
        Sample { ok: true, ms }
    }

    fn bad() -> Sample {
        Sample { ok: false, ms: 0.0 }
    }

    fn pr(v: Json) -> PreparedRequest {
        PreparedRequest { raw: v }
    }

    fn result_with(iter: u64, warmup: u64, failures: u64, timings: &[f64]) -> ProfileOutcome {
        let mut stats = stats::profile_stats(timings, 0, failures);
        stats.iterations = iter;
        stats.warmup = warmup;
        ProfileOutcome {
            request: RequestConfig::default(),
            stats,
            attempted: iter + warmup + failures,
        }
    }

    // profile-executor.test.ts: 'stats calculation matches expected ProfileResult structure'
    #[test]
    fn stats_structure_and_warmup_exclusion() {
        let results: Vec<_> = (0..10).map(|i| ok(50.0 + 5.0 * f64::from(i))).collect();
        let s = collect_stats(&results, 2);
        assert_eq!((s.iterations, s.warmup, s.timings.len()), (8, 2, 8));
        let v = serde_json::to_value(&s).unwrap();
        for k in [
            "iterations",
            "warmup",
            "min",
            "max",
            "mean",
            "median",
            "p50",
            "p95",
            "p99",
            "stdDev",
            "failures",
            "failureRate",
            "timings",
        ] {
            assert!(v.get(k).is_some(), "missing {k}");
        }
    }

    // 'profile config defaults are applied correctly'
    #[test]
    fn config_defaults_are_applied() {
        let cli = CliOptions {
            profile: Some(10),
            ..Default::default()
        };
        let c = config::build_profile_config(&cli, &GlobalConfig::default());
        assert_eq!(
            (c.iterations, c.warmup, c.concurrency, c.histogram),
            (10, Some(1), Some(1), Some(false))
        );
    }

    // 'concurrent config changes iterations behavior'
    #[test]
    fn concurrency_selects_the_parallel_path() {
        let g = GlobalConfig {
            dry_run: Some(true),
            ..Default::default()
        };
        let seq = profile_global(
            &g,
            &ProfileConfig {
                iterations: 100,
                concurrency: Some(1),
                ..Default::default()
            },
        );
        assert_eq!(seq.execution.as_deref(), Some("sequential"));
        let par = profile_global(
            &g,
            &ProfileConfig {
                iterations: 100,
                concurrency: Some(10),
                ..Default::default()
            },
        );
        assert_eq!(par.execution.as_deref(), Some("parallel"));
        assert_eq!(par.max_concurrency, Some(10));
        // never a dry run, never stops at the first failure
        assert_eq!(par.dry_run, None);
        assert_eq!(par.continue_on_error, Some(true));
        // unset concurrency is sequential
        let none = profile_global(
            &g,
            &ProfileConfig {
                iterations: 5,
                ..Default::default()
            },
        );
        assert_eq!(none.execution.as_deref(), Some("sequential"));
    }

    // 'failure tracking affects failureRate calculation'
    #[test]
    fn failures_are_counted_and_excluded_from_timings() {
        let s = collect_stats(&[ok(10.0), bad(), ok(30.0), bad(), ok(50.0)], 0);
        assert_eq!(s.failures, 2);
        assert_eq!(s.timings, vec![10.0, 30.0, 50.0]);
        // failures over the measured iterations (TS divided by the 3 successes: 66.67)
        assert_eq!(s.failure_rate, 40.0);
        // never above 100%, even when most iterations failed
        let s = collect_stats(&[ok(1.0), bad(), bad(), bad()], 0);
        assert_eq!(s.failure_rate, 75.0);
        let s = collect_stats(&[bad(), bad(), bad()], 0);
        assert_eq!((s.failure_rate, s.iterations), (100.0, 0));
        // rounded like TS: 1/3 -> 33.33
        assert_eq!(
            collect_stats(&[ok(1.0), ok(2.0), bad()], 0).failure_rate,
            33.33
        );
    }

    #[test]
    fn warmup_is_split_by_position() {
        // warmup failure counts in `failures` but not in the rate or the timings
        let s = collect_stats(&[bad(), ok(10.0), ok(20.0), bad()], 1);
        assert_eq!((s.warmup, s.iterations, s.failures), (1, 2, 2));
        assert_eq!(s.timings, vec![10.0, 20.0]);
        assert_eq!(s.failure_rate, 33.33);
        // a successful warmup run is dropped by position, a failed measured run is not
        let s = collect_stats(&[ok(500.0), bad(), ok(10.0)], 1);
        assert_eq!(s.timings, vec![10.0]);
        assert_eq!(s.failure_rate, 50.0);
    }

    // 'warmup iterations are excluded from percentile calculations'
    #[test]
    fn warmup_is_excluded_from_percentiles() {
        let mut t = vec![500.0, 400.0];
        t.extend([100.0; 8]);
        let results: Vec<_> = t.into_iter().map(ok).collect();
        let s = collect_stats(&results, 2);
        assert_eq!((s.min, s.max, s.mean), (100.0, 100.0, 100.0));
        assert_eq!((s.p50, s.p95, s.p99), (100.0, 100.0, 100.0));
    }

    #[test]
    fn warmup_is_capped_by_the_iterations_run() {
        let s = collect_stats(&[ok(5.0), bad(), bad()], 3);
        assert_eq!(
            (s.iterations, s.warmup, s.failures, s.failure_rate),
            (0, 3, 2, 0.0)
        );
        let s = collect_stats(&[], 1);
        assert_eq!(
            (s.iterations, s.warmup, s.failures, s.failure_rate),
            (0, 0, 0, 0.0)
        );
    }

    // 'export file extension determines format'
    #[test]
    fn export_extension_picks_the_format() {
        let r = result_with(3, 0, 0, &[10.0, 20.0, 30.0]);
        let two = [r.clone(), r.clone()];
        let csv = export_content(&two, "results.csv");
        assert_eq!(
            csv,
            "iteration,latency_ms\n1,10\n2,20\n3,30\n\niteration,latency_ms\n1,10\n2,20\n3,30"
        );
        let js = export_content(&two, "results.json");
        assert!(js.starts_with("[{\n  \"request\": \"\","), "{js}");
        assert!(js.contains("}\n,\n") || js.contains("},\n{"), "{js}");
        let parsed: Json = serde_json::from_str(&js).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        // not .csv -> JSON, whatever the extension
        assert!(export_content(&two, "results.txt").starts_with('['));
        assert_eq!(export_content(&[], "r.json"), "[]");
        assert_eq!(export_content(&[], "r.csv"), "");
    }

    #[test]
    fn export_names_the_request_by_name_then_url() {
        let mut r = result_with(1, 0, 0, &[1.5]);
        r.request.url = "http://x/y".into();
        assert!(export_content(&[r.clone()], "a.json").contains("\"request\": \"http://x/y\""));
        r.request.name = Some("Named".into());
        assert!(export_content(&[r], "a.json").contains("\"request\": \"Named\""));
    }

    // golden: the CSV header of --profile-export
    #[test]
    fn profile_export_csv_header() {
        let golden = include_str!("../tests/golden/profile_export.csv");
        let r = result_with(3, 0, 0, &[10.0, 20.5, 30.0]);
        assert_eq!(export_content(&[r], "t.csv"), golden.trim_end_matches('\n'));
    }

    // 'iterations must be positive' / 'concurrency defaults to 1'
    #[test]
    fn mode_needs_positive_iterations() {
        let on = CliOptions {
            profile: Some(10),
            ..Default::default()
        };
        assert_eq!(
            config::determine_execution_mode(&on, &GlobalConfig::default()),
            ExecutionMode::Profile
        );
        let zero = CliOptions {
            profile: Some(0),
            ..Default::default()
        };
        assert_eq!(
            config::determine_execution_mode(&zero, &GlobalConfig::default()),
            ExecutionMode::Normal
        );
    }

    #[test]
    fn exit_code_is_one_above_half_failures() {
        // failures (warmup included) / iterations run > 0.5
        assert_eq!(exit_code(&[result_with(5, 1, 0, &[1.0])]), 0);
        assert_eq!(exit_code(&[result_with(3, 0, 3, &[1.0])]), 0); // 3/6 = 0.5
        assert_eq!(exit_code(&[result_with(2, 0, 3, &[1.0])]), 1);
        // sums over the requests
        let a = result_with(2, 0, 0, &[1.0]);
        let b = result_with(0, 0, 3, &[]);
        let b = ProfileOutcome { attempted: 3, ..b };
        assert_eq!(exit_code(&[a, b]), 1); // 3/5
        // every iteration failed
        let all = ProfileOutcome {
            attempted: 4,
            ..result_with(0, 0, 4, &[])
        };
        assert_eq!(exit_code(&[all]), 1);
        // nothing ran (cancelled at once)
        assert_eq!(exit_code(&[]), 0);
        assert_eq!(
            exit_code(&[ProfileOutcome {
                attempted: 0,
                ..result_with(0, 0, 0, &[])
            }]),
            0
        );
    }

    #[test]
    fn iteration_request_strips_the_non_transport_fields() {
        let p = pr(json!({
            "url": "http://x", "method": "POST", "name": "n",
            "expect": {"status": 200}, "retry": {"count": 3}, "when": {"left": "a", "operator": "==", "right": "a"},
            "store": {"x": "body.id"}, "snapshot": true, "headers": {"A": "b"}
        }));
        let q = iteration_request(&p);
        assert_eq!(
            q.raw,
            json!({"url": "http://x", "method": "POST", "name": "n", "headers": {"A": "b"}})
        );
    }

    fn executor(cfg: &ProfileConfig) -> Executor {
        Executor::new(profile_global(&GlobalConfig::default(), cfg))
    }

    #[test]
    fn profiles_a_live_request_sequentially() {
        let srv = TestServer::start();
        let cfg = ProfileConfig {
            iterations: 5,
            warmup: Some(2),
            concurrency: Some(1),
            ..Default::default()
        };
        let p = pr(
            json!({"url": format!("{}/json/user", srv.base_url), "name": "User",
            "expect": {"status": 418}, "retry": {"count": 2}}),
        );
        let r = profile_request(&executor(&cfg), &p, &cfg);
        assert_eq!(r.attempted, 5);
        assert_eq!(
            (r.stats.iterations, r.stats.warmup, r.stats.failures),
            (3, 2, 0)
        );
        assert_eq!(r.request.name.as_deref(), Some("User"));
        assert_eq!(r.request.url, format!("{}/json/user", srv.base_url));
        assert!(r.stats.timings.iter().all(|t| *t > 0.0));
    }

    #[test]
    fn profiles_a_live_request_concurrently() {
        let srv = TestServer::start();
        let cfg = ProfileConfig {
            iterations: 7,
            warmup: Some(1),
            concurrency: Some(3),
            ..Default::default()
        };
        let p = pr(json!({"url": format!("{}/json/user", srv.base_url)}));
        let r = profile_request(&executor(&cfg), &p, &cfg);
        assert_eq!(r.attempted, 7);
        assert_eq!(
            (r.stats.iterations, r.stats.warmup, r.stats.failures),
            (6, 1, 0)
        );
    }

    #[test]
    fn transport_failures_count_as_failures() {
        let cfg = ProfileConfig {
            iterations: 3,
            warmup: Some(1),
            ..Default::default()
        };
        let p = pr(json!({"url": "http://127.0.0.1:1/x", "timeout": 2000}));
        let r = profile_request(&executor(&cfg), &p, &cfg);
        assert_eq!(
            (r.stats.failures, r.stats.iterations, r.stats.warmup),
            (3, 0, 1)
        );
        assert_eq!(r.stats.failure_rate, 100.0);
        assert_eq!(exit_code(&[r]), 1);
    }

    #[test]
    fn cancellation_stops_and_keeps_partial_results() {
        let srv = TestServer::start();
        let cfg = ProfileConfig {
            iterations: 100_000,
            warmup: Some(0),
            ..Default::default()
        };
        let ex = executor(&cfg);
        let cancel = ex.cancel_handle();
        let p = pr(json!({"url": format!("{}/json/user", srv.base_url)}));
        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            cancel.store(true, Ordering::SeqCst);
        });
        let start = std::time::Instant::now();
        let r = profile_request(&ex, &p, &cfg);
        t.join().unwrap();
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        assert!(r.attempted > 0 && r.attempted < 100_000);
        assert_eq!(r.stats.iterations, r.attempted);
    }

    #[test]
    fn scheduling_windows_are_bounded() {
        for c in [0, 1, 3, 10, 256, 1000, u64::MAX / 2] {
            let w = window_size(c);
            assert!(w >= c.max(1) && w.is_multiple_of(c.max(1)), "{c}: {w}");
            assert!(w <= 256 + c.max(1), "{c}: {w}");
        }
    }

    #[test]
    fn huge_iteration_counts_do_not_allocate_up_front() {
        let srv = TestServer::start();
        let cfg = ProfileConfig {
            iterations: u64::MAX,
            warmup: Some(0),
            concurrency: Some(4),
            ..Default::default()
        };
        let ex = executor(&cfg);
        let cancel = ex.cancel_handle();
        let p = pr(json!({"url": format!("{}/json/user", srv.base_url)}));
        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            cancel.store(true, Ordering::SeqCst);
        });
        let r = profile_request(&ex, &p, &cfg);
        t.join().unwrap();
        assert!(r.attempted > 0);
    }

    #[test]
    fn many_iterations_run_in_windows() {
        let srv = TestServer::start();
        let n = 20_000;
        let cfg = ProfileConfig {
            iterations: n,
            warmup: Some(10),
            concurrency: Some(8),
            ..Default::default()
        };
        let p = pr(json!({"url": format!("{}/json/user", srv.base_url)}));
        let r = profile_request(&executor(&cfg), &p, &cfg);
        assert_eq!(r.attempted, n);
        assert_eq!(
            (r.stats.iterations, r.stats.warmup, r.stats.failures),
            (n - 10, 10, 0)
        );
    }
}
