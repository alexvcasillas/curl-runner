//! Watch mode (TS `FileWatcher`, watcher/file-watcher.ts).
//!
//! Runs once, then re-runs whenever one of the watched files changes. Change
//! events are debounced; a change that arrives while a run is in progress
//! queues exactly one more run. A failing run never stops watching.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// What the event loop needs from the output layer (TS `Logger` calls).
pub trait Ui {
    fn clear(&mut self);
    fn file_changed(&mut self, name: &str);
    fn watch_ready(&mut self);
    fn watching(&mut self, files: &[String]);
    fn error(&mut self, msg: &str);
    fn warning(&mut self, msg: &str);
}

/// Messages into the event loop.
#[derive(Debug)]
pub enum Msg {
    /// A watched file changed (the name as the user passed it).
    Changed(String),
    /// Stop watching (TS `stop()`).
    #[cfg_attr(not(test), allow(dead_code))]
    Stop,
}

pub struct Config {
    pub debounce: Duration,
    pub clear: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            debounce: Duration::from_millis(300),
            clear: true,
        }
    }
}

/// Debounce loop (TS `handleFileChange` + `runRequests`). Returns when told
/// to stop or when every sender is gone. Does not run initially.
pub fn event_loop(
    rx: &Receiver<Msg>,
    cfg: &Config,
    ui: &mut dyn Ui,
    on_run: &mut dyn FnMut() -> Result<(), String>,
) {
    loop {
        let mut name = match rx.recv() {
            Ok(Msg::Changed(n)) => n,
            Ok(Msg::Stop) | Err(_) => return,
        };
        // Each change restarts the debounce window.
        let mut deadline = Instant::now() + cfg.debounce;
        loop {
            match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Msg::Changed(n)) => {
                    name = n;
                    deadline = Instant::now() + cfg.debounce;
                }
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => break,
            }
        }
        if cfg.clear {
            ui.clear();
        }
        ui.file_changed(&name);
        if !run_requests(rx, cfg, ui, on_run) {
            return;
        }
    }
}

/// One run, then one queued run if changes arrived meanwhile (TS
/// `runRequests`). Returns false when told to stop.
fn run_requests(
    rx: &Receiver<Msg>,
    cfg: &Config,
    ui: &mut dyn Ui,
    on_run: &mut dyn FnMut() -> Result<(), String>,
) -> bool {
    loop {
        if let Err(e) = on_run() {
            ui.error(&e);
        }
        let mut queued = false;
        loop {
            match rx.try_recv() {
                Ok(Msg::Changed(_)) => queued = true,
                Ok(Msg::Stop) | Err(std::sync::mpsc::TryRecvError::Disconnected) => return false,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        if !queued {
            ui.watch_ready();
            return true;
        }
        if cfg.clear {
            ui.clear();
        }
        ui.file_changed("(queued change)");
    }
}

/// Initial run, watchers, then the event loop (TS `start`). Only returns
/// when the event loop ends (a `Msg::Stop` or every sender dropped).
pub fn start(
    files: &[String],
    cfg: &Config,
    ui: &mut dyn Ui,
    on_run: &mut dyn FnMut() -> Result<(), String>,
    (tx, rx): (Sender<Msg>, Receiver<Msg>),
) {
    if !run_requests(&rx, cfg, ui, on_run) {
        return;
    }
    // Keep a sender alive so a failed watcher setup leaves the loop waiting
    // (TS warns and stays alive until a signal) instead of disconnecting.
    let _keep_alive = tx.clone();
    let _watcher = setup_watcher(files, tx, ui);
    ui.watching(files);
    event_loop(&rx, cfg, ui, on_run);
}

/// Watch the directories holding `files` (not recursively) and forward the
/// events that touch one of them. Directories rather than the files
/// themselves, so atomic saves (write a temp file, rename it over) keep
/// working. Failures are warnings, as in TS.
fn setup_watcher(files: &[String], tx: Sender<Msg>, ui: &mut dyn Ui) -> Option<RecommendedWatcher> {
    // (canonical path, name as given)
    let mut targets: Vec<(PathBuf, String)> = Vec::new();
    for f in files {
        match std::fs::canonicalize(f) {
            Ok(p) => targets.push((p, f.clone())),
            Err(e) => ui.warning(&format!("Failed to watch {f}: {e}")),
        }
    }
    let table = targets.clone();
    let mut watcher = match notify::recommended_watcher(move |res: notify::Result<Event>| match res
    {
        Ok(ev) => {
            if !matches!(
                ev.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any
            ) {
                return;
            }
            for p in &ev.paths {
                let canon = std::fs::canonicalize(p).ok().or_else(|| {
                    let dir = std::fs::canonicalize(p.parent()?).ok()?;
                    Some(dir.join(p.file_name()?))
                });
                let Some(canon) = canon else { continue };
                if let Some((_, name)) = table.iter().find(|(t, _)| *t == canon) {
                    let _ = tx.send(Msg::Changed(name.clone()));
                    break;
                }
            }
        }
        Err(e) => {
            // The loop owns the UI; surface watcher errors on stderr.
            eprintln!("⚠ Watch error: {e}");
        }
    }) {
        Ok(w) => w,
        Err(e) => {
            ui.warning(&format!("Failed to watch files: {e}"));
            return None;
        }
    };
    let mut dirs: Vec<&Path> = targets.iter().filter_map(|(p, _)| p.parent()).collect();
    dirs.sort();
    dirs.dedup();
    for d in dirs {
        if let Err(e) = watcher.watch(d, RecursiveMode::NonRecursive) {
            ui.warning(&format!("Failed to watch {}: {e}", d.display()));
        }
    }
    Some(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;

    /// Failure guard for every wait: only hit when the loop is broken.
    const TIMEOUT: Duration = Duration::from_secs(10);

    /// Records UI calls; [`Mock::wait`] blocks until the log reaches a state.
    #[derive(Clone, Default)]
    struct Mock(Arc<(Mutex<Vec<String>>, Condvar)>);
    impl Mock {
        fn log(&self, s: impl Into<String>) {
            self.0.0.lock().unwrap().push(s.into());
            self.0.1.notify_all();
        }
        fn count(&self, s: &str) -> usize {
            self.0.0.lock().unwrap().iter().filter(|x| *x == s).count()
        }
        fn all(&self) -> Vec<String> {
            self.0.0.lock().unwrap().clone()
        }
        /// Wait until `s` has been logged `n` times.
        fn wait(&self, s: &str, n: usize) {
            let (log, cv) = &*self.0;
            let (log, res) = cv
                .wait_timeout_while(log.lock().unwrap(), TIMEOUT, |l| {
                    l.iter().filter(|x| *x == s).count() < n
                })
                .unwrap();
            assert!(
                !res.timed_out(),
                "no {n}× {s:?} within {TIMEOUT:?}: {log:?}"
            );
        }
    }
    impl Ui for Mock {
        fn clear(&mut self) {
            self.log("clear");
        }
        fn file_changed(&mut self, n: &str) {
            self.log(format!("changed:{n}"));
        }
        fn watch_ready(&mut self) {
            self.log("ready");
        }
        fn watching(&mut self, _: &[String]) {
            self.log("watching");
        }
        fn error(&mut self, m: &str) {
            self.log(format!("error:{m}"));
        }
        fn warning(&mut self, m: &str) {
            self.log(format!("warning:{m}"));
        }
    }

    fn cfg(ms: u64, clear: bool) -> Config {
        Config {
            debounce: Duration::from_millis(ms),
            clear,
        }
    }

    /// Run the loop on this thread. `queued` changes are in the channel
    /// before it starts (so they always land in one debounce window); `feed`
    /// then runs on another thread, synchronizing on the mock's log, and
    /// `Stop` is sent once it returns.
    fn drive(
        c: Config,
        mock: &Mock,
        queued: &[&str],
        feed: impl FnOnce(&Sender<Msg>, &Mock) + Send + 'static,
        mut on_run: impl FnMut() -> Result<(), String>,
    ) {
        let (tx, rx) = channel();
        for n in queued {
            change(&tx, n);
        }
        let m = mock.clone();
        let h = thread::spawn(move || {
            feed(&tx, &m);
            let _ = tx.send(Msg::Stop);
        });
        event_loop(&rx, &c, &mut mock.clone(), &mut on_run);
        h.join().unwrap();
    }

    fn change(tx: &Sender<Msg>, n: &str) {
        tx.send(Msg::Changed(n.into())).unwrap();
    }

    #[test]
    fn debounces_rapid_file_changes() {
        let mock = Mock::default();
        let runs = Arc::new(Mutex::new(0));
        let r = runs.clone();
        drive(
            cfg(50, true),
            &mock,
            &["test.yaml"; 3],
            |_, m| m.wait("ready", 1),
            move || {
                *r.lock().unwrap() += 1;
                Ok(())
            },
        );
        assert_eq!(*runs.lock().unwrap(), 1);
    }

    #[test]
    fn queues_one_run_during_execution() {
        let mock = Mock::default();
        let runs = Arc::new(Mutex::new(0));
        let r = runs.clone();
        // Each run reports that it started, then blocks until released.
        let (started_tx, started) = channel();
        let (release, release_rx) = channel::<()>();
        drive(
            cfg(10, false),
            &mock,
            &["test.yaml"],
            move |tx, m| {
                started.recv_timeout(TIMEOUT).expect("first run");
                change(tx, "test.yaml"); // while the run is in progress
                change(tx, "test.yaml");
                release.send(()).unwrap();
                started.recv_timeout(TIMEOUT).expect("queued run");
                release.send(()).unwrap();
                m.wait("ready", 1);
            },
            move || {
                *r.lock().unwrap() += 1;
                started_tx.send(()).unwrap();
                release_rx.recv_timeout(TIMEOUT).expect("released");
                Ok(())
            },
        );
        assert_eq!(*runs.lock().unwrap(), 2);
        assert_eq!(mock.count("changed:(queued change)"), 1);
    }

    #[test]
    fn stop_discards_a_pending_debounce() {
        let mock = Mock::default();
        let runs = Arc::new(Mutex::new(0));
        let r = runs.clone();
        let (tx, rx) = channel();
        change(&tx, "test.yaml");
        tx.send(Msg::Stop).unwrap();
        event_loop(&rx, &cfg(1000, true), &mut mock.clone(), &mut move || {
            *r.lock().unwrap() += 1;
            Ok(())
        });
        assert_eq!(*runs.lock().unwrap(), 0);
    }

    #[test]
    fn default_debounce_is_300ms() {
        assert_eq!(Config::default().debounce, Duration::from_millis(300));
        let mock = Mock::default();
        let ran = Arc::new(Mutex::new(None));
        let r = ran.clone();
        let t0 = Instant::now();
        drive(
            Config::default(),
            &mock,
            &["test.yaml"],
            |_, m| m.wait("ready", 1),
            move || {
                *r.lock().unwrap() = Some(t0.elapsed());
                Ok(())
            },
        );
        let at = ran.lock().unwrap().expect("ran");
        assert!(at >= Duration::from_millis(300), "ran after {at:?}");
    }

    #[test]
    fn run_errors_are_logged_and_watching_continues() {
        let mock = Mock::default();
        drive(
            cfg(10, false),
            &mock,
            &["a.yaml"],
            |tx, m| {
                m.wait("ready", 1);
                change(tx, "a.yaml");
                m.wait("ready", 2);
            },
            || Err("Test error".into()),
        );
        assert_eq!(mock.count("error:Test error"), 2);
        assert_eq!(mock.count("ready"), 2);
    }

    #[test]
    fn logs_file_changed_with_filename() {
        let mock = Mock::default();
        drive(
            cfg(10, false),
            &mock,
            &["my-api.yaml"],
            |_, m| m.wait("ready", 1),
            || Ok(()),
        );
        assert_eq!(mock.count("changed:my-api.yaml"), 1);
    }

    #[test]
    fn calls_watch_ready_after_run() {
        let mock = Mock::default();
        drive(
            cfg(10, false),
            &mock,
            &["test.yaml"],
            |_, m| m.wait("ready", 1),
            || Ok(()),
        );
        assert_eq!(mock.all(), ["changed:test.yaml", "ready"]);
    }

    #[test]
    fn clear_runs_before_the_change_line_unless_disabled() {
        let mock = Mock::default();
        drive(
            cfg(10, true),
            &mock,
            &["t.yaml"],
            |_, m| m.wait("ready", 1),
            || Ok(()),
        );
        assert_eq!(mock.all(), ["clear", "changed:t.yaml", "ready"]);
    }

    #[test]
    fn start_stays_alive_when_files_cannot_be_watched() {
        let mock = Mock::default();
        let (tx, rx) = channel();
        let stop = tx.clone();
        let m = mock.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let s = stopped.clone();
        let h = thread::spawn(move || {
            m.wait("watching", 1);
            // Give a broken loop time to return on its own.
            thread::sleep(Duration::from_millis(300));
            s.store(true, Ordering::SeqCst);
            let _ = stop.send(Msg::Stop);
        });
        start(
            &["/definitely/not/here.yaml".to_string()],
            &cfg(50, false),
            &mut mock.clone(),
            &mut || Ok(()),
            (tx, rx),
        );
        assert!(stopped.load(Ordering::SeqCst), "returned before Stop");
        h.join().unwrap();
        assert_eq!(mock.count("watching"), 1);
        assert!(
            mock.all()
                .iter()
                .any(|l| l.starts_with("warning:Failed to watch"))
        );
    }

    #[test]
    fn start_runs_once_then_reports_watching() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.yaml");
        std::fs::write(&file, "x").unwrap();
        let mock = Mock::default();
        let name = file.to_string_lossy().into_owned();
        let runs = Arc::new(Mutex::new(0));
        let r = runs.clone();
        let f2 = file.clone();
        // Touch the file once the watcher reports ready, then wait for the
        // second run; poll rather than sleep a fixed time.
        let m2 = mock.clone();
        let (tx, rx) = channel();
        let stop = tx.clone();
        let h = thread::spawn(move || {
            let t0 = Instant::now();
            while m2.count("watching") == 0 && t0.elapsed() < Duration::from_secs(10) {
                thread::sleep(Duration::from_millis(20));
            }
            while *r.lock().unwrap() < 2 && t0.elapsed() < Duration::from_secs(20) {
                std::fs::write(&f2, "y").unwrap();
                thread::sleep(Duration::from_millis(300));
            }
            let _ = stop.send(Msg::Stop);
        });
        let r2 = runs.clone();
        start(
            &[name],
            &cfg(50, false),
            &mut mock.clone(),
            &mut move || {
                *r2.lock().unwrap() += 1;
                Ok(())
            },
            (tx, rx),
        );
        h.join().unwrap();
        assert!(*runs.lock().unwrap() >= 2, "no re-run after touching");
        assert_eq!(mock.count("watching"), 1);
    }
}
