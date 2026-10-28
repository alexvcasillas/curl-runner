//! Background "update available" notice (`utils/version-checker.ts`).
//!
//! Same rules as TS: skipped when `CI` is set, a 24 h cache in
//! `~/.curl-runner-version-cache.json` (same file and format), a 3 s request
//! timeout, every failure silent. Differences (see CHANGES-FROM-TS.md): the
//! check runs on a thread and the notice is printed after the command
//! finishes, waiting at most [`WAIT`]; it only runs when stdout is a terminal.
//!
//! `CURL_RUNNER_TEST_REGISTRY_URL` replaces the registry URL; it exists for
//! tests only.

use crate::colors::{Color, colorize};
use crate::output::Console;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const REGISTRY_URL: &str = "https://registry.npmjs.org/@curl-runner/cli/latest";
pub const REGISTRY_ENV: &str = "CURL_RUNNER_TEST_REGISTRY_URL";
const CACHE_DURATION_MS: u64 = 24 * 60 * 60 * 1000;
const FETCH_TIMEOUT: Duration = Duration::from_secs(3);
/// How long the finished command waits for a still-running check.
const WAIT: Duration = Duration::from_millis(300);

/// Test URL overrides are honored in debug builds and with the `test-hooks`
/// feature; release builds ignore them (an override would also serve the
/// checksums, and allows plain http).
pub const HOOKS_ENABLED: bool = cfg!(any(debug_assertions, feature = "test-hooks"));

/// `env(key)` when hooks are `enabled`, else `None`.
pub fn hook(env: &dyn Fn(&str) -> Option<String>, key: &str, enabled: bool) -> Option<String> {
    if enabled { env(key) } else { None }
}

pub struct Settings {
    pub current: String,
    pub registry_url: String,
    pub cache_file: Option<PathBuf>,
}

impl Settings {
    /// `None` when no check should happen (CI, not a terminal, no home dir).
    pub fn from_env(env: &dyn Fn(&str) -> Option<String>, stdout_is_tty: bool) -> Option<Settings> {
        Self::from_env_with(env, stdout_is_tty, HOOKS_ENABLED)
    }

    pub fn from_env_with(
        env: &dyn Fn(&str) -> Option<String>,
        stdout_is_tty: bool,
        hooks: bool,
    ) -> Option<Settings> {
        if env("CI").is_some_and(|v| !v.is_empty()) || !stdout_is_tty {
            return None;
        }
        let home = env("HOME")
            .or_else(|| env("USERPROFILE"))
            .filter(|h| !h.is_empty());
        Some(Settings {
            current: curl_runner_core::VERSION.to_string(),
            registry_url: hook(env, REGISTRY_ENV, hooks)
                .unwrap_or_else(|| REGISTRY_URL.to_string()),
            cache_file: home.map(|h| PathBuf::from(h).join(".curl-runner-version-cache.json")),
        })
    }
}

/// JS-style `Number` of a version part (`NaN || 0` is 0).
fn part(s: &str) -> f64 {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| !n.is_nan())
        .unwrap_or(0.0)
}

/// TS `isNewerVersion`: dotted numeric comparison, a leading `v` ignored.
pub fn is_newer(current: &str, latest: &str) -> bool {
    let c: Vec<f64> = current
        .trim_start_matches('v')
        .split('.')
        .map(part)
        .collect();
    let l: Vec<f64> = latest
        .trim_start_matches('v')
        .split('.')
        .map(part)
        .collect();
    for i in 0..c.len().max(l.len()) {
        let (a, b) = (
            c.get(i).copied().unwrap_or(0.0),
            l.get(i).copied().unwrap_or(0.0),
        );
        if b > a {
            return true;
        }
        if b < a {
            return false;
        }
    }
    false
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// `(lastCheck, latestVersion)` of the cache file.
fn read_cache_raw(path: &PathBuf) -> Option<(f64, String)> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some((
        v.get("lastCheck")?.as_f64()?,
        v.get("latestVersion")?.as_str()?.to_string(),
    ))
}

fn read_cache(path: &PathBuf, now: u64) -> Option<String> {
    let (last, latest) = read_cache_raw(path)?;
    ((now as f64 - last) < CACHE_DURATION_MS as f64).then_some(latest)
}

fn write_cache(path: &PathBuf, latest: &str, now: u64) {
    let body = serde_json::json!({ "lastCheck": now, "latestVersion": latest });
    let _ = std::fs::write(path, body.to_string());
}

/// Latest version from the registry, `None` on any failure.
pub fn fetch_latest_with(url: &str, timeout: Duration) -> Option<String> {
    let body = crate::net::get(url, timeout, 1 << 20, &[]).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&body).ok()?;
    v.get("version")?.as_str().map(str::to_string)
}

/// The newer version to announce, if any (cache first, then the registry).
pub fn check(s: &Settings, now: u64) -> Option<String> {
    if s.current == "0.0.0" {
        return None;
    }
    let cached = s.cache_file.as_ref().and_then(|p| read_cache(p, now));
    let latest = match cached {
        Some(l) => l,
        None => {
            // Record the attempt first (with the last known version): the
            // process may exit before a slow lookup ends, and a slow or
            // failing registry must cost at most one try per 24 h.
            if let Some(p) = &s.cache_file {
                let prev = read_cache_raw(p).map_or_else(|| s.current.clone(), |(_, v)| v);
                write_cache(p, &prev, now);
            }
            let l = fetch_latest_with(&s.registry_url, FETCH_TIMEOUT)?;
            if let Some(p) = &s.cache_file {
                write_cache(p, &l, now);
            }
            l
        }
    };
    is_newer(&s.current, &latest).then_some(latest)
}

/// The notice, one string per line (TS `compareVersions`).
pub fn banner(current: &str, latest: &str, color: bool) -> Vec<String> {
    let p = |t: &str, c: Color| if color { colorize(t, c) } else { t.to_string() };
    let bar = p("│", Color::Yellow);
    let blank = format!("{bar}{}{bar}", " ".repeat(47));
    vec![
        String::new(),
        p(
            "╭───────────────────────────────────────────────╮",
            Color::Yellow,
        ),
        blank.clone(),
        format!(
            "{bar}  {} {} → {}        {bar}",
            p("Update available!", Color::Bright),
            p(current, Color::Red),
            p(latest, Color::Green)
        ),
        blank.clone(),
        format!(
            "{bar}  Run {} to update          {bar}",
            p("curl-runner upgrade", Color::Cyan)
        ),
        blank,
        p(
            "╰───────────────────────────────────────────────╯",
            Color::Yellow,
        ),
        String::new(),
    ]
}

/// Start a check unless the environment says not to (see [`Settings::from_env`]).
pub fn start_from_env(
    env: &dyn Fn(&str) -> Option<String>,
    stdout_is_tty: bool,
) -> Option<Pending> {
    Settings::from_env(env, stdout_is_tty).map(start)
}

/// A check running in the background.
pub struct Pending {
    rx: mpsc::Receiver<Option<String>>,
    current: String,
}

/// Start the check on a thread.
pub fn start(s: Settings) -> Pending {
    let (tx, rx) = mpsc::channel();
    let current = s.current.clone();
    std::thread::spawn(move || {
        let _ = tx.send(check(&s, now_ms()));
    });
    Pending { rx, current }
}

impl Pending {
    /// Print the notice if the check finished (within [`WAIT`]) with a newer
    /// version; otherwise stay silent.
    pub fn finish(self, color: bool, out: &mut Console) {
        if let Ok(Some(latest)) = self.rx.recv_timeout(WAIT) {
            for line in banner(&self.current, &latest, color) {
                out.out(&line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use testkit::{Response, TestServer};

    fn registry(version: &'static str, status: u16) -> (TestServer, Arc<AtomicUsize>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let srv = TestServer::with_handler(move |_| {
            h.fetch_add(1, Ordering::SeqCst);
            Response::from_data(format!(r#"{{"version":"{version}"}}"#).into_bytes())
                .with_status_code(status)
        });
        (srv, hits)
    }

    fn settings(url: &str, cache: Option<PathBuf>) -> Settings {
        Settings {
            current: "1.0.0".into(),
            registry_url: url.into(),
            cache_file: cache,
        }
    }

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn is_newer_matches_ts() {
        assert!(is_newer("1.0.0", "1.0.1"));
        assert!(is_newer("v1.9.0", "1.10.0"));
        assert!(is_newer("1.0", "1.0.1"));
        assert!(!is_newer("1.0.1", "1.0.1"));
        assert!(!is_newer("2.0.0", "1.99.99"));
        assert!(!is_newer("1.0.0", "1.0.0-beta"));
        assert!(!is_newer("1.0.0", "abc"));
    }

    #[test]
    fn no_request_when_ci_is_set() {
        let (srv, hits) = registry("9.9.9", 200);
        let env = env_of(&[("CI", "true"), ("HOME", "/h")]);
        assert!(Settings::from_env(&env, true).is_none());
        // The gate is the only way to a request: nothing was made.
        let _ = &srv;
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn start_from_env_makes_no_request_with_ci() {
        let (srv, hits) = registry("9.9.9", 200);
        let url: &'static str = Box::leak(srv.base_url.clone().into_boxed_str());
        let home = tempfile::tempdir().unwrap();
        let home: &'static str =
            Box::leak(home.path().to_string_lossy().into_owned().into_boxed_str());
        let ci = move |k: &str| match k {
            "CI" => Some("1".into()),
            "HOME" => Some(home.into()),
            REGISTRY_ENV => Some(url.into()),
            _ => None,
        };
        assert!(start_from_env(&ci, true).is_none());
        assert_eq!(hits.load(Ordering::SeqCst), 0);
        // Same environment without CI: one request, and the notice is printed.
        let no_ci = move |k: &str| if k == "CI" { None } else { ci(k) };
        let (mut c, cap) = Console::capture();
        start_from_env(&no_ci, true).unwrap().finish(false, &mut c);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(
            cap.stdout().contains("Update available!"),
            "{}",
            cap.stdout()
        );
    }

    #[test]
    fn skipped_when_stdout_is_not_a_terminal_or_ci_empty() {
        let env = env_of(&[("HOME", "/h")]);
        assert!(Settings::from_env(&env, false).is_none());
        assert!(Settings::from_env(&env, true).is_some());
        let env = env_of(&[("CI", ""), ("HOME", "/h")]);
        assert!(Settings::from_env(&env, true).is_some());
    }

    #[test]
    fn registry_url_override_and_cache_path() {
        let env = env_of(&[("HOME", "/h"), (REGISTRY_ENV, "http://x/y")]);
        let s = Settings::from_env(&env, true).unwrap();
        assert_eq!(s.registry_url, "http://x/y");
        assert_eq!(
            s.cache_file.unwrap(),
            PathBuf::from("/h/.curl-runner-version-cache.json")
        );
    }

    #[test]
    fn fetches_announces_and_caches() {
        let (srv, hits) = registry("1.2.0", 200);
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("c.json");
        let s = settings(&srv.base_url, Some(cache.clone()));
        assert_eq!(check(&s, 1_000), Some("1.2.0".into()));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        let c: serde_json::Value = serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
        assert_eq!(c["latestVersion"], "1.2.0");
        assert_eq!(c["lastCheck"], 1_000);
        // Within 24 h: served from the cache, no request.
        assert_eq!(
            check(&s, 1_000 + CACHE_DURATION_MS - 1),
            Some("1.2.0".into())
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        // Stale: asks again.
        assert_eq!(check(&s, 1_000 + CACHE_DURATION_MS), Some("1.2.0".into()));
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn hooks_are_ignored_when_disabled() {
        let env = env_of(&[("HOME", "/h"), (REGISTRY_ENV, "http://evil/x")]);
        let on = Settings::from_env_with(&env, true, true).unwrap();
        assert_eq!(on.registry_url, "http://evil/x");
        let off = Settings::from_env_with(&env, true, false).unwrap();
        assert_eq!(off.registry_url, REGISTRY_URL);
        assert_eq!(hook(&env, REGISTRY_ENV, false), None);
        assert_eq!(
            hook(&env, REGISTRY_ENV, true).as_deref(),
            Some("http://evil/x")
        );
        assert_eq!(
            HOOKS_ENABLED,
            cfg!(any(debug_assertions, feature = "test-hooks"))
        );
    }

    #[test]
    fn slow_registry_is_tried_once_per_day() {
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let srv = TestServer::with_handler(move |_| {
            h.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(1000));
            Response::from_data(br#"{"version":"9.9.9"}"#.to_vec())
        });
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("c.json");
        let s = |p: &PathBuf| settings(&srv.base_url, Some(p.clone()));
        // First run: the lookup outlives the wait (the process would exit).
        let (mut c, cap) = Console::capture();
        start(s(&cache)).finish(false, &mut c);
        assert_eq!(cap.stdout(), "");
        // The attempt is on disk already, in the TS cache shape.
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
        assert_eq!(v["latestVersion"], "1.0.0");
        assert!(v["lastCheck"].as_u64().unwrap() > 0);
        // Second run: no new request, no wait.
        let t = std::time::Instant::now();
        start(s(&cache)).finish(false, &mut c);
        assert!(t.elapsed() < Duration::from_millis(250));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_lookup_keeps_previous_latest_and_is_not_retried() {
        let (srv, hits) = registry("1.0.0", 500);
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("c.json");
        std::fs::write(&cache, r#"{"lastCheck":0,"latestVersion":"1.4.0"}"#).unwrap();
        let s = settings(&srv.base_url, Some(cache));
        assert_eq!(check(&s, CACHE_DURATION_MS + 5), None);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        // Within a day: no request, the previously known newer version shows.
        assert_eq!(check(&s, CACHE_DURATION_MS + 10), Some("1.4.0".into()));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn up_to_date_is_silent() {
        let (srv, _) = registry("1.0.0", 200);
        assert_eq!(check(&settings(&srv.base_url, None), 5), None);
    }

    #[test]
    fn failures_are_silent() {
        let (srv, _) = registry("9.0.0", 500);
        assert_eq!(check(&settings(&srv.base_url, None), 5), None);
        assert_eq!(check(&settings("http://127.0.0.1:1/x", None), 5), None);
        let junk = TestServer::with_handler(|_| Response::from_data(b"nope".to_vec()));
        assert_eq!(check(&settings(&junk.base_url, None), 5), None);
    }

    #[test]
    fn dev_version_never_checks() {
        let (srv, hits) = registry("9.0.0", 200);
        let mut s = settings(&srv.base_url, None);
        s.current = "0.0.0".into();
        assert_eq!(check(&s, 5), None);
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn corrupt_cache_is_ignored() {
        let (srv, hits) = registry("1.5.0", 200);
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("c.json");
        std::fs::write(&cache, "{not json").unwrap();
        assert_eq!(
            check(&settings(&srv.base_url, Some(cache)), 5),
            Some("1.5.0".into())
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn banner_text() {
        let b = banner("1.0.0", "1.2.0", false);
        assert_eq!(b.len(), 9);
        assert_eq!(b[3], "│  Update available! 1.0.0 → 1.2.0        │");
        assert_eq!(b[5], "│  Run curl-runner upgrade to update          │");
        assert!(banner("1.0.0", "1.2.0", true)[3].contains("\x1b[31m1.0.0"));
    }
}
