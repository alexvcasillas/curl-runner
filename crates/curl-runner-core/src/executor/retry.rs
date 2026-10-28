//! Retry policy (`core/execution/retry.ts` + the `withRetry` options built in
//! `request-executor.ts`). Shared by the sequential and parallel runners.

use crate::model::{HeaderValue, Headers, RequestConfig};
use crate::transport::{Response, TransportError};
use std::time::{Duration, Instant};

/// Default HTTP statuses that trigger a retry.
pub const DEFAULT_RETRYABLE_STATUSES: [u16; 5] = [429, 500, 502, 503, 504];

/// Error TS `withRetry` reports when every attempt returned a retriable
/// status (kept verbatim for parity).
pub const RETRIABLE_EXHAUSTED: &str = "Operation returned retriable result";

/// Longest retry wait (24 h): larger configured delays, backoff growth and
/// `Retry-After` values are clamped to it.
pub const MAX_DELAY_MS: f64 = 86_400_000.0;

/// Clamp a computed wait to `0..=MAX_DELAY_MS` (`inf` -> max, NaN -> 0).
pub fn clamp_delay(ms: f64) -> f64 {
    if ms.is_nan() {
        0.0
    } else {
        ms.clamp(0.0, MAX_DELAY_MS)
    }
}

/// Retry wait as a `Duration`, clamped (never panics).
pub fn delay_duration(ms: f64) -> Duration {
    Duration::try_from_secs_f64(clamp_delay(ms) / 1000.0).unwrap_or_default()
}

/// `now + d`, saturating at the latest representable instant reachable.
pub fn deadline(now: Instant, d: Duration) -> Instant {
    now.checked_add(d)
        .or_else(|| now.checked_add(delay_duration(MAX_DELAY_MS)))
        .unwrap_or(now)
}

/// One attempt's outcome.
pub type Attempt = Result<Response, TransportError>;

/// `baseDelay * backoff^(attempt - 1)`; `attempt` is 1-based.
pub fn backoff_delay(attempt: u32, base_delay: f64, backoff: f64) -> f64 {
    base_delay * backoff.powi(attempt.saturating_sub(1).min(i32::MAX as u32) as i32)
}

/// `Retry-After` in milliseconds when it holds a number of seconds (JS
/// `Number(value)` semantics: blank is 0, a date or junk is ignored).
pub fn retry_after_ms(headers: &Headers) -> Option<f64> {
    let raw = match headers.get("retry-after")? {
        HeaderValue::One(s) => s.as_str(),
        HeaderValue::Many(v) => v.first()?.as_str(),
    };
    if raw.is_empty() {
        return None;
    }
    let t = raw.trim();
    let secs = if t.is_empty() {
        0.0
    } else {
        t.parse::<f64>().ok().filter(|f| f.is_finite())?
    };
    Some(secs * 1000.0)
}

/// Retry settings of one request.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// `count + 1`.
    pub max_attempts: u32,
    pub delay: f64,
    pub backoff: f64,
    pub retryable_statuses: Vec<u16>,
}

impl RetryPolicy {
    pub fn for_request(req: &RequestConfig) -> Self {
        let r = req.retry.clone().unwrap_or_default();
        let count = r.count.unwrap_or(0).min(u32::MAX as u64 - 1) as u32;
        RetryPolicy {
            max_attempts: count + 1,
            delay: r.delay.unwrap_or(0) as f64,
            backoff: r.backoff.unwrap_or(1.0),
            retryable_statuses: r
                .retryable_statuses
                .unwrap_or_else(|| DEFAULT_RETRYABLE_STATUSES.to_vec()),
        }
    }

    /// Retries configured (`retry.count > 0`).
    pub fn has_retry(&self) -> bool {
        self.max_attempts > 1
    }

    /// A completed response whose status counts as retriable (only with
    /// retries configured, as in TS).
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.has_retry() && self.retryable_statuses.contains(&status)
    }

    /// TS `shouldRetry`: transport failures always, statuses only with retries.
    pub fn should_retry(&self, attempt: &Attempt) -> bool {
        match attempt {
            Err(_) => true,
            Ok(r) => self.is_retryable_status(r.status),
        }
    }

    /// Whether another attempt follows after `done` attempts ending in `last`.
    pub fn wants_another(&self, done: u32, last: &Attempt) -> bool {
        done < self.max_attempts && self.should_retry(last)
    }

    /// Wait before retry `attempt` (1-based) in ms: `Retry-After` on a 429,
    /// else `delay * backoff^(attempt-1)` (0 without a delay). Clamped to
    /// `0..=MAX_DELAY_MS`.
    pub fn delay_ms(&self, attempt: u32, last: &Attempt) -> f64 {
        if let Ok(r) = last
            && r.status == 429
            && let Some(ms) = retry_after_ms(&r.headers)
        {
            return clamp_delay(ms);
        }
        if self.delay > 0.0 {
            clamp_delay(backoff_delay(attempt, self.delay, self.backoff))
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Json, Metrics, RetryConfig};
    use std::time::{Duration, Instant};

    #[test]
    fn backoff_first_attempt_is_base() {
        assert_eq!(backoff_delay(1, 1000.0, 2.0), 1000.0);
    }
    #[test]
    fn backoff_grows() {
        assert_eq!(backoff_delay(2, 1000.0, 2.0), 2000.0);
        assert_eq!(backoff_delay(3, 1000.0, 2.0), 4000.0);
        assert_eq!(backoff_delay(4, 1000.0, 2.0), 8000.0);
    }
    #[test]
    fn backoff_multiplier_one_is_flat() {
        for a in 1..4 {
            assert_eq!(backoff_delay(a, 1000.0, 1.0), 1000.0);
        }
    }
    #[test]
    fn backoff_fractional() {
        assert_eq!(backoff_delay(2, 1000.0, 1.5), 1500.0);
        assert_eq!(backoff_delay(3, 1000.0, 1.5), 2250.0);
    }
    #[test]
    fn backoff_zero_delay() {
        assert_eq!(backoff_delay(1, 0.0, 2.0), 0.0);
        assert_eq!(backoff_delay(5, 0.0, 2.0), 0.0);
    }

    fn resp(status: u16, retry_after: Option<&str>) -> Attempt {
        let mut headers = Headers::new();
        if let Some(v) = retry_after {
            headers.insert("retry-after".into(), HeaderValue::One(v.into()));
        }
        Ok(Response {
            status,
            headers,
            header_history: vec![],
            body: Json::Null,
            metrics: Metrics::default(),
        })
    }
    fn err() -> Attempt {
        Err(TransportError {
            code: Some(7),
            message: "x".into(),
        })
    }
    fn policy(count: Option<u64>, delay: Option<u64>, statuses: Option<Vec<u16>>) -> RetryPolicy {
        RetryPolicy::for_request(&RequestConfig {
            retry: Some(RetryConfig {
                count,
                delay,
                backoff: None,
                retryable_statuses: statuses,
            }),
            ..Default::default()
        })
    }

    #[test]
    fn no_config_means_one_attempt() {
        let p = RetryPolicy::for_request(&RequestConfig::default());
        assert_eq!(p.max_attempts, 1);
        assert!(!p.wants_another(1, &err()));
    }
    #[test]
    fn statuses_retry_only_with_count() {
        assert!(!policy(None, None, None).should_retry(&resp(503, None)));
        assert!(policy(Some(2), None, None).should_retry(&resp(503, None)));
        assert!(!policy(Some(2), None, None).should_retry(&resp(404, None)));
        assert!(policy(Some(2), None, Some(vec![404])).should_retry(&resp(404, None)));
        assert!(!policy(Some(2), None, Some(vec![404])).should_retry(&resp(503, None)));
    }
    #[test]
    fn transport_errors_always_retry_while_attempts_remain() {
        let p = policy(Some(2), None, None);
        assert!(p.wants_another(1, &err()));
        assert!(p.wants_another(2, &err()));
        assert!(!p.wants_another(3, &err()));
    }
    #[test]
    fn delay_uses_backoff_or_zero() {
        let p = RetryPolicy {
            backoff: 2.0,
            ..policy(Some(3), Some(50), None)
        };
        assert_eq!(p.delay_ms(1, &err()), 50.0);
        assert_eq!(p.delay_ms(2, &err()), 100.0);
        assert_eq!(policy(Some(3), None, None).delay_ms(1, &err()), 0.0);
    }
    #[test]
    fn retry_after_on_429_overrides_delay() {
        let p = policy(Some(3), Some(1000), None);
        assert_eq!(p.delay_ms(1, &resp(429, Some("2"))), 2000.0);
        assert_eq!(p.delay_ms(1, &resp(429, Some(" 1.5 "))), 1500.0);
        assert_eq!(p.delay_ms(1, &resp(429, Some("  "))), 0.0);
        // junk / dates / empty fall back to the configured delay
        assert_eq!(p.delay_ms(1, &resp(429, Some("soon"))), 1000.0);
        assert_eq!(p.delay_ms(1, &resp(429, Some(""))), 1000.0);
        // only on 429
        assert_eq!(p.delay_ms(1, &resp(503, Some("2"))), 1000.0);
        // negative -> no wait
        assert_eq!(p.delay_ms(1, &resp(429, Some("-1"))), 0.0);
    }

    #[test]
    fn huge_retry_after_and_backoff_are_clamped() {
        let p = policy(Some(3), Some(1000), None);
        assert_eq!(p.delay_ms(1, &resp(429, Some("1e300"))), MAX_DELAY_MS);
        let big = RetryPolicy {
            backoff: 1e300,
            ..policy(Some(3), Some(u64::MAX), None)
        };
        assert_eq!(big.delay_ms(2, &err()), MAX_DELAY_MS);
        assert_eq!(big.delay_ms(u32::MAX, &err()), MAX_DELAY_MS);
        let nan = RetryPolicy {
            backoff: f64::NAN,
            ..policy(Some(3), Some(10), None)
        };
        assert_eq!(nan.delay_ms(2, &err()), 0.0);
    }

    #[test]
    fn delay_duration_and_deadline_never_panic() {
        for ms in [f64::INFINITY, f64::NAN, -5.0, 1e300, MAX_DELAY_MS] {
            let d = delay_duration(ms);
            assert!(d <= Duration::from_secs_f64(MAX_DELAY_MS / 1000.0));
            let _ = deadline(Instant::now(), d);
        }
        assert_eq!(delay_duration(1500.0), Duration::from_millis(1500));
    }
}
