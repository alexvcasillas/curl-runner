//! [`PendingSignIns`]: the sign-ins this studio started, so that only
//! their callbacks are redeemed. Every backend checks callbacks here first.

use super::{AuthCallback, BackendError, FOREIGN_CALLBACK};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a started sign-in waits for its callback.
pub const SIGN_IN_WINDOW: Duration = Duration::from_secs(10 * 60);

/// What "now" is (tests move it).
pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

pub struct PendingSignIns {
    clock: Clock,
    /// Deadline by server URL, without a trailing `/`.
    deadlines: Mutex<HashMap<String, Instant>>,
}

impl Default for PendingSignIns {
    fn default() -> Self {
        PendingSignIns::with_clock(Arc::new(Instant::now))
    }
}

impl PendingSignIns {
    pub fn with_clock(clock: Clock) -> PendingSignIns {
        PendingSignIns {
            clock,
            deadlines: Mutex::default(),
        }
    }

    /// A sign-in with `server` started now: its callback is good for
    /// [`SIGN_IN_WINDOW`]. Starting again restarts the window.
    pub fn start(&self, server: &str) {
        let now = (self.clock)();
        let mut d = self.deadlines.lock().unwrap_or_else(|e| e.into_inner());
        d.retain(|_, deadline| *deadline >= now);
        d.insert(normalize(server), now + SIGN_IN_WINDOW);
    }

    /// Takes the pending sign-in `callback` answers. Refused with
    /// `Invalid(`[`FOREIGN_CALLBACK`]`)` when the callback's server is not
    /// https (plain http only on loopback), or no sign-in with it started
    /// in the last [`SIGN_IN_WINDOW`]. A refusal leaves the others pending.
    pub fn redeem(&self, callback: &AuthCallback) -> Result<(), BackendError> {
        let refused = || BackendError::Invalid(FOREIGN_CALLBACK.into());
        curl_runner_cloud_client::check_server(&callback.server).map_err(|_| refused())?;
        let now = (self.clock)();
        let mut d = self.deadlines.lock().unwrap_or_else(|e| e.into_inner());
        let server = normalize(&callback.server);
        match d.get(&server) {
            Some(deadline) if *deadline >= now => {
                d.remove(&server);
                Ok(())
            }
            _ => Err(refused()),
        }
    }
}

fn normalize(server: &str) -> String {
    server.trim().trim_end_matches('/').to_string()
}
