//! Shared application state and the pieces later tasks fill in.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

use crate::auth::providers::{OAuthProviders, RealProviders};
use crate::billing::stripe::StripeBilling;
use crate::billing::{Billing, PriceCache};
use crate::clock::Clock;
use crate::config::Config;
use crate::error::ApiError;
use crate::mail::Mailer;
use crate::metrics::Metrics;
use crate::settings::{Settings, SettingsCache};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub clock: Arc<dyn Clock>,
    pub mailer: Mailer,
    pub oauth: Arc<dyn OAuthProviders>,
    pub limiter: RateLimiter,
    /// `None` when `STRIPE_*` is not configured.
    pub billing: Option<Arc<dyn Billing>>,
    pub price_cache: PriceCache,
    /// The `server_settings` row behind a 5 second cache.
    pub settings_cache: SettingsCache,
    /// In-process numbers for the admin health page. Reset on restart.
    pub metrics: Arc<Metrics>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, clock: Arc<dyn Clock>) -> Self {
        let metrics = Arc::new(Metrics::new(clock.now()));
        Self {
            mailer: Mailer::new(clock.clone()),
            oauth: Arc::new(RealProviders::new(
                config.github.clone(),
                config.google.clone(),
            )),
            limiter: RateLimiter::new(clock.clone()).with_metrics(metrics.clone()),
            metrics,
            billing: config
                .billing
                .clone()
                .map(|b| Arc::new(StripeBilling::new(b.secret_key)) as Arc<dyn Billing>),
            price_cache: PriceCache::default(),
            settings_cache: SettingsCache::new(clock.clone()),
            db,
            config: Arc::new(config),
            clock,
        }
    }

    /// The server settings: the one place to read signup, plans, grace days,
    /// the banner and the publishing pause.
    pub async fn settings(&self) -> Result<Arc<Settings>, ApiError> {
        self.settings_cache.get(&self.db, &self.config).await
    }

    /// Settings for discovery: the last ones read, or the env's, when the
    /// database is down. See [`crate::settings::SettingsCache::get_or_last`].
    pub async fn settings_or_last(&self) -> Arc<Settings> {
        self.settings_cache
            .get_or_last(&self.db, &self.config)
            .await
    }
}

/// In-memory fixed-window rate limiter. Single-instance by design: the server
/// runs as one replica, so there is no shared store.
#[derive(Clone)]
pub struct RateLimiter {
    clock: Arc<dyn Clock>,
    windows: Arc<Mutex<HashMap<String, Window>>>,
    /// Counts refusals per key prefix, when set.
    metrics: Option<Arc<Metrics>>,
}

struct Window {
    ends_at: DateTime<Utc>,
    count: u32,
}

/// Above this many keys, expired windows are dropped on the next check.
const PRUNE_ABOVE: usize = 10_000;

impl RateLimiter {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            windows: Arc::default(),
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Count one hit for `key`. Fails with `429 rate_limited` once `key` has
    /// had `limit` hits in the current `window`, with `Retry-After` set to
    /// the seconds left in it (at least 1).
    pub fn check(&self, key: &str, limit: u32, window: Duration) -> Result<(), ApiError> {
        let now = self.clock.now();
        let mut windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if windows.len() > PRUNE_ABOVE {
            windows.retain(|_, w| w.ends_at > now);
        }
        let entry = match windows.get_mut(key) {
            Some(w) if w.ends_at > now => w,
            _ => {
                let fresh = Window {
                    ends_at: now + window,
                    count: 0,
                };
                windows.insert(key.to_owned(), fresh);
                windows.get_mut(key).expect("just inserted")
            }
        };
        if entry.count >= limit {
            if let Some(metrics) = &self.metrics {
                metrics.rate_limited(key, now);
            }
            let left_ms = (entry.ends_at - now)
                .num_milliseconds()
                .max(1)
                .unsigned_abs();
            return Err(ApiError::rate_limited(left_ms.div_ceil(1000)));
        }
        entry.count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use curl_runner_cloud_proto::ErrorCode;

    fn limiter() -> (Arc<FixedClock>, RateLimiter) {
        let clock = Arc::new(FixedClock::new("2026-11-04T10:00:00Z".parse().unwrap()));
        (clock.clone(), RateLimiter::new(clock))
    }

    #[test]
    fn allows_up_to_limit_then_429() {
        let (_, rl) = limiter();
        let w = Duration::minutes(15);
        for _ in 0..5 {
            rl.check("a", 5, w).unwrap();
        }
        let err = rl.check("a", 5, w).unwrap_err();
        assert_eq!(err.code, ErrorCode::RateLimited);
        assert_eq!(err.status().as_u16(), 429);
        assert_eq!(err.retry_after, Some(15 * 60));
    }

    #[test]
    fn retry_after_counts_down_to_the_window_end() {
        let (clock, rl) = limiter();
        let w = Duration::minutes(15);
        rl.check("a", 1, w).unwrap();
        clock.advance(Duration::minutes(10));
        assert_eq!(rl.check("a", 1, w).unwrap_err().retry_after, Some(5 * 60));
        clock.advance(Duration::seconds(30) + Duration::milliseconds(200));
        assert_eq!(rl.check("a", 1, w).unwrap_err().retry_after, Some(270));
    }

    #[test]
    fn keys_are_independent() {
        let (_, rl) = limiter();
        let w = Duration::minutes(15);
        rl.check("a", 1, w).unwrap();
        assert!(rl.check("a", 1, w).is_err());
        rl.check("b", 1, w).unwrap();
    }

    #[test]
    fn window_resets_after_it_ends() {
        let (clock, rl) = limiter();
        let w = Duration::minutes(15);
        rl.check("a", 1, w).unwrap();
        clock.advance(Duration::minutes(14));
        assert!(rl.check("a", 1, w).is_err());
        clock.advance(Duration::minutes(1));
        rl.check("a", 1, w).unwrap();
        assert!(rl.check("a", 1, w).is_err());
    }

    #[test]
    fn expired_windows_are_pruned() {
        let (clock, rl) = limiter();
        let w = Duration::minutes(1);
        for i in 0..=PRUNE_ABOVE {
            rl.check(&i.to_string(), 1, w).unwrap();
        }
        clock.advance(Duration::minutes(2));
        rl.check("new", 1, w).unwrap();
        assert_eq!(rl.windows.lock().unwrap().len(), 1);
    }
}
