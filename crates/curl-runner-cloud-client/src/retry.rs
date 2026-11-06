//! A client that retries transport errors, 5xx and 429, with backoff.

use crate::client::{Client, Reply};
use crate::escape_control;
use curl_runner_cloud_proto::ErrorBody;
use std::time::Duration;

const ATTEMPTS: usize = 5;
const BASE_DELAYS: [u64; ATTEMPTS - 1] = [1, 2, 4, 8];
const MAX_RETRY_AFTER: u64 = 60;

/// Seconds to wait before retry number `retry` (0-based). `unit` in [0, 1)
/// places the wait inside the +-25 % jitter band. A `Retry-After` is
/// honored as sent, up to a minute.
fn delay(retry: usize, retry_after: Option<u64>, unit: f64) -> Duration {
    if let Some(s) = retry_after {
        return Duration::from_secs(s.min(MAX_RETRY_AFTER));
    }
    let base = BASE_DELAYS[retry.min(BASE_DELAYS.len() - 1)] as f64;
    Duration::from_secs_f64(base * (0.75 + 0.5 * unit))
}

/// A random number in [0, 1). `RandomState` is seeded by the OS, which
/// avoids a clock whose low digits are always zero on some platforms.
fn jitter_unit() -> f64 {
    use std::hash::{BuildHasher, Hasher};
    let n = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    (n >> 11) as f64 / (1u64 << 53) as f64
}

pub struct Api<'a> {
    client: Client,
    server: &'a str,
    sleep: bool,
}

impl<'a> Api<'a> {
    /// `sleep: false` skips the waits between attempts (tests).
    pub fn new(server: &'a str, token: Option<String>, sleep: bool) -> Api<'a> {
        Api {
            client: Client::new(server, token),
            server,
            sleep,
        }
    }

    /// One logical request. Any non-retryable HTTP status comes back as
    /// `Ok`; running out of attempts is `Err` with a printable message.
    /// `notice` hears about each retry.
    pub fn request(
        &self,
        notice: &mut dyn FnMut(&str),
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        gzip: bool,
    ) -> Result<(u16, Vec<u8>), String> {
        for attempt in 0..ATTEMPTS {
            let last = attempt + 1 == ATTEMPTS;
            let (why, retry_after) = match self.client.send_full(method, path, body, gzip) {
                Ok(Reply {
                    status,
                    body,
                    retry_after,
                }) => {
                    if status != 429 && status < 500 {
                        return Ok((status, body));
                    }
                    if last {
                        return Err(match serde_json::from_slice::<ErrorBody>(&body) {
                            Ok(b) => escape_control(&b.message),
                            Err(_) => format!(
                                "Cannot reach {}: unexpected reply (HTTP {status}).",
                                escape_control(self.server)
                            ),
                        });
                    }
                    (format!("HTTP {status}"), retry_after)
                }
                Err(e) => {
                    if last {
                        return Err(format!(
                            "Cannot reach {}: {e}.",
                            escape_control(self.server)
                        ));
                    }
                    (e.to_string(), None)
                }
            };
            let wait = delay(attempt, retry_after, jitter_unit());
            notice(&format!(
                "{why}. Retrying in {:.0}s (attempt {} of {ATTEMPTS}).",
                wait.as_secs_f64(),
                attempt + 2
            ));
            if self.sleep {
                std::thread::sleep(wait);
            }
        }
        unreachable!("the last attempt returns")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_1_2_4_8_with_25_percent_jitter() {
        for (i, base) in [1.0, 2.0, 4.0, 8.0].into_iter().enumerate() {
            assert_eq!(delay(i, None, 0.5).as_secs_f64(), base);
            assert!((delay(i, None, 0.0).as_secs_f64() - base * 0.75).abs() < 1e-9);
            assert!((delay(i, None, 0.999).as_secs_f64() - base * 1.2495).abs() < 1e-6);
        }
    }

    #[test]
    fn jitter_stays_within_band_and_varies() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            let u = jitter_unit();
            assert!((0.0..1.0).contains(&u));
            let d = delay(2, None, u).as_secs_f64();
            assert!((3.0..=5.0).contains(&d), "{d}");
            seen.insert(d.to_bits());
        }
        assert!(seen.len() > 1);
    }

    #[test]
    fn retry_after_is_honored_and_capped() {
        assert_eq!(delay(0, Some(7), 0.9), Duration::from_secs(7));
        assert_eq!(delay(0, Some(3600), 0.9), Duration::from_secs(60));
    }
}
