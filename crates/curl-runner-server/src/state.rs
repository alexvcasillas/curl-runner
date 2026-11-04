//! Shared application state and the pieces later tasks fill in.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::ErrorCode;
use sqlx::PgPool;

use crate::auth::providers::{OAuthProviders, RealProviders};
use crate::clock::Clock;
use crate::config::Config;
use crate::error::ApiError;
use crate::mail::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub clock: Arc<dyn Clock>,
    pub mailer: Mailer,
    pub oauth: Arc<dyn OAuthProviders>,
    pub limiter: RateLimiter,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, clock: Arc<dyn Clock>) -> Self {
        Self {
            mailer: Mailer::new(clock.clone()),
            oauth: Arc::new(RealProviders::new(
                config.github.clone(),
                config.google.clone(),
            )),
            limiter: RateLimiter::new(clock.clone()),
            db,
            config: Arc::new(config),
            clock,
        }
    }
}

/// In-memory fixed-window rate limiter. Single-instance by design: the server
/// runs as one replica, so there is no shared store.
#[derive(Clone)]
pub struct RateLimiter {
    clock: Arc<dyn Clock>,
    windows: Arc<Mutex<HashMap<String, Window>>>,
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
        }
    }

    /// Count one hit for `key`. Fails with `429 rate_limited` once `key` has
    /// had `limit` hits in the current `window`.
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
            return Err(ApiError::new(
                ErrorCode::RateLimited,
                "Too many requests. Try again in a few minutes.",
            ));
        }
        entry.count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;

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
