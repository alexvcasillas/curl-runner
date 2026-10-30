//! The workspace's runs: each file's last results, the run in flight and
//! the drain that applies its events.

use super::inspector::{InspectorTab, ResultView};
use super::result_marks::ResultMarks;
use super::run_button::run_state;
use super::workspace::Workspace;
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
    /// Each request as a curl command line, redacted (Copy as curl).
    pub commands: HashMap<usize, String>,
    /// [`text_hash`] of the file text each result ran from: its hints show
    /// only while the file still reads that way.
    pub ran: HashMap<usize, u64>,
    /// Each request's name as the run announced it, redacted.
    pub names: HashMap<usize, String>,
    /// The request that saved each store value, by value name.
    pub captured_by: HashMap<String, String>,
    /// The file's response store at the end of the run. Unredacted: it only
    /// seeds the next run, never the screen.
    pub store: Store,
    /// The parser's message when the file did not load.
    pub load_error: Option<String>,
    /// The Keychain secrets the run read, names only (a received copy's
    /// `ran with your API_TOKEN`).
    pub keychain_names: Vec<String>,
    pub env: String,
    pub at: DateTime<Local>,
}

impl FileRun {
    pub(super) fn new(env: &str) -> Self {
        FileRun {
            results: BTreeMap::new(),
            views: BTreeMap::new(),
            attempts: HashMap::new(),
            waits: HashMap::new(),
            commands: HashMap::new(),
            ran: HashMap::new(),
            names: HashMap::new(),
            captured_by: HashMap::new(),
            store: Store::default(),
            load_error: None,
            keychain_names: Vec::new(),
            env: env.to_string(),
            at: Local::now(),
        }
    }

    /// Transfers request `index` made in this run.
    pub fn attempt_count(&self, index: usize) -> usize {
        self.attempts.get(&index).map_or(0, Vec::len)
    }
}

/// A tree row's last-run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    NotRun,
    Passed,
    Failed,
}

/// The request being sent right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub group: usize,
    pub file: PathBuf,
    pub index: usize,
    /// Transfers finished so far.
    pub done: u32,
    /// `retry.count + 1`.
    pub max_attempts: u32,
}

impl Progress {
    fn is(&self, group: usize, index: usize) -> bool {
        self.group == group && self.index == index
    }

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
    /// [`text_hash`] of each group's file as the run read it from disk.
    group_hashes: Vec<Option<u64>>,
    env: String,
    root: PathBuf,
    /// The last request started, by group and index: the one a `Stored`
    /// event belongs to.
    last_started: Option<(usize, usize)>,
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
        self.last_started = None;
        self.env = env.to_string();
        self.root = root.to_path_buf();
    }

    /// What the active run runs.
    pub fn target(&self) -> Option<&RunTarget> {
        self.target.as_ref()
    }

    /// The environment the active run uses.
    pub fn env(&self) -> &str {
        &self.env
    }

    /// Request `index` of `file`, or with `None` the whole file: failed if
    /// any result failed (or the file did not load), passed once every
    /// result so far passed, else not run.
    pub fn status(&self, file: &Path, index: Option<usize>) -> RunStatus {
        let Some(run) = self.last.get(file) else {
            return RunStatus::NotRun;
        };
        let passed = |r: &ExecutionResult| r.success || r.skipped == Some(true);
        match index {
            Some(i) => match run.results.get(&i) {
                Some(r) if passed(r) => RunStatus::Passed,
                Some(_) => RunStatus::Failed,
                None => RunStatus::NotRun,
            },
            None if run.load_error.is_some() || run.results.values().any(|r| !passed(r)) => {
                RunStatus::Failed
            }
            None if run.results.is_empty() => RunStatus::NotRun,
            None => RunStatus::Passed,
        }
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

    /// Whether request `index` of `file` is still to run in the active run:
    /// a one-request run leaves the file's other requests alone.
    pub fn is_pending(&self, file: &Path, index: usize) -> bool {
        match &self.target {
            Some(RunTarget::Request { file: f, index: i }) => {
                self.active.is_some() && f == file && *i == index
            }
            _ => self.is_running_file(file),
        }
    }

    /// Applies one event; `true` once the run is over.
    pub fn apply(&mut self, event: RunEvent) -> bool {
        match event {
            RunEvent::Started {
                groups,
                keychain_names,
                ..
            } => {
                self.groups = groups.into_iter().map(|(f, _)| f).collect();
                // The run reads disk once it starts; so does this.
                self.group_hashes = self
                    .groups
                    .iter()
                    .map(|f| {
                        std::fs::read_to_string(self.root.join(f))
                            .ok()
                            .map(|t| text_hash(&t))
                    })
                    .collect();
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
                            run.commands.remove(index);
                            run.ran.remove(index);
                            run.load_error = None;
                            run.keychain_names = keychain_names.clone();
                            run.env = self.env.clone();
                            run.at = Local::now();
                        }
                        _ => {
                            // The store carries over: it seeds the next run.
                            let (store, captured_by) = self
                                .last
                                .remove(file)
                                .map(|r| (r.store, r.captured_by))
                                .unwrap_or_default();
                            self.last.insert(
                                file.clone(),
                                FileRun {
                                    store,
                                    captured_by,
                                    keychain_names: keychain_names.clone(),
                                    ..fresh
                                },
                            );
                        }
                    }
                }
            }
            RunEvent::RequestStarted { group, index, name } => {
                self.last_started = Some((group, index));
                if let Some(run) = self.run_of(group) {
                    run.names.insert(index, name);
                }
                if let Some(file) = self.groups.get(group).cloned() {
                    let max_attempts = self.max_attempts_of(&file, index);
                    self.progress = Some(Progress {
                        group,
                        file,
                        index,
                        done: 0,
                        max_attempts,
                    });
                }
            }
            RunEvent::Command {
                group,
                index,
                command,
            } => {
                if let Some(run) = self.run_of(group) {
                    run.commands.insert(index, command);
                }
            }
            RunEvent::Attempt {
                group,
                index,
                attempt,
                outcome,
            } => {
                if let Some(p) = &mut self.progress
                    && p.is(group, index)
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
                if self.progress.as_ref().is_some_and(|p| p.is(group, index)) {
                    self.progress = None;
                }
                let hash = self.group_hashes.get(group).copied().flatten();
                if let Some(run) = self.run_of(group) {
                    if let Some(hash) = hash {
                        run.ran.insert(index, hash);
                    }
                    run.views.insert(index, ResultView::new(&result));
                    run.results.insert(index, *result);
                }
            }
            RunEvent::Stored { group, values } => {
                let by = self
                    .last_started
                    .filter(|(g, _)| *g == group)
                    .map(|(_, index)| index);
                if let Some(run) = self.run_of(group) {
                    if let Some(name) = by.and_then(|i| run.names.get(&i)).cloned() {
                        for key in values.keys() {
                            run.captured_by.insert(key.clone(), name.clone());
                        }
                    }
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

/// A cheap fingerprint of a file's text, to tell whether a result ran from
/// what the editor shows.
pub fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
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
    /// Ignored while a run is in flight (Stop comes first).
    pub fn run(&mut self, target: RunTarget, window: &mut Window, cx: &mut Context<Self>) {
        if self.runs.active.is_some() {
            return;
        }
        let Some(collection) = &self.state.collection else {
            return;
        };
        let root = collection.root.clone();
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
        let Some(collection) = &self.state.collection else {
            return;
        };
        let ctx = self.env_ctx(collection, cx);
        let handle = start_run(
            RunRequest {
                target: target.clone(),
                ctx: ctx.clone(),
                seed,
            },
            self.secrets.clone(),
        );
        self.runs
            .begin(target.clone(), &root, &self.environment, handle);
        if matches!(target, RunTarget::Collection | RunTarget::Files(_)) {
            self.open_run_view(target, ctx, window, cx);
        }
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

    /// Tab `ix`'s failed-check hints from its file's last run: only the
    /// results that ran from the text the tab holds now.
    pub(super) fn refresh_result_marks(&mut self, ix: usize) {
        let doc = &self.state.open[ix];
        self.tabs[ix].results = match self.runs.last.get(&doc.rel_path) {
            Some(run) => ResultMarks::new(&doc.text, run),
            None => ResultMarks::default(),
        };
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
        let inspected = self.state.active.map(|ix| {
            (
                self.state.open[ix].rel_path.clone(),
                self.tabs[ix].cursor_request,
            )
        });
        let mut over = false;
        let mut shown_failed = false;
        for e in events {
            if let (RunEvent::Result { group, index, .. }, Some((file, at))) = (&e, &inspected) {
                shown_failed |= self.runs.groups.get(*group) == Some(file) && index == at;
            }
            self.forward_run_event(&e, cx);
            over |= self.runs.apply(e);
        }
        // A fresh result with failed checks opens on them, unless the user
        // picked a tab other than Response.
        if shown_failed && self.inspector_tab == InspectorTab::Response {
            let failed = inspected
                .and_then(|(file, at)| Some(self.runs.last.get(&file)?.views.get(&at)?.failed()))
                .is_some_and(|n| n > 0);
            if failed {
                self.inspector_tab = InspectorTab::Checks;
            }
        }
        if gone && !over {
            self.runs.finish();
            over = true;
        }
        if over {
            // Captured values may now resolve `${store.*}`.
            self.refresh_vars(cx);
            // Only the files this run ran: others keep hints that followed
            // their edits.
            for ix in 0..self.tabs.len() {
                if self.runs.groups.contains(&self.state.open[ix].rel_path) {
                    self.refresh_result_marks(ix);
                }
            }
        }
        if changed || over {
            self.sync_run_status(cx);
            cx.notify();
        }
        over
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptOutcome, ExecutionResult, Local, Path, PathBuf, Progress, RunEvent, RunStatus,
        RunTarget, Runs, Store, max_attempts,
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
            redactor: Default::default(),
            keychain_names: vec![],
        })
    }

    fn started(file: &str, n: usize) -> RunEvent {
        RunEvent::Started {
            total: n,
            groups: vec![(file.into(), n)],
            config: Default::default(),
            keychain_names: vec![],
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
    fn commands_and_capturers_are_kept_per_request() {
        let mut r = runs(RunTarget::File("a.yaml".into()));
        r.apply(started("a.yaml", 1));
        r.apply(RunEvent::RequestStarted {
            group: 0,
            index: 0,
            name: "create-charge".into(),
        });
        r.apply(RunEvent::Command {
            group: 0,
            index: 0,
            command: "curl -X POST https://api.example.com".into(),
        });
        r.apply(RunEvent::Result {
            group: 0,
            index: 0,
            result: result(201),
        });
        r.apply(RunEvent::Stored {
            group: 0,
            values: Store::from([("CHARGE_ID".to_string(), "ch_1".to_string())]),
        });
        r.apply(finished(vec![]));
        let run = &r.last[Path::new("a.yaml")];
        assert_eq!(run.commands[&0], "curl -X POST https://api.example.com");
        assert_eq!(run.captured_by["CHARGE_ID"], "create-charge");
        // A new file run keeps what seeds it, and who captured it.
        r.target = Some(RunTarget::File("a.yaml".into()));
        r.apply(started("a.yaml", 1));
        let run = &r.last[Path::new("a.yaml")];
        assert!(run.commands.is_empty());
        assert_eq!(run.captured_by["CHARGE_ID"], "create-charge");
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
    fn status_aggregates_a_file_and_reads_each_request() {
        let mut r = runs(RunTarget::File("a.yaml".into()));
        let file = Path::new("a.yaml");
        assert_eq!(r.status(file, None), RunStatus::NotRun);
        r.apply(started("a.yaml", 2));
        assert_eq!(r.status(file, None), RunStatus::NotRun);
        r.apply(RunEvent::Result {
            group: 0,
            index: 0,
            result: result(200),
        });
        assert_eq!(r.status(file, None), RunStatus::Passed);
        let mut failed = result(500);
        failed.success = false;
        r.apply(RunEvent::Result {
            group: 0,
            index: 1,
            result: failed,
        });
        assert_eq!(r.status(file, None), RunStatus::Failed);
        assert_eq!(r.status(file, Some(0)), RunStatus::Passed);
        assert_eq!(r.status(file, Some(1)), RunStatus::Failed);
        assert_eq!(r.status(file, Some(2)), RunStatus::NotRun);
        r.apply(RunEvent::LoadFailed {
            file: "b.yaml".into(),
            message: "bad".into(),
        });
        assert_eq!(r.status(Path::new("b.yaml"), None), RunStatus::Failed);
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
    fn a_request_run_leaves_the_other_requests_settled() {
        let (tx, events) = std::sync::mpsc::channel::<RunEvent>();
        drop(tx);
        let mut r = runs(RunTarget::Request {
            file: "a.yaml".into(),
            index: 1,
        });
        r.active = Some(crate::model::run::RunHandle::for_test(events));
        assert!(r.is_pending(Path::new("a.yaml"), 1));
        assert!(!r.is_pending(Path::new("a.yaml"), 0));
        assert!(!r.is_pending(Path::new("b.yaml"), 1));
        r.target = Some(RunTarget::File("a.yaml".into()));
        assert!(r.is_pending(Path::new("a.yaml"), 0));
    }

    #[test]
    fn progress_is_matched_by_group_and_index() {
        let mut r = Runs {
            target: Some(RunTarget::Collection),
            ..Runs::default()
        };
        r.apply(RunEvent::Started {
            total: 2,
            groups: vec![("a.yaml".into(), 1), ("b.yaml".into(), 1)],
            config: Default::default(),
            keychain_names: vec![],
        });
        r.apply(RunEvent::RequestStarted {
            group: 1,
            index: 0,
            name: "b".into(),
        });
        // Same index, other group: not the running request.
        r.apply(RunEvent::Result {
            group: 0,
            index: 0,
            result: result(200),
        });
        assert_eq!(r.progress.as_ref().map(|p| p.group), Some(1));
        r.apply(RunEvent::Result {
            group: 1,
            index: 0,
            result: result(200),
        });
        assert_eq!(r.progress, None);
    }

    #[test]
    fn progress_label_counts_attempts() {
        let mut p = Progress {
            group: 0,
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
    use super::super::inspector::InspectorTab;
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
                    Workspace::new(
                        Some(root),
                        StateStore::none(),
                        test_secrets(),
                        super::super::workspace::test_backend(),
                        window,
                        cx,
                    )
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

    use super::super::workspace::wait_for_run;

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
    fn failed_run_shows_checks_and_timeline(cx: &mut TestAppContext) {
        let calls = Arc::new(Mutex::new(0));
        let count = calls.clone();
        let srv = TestServer::with_handler(move |_| {
            let mut n = count.lock().unwrap();
            *n += 1;
            if *n < 3 {
                Response::from_data(b"{}".to_vec()).with_status_code(503)
            } else {
                Response::from_data(br#"{"id":"ch_1","amount":2000,"status":"pending"}"#.to_vec())
                    .with_header("Content-Type", "application/json")
            }
        });
        let yaml = format!(
            "request:\n  url: {}/charge\n  retry:\n    count: 2\n    delay: 1\n  expect:\n    status: 200\n    body:\n      id: \"^ch_\"\n      amount: 2500\n      status: succeeded\n",
            srv.base_url
        );
        let (_dir, window, ws) = open(cx, &[("charge.yaml", yaml)]);
        run_file(cx, window, &ws, "charge.yaml");
        // Failed checks: the inspector opened on them.
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.inspector_tab, InspectorTab::Checks)
        });
        assert!(inspector_has(cx, window, "2 of 4 checks failed"));
        assert!(inspector_has(cx, window, "body.amount mismatch"));
        assert!(inspector_has(cx, window, "Attempt 3"));
        assert!(inspector_has(
            cx,
            window,
            "Response received · 2 checks failed — no retries left"
        ));
        assert!(inspector_has(cx, window, "Copy as curl"));
        assert!(inspector_has(cx, window, "Re-run"));
        ws.update(cx, |ws, cx| ws.copy_as_curl(cx));
        assert!(inspector_has(cx, window, "Copied"));
        let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
        assert!(copied.starts_with("curl "), "{copied}");
        cx.executor().advance_clock(Duration::from_millis(1600));
        cx.run_until_parked();
        assert!(inspector_has(cx, window, "Copy as curl"));
        ws.read_with(cx, |ws, _| {
            let hints: Vec<&str> = ws.tabs[0]
                .results
                .hints
                .iter()
                .map(|h| h.text.as_str())
                .collect();
            assert_eq!(hints, ["got 2000", "got \"pending\""]);
        });
    }

    #[gpui_kit::test]
    fn failed_checks_keep_a_tab_the_user_picked(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let bad = format!(
            "request:\n  url: {}/status/500\n  expect:\n    status: 200\n",
            srv.base_url
        );
        let ok = format!(
            "request:\n  url: {}/status/200\n  expect:\n    status: 200\n",
            srv.base_url
        );
        let (_dir, window, ws) = open(cx, &[("bad.yaml", bad), ("ok.yaml", ok)]);
        run_file(cx, window, &ws, "ok.yaml");
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.inspector_tab, InspectorTab::Response)
        });
        ws.update(cx, |ws, _| ws.inspector_tab = InspectorTab::Headers);
        run_file(cx, window, &ws, "bad.yaml");
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.inspector_tab, InspectorTab::Headers)
        });
    }

    fn hint_lines(cx: &mut TestAppContext, ws: &Entity<Workspace>, rel: &str) -> Vec<u32> {
        ws.read_with(cx, |ws, _| {
            let ix = ws.state.position(Path::new(rel)).expect("open");
            ws.tabs[ix].results.hints.iter().map(|h| h.line).collect()
        })
    }

    fn failing(base: &str) -> String {
        format!("request:\n  url: {base}/status/500\n  expect:\n    status: 200\n")
    }

    #[gpui_kit::test]
    fn an_edited_hint_stays_gone_after_other_runs_and_reopening(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let ok = format!("request:\n  url: {}/status/200\n", srv.base_url);
        let (_dir, window, ws) = open(cx, &[("a.yaml", failing(&srv.base_url)), ("b.yaml", ok)]);
        run_file(cx, window, &ws, "a.yaml");
        assert_eq!(hint_lines(cx, &ws, "a.yaml"), [3]);
        // Type over the hinted line, then save.
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let text = ws.state.open[0].text.clone();
                let at = text.find("200").unwrap();
                ws.tabs[0].editor.update(cx, |e, cx| {
                    e.set_selected_range(at..at + 3, cx);
                    e.replace("500", window, cx);
                });
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(hint_lines(cx, &ws, "a.yaml").is_empty(), "edited away");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| assert!(ws.save(0, window, cx)));
        })
        .unwrap();
        run_file(cx, window, &ws, "b.yaml");
        assert!(hint_lines(cx, &ws, "a.yaml").is_empty(), "another run");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.request_close_tab(0, window, cx);
                ws.open_file(Path::new("a.yaml"), window, cx);
            });
        })
        .unwrap();
        assert!(hint_lines(cx, &ws, "a.yaml").is_empty(), "reopened");
    }

    #[gpui_kit::test]
    fn a_reload_moves_hints_with_their_line(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let text = failing(&srv.base_url);
        let (dir, window, ws) = open(cx, &[("a.yaml", text.clone())]);
        run_file(cx, window, &ws, "a.yaml");
        assert_eq!(hint_lines(cx, &ws, "a.yaml"), [3]);
        std::fs::write(dir.path().join("a.yaml"), format!("# added\n{text}")).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.disk_changed(&["a.yaml".into()], window, cx));
        })
        .unwrap();
        assert_eq!(hint_lines(cx, &ws, "a.yaml"), [4]);
        let changed = format!("# added\n{}", text.replace("200", "201"));
        std::fs::write(dir.path().join("a.yaml"), changed).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.disk_changed(&["a.yaml".into()], window, cx));
        })
        .unwrap();
        assert!(hint_lines(cx, &ws, "a.yaml").is_empty());
    }

    #[gpui_kit::test]
    fn a_quick_fix_above_moves_the_hint_with_its_line(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        // `extra` is an unknown top-level key: a warning (the file still
        // runs) whose fix deletes its line.
        let text = format!("extra: 1\n{}", failing(&srv.base_url));
        let (_dir, window, ws) = open(cx, &[("a.yaml", text)]);
        run_file(cx, window, &ws, "a.yaml");
        assert_eq!(hint_lines(cx, &ws, "a.yaml"), [4]);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.tabs[0].editor.update(cx, |e, cx| {
                    e.set_cursor_position(
                        // On the warning's squiggle (the value).
                        gpui_kit::component::input::Position::new(0, 7),
                        window,
                        cx,
                    )
                });
            });
            window.render_frame(cx);
            window.dispatch_action(super::super::app::QuickFix.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            assert!(
                !ws.state.open[0].text.contains("extra"),
                "the fix ran: {:?}",
                ws.state.open[0].diagnostics
            );
        });
        assert_eq!(hint_lines(cx, &ws, "a.yaml"), [3], "the hint followed");
        // Typing the line back in above: the hint moves down with it.
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.tabs[0].editor.update(cx, |e, cx| {
                    e.set_cursor_position(
                        gpui_kit::component::input::Position::new(0, 0),
                        window,
                        cx,
                    );
                    e.replace("extra: 1\n", window, cx);
                });
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(hint_lines(cx, &ws, "a.yaml"), [4], "an inserted line above");
    }

    fn tree_labels(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> usize {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("collection-tree")
                .find_all_by_label(label)
                .len()
        })
        .unwrap()
    }

    #[gpui_kit::test]
    fn tree_dots_show_the_last_run(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let ok = format!(
            "request:\n  url: {}/status/200\n  expect:\n    status: 200\n",
            srv.base_url
        );
        let bad = format!(
            "request:\n  url: {}/status/500\n  expect:\n    status: 200\n",
            srv.base_url
        );
        let (_dir, window, ws) = open(cx, &[("bad.yaml", bad), ("ok.yaml", ok)]);
        assert_eq!(tree_labels(cx, window, "Not run"), 2);
        run_file(cx, window, &ws, "ok.yaml");
        assert_eq!(tree_labels(cx, window, "Passed"), 1);
        assert_eq!(tree_labels(cx, window, "Not run"), 1);
        run_file(cx, window, &ws, "bad.yaml");
        assert_eq!(tree_labels(cx, window, "Failed"), 1);
        assert_eq!(tree_labels(cx, window, "Passed"), 1);
    }

    #[gpui_kit::test]
    fn run_is_ignored_while_a_run_is_active(cx: &mut TestAppContext) {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = TestServer::with_handler(move |req| {
            log.lock().unwrap().push(req.url.clone());
            std::thread::sleep(Duration::from_millis(400));
            Response::from_data(b"{}".to_vec())
        });
        let yaml = format!("request:\n  url: {}/slow\n", srv.base_url);
        let (_dir, window, ws) = open(cx, &[("slow.yaml", yaml)]);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("slow.yaml"), window, cx);
                ws.run_active(window, cx);
            });
        })
        .unwrap();
        // The first request is in flight before Run is pressed again.
        let deadline = Instant::now() + Duration::from_secs(5);
        while seen.lock().unwrap().is_empty() {
            assert!(Instant::now() < deadline, "the run did not start");
            std::thread::sleep(Duration::from_millis(5));
        }
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.run_active(window, cx);
                ws.run_request_under_cursor(window, cx);
                ws.run(crate::model::run::RunTarget::Collection, window, cx);
            });
        })
        .unwrap();
        wait_for_run(cx, &ws);
        assert_eq!(*seen.lock().unwrap(), ["/slow"], "one run, not restarted");
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
