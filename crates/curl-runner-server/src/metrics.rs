//! In-process numbers for the admin health page: request latency, when each
//! background job last ran, webhook oddities and rate-limit hits. They live in
//! memory only, so they start over on every restart (the page says so).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::clock::Clock;

/// Requests older than this no longer count towards p95.
pub const LATENCY_WINDOW: ChronoDuration = ChronoDuration::minutes(5);
/// The ring holds this many requests; past it the oldest are overwritten.
const RING: usize = 8192;
/// Rate-limit hits are reported over this window.
pub const HIT_WINDOW: ChronoDuration = ChronoDuration::hours(1);
/// Webhook oddities are reported over this window.
pub const WEBHOOK_WINDOW: ChronoDuration = ChronoDuration::hours(24);
/// Most timestamps kept per counter.
const MAX_EVENTS: usize = 10_000;
/// A job that has not finished a run for this long is `stale`.
pub const STALE_AFTER: ChronoDuration = ChronoDuration::minutes(5);

/// The background jobs, in the order the health page lists them.
pub const JOBS: [&str; 6] = [
    "mail_worker",
    "billing_sync",
    "override_expiry",
    "broadcast_fanout",
    "mrr_snapshot",
    "audit_retention",
];

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Copy, Default)]
struct Sample {
    at_ms: i64,
    micros: u32,
    status: u16,
}

#[derive(Default)]
struct Ring {
    buf: Vec<Sample>,
    next: usize,
}

/// When something happened, newest last, bounded.
#[derive(Default)]
struct Times(VecDeque<DateTime<Utc>>);

impl Times {
    fn push(&mut self, at: DateTime<Utc>) {
        if self.0.len() >= MAX_EVENTS {
            self.0.pop_front();
        }
        self.0.push_back(at);
    }

    /// Drop what is older than `window`, then count what is left.
    fn count(&mut self, now: DateTime<Utc>, window: ChronoDuration) -> usize {
        while self.0.front().is_some_and(|at| *at <= now - window) {
            self.0.pop_front();
        }
        self.0.len()
    }
}

/// How a job's last run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobRun {
    pub at: DateTime<Utc>,
    pub ok: bool,
}

#[derive(Default)]
pub struct Metrics {
    started: Mutex<Option<DateTime<Utc>>>,
    ring: Mutex<Ring>,
    jobs: Mutex<HashMap<&'static str, JobRun>>,
    bad_signatures: Mutex<Times>,
    duplicates: Mutex<Times>,
    rate_hits: Mutex<HashMap<String, Times>>,
}

impl Metrics {
    pub fn new(started: DateTime<Utc>) -> Self {
        let m = Self::default();
        *lock(&m.started) = Some(started);
        m
    }

    pub fn started(&self) -> Option<DateTime<Utc>> {
        *lock(&self.started)
    }

    /// One finished request.
    pub fn record_request(&self, at: DateTime<Utc>, took: Duration, status: u16) {
        let sample = Sample {
            at_ms: at.timestamp_millis(),
            micros: u32::try_from(took.as_micros()).unwrap_or(u32::MAX),
            status,
        };
        let mut ring = lock(&self.ring);
        if ring.buf.len() < RING {
            ring.buf.push(sample);
        } else {
            let i = ring.next;
            ring.buf[i] = sample;
        }
        ring.next = (ring.next + 1) % RING;
    }

    fn recent(&self, now: DateTime<Utc>) -> Vec<Sample> {
        let from = (now - LATENCY_WINDOW).timestamp_millis();
        lock(&self.ring)
            .buf
            .iter()
            .filter(|s| s.at_ms > from)
            .copied()
            .collect()
    }

    /// The 95th percentile latency over the last 5 minutes, in milliseconds.
    /// `None` when there were no requests.
    pub fn p95_ms(&self, now: DateTime<Utc>) -> Option<u32> {
        let mut micros: Vec<u32> = self.recent(now).iter().map(|s| s.micros).collect();
        if micros.is_empty() {
            return None;
        }
        micros.sort_unstable();
        // Nearest rank: the smallest value with 95% of the samples at or below it.
        let rank = (micros.len() * 95).div_ceil(100).max(1);
        Some(micros[rank - 1].div_ceil(1000))
    }

    /// Requests seen in the last 5 minutes.
    pub fn requests_5m(&self, now: DateTime<Utc>) -> usize {
        self.recent(now).len()
    }

    /// Of those, the ones that answered 5xx.
    pub fn errors_5xx_5m(&self, now: DateTime<Utc>) -> usize {
        self.recent(now).iter().filter(|s| s.status >= 500).count()
    }

    /// A job finished a run, successfully or not.
    pub fn job_ran(&self, name: &'static str, at: DateTime<Utc>, ok: bool) {
        lock(&self.jobs).insert(name, JobRun { at, ok });
    }

    pub fn job(&self, name: &str) -> Option<JobRun> {
        lock(&self.jobs).get(name).copied()
    }

    pub fn bad_signature(&self, at: DateTime<Utc>) {
        lock(&self.bad_signatures).push(at);
    }

    pub fn duplicate_event(&self, at: DateTime<Utc>) {
        lock(&self.duplicates).push(at);
    }

    pub fn bad_signatures_24h(&self, now: DateTime<Utc>) -> usize {
        lock(&self.bad_signatures).count(now, WEBHOOK_WINDOW)
    }

    pub fn duplicates_24h(&self, now: DateTime<Utc>) -> usize {
        lock(&self.duplicates).count(now, WEBHOOK_WINDOW)
    }

    /// A request was refused for `key`. Only the prefix is kept: keys end in
    /// an email address, an IP or an id, and none of those are stored.
    pub fn rate_limited(&self, key: &str, at: DateTime<Utc>) {
        lock(&self.rate_hits)
            .entry(key_prefix(key))
            .or_default()
            .push(at);
    }

    /// Hits in the last hour per key prefix, most first.
    pub fn rate_limit_hits(&self, now: DateTime<Utc>) -> Vec<(String, usize)> {
        let mut hits = lock(&self.rate_hits);
        let mut out: Vec<(String, usize)> = hits
            .iter_mut()
            .map(|(prefix, times)| (prefix.clone(), times.count(now, HIT_WINDOW)))
            .filter(|(_, n)| *n > 0)
            .collect();
        hits.retain(|_, times| !times.0.is_empty());
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }
}

/// `email-start:email:ada@example.com` becomes `email-start:email`. Every key
/// is `what:kind:value`; the value may itself hold colons (an IPv6 address).
pub fn key_prefix(key: &str) -> String {
    let mut parts = key.splitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(a), Some(b), Some(_)) => format!("{a}:{b}"),
        // No value part to hide: keep only the first word, to be safe.
        (Some(a), _, _) => a.to_owned(),
        _ => String::new(),
    }
}

/// Records every response's latency and status in [`Metrics`].
pub async fn track(
    State((metrics, clock)): State<(Arc<Metrics>, Arc<dyn Clock>)>,
    req: Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let res = next.run(req).await;
    metrics.record_request(clock.now(), start.elapsed(), res.status().as_u16());
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn p95_is_the_nearest_rank() {
        let m = Metrics::default();
        let now = at("2026-11-04T10:00:00Z");
        assert_eq!(m.p95_ms(now), None);
        for ms in 1..=100 {
            m.record_request(now, Duration::from_millis(ms), 200);
        }
        assert_eq!(m.p95_ms(now), Some(95));
        assert_eq!(m.requests_5m(now), 100);
    }

    #[test]
    fn old_requests_age_out_of_the_window() {
        let m = Metrics::default();
        let t = at("2026-11-04T10:00:00Z");
        m.record_request(t, Duration::from_millis(900), 500);
        assert_eq!(m.errors_5xx_5m(t + ChronoDuration::minutes(4)), 1);
        assert_eq!(m.p95_ms(t + ChronoDuration::minutes(5)), None);
    }

    #[test]
    fn the_ring_overwrites_the_oldest() {
        let m = Metrics::default();
        let t = at("2026-11-04T10:00:00Z");
        for _ in 0..RING {
            m.record_request(t, Duration::from_millis(500), 200);
        }
        for _ in 0..RING {
            m.record_request(t, Duration::from_millis(5), 200);
        }
        assert_eq!(m.requests_5m(t), RING);
        assert_eq!(m.p95_ms(t), Some(5));
    }

    #[test]
    fn key_prefix_drops_the_value() {
        assert_eq!(
            key_prefix("email-start:email:ada@example.com"),
            "email-start:email"
        );
        assert_eq!(key_prefix("shared:ip:2001:db8::1"), "shared:ip");
        assert_eq!(key_prefix("publish:team:0f3e"), "publish:team");
        assert_eq!(key_prefix("odd:ada@example.com"), "odd");
    }

    #[test]
    fn hits_count_per_prefix_over_an_hour() {
        let m = Metrics::default();
        let t = at("2026-11-04T10:00:00Z");
        m.rate_limited("email-start:ip:1.2.3.4", t);
        m.rate_limited("email-start:ip:5.6.7.8", t + ChronoDuration::minutes(30));
        m.rate_limited("shared:ip:1.2.3.4", t);
        assert_eq!(
            m.rate_limit_hits(t + ChronoDuration::minutes(31)),
            [("email-start:ip".into(), 2), ("shared:ip".into(), 1)]
        );
        assert_eq!(
            m.rate_limit_hits(t + ChronoDuration::minutes(61)),
            [("email-start:ip".into(), 1)]
        );
        assert!(m.rate_limit_hits(t + ChronoDuration::hours(2)).is_empty());
    }
}
