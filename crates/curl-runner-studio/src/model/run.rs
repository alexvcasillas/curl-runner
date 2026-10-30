//! Runs on a background thread, streamed as [`RunEvent`]s.
//!
//! Mirrors the CLI's multi-file run (`RunCtx::execute` in the `curl-runner`
//! crate): one `Executor` per file group with its own response store, the
//! same env lookup, a shared redactor and the snapshot hook; a failure stops
//! only its own group (unless `continueOnError`), the next group still runs.
//! Every result is redacted before it leaves the run thread.

use crate::model::environments::{EnvCtx, effective_environment, is_secret};
use crate::model::secrets::SecretStore;
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
use curl_runner_core::{config, env};
use std::collections::{BTreeMap, HashMap};
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
    /// The file's response store from its last run; seeds `Request`
    /// targets only (a lone request reads what the requests before it
    /// captured). Every other run starts empty, like the CLI; ignored too
    /// with `execution: parallel` (core's parallel runner has no store).
    pub seed: Option<Store>,
}

/// `group` is the position in `Started::groups`; `index` the zero-based
/// position of the request in its file. Its `Debug` shows a store's names,
/// never its values.
#[derive(Clone)]
pub enum RunEvent {
    Started {
        total: usize,
        /// Each file and its request count, in run order.
        groups: Vec<(PathBuf, usize)>,
        config: RunConfig,
        /// Names (never values) of the secrets read from the Keychain,
        /// sorted: what the CLI needs set to repeat the run.
        keychain_names: Vec<String>,
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

/// The run's effective execution settings (the merged global config), for
/// the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunConfig {
    /// Requests in flight at once: 1 when sequential, `None` for parallel
    /// without `maxConcurrency` (unbounded).
    pub concurrency: Option<u64>,
    /// `continueOnError`: a failure does not stop its file's group.
    pub continue_on_error: bool,
}

/// The redactor a run used, shared so exports and history mask exactly what
/// the run masked. Its `Debug` hides the secrets it holds.
#[derive(Clone, Default)]
pub struct SharedRedactor(pub Arc<Mutex<Redactor>>);

impl SharedRedactor {
    /// The redactor itself, for the export and history functions.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Redactor> {
        lock(&self.0)
    }
}

impl std::fmt::Debug for SharedRedactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedRedactor(..)")
    }
}

/// Its `Debug` shows the stores' names, never their values.
#[derive(Clone)]
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
    /// The redactor the run used (every secret value it knew): exports and
    /// history redact with it, so they mask exactly what the run masked.
    pub redactor: SharedRedactor,
    /// Names (never values) of the secrets read from the Keychain, sorted:
    /// what the CLI needs set in its environment to repeat the run.
    pub keychain_names: Vec<String>,
}

/// A store in `Debug` output: its names only (the values are unredacted).
struct StoreNames<'a>(&'a Store);

impl std::fmt::Debug for StoreNames<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.0.keys()).finish()
    }
}

impl std::fmt::Debug for RunEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunEvent::Started {
                total,
                groups,
                config,
                keychain_names,
            } => f
                .debug_struct("Started")
                .field("total", total)
                .field("groups", groups)
                .field("config", config)
                .field("keychain_names", keychain_names)
                .finish(),
            RunEvent::RequestStarted { group, index, name } => f
                .debug_struct("RequestStarted")
                .field("group", group)
                .field("index", index)
                .field("name", name)
                .finish(),
            RunEvent::Command {
                group,
                index,
                command,
            } => f
                .debug_struct("Command")
                .field("group", group)
                .field("index", index)
                .field("command", command)
                .finish(),
            RunEvent::Attempt {
                group,
                index,
                attempt,
                outcome,
            } => f
                .debug_struct("Attempt")
                .field("group", group)
                .field("index", index)
                .field("attempt", attempt)
                .field("outcome", outcome)
                .finish(),
            RunEvent::RetryWait {
                group,
                index,
                delay_ms,
            } => f
                .debug_struct("RetryWait")
                .field("group", group)
                .field("index", index)
                .field("delay_ms", delay_ms)
                .finish(),
            RunEvent::Result {
                group,
                index,
                result,
            } => f
                .debug_struct("Result")
                .field("group", group)
                .field("index", index)
                .field("result", result)
                .finish(),
            RunEvent::Stored { group, values } => f
                .debug_struct("Stored")
                .field("group", group)
                .field("values", &StoreNames(values))
                .finish(),
            RunEvent::LoadFailed { file, message } => f
                .debug_struct("LoadFailed")
                .field("file", file)
                .field("message", message)
                .finish(),
            RunEvent::Finished(o) => f.debug_tuple("Finished").field(o).finish(),
        }
    }
}

impl std::fmt::Debug for RunOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stores: Vec<_> = self
            .stores
            .iter()
            .map(|(file, s)| (file, StoreNames(s)))
            .collect();
        f.debug_struct("RunOutcome")
            .field("summary", &self.summary)
            .field("stopped", &self.stopped)
            .field("stores", &stores)
            .field("attempts", &self.attempts)
            .field("started_at", &self.started_at)
            .field("redactor", &self.redactor)
            .field("keychain_names", &self.keychain_names)
            .finish()
    }
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
            redactor: SharedRedactor::default(),
            keychain_names: Vec::new(),
        };
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute(&req, secrets, &flags, &tx, &mut outcome)
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
    secrets: Arc<dyn SecretStore>,
    cancel: &Cancel,
    tx: &Sender<RunEvent>,
    out: &mut RunOutcome,
) {
    let root = req.ctx.root.as_path();
    let marked = &req.ctx.marked_secret;
    let files = target_files(&req.target, root);
    let paths: Vec<PathBuf> = files.iter().map(|f| root.join(f)).collect();

    let redactor = Arc::new(Mutex::new(Redactor::default()));
    out.redactor = SharedRedactor(redactor.clone());
    let source = Arc::new(EnvSource {
        secrets,
        service: req.ctx.service(),
        marked: marked.clone(),
        redactor: redactor.clone(),
        read: Mutex::default(),
    });
    // Up front, so their values are masked whatever fails first: the marked
    // names and every secret the files reference. Any other secret name is
    // read when core first looks it up, always before an event can carry
    // its value. (Core never interpolates `curl-runner.yaml`, as in the
    // CLI: its references are sent literally and need no lookup.)
    let texts: Vec<String> = paths
        .iter()
        .map(|p| std::fs::read_to_string(p).unwrap_or_default())
        .collect();
    for name in marked.iter().cloned().chain(texts.iter().flat_map(|t| {
        extract_variables(t)
            .into_iter()
            .map(|v| v.name.split(':').next().unwrap_or_default().to_string())
    })) {
        if is_secret(&name, marked) {
            source.lookup(&name);
        }
    }
    // Keychain (secret names only), then the process env; core adds the
    // `.env` files after both. A received copy's shared default is locked:
    // it comes before all of them.
    let shared = Arc::new(shared_lock(&req.ctx));
    let lookup = {
        let source = source.clone();
        move |k: &str| shared(k).or_else(|| source.lookup(k))
    };
    let cli = req.ctx.cli_options();
    let project = config::load_config_file(root).config;
    let run = match executor::load_run(&paths, Some(&project), &cli, &lookup, root) {
        Ok(r) => r,
        Err(e) => {
            let file = e.file.strip_prefix(root).unwrap_or(&e.file).to_path_buf();
            let message = load_failed_message(
                &req.ctx,
                &files,
                &mut lock(&redactor),
                &e.source.to_string(),
            );
            let _ = tx.send(RunEvent::LoadFailed { file, message });
            out.keychain_names = source.keychain_names();
            return;
        }
    };

    {
        // Every value a secret name has, whatever its source.
        let mut r = lock(&redactor);
        let vars = run.global.variables.clone().unwrap_or_default();
        for (k, v) in run.dotenv.iter().chain(&vars) {
            if is_secret(k, marked) {
                r.add_secrets([v.clone()].into_iter().chain(interpolate::process_env(k)));
            }
        }
        // As the CLI does: every request's inline secrets up front, so a
        // later request's token echoed by an earlier response is masked.
        let typed: Vec<RequestConfig> = run
            .requests
            .iter()
            .filter_map(|p| serde_json::from_value(p.raw.clone()).ok())
            .collect();
        r.register_request_secrets(&typed);
    }

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
    let parallel = run.global.execution.as_deref() == Some("parallel");
    let _ = tx.send(RunEvent::Started {
        total,
        groups: files
            .iter()
            .zip(&groups)
            .map(|(f, (_, r))| (f.clone(), r.len()))
            .collect(),
        config: RunConfig {
            concurrency: match run.global.max_concurrency {
                _ if !parallel => Some(1),
                Some(n) if n > 0 => Some(n),
                _ => None,
            },
            continue_on_error: run.global.continue_on_error == Some(true),
        },
        keychain_names: source.keychain_names(),
    });

    // Only a lone request starts from its file's last store (`store.*` of
    // the requests before it); a file run starts empty, like the CLI, and
    // core's parallel runner has no store.
    let seed = match req.target {
        RunTarget::Request { .. } if !parallel => req.seed.clone(),
        _ => None,
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
            // What the group captures (the seed it started from included).
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
    out.keychain_names = source.keychain_names();
    let mut summary = single.unwrap_or_else(|| create_summary(all, duration));
    // Skipped groups and cancelled requests leave no result.
    out.stopped = cancel.stopped.load(Ordering::SeqCst) && summary.results.len() < total;
    let r = lock(&redactor);
    for result in &mut summary.results {
        prepare_result(result, &r, root);
    }
    out.summary = summary;
}

/// Where a run's `${VAR}` values come from before core's `.env` fallback:
/// the Keychain for secret names (each item read once per run), then the
/// process env. Every secret value it hands out is added to the run's
/// redactor first.
struct EnvSource {
    secrets: Arc<dyn SecretStore>,
    service: String,
    marked: Vec<String>,
    redactor: Arc<Mutex<Redactor>>,
    /// Secret names looked up so far, with their Keychain value.
    read: Mutex<BTreeMap<String, Option<String>>>,
}

impl EnvSource {
    fn lookup(&self, name: &str) -> Option<String> {
        if !is_secret(name, &self.marked) {
            return interpolate::process_env(name);
        }
        let held = lock(&self.read)
            .entry(name.to_string())
            .or_insert_with(|| self.secrets.get(&self.service, name))
            .clone();
        let value = held.or_else(|| interpolate::process_env(name));
        if let Some(v) = &value {
            lock(&self.redactor).add_secret(v.clone());
        }
        value
    }

    /// Names (never values) of the secrets read from the Keychain, sorted.
    fn keychain_names(&self) -> Vec<String> {
        lock(&self.read)
            .iter()
            .filter(|(_, v)| v.is_some())
            .map(|(k, _)| k.clone())
            .collect()
    }
}

/// A received copy's defaults as the first lookup: the owner locked them,
/// so they win over the process env and the `.env` files. Secret names
/// never take one (the Keychain's), nor the `CURL_RUNNER_*` overrides the
/// lookup also answers for the config.
fn shared_lock(ctx: &EnvCtx) -> impl Fn(&str) -> Option<String> + use<> {
    let shared = ctx.shared.clone();
    let marked = ctx.marked_secret.clone();
    move |k: &str| {
        if k.starts_with("CURL_RUNNER_") || is_secret(k, &marked) {
            return None;
        }
        shared.get(k).map(str::to_string)
    }
}

/// A load error's text, safe to send: it may quote an interpolated value
/// (a secret in a typed field), so `r` (the Keychain values read so far)
/// also learns every secret name's value in the process env and the run's
/// `.env` files first.
fn load_failed_message(ctx: &EnvCtx, files: &[PathBuf], r: &mut Redactor, message: &str) -> String {
    let environment = effective_environment(ctx, files, &interpolate::process_env);
    let process = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)));
    r.add_secrets(
        env::load_env_files(&ctx.root, environment.as_deref())
            .into_iter()
            .chain(process)
            .filter(|(k, _)| is_secret(k, &ctx.marked_secret))
            .map(|(_, v)| v),
    );
    r.redact(message)
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

/// A collection-run table row's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Queued,
    Running,
    Passed,
    Failed,
    /// Not run: skipped by its config, or left over when the run stopped or
    /// its file stopped at a failure.
    Skipped,
}

/// One request of a collection run: `group` and `index` as in the events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowRef {
    pub group: usize,
    pub index: usize,
    pub file: PathBuf,
    pub name: String,
}

/// The rows of one top-level folder, in run order. `folder` is empty for
/// files at the collection root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderGroup {
    pub folder: String,
    pub rows: Vec<RowRef>,
}

/// The run's groups (`RunEvent::Started`) as table rows under their
/// top-level folder, folders in order of first appearance. `names` labels
/// request `index` of a file.
pub fn group_rows(
    groups: &[(PathBuf, usize)],
    names: impl Fn(&Path, usize) -> String,
) -> Vec<FolderGroup> {
    let mut out: Vec<FolderGroup> = Vec::new();
    for (group, (file, count)) in groups.iter().enumerate() {
        let mut parts = file.components().filter_map(|c| match c {
            Component::Normal(p) => Some(p.to_string_lossy().into_owned()),
            _ => None,
        });
        let first = parts.next().unwrap_or_default();
        let folder = if parts.next().is_some() {
            first
        } else {
            String::new()
        };
        let rows = (0..*count).map(|index| RowRef {
            group,
            index,
            file: file.clone(),
            name: names(file, index),
        });
        match out.iter_mut().find(|g| g.folder == folder) {
            Some(g) => g.rows.extend(rows),
            None => out.push(FolderGroup {
                folder,
                rows: rows.collect(),
            }),
        }
    }
    out
}

/// Time left: the average of the finished requests' durations times the
/// requests still to run; `None` before the first finishes.
pub fn eta_ms(done: &[f64], remaining: usize) -> Option<f64> {
    if done.is_empty() {
        return None;
    }
    let avg = done.iter().sum::<f64>() / done.len() as f64;
    Some(avg * remaining as f64)
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
            collection_id: "0a1b2c3d".into(),
            environment: DEFAULT_ENVIRONMENT.into(),
            marked_secret: vec![],
            shared: Default::default(),
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
    fn a_received_copy_sends_the_owners_locked_defaults_over_local_values() {
        let (srv, seen) = recording_server();
        let d = tempfile::tempdir().unwrap();
        let yaml = format!(
            "request:\n  url: {}/rates?c=${{SHIP_CURRENCY_R}}&r=${{SHIP_REGION_R}}\n",
            srv.base_url
        );
        fs::write(d.path().join("a.yaml"), yaml).unwrap();
        fs::write(d.path().join(".env"), "SHIP_REGION_R=eu\n").unwrap();
        let ctx = EnvCtx {
            shared: crate::model::environments::SharedDefaults {
                org: "Acme".into(),
                values: vec![
                    ("SHIP_CURRENCY_R".into(), "eur".into()),
                    ("SHIP_REGION_R".into(), "us".into()),
                ],
            },
            ..ctx(d.path())
        };
        let h = start_run(
            RunRequest {
                target: RunTarget::File("a.yaml".into()),
                ctx,
                seed: None,
            },
            Arc::new(MemorySecrets::default()),
        );
        let events: Vec<RunEvent> = h.events.iter().collect();
        assert!(results(&events)[0].success);
        // `.env` says `eu`; the owner's `us` is locked.
        assert_eq!(*seen.lock().unwrap(), ["/rates?c=eur&r=us"]);
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
                RunEvent::Started { total, groups, .. } => {
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

    #[test]
    fn group_rows_by_top_folder() {
        let groups = [
            (PathBuf::from("auth/login.yaml"), 1),
            (PathBuf::from("charges/create.yaml"), 1),
            (PathBuf::from("smoke.yaml"), 1),
            (PathBuf::from("auth/deep/refresh.yaml"), 2),
        ];
        let g = group_rows(&groups, |f, i| {
            format!("{}#{i}", f.file_stem().unwrap().to_string_lossy())
        });
        type Shape<'a> = Vec<(&'a str, Vec<(usize, usize, &'a str)>)>;
        let shape: Shape = g
            .iter()
            .map(|g| {
                (
                    g.folder.as_str(),
                    g.rows
                        .iter()
                        .map(|r| (r.group, r.index, r.name.as_str()))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                (
                    "auth",
                    vec![(0, 0, "login#0"), (3, 0, "refresh#0"), (3, 1, "refresh#1")]
                ),
                ("charges", vec![(1, 0, "create#0")]),
                ("", vec![(2, 0, "smoke#0")]),
            ]
        );
        assert_eq!(g[0].rows[1].file, Path::new("auth/deep/refresh.yaml"));
        assert!(group_rows(&[], |_, _| String::new()).is_empty());
    }

    #[test]
    fn eta_from_average() {
        assert_eq!(eta_ms(&[], 3), None);
        assert_eq!(eta_ms(&[100.0, 300.0], 3), Some(600.0));
        assert_eq!(eta_ms(&[100.0], 0), Some(0.0));
    }

    #[test]
    fn project_config_references_are_sent_as_the_cli_sends_them() {
        // Core (and so the CLI) never interpolates `curl-runner.yaml`: the
        // header goes out literally, and the Keychain value nowhere.
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = TestServer::with_handler(move |req| {
            log.lock()
                .unwrap()
                .push(req.header("Authorization").unwrap_or_default().to_string());
            Response::from_data(b"{}".to_vec())
        });
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("curl-runner.yaml"),
            "global:\n  defaults:\n    headers:\n      Authorization: Bearer ${SECRET_TOKEN}\n",
        )
        .unwrap();
        fs::write(
            d.path().join("a.yaml"),
            format!("request:\n  url: {}/a\n", srv.base_url),
        )
        .unwrap();
        let c = ctx(d.path());
        let secrets = MemorySecrets::default();
        secrets
            .set(&c.service(), "SECRET_TOKEN", "kc-only-token-77")
            .unwrap();
        let events = run_with(
            d.path(),
            RunTarget::File("a.yaml".into()),
            None,
            Arc::new(secrets),
        );
        assert_eq!(*seen.lock().unwrap(), ["Bearer ${SECRET_TOKEN}"]);
        for e in &events {
            let dbg = format!("{e:?}");
            assert!(!dbg.contains("kc-only-token-77"), "{dbg}");
        }
        assert!(outcome(&events).keychain_names.is_empty());
    }

    #[test]
    fn env_source_reads_each_secret_once_and_masks_it_first() {
        #[derive(Default)]
        struct Counting(MemorySecrets, Mutex<usize>);
        impl SecretStore for Counting {
            fn get(&self, service: &str, name: &str) -> Option<String> {
                *self.1.lock().unwrap() += 1;
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
        let secrets = Arc::new(Counting::default());
        secrets.set("svc", "SECRET_LATE", "zzlate").unwrap();
        let source = EnvSource {
            secrets: secrets.clone(),
            service: "svc".into(),
            marked: vec![],
            redactor: Arc::default(),
            read: Mutex::default(),
        };
        assert_eq!(source.lookup("PLAIN_NOT_SECRET_X"), None);
        assert_eq!(
            *secrets.1.lock().unwrap(),
            0,
            "plain names skip the Keychain"
        );
        assert_eq!(source.lookup("SECRET_LATE").as_deref(), Some("zzlate"));
        assert_eq!(source.lookup("SECRET_LATE").as_deref(), Some("zzlate"));
        assert_eq!(*secrets.1.lock().unwrap(), 1, "memoised per run");
        assert_eq!(
            lock(&source.redactor).redact("a zzlate b"),
            "a [REDACTED] b"
        );
        assert_eq!(source.lookup("SECRET_NONE"), None);
        assert_eq!(source.keychain_names(), ["SECRET_LATE"]);
    }

    #[test]
    fn secret_in_a_typed_field_never_reaches_an_event() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("t.yaml"),
            "request:\n  url: http://localhost/x\n  timeout: ${SECRET_T}\n",
        )
        .unwrap();
        fs::write(
            d.path().join("u.yaml"),
            "request:\n  url: http://localhost/x\n  timeout: ${SECRET_U}\n",
        )
        .unwrap();
        fs::write(d.path().join(".env"), "SECRET_U=dotenvbad\n").unwrap();
        let c = ctx(d.path());
        let secrets = MemorySecrets::default();
        secrets.set(&c.service(), "SECRET_T", "abc").unwrap();
        let secrets: Arc<dyn SecretStore> = Arc::new(secrets);
        for (file, value) in [("t.yaml", "abc"), ("u.yaml", "dotenvbad")] {
            let events = run_with(
                d.path(),
                RunTarget::File(file.into()),
                None,
                secrets.clone(),
            );
            let r = results(&events);
            assert!(
                r[0].error
                    .as_deref()
                    .is_some_and(|e| e.contains("[REDACTED]")),
                "{r:?}"
            );
            for e in &events {
                assert!(!format!("{e:?}").contains(value), "{e:?}");
            }
        }
    }

    #[test]
    fn load_failed_message_masks_every_secret_value() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join(".env"), "SECRET_U=dotenvbad\nPLAIN=visible\n").unwrap();
        let mut c = ctx(d.path());
        c.marked_secret = vec!["API_KEY".into()];
        fs::write(d.path().join(".env.local"), "API_KEY=markedbad\n").unwrap();
        let mut r = Redactor::default();
        r.add_secret("abc");
        let message = load_failed_message(
            &c,
            &[],
            &mut r,
            "got \"abc\", \"dotenvbad\", \"markedbad\" and \"visible\"",
        );
        assert_eq!(
            message,
            "got \"[REDACTED]\", \"[REDACTED]\", \"[REDACTED]\" and \"visible\""
        );
    }

    #[test]
    fn debug_never_prints_a_stored_value() {
        let mut values = Store::new();
        values.insert("TOKEN".into(), "captured-value-9".into());
        let stored = RunEvent::Stored {
            group: 0,
            values: values.clone(),
        };
        let finished = RunEvent::Finished(RunOutcome {
            summary: create_summary(Vec::new(), 0.0),
            stopped: false,
            stores: vec![("a.yaml".into(), values)],
            attempts: vec![],
            started_at: chrono::Local::now(),
            redactor: SharedRedactor::default(),
            keychain_names: vec![],
        });
        for e in [stored, finished] {
            let dbg = format!("{e:?}");
            assert!(dbg.contains("TOKEN"), "{dbg}");
            assert!(!dbg.contains("captured-value-9"), "{dbg}");
        }
    }

    #[test]
    fn file_runs_ignore_the_seed() {
        let (srv, seen) = recording_server();
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("one.yaml"),
            format!("request:\n  url: {}/item/${{store.ID}}\n", srv.base_url),
        )
        .unwrap();
        let mut seed = Store::new();
        seed.insert("ID".into(), "x".into());
        let events = run(d.path(), RunTarget::File("one.yaml".into()), Some(seed));
        assert_eq!(*seen.lock().unwrap(), ["/item/${store.ID}"]);
        assert!(outcome(&events).stores[0].1.is_empty());
    }

    #[test]
    fn parallel_request_run_reports_no_seed_store() {
        let (srv, _) = recording_server();
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("p.yaml"),
            format!(
                "global:\n  execution: parallel\nrequests:\n  - url: {0}/a\n  - url: {0}/b\n",
                srv.base_url
            ),
        )
        .unwrap();
        let mut seed = Store::new();
        seed.insert("ID".into(), "x".into());
        let events = run(
            d.path(),
            RunTarget::Request {
                file: "p.yaml".into(),
                index: 1,
            },
            Some(seed),
        );
        assert!(outcome(&events).stores[0].1.is_empty());
    }

    #[test]
    fn started_carries_the_effective_config() {
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        let url = format!("{}/status/200", srv.base_url);
        fs::write(
            d.path().join("a.yaml"),
            format!("global:\n  continueOnError: true\nrequest:\n  url: {url}\n"),
        )
        .unwrap();
        fs::write(
            d.path().join("b.yaml"),
            format!(
                "global:\n  execution: parallel\n  maxConcurrency: 3\nrequest:\n  url: {url}\n"
            ),
        )
        .unwrap();
        let config = |target| {
            run(d.path(), target, None)
                .into_iter()
                .find_map(|e| match e {
                    RunEvent::Started { config, .. } => Some(config),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(
            config(RunTarget::File("a.yaml".into())),
            RunConfig {
                concurrency: Some(1),
                continue_on_error: true
            }
        );
        assert_eq!(
            config(RunTarget::File("b.yaml".into())),
            RunConfig {
                concurrency: Some(3),
                continue_on_error: false
            }
        );
    }

    #[test]
    fn reports_from_a_real_run_never_hold_a_secret() {
        use crate::model::export::{to_json, to_junit, to_markdown};
        use crate::model::history::{EntryKind, entry_from_outcome};
        let srv = TestServer::start();
        let d = tempfile::tempdir().unwrap();
        // The echo returns the secret in the body and headers; the check
        // fails, so the failure text quotes it too.
        let yaml = format!(
            "request:\n  name: auth ${{SECRET_PLAIN}}\n  url: {}/echo?k=${{SECRET_PLAIN}}\n  method: POST\n  body:\n    note: ${{SECRET_PLAIN}}\n  expect:\n    status: 201\n",
            srv.base_url
        );
        fs::write(d.path().join("auth.yaml"), yaml).unwrap();
        let c = ctx(d.path());
        let secrets = MemorySecrets::default();
        secrets
            .set(&c.service(), "SECRET_PLAIN", "opensesame42")
            .unwrap();
        let h = start_run(
            RunRequest {
                target: RunTarget::Collection,
                ctx: c,
                seed: None,
            },
            Arc::new(secrets),
        );
        let events: Vec<RunEvent> = h.events.iter().collect();
        let o = outcome(&events);
        assert_eq!(o.summary.results.len(), 1);
        assert_eq!(o.keychain_names, ["SECRET_PLAIN"]);
        let started = events.iter().find_map(|e| match e {
            RunEvent::Started { keychain_names, .. } => Some(keychain_names.clone()),
            _ => None,
        });
        assert_eq!(started.unwrap(), ["SECRET_PLAIN"]);
        let r = o.redactor.lock();
        let groups = [(PathBuf::from("auth.yaml"), 1)];
        let entry = entry_from_outcome(o, "collection", EntryKind::CollectionRun, "Default", &r);
        for text in [
            to_json(&o.summary, &r),
            to_junit(&o.summary, &groups, &r),
            to_markdown(&o.summary, "Run · Payments API", "Default", &r),
            serde_json::to_string(&entry).unwrap(),
            format!("{:?}", o.redactor),
        ] {
            assert!(!text.contains("opensesame42"), "{text}");
        }
        assert!(to_json(&o.summary, &r).contains("[REDACTED]"));
    }
}
