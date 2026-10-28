//! End-to-end `curl-runner --watch` runs against `testkit::TestServer`.
//!
//! Filesystem event latency differs per platform (FSEvents, inotify), so
//! every wait polls the child's output with a generous timeout.
//! The live watch tests send signals, so they and their helpers are Unix-only.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use testkit::TestServer;

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");
const TIMEOUT: Duration = Duration::from_secs(30);

struct Watch {
    child: Child,
    out: Arc<Mutex<String>>,
    err: Arc<Mutex<String>>,
}

fn pump(mut r: impl Read + Send + 'static, into: Arc<Mutex<String>>) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            into.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&buf[..n]));
        }
    });
}

impl Watch {
    fn spawn(dir: &std::path::Path, args: &[&str]) -> Watch {
        let mut child = Command::new(BIN)
            .env_clear()
            .env("NO_COLOR", "1")
            .current_dir(dir)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (out, err) = (
            Arc::new(Mutex::new(String::new())),
            Arc::new(Mutex::new(String::new())),
        );
        pump(child.stdout.take().unwrap(), out.clone());
        pump(child.stderr.take().unwrap(), err.clone());
        Watch { child, out, err }
    }

    fn wait_for(&self, stderr: bool, needle: &str) {
        let buf = if stderr { &self.err } else { &self.out };
        let t0 = Instant::now();
        while !buf.lock().unwrap().contains(needle) {
            assert!(
                t0.elapsed() < TIMEOUT,
                "timed out waiting for {needle:?}\nstdout:\n{}\nstderr:\n{}",
                self.out.lock().unwrap(),
                self.err.lock().unwrap()
            );
            thread::sleep(Duration::from_millis(25));
        }
    }

    /// Rewrite `file` until `needle` shows up (a write can be missed while
    /// the previous run is still going, or arrive before the watcher is up).
    fn write_until(&self, file: &std::path::Path, content: &str, needle: &str) {
        let t0 = Instant::now();
        loop {
            std::fs::write(file, content).unwrap();
            let step = Instant::now();
            while step.elapsed() < Duration::from_secs(3) {
                if self.out.lock().unwrap().contains(needle) {
                    return;
                }
                thread::sleep(Duration::from_millis(25));
            }
            assert!(
                t0.elapsed() < TIMEOUT,
                "timed out waiting for {needle:?}\nstdout:\n{}\nstderr:\n{}",
                self.out.lock().unwrap(),
                self.err.lock().unwrap()
            );
        }
    }

    #[cfg(unix)]
    fn sigint_and_wait(&mut self) -> Option<i32> {
        self.signal_and_wait("-INT")
    }

    #[cfg(unix)]
    fn signal_and_wait(&mut self, sig: &str) -> Option<i32> {
        let _ = Command::new("kill")
            .args([sig, &self.child.id().to_string()])
            .status();
        let t0 = Instant::now();
        loop {
            if let Some(st) = self.child.try_wait().unwrap() {
                return st.code();
            }
            if t0.elapsed() > TIMEOUT {
                let _ = self.child.kill();
                panic!("did not exit after SIGINT");
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn yaml(server: &TestServer, name: &str, status: u16) -> String {
    format!(
        "requests:\n  - name: {name}\n    url: \"{}/json/user\"\n    expect: {{ status: {status} }}\n",
        server.base_url
    )
}

#[cfg(unix)]
#[test]
fn rerun_on_change_reloads_the_file_and_sigint_exits_0() {
    let server = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("api.yaml");
    std::fs::write(&file, yaml(&server, "FirstVersion", 200)).unwrap();

    let mut w = Watch::spawn(
        dir.path(),
        &["api.yaml", "--watch", "--watch-debounce", "50"],
    );
    w.wait_for(false, "FirstVersion");
    w.wait_for(false, "Files: api.yaml");
    assert!(w.out.lock().unwrap().contains("Watching for changes..."));

    w.write_until(&file, &yaml(&server, "SecondVersion", 200), "SecondVersion");
    w.wait_for(false, "File changed: api.yaml");

    let out = w.out.lock().unwrap().clone();
    assert!(!out.contains("Interrupted"), "{out}");
    assert_eq!(w.sigint_and_wait(), Some(0));
    assert!(
        w.err.lock().unwrap().is_empty(),
        "{}",
        w.err.lock().unwrap()
    );
}

#[cfg(unix)]
#[test]
fn sigterm_in_watch_mode_exits_0() {
    let server = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("api.yaml"), yaml(&server, "Term", 200)).unwrap();
    let mut w = Watch::spawn(dir.path(), &["api.yaml", "--watch"]);
    w.wait_for(false, "Files: api.yaml");
    assert_eq!(w.signal_and_wait("-TERM"), Some(0));
}

#[cfg(unix)]
#[test]
fn failing_runs_and_bad_reloads_do_not_stop_watching() {
    let server = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("api.yaml");
    // The first run fails its expectation.
    std::fs::write(&file, yaml(&server, "Failing", 201)).unwrap();

    let mut w = Watch::spawn(dir.path(), &["api.yaml", "-w", "--watch-debounce", "50"]);
    w.wait_for(false, "Failing");
    w.wait_for(false, "Files: api.yaml");

    // A file that no longer parses is reported, not fatal.
    std::fs::write(&file, "requests: [ {").unwrap();
    w.wait_for(true, "api.yaml");
    assert!(w.child.try_wait().unwrap().is_none(), "watch exited");

    w.write_until(&file, &yaml(&server, "Recovered", 200), "Recovered");
    assert_eq!(w.sigint_and_wait(), Some(0));
}

#[test]
fn watch_without_files_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(BIN)
        .env_clear()
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .args(["missing.yaml", "--watch"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("No YAML files found"));
}
