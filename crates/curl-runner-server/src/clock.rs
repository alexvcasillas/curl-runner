//! Injectable time, so expiry logic is testable without sleeping.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// The real wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A clock that only moves when told to. For tests.
#[derive(Debug)]
pub struct FixedClock(Mutex<DateTime<Utc>>);

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self(Mutex::new(now))
    }

    /// Move the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        let mut now = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *now += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_only_moves_on_advance() {
        let start: DateTime<Utc> = "2026-11-04T10:00:00Z".parse().unwrap();
        let clock = FixedClock::new(start);
        assert_eq!(clock.now(), start);
        assert_eq!(clock.now(), start);
        clock.advance(Duration::minutes(10));
        assert_eq!(clock.now(), start + Duration::minutes(10));
    }

    #[test]
    fn system_clock_is_close_to_now() {
        let diff = Utc::now() - SystemClock.now();
        assert!(diff.num_seconds().abs() < 5);
    }
}
