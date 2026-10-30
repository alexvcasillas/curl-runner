//! Runs on a background thread, streamed as [`RunEvent`]s.
//!
//! Mirrors the CLI's multi-file run (`RunCtx::execute` in the `curl-runner`
//! crate): one `Executor` per file group with its own response store, the
//! same env lookup, a shared redactor and the snapshot hook; a failure stops
//! only its own group (unless `continueOnError`), the next group still runs.
//! Every result is redacted before it leaves the run thread.

use crate::model::environments::{EnvCtx, is_secret};
use crate::model::secrets::SecretStore;
use curl_runner_core::config;
use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::executor::{
    self, AttemptOutcome, EventSink, Executor, PreparedRequest, create_summary,
};
use curl_runner_core::interpolate::{self, Store, extract_variables};
use curl_runner_core::model::{
    ExecutionResult, ExecutionSummary, HeaderValue, Headers, Json, RequestConfig,
};
use curl_runner_core::security::redact::Redactor;
use curl_runner_core::snapshot::ManagerHook;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// What to run. Paths are relative to the collection root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunTarget {
    File(PathBuf),
    /// One request (zero-based) of a file, alone.
    Request {
        file: PathBuf,
        index: usize,
    },
    /// Every request file of the collection.
    Collection,
    /// These files, in order (Re-run failed).
    Files(Vec<PathBuf>),
}

#[derive(Debug, Clone)]
pub struct RunRequest {
    pub target: RunTarget,
    pub ctx: EnvCtx,
    /// The file's response store from its last run; seeds single-file
    /// targets (`File`, `Request`). Ignored for files with `execution:
    /// parallel` (core's parallel runner has no store).
    pub seed: Option<Store>,
}

/// `group` is the position in `Started::groups`; `index` the zero-based
/// position of the request in its file.
#[derive(Debug, Clone)]
pub enum RunEvent {
    Started {
        total: usize,
        /// Each file and its request count, in run order.
        groups: Vec<(PathBuf, usize)>,
    },
    RequestStarted {
        group: usize,
        index: usize,
        name: String,
    },
    /// The request as a curl command line (the CLI's dry-run output),
    /// redacted.
    Command {
        group: usize,
        index: usize,
        command: String,
    },
    Attempt {
        group: usize,
        index: usize,
        attempt: u32,
        outcome: AttemptOutcome,
    },
    RetryWait {
        group: usize,
        index: usize,
        delay_ms: u64,
    },
    Result {
        group: usize,
        index: usize,
        result: Box<ExecutionResult>,
    },
    /// Values a request saved to its group's store. Unredacted (they seed
    /// the next run): never persist or display them without
    /// `Redactor::redact`.
    Stored {
        group: usize,
        values: Store,
    },
    /// The run could not load (`file` relative to the root); `Finished`
    /// follows.
    LoadFailed {
        file: PathBuf,
        message: String,
    },
    Finished(RunOutcome),
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// Redacted, like every event.
    pub summary: ExecutionSummary,
    /// The run ended early: [`RunHandle::stop`] was called and at least one
    /// request did not run.
    pub stopped: bool,
    /// Each group's response store at the end (seed included). Unredacted
    /// (they seed the next run): never persist or display them without
    /// `Redactor::redact`.
    pub stores: Vec<(PathBuf, Store)>,
    /// Transfers per result (retries included), aligned with
    /// `summary.results`; 0 for skipped requests and load failures.
    pub attempts: Vec<u32>,
    pub started_at: chrono::DateTime<chrono::Local>,
}

/// A run in progress. Dropping it stops the run.
pub struct RunHandle {
    pub events: Receiver<RunEvent>,
    cancel: Arc<Cancel>,
}

impl RunHandle {
    /// Stop the run: the in-flight request or retry wait ends, no further
    /// request or group starts.
    pub fn stop(&self) {
        self.cancel.stopped.store(true, Ordering::SeqCst);
        if let Some(flag) = lock(&self.cancel.current).clone() {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
impl RunHandle {
    /// A handle on `events` with no run behind it.
    pub fn for_test(events: Receiver<RunEvent>) -> Self {
        RunHandle {
            events,
            cancel: Arc::default(),
        }
    }
}

impl Drop for RunHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Default)]
struct Cancel {
    stopped: AtomicBool,
    /// The running group's executor flag.
    current: Mutex<Option<Arc<AtomicBool>>>,
}

/// Start `req` on its own thread. The last event is always `Finished`.
pub fn start_run(req: RunRequest, secrets: Arc<dyn SecretStore>) -> RunHandle {
    let (tx, events) = mpsc::channel();
    let cancel = Arc::new(Cancel::default());
    let flags = cancel.clone();
    std::thread::spawn(move || {
        let started_at = chrono::Local::now();
        let mut outcome = RunOutcome {
            summary: create_summary(Vec::new(), 0.0),
            stopped: false,
            stores: Vec::new(),
            attempts: Vec::new(),
            started_at,
        };
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute(&req, &*secrets, &flags, &tx, &mut outcome)
        }));
        if run.is_err() {
            let _ = tx.send(RunEvent::LoadFailed {
                file: PathBuf::new(),
                message: "The run stopped unexpectedly".into(),
            });
        }
        if let Ok(mut slot) = flags.current.lock() {
            *slot = None;
        }
        let _ = tx.send(RunEvent::Finished(outcome));
    });
    RunHandle { events, cancel }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The run itself; fills `out` as groups finish.
fn execute(
    req: &RunRequest,
    secrets: &dyn SecretStore,
    cancel: &Cancel,
    tx: &Sender<RunEvent>,
    out: &mut RunOutcome,
) {
    let root = req.ctx.root.as_path();
    let marked = &req.ctx.marked_secret;
    let files = target_files(&req.target, root);
    let paths: Vec<PathBuf> = files.iter().map(|f| root.join(f)).collect();

    // Secret names: marked ones and every secret the files reference,
    // each read from the Keychain once per run (no item search).
    let mut secret_names: Vec<String> = marked.clone();
    for p in &paths {
        let text = std::fs::read_to_string(p).unwrap_or_default();
        for v in extract_variables(&text) {
            let name = v.name.split(':').next().unwrap_or_default();
            if is_secret(name, marked) && !secret_names.iter().any(|n| n == name) {
                secret_names.push(name.to_string());
            }
        }
    }
    let service = req.ctx.service();
    let keychain: Arc<HashMap<String, String>> = Arc::new(
        secret_names
            .iter()
            .filter_map(|n| Some((n.clone(), secrets.get(&service, n)?)))
            .collect(),
    );
    // Keychain, then the process env; core adds the `.env` files after both.
    let lookup = {
        let keychain = keychain.clone();
        move |k: &str| {
            keychain
                .get(k)
                .cloned()
                .or_else(|| interpolate::process_env(k))
        }
    };
    let cli = req.ctx.cli_options();
    let project = config::load_config_file(root).config;
    let run = match executor::load_run(&paths, Some(&project), &cli, &lookup, root) {
        Ok(r) => r,
        Err(e) => {
            let file = e.file.strip_prefix(root).unwrap_or(&e.file).to_path_buf();
            let _ = tx.send(RunEvent::LoadFailed {
                file,
                message: e.source.to_string(),
            });
            return;
        }
    };

    let redactor = {
        // Every value a secret name has, whatever its source.
        let mut r = Redactor::default();
        let vars = run.global.variables.clone().unwrap_or_default();
        secret_names.extend(
            run.dotenv
                .keys()
                .chain(vars.keys())
                .filter(|k| is_secret(k, marked))
                .cloned(),
        );
        for name in &secret_names {
            r.add_secrets(
                [
                    keychain.get(name).cloned(),
                    interpolate::process_env(name),
                    run.dotenv.get(name).cloned(),
                    vars.get(name).cloned(),
                ]
                .into_iter()
                .flatten(),
            );
        }
        // As the CLI does: every request's inline secrets up front, so a
        // later request's token echoed by an earlier response is masked.
        let typed: Vec<RequestConfig> = run
            .requests
            .iter()
            .filter_map(|p| serde_json::from_value(p.raw.clone()).ok())
            .collect();
        r.register_request_secrets(&typed);
        Arc::new(Mutex::new(r))
    };

    let mut groups = group_by_file(&paths, run.requests);
    let mut offset = 0;
    if let RunTarget::Request { file, index } = &req.target {
        let reqs = std::mem::take(&mut groups[0].1);
        let Some(one) = reqs.into_iter().nth(*index) else {
            let _ = tx.send(RunEvent::LoadFailed {
                file: file.clone(),
                message: format!("{} has no request {}", file.display(), index + 1),
            });
            return;
        };
        groups[0].1.push(one);
        offset = *index;
    }
    let total: usize = groups.iter().map(|(_, r)| r.len()).sum();
    let _ = tx.send(RunEvent::Started {
        total,
        groups: files
            .iter()
            .zip(&groups)
            .map(|(f, (_, r))| (f.clone(), r.len()))
            .collect(),
    });

    let seed = match req.target {
        RunTarget::File(_) | RunTarget::Request { .. } => req.seed.clone(),
        RunTarget::Collection | RunTarget::Files(_) => None,
    };
    let global = &run.global;
    let multi = groups.len() > 1;
    let mut all = Vec::new();
    let mut duration = 0.0;
    let mut single = None;
    for (group, ((_, reqs), rel)) in groups.into_iter().zip(files).enumerate() {
        if cancel.stopped.load(Ordering::SeqCst) {
            break;
        }
        let (lookup, dotenv) = (lookup.clone(), run.dotenv.clone());
        let mut ex = Executor::new(global.clone())
            .with_cwd(root)
            .with_env(move |k| lookup(k).or_else(|| dotenv.get(k).cloned()))
            .with_redactor(redactor.clone())
            .with_snapshot_hook(Box::new(ManagerHook::from_global(
                global.snapshot.as_ref(),
                root,
            )));
        if let Some(s) = &seed {
            ex = ex.with_store(s.clone());
        }
        let flag = ex.cancel_handle();
        *lock(&cancel.current) = Some(flag.clone());
        if cancel.stopped.load(Ordering::SeqCst) {
            flag.store(true, Ordering::SeqCst);
        }
        let mut sink = Sink {
            tx: tx.clone(),
            group,
            offset,
            redactor: redactor.clone(),
            root: root.to_path_buf(),
            store: seed.clone().unwrap_or_default(),
            attempts: HashMap::new(),
        };
        let s = ex.run(reqs, &mut sink);
        out.stores.push((rel, sink.store));
        // Results are in request order, executor indexes are 1-based.
        out.attempts
            .extend((1..=s.results.len()).map(|i| sink.attempts.get(&i).copied().unwrap_or(0)));
        duration += s.duration;
        if multi {
            all.extend(s.results);
        } else {
            single = Some(s);
        }
    }
    let mut summary = single.unwrap_or_else(|| create_summary(all, duration));
    // Skipped groups and cancelled requests leave no result.
    out.stopped = cancel.stopped.load(Ordering::SeqCst) && summary.results.len() < total;
    let r = lock(&redactor);
    for result in &mut summary.results {
        prepare_result(result, &r, root);
    }
    out.summary = summary;
}

/// The files of `target`, relative to `root`, in run order.
fn target_files(target: &RunTarget, root: &Path) -> Vec<PathBuf> {
    match target {
        RunTarget::File(f) | RunTarget::Request { file: f, .. } => vec![f.clone()],
        RunTarget::Files(files) => files.clone(),
        // The tree's files: every YAML file but the project config.
        RunTarget::Collection => find_yaml_files(&[], true, root)
            .into_iter()
            .map(|f| relative_to(Path::new(&f), root))
            .filter(|p| !config::CONFIG_FILE_PATHS.iter().any(|c| p == Path::new(c)))
            .collect(),
    }
}

/// `path` relative to `root` (when under it), without `.` components.
fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

/// Requests per file, in file order (files with no requests keep an empty
/// group); the CLI's `group_by_file`.
fn group_by_file(
    files: &[PathBuf],
    requests: Vec<PreparedRequest>,
) -> Vec<(PathBuf, Vec<PreparedRequest>)> {
    let mut groups: Vec<(PathBuf, Vec<PreparedRequest>)> =
        files.iter().map(|f| (f.clone(), Vec::new())).collect();
    for r in requests {
        let src = r.raw.get("sourceFile").and_then(Json::as_str).unwrap_or("");
        let at = groups
            .iter()
            .position(|(f, _)| f == Path::new(src))
            .unwrap_or(0);
        groups[at].1.push(r);
    }
    groups
}

/// Forwards one group's executor events, redacted.
struct Sink {
    tx: Sender<RunEvent>,
    group: usize,
    /// Position in the file of the group's first request.
    offset: usize,
    redactor: Arc<Mutex<Redactor>>,
    root: PathBuf,
    store: Store,
    /// Transfers per executor index (1-based).
    attempts: HashMap<usize, u32>,
}

impl Sink {
    /// The executor's 1-based run index as a zero-based file index.
    fn at(&self, index: usize) -> usize {
        self.offset + index.saturating_sub(1)
    }

    fn send(&self, e: RunEvent) {
        let _ = self.tx.send(e);
    }
}

impl EventSink for Sink {
    fn on_start(&mut self, index: usize, req: &RequestConfig) {
        let name = req
            .name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| req.url.clone());
        let name = lock(&self.redactor).redact(&name);
        self.send(RunEvent::RequestStarted {
            group: self.group,
            index: self.at(index),
            name,
        });
    }

    fn on_command(&mut self, index: usize, _req: &RequestConfig, command: &str) {
        let command = lock(&self.redactor).redact(command);
        self.send(RunEvent::Command {
            group: self.group,
            index: self.at(index),
            command,
        });
    }

    fn on_attempt(
        &mut self,
        index: usize,
        _req: &RequestConfig,
        attempt: u32,
        outcome: &AttemptOutcome,
    ) {
        *self.attempts.entry(index).or_default() += 1;
        let mut outcome = outcome.clone();
        if let Some(e) = &mut outcome.error {
            *e = lock(&self.redactor).redact(e);
        }
        self.send(RunEvent::Attempt {
            group: self.group,
            index: self.at(index),
            attempt,
            outcome,
        });
    }

    fn on_retry(&mut self, index: usize, _req: &RequestConfig, _attempt: u32, delay_ms: u64) {
        self.send(RunEvent::RetryWait {
            group: self.group,
            index: self.at(index),
            delay_ms,
        });
    }

    fn on_result(&mut self, index: usize, result: &ExecutionResult) {
        let mut result = result.clone();
        prepare_result(&mut result, &lock(&self.redactor), &self.root);
        self.send(RunEvent::Result {
            group: self.group,
            index: self.at(index),
            result: Box::new(result),
        });
    }

    fn on_stored(&mut self, values: &Store) {
        self.store.extend(values.clone());
        self.send(RunEvent::Stored {
            group: self.group,
            values: values.clone(),
        });
    }
}

/// Make a result safe to leave the run thread: every string redacted and
/// `source_file` relative to the root, like the CLI's paths.
fn prepare_result(result: &mut ExecutionResult, r: &Redactor, root: &Path) {
    redact_result(result, r);
    if let Some(f) = &mut result.request.source_file {
        *f = relative_to(Path::new(f), root)
            .to_string_lossy()
            .into_owned();
    }
}

/// Mask every string of `result`: a JSON round trip of the whole result, so
/// new fields are covered too; field by field if the round trip fails.
fn redact_result(result: &mut ExecutionResult, r: &Redactor) {
    if let Ok(mut j) = serde_json::to_value(&*result) {
        redact_json(&mut j, r);
        if let Ok(v) = serde_json::from_value(j) {
            *result = v;
            return;
        }
    }
    /// A JSON round trip of a serde type with every string redacted.
    macro_rules! via_json {
        ($v:expr) => {
            serde_json::to_value(&$v).ok().and_then(|mut j| {
                redact_json(&mut j, r);
                serde_json::from_value(j).ok()
            })
        };
    }
    result.request = via_json!(result.request).unwrap_or_else(|| RequestConfig {
        name: result.request.name.as_deref().map(|n| r.redact(n)),
        url: r.redact(&result.request.url),
        method: result.request.method.clone(),
        source_file: result.request.source_file.clone(),
        ..Default::default()
    });
    if let Some(h) = &mut result.headers {
        redact_headers(h, r);
    }
    for block in result.header_history.iter_mut().flatten() {
        redact_headers(&mut block.headers, r);
    }
    if let Some(body) = &mut result.body {
        redact_json(body, r);
    }
    for s in [&mut result.error, &mut result.skip_reason]
        .into_iter()
        .flatten()
    {
        *s = r.redact(s);
    }
    if let Some(s) = result.snapshot_result.take() {
        result.snapshot_result = via_json!(s);
    }
    if let Some(d) = result.diff_result.take() {
        result.diff_result = via_json!(d);
    }
}

fn redact_headers(headers: &mut Headers, r: &Redactor) {
    for v in headers.values_mut() {
        match v {
            HeaderValue::One(s) => *s = r.redact(s),
            HeaderValue::Many(all) => all.iter_mut().for_each(|s| *s = r.redact(s)),
        }
    }
}

fn redact_json(v: &mut Json, r: &Redactor) {
    match v {
        Json::String(s) => *s = r.redact(s),
        Json::Array(items) => items.iter_mut().for_each(|v| redact_json(v, r)),
        Json::Object(map) => map.values_mut().for_each(|v| redact_json(v, r)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::environments::DEFAULT_ENVIRONMENT;
    use crate::model::secrets::MemorySecrets;
    use curl_runner_core::config::CliOptions;
    use std::fs;
    use std::time::{Duration, Instant};
    use testkit::{Response, TestServer};

    fn ctx(root: &Path) -> EnvCtx {
        EnvCtx {
            root: root.to_path_buf(),
            collection: "Payments API".into(),
            environment: DEFAULT_ENVIRONMENT.into(),
            marked_secret: vec![],
        }
    }

    fn run(root: &Path, target: RunTarget, seed: Option<Store>) -> Vec<RunEvent> {
        run_with(root, target, seed, Arc::new(MemorySecrets::default()))
    }

    fn run_with(
        root: &Path,
        target: RunTarget,
        seed: Option<Store>,
        secrets: Arc<dyn SecretStore>,
    ) -> Vec<RunEvent> {
        let h = start_run(
            RunRequest {
                target,
                ctx: ctx(root),
                seed,
            },
            secrets,
        );
        let events: Vec<RunEvent> = h.events.iter().collect();
        assert!(matches!(events.last(), Some(RunEvent::Finished(_))));
        events
    }

    fn outcome(events: &[RunEvent]) -> &RunOutcome {
        match events.last() {
            Some(RunEvent::Finished(o)) => o,
            _ => panic!("no Finished"),
        }
    }

    fn results(events: &[RunEvent]) -> Vec<&ExecutionResult> {
        events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Result { result, .. } => Some(&**result),
                _ => None,
            })
            .collect()
    }

    /// A server answering `/a` with `{"id":"abc"}` and recording every path.
    fn recording_server() -> (TestServer, Arc<Mutex<Vec<String>>>) {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = TestServer::with_handler(move |req| {
            log.lock().unwrap().push(req.url.clone());
            Response::from_data(br#"{"id":"abc"}"#.to_vec())
                .with_header("Content-Type", "application/json")
        });
        (srv, seen)
    }

    #[test]
    fn outcome_attempts_align_with_results() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "requests:\n  - name: fine\n    url: {0}/status/200\n  - name: flaky\n    url: {0}/status/503\n    retry:\n      count: 2\n      delay: 10\n",
            srv.base_url
        );
        fs::write(d.path().join("a.yaml"), yaml).unwrap();
        let events = run(d.path(), RunTarget::File("a.yaml".into()), None);
        let o = outcome(&events);
        assert_eq!(o.summary.results.len(), 2);
        assert_eq!(o.attempts, [1, 3]);
    }

    #[test]
    fn run_file_streams_events_and_finishes() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "requests:\n  - name: one\n    url: {0}/echo\n  - name: two\n    url: {0}/status/200\n",
            srv.base_url
        );
        fs::write(d.path().join("two.yaml"), yaml).unwrap();
        let events = run(d.path(), RunTarget::File("two.yaml".into()), None);
        let kinds: Vec<&str> = events
            .iter()
            .map(|e| match e {
                RunEvent::Started { total, groups } => {
                    assert_eq!(*total, 2);
                    assert_eq!(groups, &[(PathBuf::from("two.yaml"), 2)]);
                    "started"
                }
                RunEvent::RequestStarted { .. } => "request",
                RunEvent::Command { .. } => "command",
                RunEvent::Attempt { .. } => "attempt",
                RunEvent::Result { .. } => "result",
                RunEvent::Finished(_) => "finished",
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "started", "request", "command", "attempt", "result", "request", "command",
                "attempt", "result", "finished"
            ]
        );
        let indexes: Vec<usize> = events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Result {
                    group: 0, index, ..
                } => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(indexes, [0, 1]);
        for r in results(&events) {
            assert_eq!(r.request.source_file.as_deref(), Some("two.yaml"));
        }
        let o = outcome(&events);
        assert_eq!(o.summary.successful, 2);
        assert!(!o.stopped);
    }

    #[test]
    fn collection_groups_have_own_stores() {
        let (srv, seen) = recording_server();
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("a.yaml"),
            format!(
                "request:\n  url: {}/a\n  store:\n    ID: body.id\n",
                srv.base_url
            ),
        )
        .unwrap();
        fs::write(
            d.path().join("b.yaml"),
            format!("request:\n  url: {}/${{store.ID}}\n", srv.base_url),
        )
        .unwrap();

        // Reference: the CLI's run (`load_run`, then one Executor per file).
        let files = [d.path().join("a.yaml"), d.path().join("b.yaml")];
        let env = |k: &str| interpolate::process_env(k);
        let loaded =
            executor::load_run(&files, None, &CliOptions::default(), &env, d.path()).unwrap();
        for (_, reqs) in group_by_file(&files, loaded.requests) {
            Executor::new(loaded.global.clone())
                .with_cwd(d.path())
                .run(reqs, &mut executor::NoopSink);
        }
        let cli_paths = std::mem::take(&mut *seen.lock().unwrap());
        assert_eq!(cli_paths.first().map(String::as_str), Some("/a"));

        let events = run(d.path(), RunTarget::Collection, None);
        let studio_paths = seen.lock().unwrap().clone();
        assert_eq!(studio_paths, cli_paths);
        assert!(!studio_paths.iter().any(|p| p == "/abc"));
        let o = outcome(&events);
        assert_eq!(o.stores.len(), 2);
        assert_eq!(o.stores[0].1.get("ID").map(String::as_str), Some("abc"));
        assert!(o.stores[1].1.is_empty());
        assert!(events.iter().any(
            |e| matches!(e, RunEvent::Stored { group: 0, values } if values.get("ID").is_some())
        ));
    }

    #[test]
    fn request_target_runs_one_with_seed() {
        let (srv, seen) = recording_server();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "requests:\n  - url: {0}/one\n  - url: {0}/two\n  - url: {0}/item/${{store.ID}}\n",
            srv.base_url
        );
        fs::write(d.path().join("three.yaml"), yaml).unwrap();
        let mut seed = Store::new();
        seed.insert("ID".into(), "x".into());
        let events = run(
            d.path(),
            RunTarget::Request {
                file: "three.yaml".into(),
                index: 2,
            },
            Some(seed),
        );
        let r = results(&events);
        assert_eq!(r.len(), 1);
        assert!(
            r[0].request.url.ends_with("/item/x"),
            "{}",
            r[0].request.url
        );
        assert_eq!(*seen.lock().unwrap(), ["/item/x"]);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunEvent::Started { total: 1, .. }))
        );
        assert!(events.iter().any(|e| matches!(
            e,
            RunEvent::Result {
                group: 0,
                index: 2,
                ..
            }
        )));
    }

    #[test]
    fn stop_ends_run_quickly() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "requests:\n  - url: {0}/delay/5000\n  - url: {0}/echo\n",
            srv.base_url
        );
        fs::write(d.path().join("slow.yaml"), yaml).unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::File("slow.yaml".into()),
                ctx: ctx(d.path()),
                seed: None,
            },
            Arc::new(MemorySecrets::default()),
        );
        loop {
            match h.events.recv_timeout(Duration::from_secs(5)).unwrap() {
                RunEvent::RequestStarted { .. } => break,
                RunEvent::Finished(_) => panic!("finished early"),
                _ => {}
            }
        }
        let at = Instant::now();
        h.stop();
        let finished = loop {
            match h.events.recv_timeout(Duration::from_secs(5)).unwrap() {
                RunEvent::Finished(o) => break o,
                RunEvent::Result { .. } => panic!("a cancelled request is not reported"),
                _ => {}
            }
        };
        assert!(at.elapsed() < Duration::from_secs(1), "{:?}", at.elapsed());
        assert!(finished.stopped);
    }

    #[test]
    fn stop_after_last_request_is_not_stopped() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("one.yaml"),
            format!("request:\n  url: {}/echo\n", srv.base_url),
        )
        .unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::File("one.yaml".into()),
                ctx: ctx(d.path()),
                seed: None,
            },
            Arc::new(MemorySecrets::default()),
        );
        let mut finished = None;
        for e in h.events.iter() {
            match e {
                RunEvent::Result { .. } => h.stop(),
                RunEvent::Finished(o) => finished = Some(o),
                _ => {}
            }
        }
        let o = finished.expect("Finished");
        assert_eq!(o.summary.results.len(), 1);
        assert!(!o.stopped);
    }

    #[test]
    fn load_failure_is_an_event_not_a_panic() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("bad.yaml"), "request: [\n").unwrap();
        let events = run(d.path(), RunTarget::File("bad.yaml".into()), None);
        assert_eq!(events.len(), 2, "{events:?}");
        match &events[0] {
            RunEvent::LoadFailed { file, message } => {
                assert_eq!(file, Path::new("bad.yaml"));
                assert!(!message.is_empty());
            }
            other => panic!("expected LoadFailed, got {other:?}"),
        }
        assert!(!outcome(&events).stopped);
    }

    #[test]
    fn secret_never_in_events() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "request:\n  url: {}/echo?k=${{SECRET_PLAIN}}\n  method: POST\n  headers:\n    Authorization: Bearer ${{API_TOKEN}}\n  body:\n    note: ${{SECRET_PLAIN}}\n    file: ${{SECRET_FILE}}\n",
            srv.base_url
        );
        fs::write(d.path().join("auth.yaml"), yaml).unwrap();
        fs::write(d.path().join(".env"), "SECRET_FILE=dotenvvalue99\n").unwrap();
        let mut c = ctx(d.path());
        c.marked_secret = vec!["API_TOKEN".into()];
        let secrets = MemorySecrets::default();
        secrets
            .set(&c.service(), "API_TOKEN", "sk_live_abc")
            .unwrap();
        // No built-in token pattern matches this one.
        secrets
            .set(&c.service(), "SECRET_PLAIN", "opensesame42")
            .unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::File("auth.yaml".into()),
                ctx: c,
                seed: None,
            },
            // An item search that finds nothing must not drop secrets.
            Arc::new(NoNames(secrets)),
        );
        let events: Vec<RunEvent> = h.events.iter().collect();
        let r = results(&events);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].status, Some(200));
        // The secrets reached the server (the echo shows the redacted value).
        assert!(format!("{:?}", r[0].body).contains("[REDACTED]"));
        for e in &events {
            let dbg = format!("{e:?}");
            assert!(!dbg.contains("sk_live_abc"), "{dbg}");
            assert!(!dbg.contains("opensesame42"), "{dbg}");
            assert!(!dbg.contains("dotenvvalue99"), "{dbg}");
        }
    }

    #[test]
    fn command_is_sent_redacted() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "requests:\n  - url: {0}/status/200\n  - url: {0}/echo\n    headers:\n      X-Key: ${{SECRET_KEY}}\n",
            srv.base_url
        );
        fs::write(d.path().join("cmd.yaml"), yaml).unwrap();
        let c = ctx(d.path());
        let secrets = MemorySecrets::default();
        secrets
            .set(&c.service(), "SECRET_KEY", "opensesame42")
            .unwrap();
        let events = run_with(
            d.path(),
            RunTarget::File("cmd.yaml".into()),
            None,
            Arc::new(secrets),
        );
        let commands: Vec<(usize, &str)> = events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Command {
                    group: 0,
                    index,
                    command,
                } => Some((*index, command.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(commands.len(), 2, "{commands:?}");
        assert_eq!(commands[1].0, 1);
        assert!(commands[1].1.starts_with("curl "), "{}", commands[1].1);
        assert!(
            commands[1].1.contains("X-Key: [REDACTED]"),
            "{}",
            commands[1].1
        );
        assert!(!commands[1].1.contains("opensesame42"));
    }

    /// A store whose item search finds nothing (a failed Keychain search).
    struct NoNames(MemorySecrets);

    impl SecretStore for NoNames {
        fn get(&self, service: &str, name: &str) -> Option<String> {
            self.0.get(service, name)
        }
        fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
            self.0.set(service, name, value)
        }
        fn delete(&self, service: &str, name: &str) -> Result<(), String> {
            self.0.delete(service, name)
        }
        fn names(&self, _service: &str) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn secret_in_skip_reason_is_masked() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "request:\n  url: {}/echo\n  when: \"store.code == ${{SECRET_CODE}}\"\n",
            srv.base_url
        );
        fs::write(d.path().join("when.yaml"), yaml).unwrap();
        let c = ctx(d.path());
        let secrets = MemorySecrets::default();
        secrets
            .set(&c.service(), "SECRET_CODE", "opensesame42")
            .unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::File("when.yaml".into()),
                ctx: c,
                seed: None,
            },
            Arc::new(secrets),
        );
        let events: Vec<RunEvent> = h.events.iter().collect();
        let r = results(&events);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].skipped, Some(true));
        let reason = r[0].skip_reason.as_deref().unwrap_or_default();
        assert!(reason.contains("[REDACTED]"), "{reason}");
        for e in &events {
            assert!(!format!("{e:?}").contains("opensesame42"));
        }
    }

    #[test]
    fn redaction_round_trip_keeps_a_clean_result() {
        let srv = TestServer::start();
        let req = RequestConfig {
            name: Some("echo".into()),
            url: format!("{}/echo?a=1", srv.base_url),
            method: Some("POST".into()),
            body: Some(serde_json::json!({"n": 1.5, "s": "x", "z": null})),
            ..Default::default()
        };
        let s = Executor::new(Default::default()).run(
            vec![PreparedRequest::from_typed(&req)],
            &mut executor::NoopSink,
        );
        let original = s.results[0].clone();
        assert!(original.body.is_some() && original.headers.is_some());
        let mut redacted = original.clone();
        let mut r = Redactor::default();
        r.set_pattern_redaction(false);
        redact_result(&mut redacted, &r);
        assert_eq!(redacted, original);
    }

    #[test]
    fn stop_during_retry_wait_ends_quickly() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "request:\n  url: {}/status/503\n  retry:\n    count: 3\n    delay: 5000\n",
            srv.base_url
        );
        fs::write(d.path().join("retry.yaml"), yaml).unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::File("retry.yaml".into()),
                ctx: ctx(d.path()),
                seed: None,
            },
            Arc::new(MemorySecrets::default()),
        );
        loop {
            match h.events.recv_timeout(Duration::from_secs(5)).unwrap() {
                RunEvent::RetryWait { delay_ms, .. } => {
                    assert_eq!(delay_ms, 5000);
                    break;
                }
                RunEvent::Finished(_) => panic!("finished early"),
                _ => {}
            }
        }
        let at = Instant::now();
        h.stop();
        let finished = loop {
            if let RunEvent::Finished(o) = h.events.recv_timeout(Duration::from_secs(5)).unwrap() {
                break o;
            }
        };
        assert!(at.elapsed() < Duration::from_secs(1), "{:?}", at.elapsed());
        assert!(finished.stopped);
    }

    #[test]
    fn dropping_the_handle_stops_the_run() {
        let d = tempfile::tempdir().unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::Collection,
                ctx: ctx(d.path()),
                seed: None,
            },
            Arc::new(MemorySecrets::default()),
        );
        let cancel = h.cancel.clone();
        drop(h);
        assert!(cancel.stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn relative_to_handles_dot_and_root_prefixes() {
        let root = Path::new("/c");
        assert_eq!(
            relative_to(Path::new("./a/b.yaml"), root),
            Path::new("a/b.yaml")
        );
        assert_eq!(
            relative_to(Path::new("/c/a.yaml"), root),
            Path::new("a.yaml")
        );
    }
}
