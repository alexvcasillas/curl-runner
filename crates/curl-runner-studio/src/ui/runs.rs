//! The workspace's runs: each file's last results, the run in flight and
//! the drain that applies its events.

use super::inspector::ResultView;
use super::run_button::run_state;
use super::workspace::Workspace;
use crate::model::environments::EnvCtx;
use crate::model::run::{RunEvent, RunHandle, RunRequest, RunTarget, start_run};
use chrono::{DateTime, Local};
use curl_runner_core::executor::AttemptOutcome;
use curl_runner_core::interpolate::Store;
use curl_runner_core::model::ExecutionResult;
use curl_runner_core::parser::parse_raw_stream;
use gpui_kit::*;
use serde_json::Value as Json;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

/// How often the drain looks for new run events.
const DRAIN_EVERY: Duration = Duration::from_millis(16);

/// One file's last run. Keyed by the request's zero-based position in the
/// file, so a one-request run replaces only its own entry.
pub struct FileRun {
    pub results: BTreeMap<usize, ExecutionResult>,
    /// What the inspector draws for each result, built once.
    pub views: BTreeMap<usize, ResultView>,
    pub attempts: HashMap<usize, Vec<(u32, AttemptOutcome)>>,
    pub waits: HashMap<usize, Vec<u64>>,
    /// The file's response store at the end of the run. Unredacted: it only
    /// seeds the next run, never the screen.
    pub store: Store,
    /// The parser's message when the file did not load.
    pub load_error: Option<String>,
    pub env: String,
    pub at: DateTime<Local>,
}

impl FileRun {
    fn new(env: &str) -> Self {
        FileRun {
            results: BTreeMap::new(),
            views: BTreeMap::new(),
            attempts: HashMap::new(),
            waits: HashMap::new(),
            store: Store::default(),
            load_error: None,
            env: env.to_string(),
            at: Local::now(),
        }
    }

    /// Transfers request `index` made in this run.
    pub fn attempt_count(&self, index: usize) -> usize {
        self.attempts.get(&index).map_or(0, Vec::len)
    }
}

/// The request being sent right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub file: PathBuf,
    pub index: usize,
    /// Transfers finished so far.
    pub done: u32,
    /// `retry.count + 1`.
    pub max_attempts: u32,
}

impl Progress {
    /// `Running… attempt 2 of 3`, or `Running…` without retries.
    pub fn label(&self) -> String {
        if self.max_attempts > 1 {
            let attempt = (self.done + 1).min(self.max_attempts);
            format!("Running… attempt {attempt} of {}", self.max_attempts)
        } else {
            "Running…".to_string()
        }
    }
}

#[derive(Default)]
pub struct Runs {
    pub last: HashMap<PathBuf, FileRun>,
    /// The run in flight; dropping it stops the run.
    pub active: Option<RunHandle>,
    pub progress: Option<Progress>,
    /// What the active run runs.
    target: Option<RunTarget>,
    /// The active run's files, by group.
    groups: Vec<PathBuf>,
    env: String,
    root: PathBuf,
    /// Each file's `retry.count + 1` per request, read once per run.
    max_attempts: HashMap<PathBuf, Vec<u32>>,
    drain: Option<Task<()>>,
}

impl Runs {
    /// Starts tracking `handle`, replacing (and so stopping) any active run.
    pub fn begin(&mut self, target: RunTarget, root: &Path, env: &str, handle: RunHandle) {
        self.active = Some(handle);
        self.target = Some(target);
        self.groups.clear();
        self.progress = None;
        self.max_attempts.clear();
        self.env = env.to_string();
        self.root = root.to_path_buf();
    }

    /// Whether `file` is part of the active run and has not finished yet.
    pub fn is_running_file(&self, file: &Path) -> bool {
        self.active.is_some()
            && match &self.target {
                Some(RunTarget::File(f)) | Some(RunTarget::Request { file: f, .. }) => f == file,
                Some(RunTarget::Files(files)) => files.iter().any(|f| f == file),
                Some(RunTarget::Collection) => true,
                None => false,
            }
    }

    /// Applies one event; `true` once the run is over.
    pub fn apply(&mut self, event: RunEvent) -> bool {
        match event {
            RunEvent::Started { groups, .. } => {
                self.groups = groups.into_iter().map(|(f, _)| f).collect();
                for file in &self.groups {
                    let fresh = FileRun::new(&self.env);
                    match &self.target {
                        // One request: the file's other results stay.
                        Some(RunTarget::Request { index, .. }) => {
                            let run = self.last.entry(file.clone()).or_insert(fresh);
                            run.results.remove(index);
                            run.views.remove(index);
                            run.attempts.remove(index);
                            run.waits.remove(index);
                            run.load_error = None;
                            run.env = self.env.clone();
                            run.at = Local::now();
                        }
                        _ => {
                            // The store carries over: it seeds the next run.
                            let store = self.last.remove(file).map(|r| r.store).unwrap_or_default();
                            self.last.insert(file.clone(), FileRun { store, ..fresh });
                        }
                    }
                }
            }
            RunEvent::RequestStarted { group, index, .. } => {
                if let Some(file) = self.groups.get(group).cloned() {
                    let max_attempts = self.max_attempts_of(&file, index);
                    self.progress = Some(Progress {
                        file,
                        index,
                        done: 0,
                        max_attempts,
                    });
                }
            }
            RunEvent::Attempt {
                group,
                index,
                attempt,
                outcome,
            } => {
                if let Some(p) = &mut self.progress
                    && p.index == index
                {
                    p.done = attempt;
                }
                if let Some(run) = self.run_of(group) {
                    run.attempts
                        .entry(index)
                        .or_default()
                        .push((attempt, outcome));
                }
            }
            RunEvent::RetryWait {
                group,
                index,
                delay_ms,
            } => {
                if let Some(run) = self.run_of(group) {
                    run.waits.entry(index).or_default().push(delay_ms);
                }
            }
            RunEvent::Result {
                group,
                index,
                result,
            } => {
                if self.progress.as_ref().is_some_and(|p| p.index == index) {
                    self.progress = None;
                }
                if let Some(run) = self.run_of(group) {
                    run.views.insert(index, ResultView::new(&result));
                    run.results.insert(index, *result);
                }
            }
            RunEvent::Stored { group, values } => {
                if let Some(run) = self.run_of(group) {
                    run.store.extend(values);
                }
            }
            RunEvent::LoadFailed { file, message } => {
                let env = self.env.clone();
                self.last
                    .entry(file)
                    .or_insert_with(|| FileRun::new(&env))
                    .load_error = Some(message);
            }
            RunEvent::Finished(outcome) => {
                for (file, store) in outcome.stores {
                    if let Some(run) = self.last.get_mut(&file) {
                        run.store = store;
                    }
                }
                self.finish();
                return true;
            }
        }
        false
    }

    /// The run is over (finished, or its thread went away).
    fn finish(&mut self) {
        self.active = None;
        self.progress = None;
        self.target = None;
    }

    fn run_of(&mut self, group: usize) -> Option<&mut FileRun> {
        self.last.get_mut(self.groups.get(group)?)
    }

    fn max_attempts_of(&mut self, file: &Path, index: usize) -> u32 {
        let root = &self.root;
        let counts = self
            .max_attempts
            .entry(file.to_path_buf())
            .or_insert_with(|| {
                std::fs::read_to_string(root.join(file))
                    .map(|text| max_attempts(&text))
                    .unwrap_or_default()
            });
        counts.get(index).copied().unwrap_or(1)
    }
}

/// `retry.count + 1` for each request of `text`, in run order (`request`,
/// `requests`, `collection.requests`); a request without `retry` takes the
/// file's `defaults`.
pub fn max_attempts(text: &str) -> Vec<u32> {
    let Ok(doc) = parse_raw_stream(text) else {
        return Vec::new();
    };
    let count = |r: &Json| {
        r.get("retry")
            .and_then(|r| r.get("count"))
            .and_then(Json::as_u64)
    };
    let collection = doc.get("collection");
    let default = [
        collection.and_then(|c| c.get("defaults")),
        doc.get("global").and_then(|g| g.get("defaults")),
    ]
    .into_iter()
    .flatten()
    .find_map(count);
    let requests = doc
        .get("request")
        .filter(|r| r.is_object())
        .into_iter()
        .chain(
            [
                doc.get("requests"),
                collection.and_then(|c| c.get("requests")),
            ]
            .into_iter()
            .flatten()
            .filter_map(Json::as_array)
            .flatten(),
        );
    requests
        .map(|r| {
            let n = count(r).or(default).unwrap_or(0);
            u32::try_from(n.saturating_add(1)).unwrap_or(u32::MAX)
        })
        .collect()
}

impl Workspace {
    /// Saves what `target` reads, then runs it in the picked environment.
    /// A file with errors does not run.
    pub fn run(&mut self, target: RunTarget, window: &mut Window, cx: &mut Context<Self>) {
        let Some(collection) = &self.state.collection else {
            return;
        };
        let root = collection.root.clone();
        let name = collection.name.clone();
        let file = match &target {
            RunTarget::File(f) | RunTarget::Request { file: f, .. } => Some(f.clone()),
            RunTarget::Collection | RunTarget::Files(_) => None,
        };
        // The CLI reads disk, so the buffer goes there first.
        let to_save: Vec<usize> = match &file {
            Some(f) => self.state.position(f).into_iter().collect(),
            None => (0..self.state.open.len()).collect(),
        };
        for ix in to_save {
            if file.is_some() {
                self.flush_diagnostics(ix, cx);
                if run_state(&self.state.open[ix].diagnostics).is_err() {
                    return;
                }
            }
            if self.state.open[ix].is_dirty() && !self.save(ix, window, cx) {
                return;
            }
        }
        let seed = file
            .as_ref()
            .and_then(|f| self.runs.last.get(f))
            .map(|r| r.store.clone());
        let ctx = EnvCtx {
            root: root.clone(),
            collection: name,
            environment: self.environment.clone(),
            marked_secret: Vec::new(),
        };
        let handle = start_run(
            RunRequest {
                target: target.clone(),
                ctx,
                seed,
            },
            self.secrets.clone(),
        );
        self.runs.begin(target, &root, &self.environment, handle);
        self.runs.drain = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(DRAIN_EVERY).await;
                let over = this
                    .update(cx, |this, cx| this.drain_run_events(cx))
                    .unwrap_or(true);
                if over {
                    break;
                }
            }
        }));
        cx.notify();
    }

    /// Runs the active file (Run, ⌘↵).
    pub(super) fn run_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(doc) = self.state.active_document() {
            let file = doc.rel_path.clone();
            self.run(RunTarget::File(file), window, cx);
        }
    }

    /// Runs only the active file's request under the cursor (⌥⌘↵).
    pub(super) fn run_request_under_cursor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.state.active else { return };
        let index = self.request_under_cursor(ix, cx);
        let file = self.state.open[ix].rel_path.clone();
        self.run(RunTarget::Request { file, index }, window, cx);
    }

    /// Applies the events that arrived; `true` once the run is over.
    fn drain_run_events(&mut self, cx: &mut Context<Self>) -> bool {
        let mut events = Vec::new();
        let mut gone = false;
        if let Some(handle) = &self.runs.active {
            loop {
                match handle.events.try_recv() {
                    Ok(e) => events.push(e),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        gone = true;
                        break;
                    }
                }
            }
        } else {
            return true;
        }
        let changed = !events.is_empty();
        let mut over = false;
        for e in events {
            over |= self.runs.apply(e);
        }
        if gone && !over {
            self.runs.finish();
            over = true;
        }
        if over {
            // Captured values may now resolve `${store.*}`.
            self.refresh_vars(cx);
        }
        if changed || over {
            cx.notify();
        }
        over
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptOutcome, ExecutionResult, Local, Path, PathBuf, Progress, RunEvent, RunTarget, Runs,
        Store, max_attempts,
    };
    use crate::model::run::RunOutcome;
    use curl_runner_core::executor::create_summary;
    use curl_runner_core::model::RequestConfig;

    fn result(status: u16) -> Box<ExecutionResult> {
        let mut r = ExecutionResult::new(
            RequestConfig {
                url: "https://api.example.com/charges".into(),
                ..Default::default()
            },
            true,
        );
        r.status = Some(status);
        Box::new(r)
    }

    fn finished(stores: Vec<(PathBuf, Store)>) -> RunEvent {
        RunEvent::Finished(RunOutcome {
            summary: create_summary(vec![], 0.),
            stopped: false,
            stores,
            attempts: vec![],
            started_at: Local::now(),
        })
    }

    fn started(file: &str, n: usize) -> RunEvent {
        RunEvent::Started {
            total: n,
            groups: vec![(file.into(), n)],
        }
    }

    fn runs(target: RunTarget) -> Runs {
        Runs {
            target: Some(target),
            env: "staging".into(),
            ..Runs::default()
        }
    }

    #[test]
    fn events_fill_the_file_run_by_request_index() {
        let mut r = runs(RunTarget::File("a.yaml".into()));
        let outcome = |status| AttemptOutcome {
            status: Some(status),
            duration_ms: 5.0,
            error: None,
        };
        assert!(!r.apply(started("a.yaml", 2)));
        r.apply(RunEvent::RequestStarted {
            group: 0,
            index: 1,
            name: "two".into(),
        });
        assert_eq!(r.progress.as_ref().map(|p| p.index), Some(1));
        r.apply(RunEvent::Attempt {
            group: 0,
            index: 1,
            attempt: 1,
            outcome: outcome(503),
        });
        r.apply(RunEvent::RetryWait {
            group: 0,
            index: 1,
            delay_ms: 100,
        });
        r.apply(RunEvent::Attempt {
            group: 0,
            index: 1,
            attempt: 2,
            outcome: outcome(200),
        });
        r.apply(RunEvent::Result {
            group: 0,
            index: 1,
            result: result(200),
        });
        assert_eq!(r.progress, None);
        let store = Store::from([("id".to_string(), "abc".to_string())]);
        assert!(r.apply(finished(vec![("a.yaml".into(), store.clone())])));
        let run = &r.last[Path::new("a.yaml")];
        assert_eq!(run.results.keys().copied().collect::<Vec<_>>(), [1]);
        assert_eq!(run.attempt_count(1), 2);
        assert_eq!(run.waits[&1], [100]);
        assert_eq!(run.store, store);
        assert_eq!(run.env, "staging");
        assert!(r.active.is_none() && r.target.is_none());
    }

    #[test]
    fn a_request_run_keeps_the_other_results() {
        let mut r = runs(RunTarget::File("a.yaml".into()));
        r.apply(started("a.yaml", 2));
        for index in 0..2 {
            r.apply(RunEvent::Result {
                group: 0,
                index,
                result: result(200),
            });
        }
        r.apply(finished(vec![]));
        r.target = Some(RunTarget::Request {
            file: "a.yaml".into(),
            index: 1,
        });
        r.apply(started("a.yaml", 1));
        let run = &r.last[Path::new("a.yaml")];
        assert_eq!(run.results.keys().copied().collect::<Vec<_>>(), [0]);
        r.apply(RunEvent::Result {
            group: 0,
            index: 1,
            result: result(201),
        });
        assert_eq!(r.last[Path::new("a.yaml")].results[&1].status, Some(201));
    }

    #[test]
    fn a_file_run_starts_fresh_but_keeps_the_store() {
        let mut r = runs(RunTarget::File("a.yaml".into()));
        r.apply(started("a.yaml", 1));
        r.apply(RunEvent::Result {
            group: 0,
            index: 0,
            result: result(200),
        });
        let store = Store::from([("id".to_string(), "abc".to_string())]);
        r.apply(finished(vec![("a.yaml".into(), store.clone())]));
        r.target = Some(RunTarget::File("a.yaml".into()));
        r.apply(started("a.yaml", 1));
        let run = &r.last[Path::new("a.yaml")];
        assert!(run.results.is_empty());
        assert_eq!(run.store, store);
    }

    #[test]
    fn load_failure_is_kept_for_the_file() {
        let mut r = runs(RunTarget::File("bad.yaml".into()));
        r.apply(RunEvent::LoadFailed {
            file: "bad.yaml".into(),
            message: "expected a mapping".into(),
        });
        assert_eq!(
            r.last[Path::new("bad.yaml")].load_error.as_deref(),
            Some("expected a mapping")
        );
    }

    #[test]
    fn progress_label_counts_attempts() {
        let mut p = Progress {
            file: "a.yaml".into(),
            index: 0,
            done: 1,
            max_attempts: 3,
        };
        assert_eq!(p.label(), "Running… attempt 2 of 3");
        p.done = 5;
        assert_eq!(p.label(), "Running… attempt 3 of 3");
        p.max_attempts = 1;
        assert_eq!(p.label(), "Running…");
    }

    #[test]
    fn max_attempts_reads_retry_counts_in_run_order() {
        let yaml = "global:\n  defaults:\n    retry:\n      count: 1\nrequests:\n  - url: a\n    retry:\n      count: 2\n  - url: b\n";
        assert_eq!(max_attempts(yaml), [3, 2]);
        assert_eq!(max_attempts("request:\n  url: a\n"), [1]);
        assert!(max_attempts("[").is_empty());
    }
}

#[cfg(test)]
mod ui_tests {
    use super::super::app::RunActive;
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_secrets};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Action as _, AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use testkit::{Response, TestServer};

    /// A server answering every path with `{"ok":true}` and recording it.
    fn recording_server() -> (TestServer, Arc<Mutex<Vec<String>>>) {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = TestServer::with_handler(move |req| {
            log.lock().unwrap().push(req.url.clone());
            Response::from_data(br#"{"ok":true}"#.to_vec())
                .with_header("Content-Type", "application/json")
        });
        (srv, seen)
    }

    fn open(
        cx: &mut TestAppContext,
        files: &[(&str, String)],
    ) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            std::fs::write(dir.path().join(rel), text).unwrap();
        }
        let root = dir.path().to_path_buf();
        let (window, ws) = cx.update(|cx| {
            gpui_kit::init(cx);
            super::super::app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(1440.), px(900.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    Workspace::new(Some(root), StateStore::none(), test_secrets(), window, cx)
                })
            })
            .unwrap()
        });
        (dir, window, ws)
    }

    /// Opens `rel` and presses ⌘↵.
    fn run_file(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        rel: &str,
    ) {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_file(Path::new(rel), window, cx));
            window.render_frame(cx);
            window.dispatch_action(RunActive.boxed_clone(), cx);
        })
        .unwrap();
        wait_for_run(cx, ws);
    }

    /// Lets the drain apply events until the run is over (real network, so
    /// real time passes too).
    fn wait_for_run(cx: &mut TestAppContext, ws: &Entity<Workspace>) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            cx.executor().advance_clock(Duration::from_millis(20));
            cx.run_until_parked();
            if ws.read_with(cx, |ws, _| ws.runs.active.is_none()) {
                return;
            }
            assert!(Instant::now() < deadline, "the run did not finish");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn inspector_has(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("inspector")
                .try_find_by_label(label)
                .is_some()
        })
        .unwrap()
    }

    #[gpui_kit::test]
    fn run_shows_response_in_inspector(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let yaml = format!(
            "request:\n  url: {}/status/200\n  expect:\n    status: 200\n",
            srv.base_url
        );
        let (_dir, window, ws) = open(cx, &[("ok.yaml", yaml)]);
        assert!(!inspector_has(cx, window, "200 OK"));
        run_file(cx, window, &ws, "ok.yaml");
        assert!(inspector_has(cx, window, "200 OK"), "status pill");
        assert!(inspector_has(cx, window, "1 of 1 checks passed"));
    }

    #[gpui_kit::test]
    fn run_saves_dirty_buffer_first(cx: &mut TestAppContext) {
        let (srv, seen) = recording_server();
        let yaml = format!("request:\n  url: {}/before\n", srv.base_url);
        let (dir, window, ws) = open(cx, &[("ok.yaml", yaml.clone())]);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("ok.yaml"), window, cx);
                let edited = yaml.replace("/before", "/after");
                // As typing does: the editor's text, then the document's.
                ws.tabs[0]
                    .editor
                    .update(cx, |e, cx| e.set_value(edited.clone(), window, cx));
                ws.state.open[0].set_buffer(edited);
            });
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(ws.state.open[0].is_dirty()));
        run_file(cx, window, &ws, "ok.yaml");
        assert_eq!(*seen.lock().unwrap(), ["/after"]);
        let disk = std::fs::read_to_string(dir.path().join("ok.yaml")).unwrap();
        assert!(disk.contains("/after"), "saved before the run");
    }

    #[gpui_kit::test]
    fn blocked_file_does_not_run(cx: &mut TestAppContext) {
        let (srv, seen) = recording_server();
        let yaml = format!("request:\n  url: {}/blocked\n  headers: [\n", srv.base_url);
        let (_dir, window, ws) = open(cx, &[("bad.yaml", yaml)]);
        run_file(cx, window, &ws, "bad.yaml");
        ws.read_with(cx, |ws, _| {
            assert!(!ws.state.open[0].diagnostics.is_empty());
            assert!(ws.runs.last.is_empty(), "nothing ran");
        });
        std::thread::sleep(Duration::from_millis(50));
        assert!(seen.lock().unwrap().is_empty());
    }

    #[gpui_kit::test]
    fn environment_picker_switches_values(cx: &mut TestAppContext) {
        let (srv, seen) = recording_server();
        let (_dir, window, ws) = open(
            cx,
            &[
                (".env.staging", format!("BASE_URL={}/s\n", srv.base_url)),
                (".env.production", format!("BASE_URL={}/p\n", srv.base_url)),
                ("env.yaml", "request:\n  url: ${BASE_URL}/x\n".to_string()),
            ],
        );
        ws.update(cx, |ws, cx| {
            assert_eq!(ws.environments, ["Default", "production", "staging"]);
            ws.set_environment("production", cx);
        });
        run_file(cx, window, &ws, "env.yaml");
        assert_eq!(*seen.lock().unwrap(), ["/p/x"]);
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.runs.last[Path::new("env.yaml")].env, "production");
        });
    }
}
