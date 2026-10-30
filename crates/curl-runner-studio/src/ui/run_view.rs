//! The collection run screen (board: RunCollection): header with Stop,
//! Re-run failed and Export, the progress bar, stat cards and the results
//! table grouped by top-level folder. It takes the main area and the
//! inspector's column while it is open.
//!
//! The workspace feeds it the run's events (never `Stored`, which is
//! unredacted) and acts on what it emits.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::sidebar::method_badge;
use super::workspace::notify_error;
use crate::model::duration_text;
use crate::model::environments::EnvCtx;
use crate::model::export::{cli_command, to_json, to_junit, to_markdown};
use crate::model::result_view::checks;
use crate::model::run::{
    FolderGroup, RowState, RunConfig, RunEvent, RunOutcome, RunTarget, eta_ms, group_rows,
};
use chrono::{DateTime, Local};
use gpui_kit::component::Sizable as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often Elapsed and the ETA refresh while the run is going.
const TICK: Duration = Duration::from_secs(1);
/// Above this many requests the progress bar is drawn as proportions.
const MAX_SEGMENTS: usize = 120;
/// The longest duration's bar fills this much of its track (the board).
const BAR_SCALE: f32 = 0.94;
const ROW_HEIGHT: f32 = 28.;
/// The method column: the widest badge (`DELETE`, `OPTIONS` drawn smaller,
/// as the tree's `badge_text_size` does; about 37px of 10px Geist Mono)
/// with room to spare, so `Method` and `Request` read apart in the header
/// and no badge reaches the name. Wider than the tree's 34px slot, which
/// lives in a 264px column.
const METHOD_COLUMN: f32 = 46.;

/// Each file's requests as `(method, name)`, in run order: what a row shows
/// before its result arrives.
pub type Labels = HashMap<PathBuf, Vec<(String, String)>>;

/// What a finished row shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowCell {
    pub method: String,
    pub status: Option<u16>,
    pub duration_ms: Option<f64>,
    /// Passed and total checks.
    pub checks: Option<(usize, usize)>,
    pub attempts: u32,
    /// A result arrived.
    pub done: bool,
}

/// The run's overall state, as the header's chip reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardStatus {
    Running,
    Passed,
    Failed,
    Stopped,
}

impl BoardStatus {
    fn label(self) -> &'static str {
        match self {
            BoardStatus::Running => "Running",
            BoardStatus::Passed => "Passed",
            BoardStatus::Failed => "Failed",
            BoardStatus::Stopped => "Stopped",
        }
    }
}

/// The run's rows and counts, built from its events. No GPUI.
#[derive(Debug)]
pub struct Board {
    pub groups: Vec<(PathBuf, usize)>,
    /// The first row of each group.
    offsets: Vec<usize>,
    pub folders: Vec<FolderGroup>,
    /// One per request, in run order (groups, then index).
    pub rows: Vec<RowState>,
    pub cells: Vec<RowCell>,
    pub names: Vec<String>,
    pub total: usize,
    pub running: Option<usize>,
    running_since: Option<Instant>,
    /// A transfer happened for the row: a request that failed before
    /// sending (a typing error) has a result but no attempt.
    attempted: Vec<bool>,
    /// The Keychain secrets the run reads, names only.
    pub keychain_names: Vec<String>,
    pub config: RunConfig,
    /// Files that did not load, with the parser's message.
    pub load_errors: Vec<(PathBuf, String)>,
    pub status: BoardStatus,
    started: Instant,
    finished: Option<Instant>,
    labels: Labels,
}

impl Board {
    pub fn new(labels: Labels) -> Self {
        Board {
            groups: Vec::new(),
            offsets: Vec::new(),
            folders: Vec::new(),
            rows: Vec::new(),
            cells: Vec::new(),
            names: Vec::new(),
            total: 0,
            running: None,
            running_since: None,
            attempted: Vec::new(),
            keychain_names: Vec::new(),
            config: RunConfig::default(),
            load_errors: Vec::new(),
            status: BoardStatus::Running,
            started: Instant::now(),
            finished: None,
            labels,
        }
    }

    fn row(&self, group: usize, index: usize) -> Option<usize> {
        let at = self.offsets.get(group)? + index;
        (at < self.rows.len()).then_some(at)
    }

    pub fn apply(&mut self, event: &RunEvent) {
        match event {
            RunEvent::Started {
                total,
                groups,
                config,
                keychain_names,
            } => {
                self.total = *total;
                self.keychain_names = keychain_names.clone();
                self.groups = groups.clone();
                self.config = *config;
                let labels = &self.labels;
                let label = |file: &Path, index: usize| -> Option<&(String, String)> {
                    labels.get(file)?.get(index)
                };
                self.folders = group_rows(groups, |file, index| {
                    label(file, index).map_or_else(
                        || fallback_name(file, index, groups_len(groups, file)),
                        |(_, name)| name.clone(),
                    )
                });
                self.offsets.clear();
                self.rows.clear();
                self.cells.clear();
                self.names.clear();
                self.attempted.clear();
                for (file, n) in groups {
                    self.offsets.push(self.rows.len());
                    for index in 0..*n {
                        self.rows.push(RowState::Queued);
                        self.attempted.push(false);
                        let (method, name) = label(file, index)
                            .cloned()
                            .unwrap_or_else(|| ("GET".into(), fallback_name(file, index, *n)));
                        self.names.push(name);
                        self.cells.push(RowCell {
                            method,
                            ..RowCell::default()
                        });
                    }
                }
            }
            RunEvent::RequestStarted { group, index, .. } => {
                if let Some(row) = self.row(*group, *index) {
                    self.rows[row] = RowState::Running;
                    self.running = Some(row);
                    self.running_since = Some(Instant::now());
                }
            }
            RunEvent::Attempt {
                group,
                index,
                attempt,
                ..
            } => {
                if let Some(row) = self.row(*group, *index) {
                    self.cells[row].attempts = self.cells[row].attempts.max(*attempt);
                    self.attempted[row] = true;
                }
            }
            RunEvent::Result {
                group,
                index,
                result,
            } => {
                let Some(row) = self.row(*group, *index) else {
                    return;
                };
                let skipped = result.skipped == Some(true);
                self.rows[row] = if skipped {
                    RowState::Skipped
                } else if result.success {
                    RowState::Passed
                } else {
                    RowState::Failed
                };
                let all = checks(result);
                let cell = &mut self.cells[row];
                if let Some(m) = &result.request.method {
                    cell.method = m.to_uppercase();
                }
                cell.status = result.status;
                // Never sent (no attempt): no time and no attempts to show,
                // whatever the result's metrics say. A transport error did
                // send, and keeps both.
                let sent = self.attempted[row];
                cell.duration_ms = result.metrics.as_ref().map(|m| m.duration).filter(|_| sent);
                cell.checks =
                    (!all.is_empty()).then(|| (all.iter().filter(|c| c.passed).count(), all.len()));

                cell.done = true;
                if self.running == Some(row) {
                    self.running = None;
                    self.running_since = None;
                }
            }
            RunEvent::LoadFailed { file, message } => {
                self.load_errors.push((file.clone(), message.clone()));
            }
            RunEvent::Finished(o) => {
                for (row, state) in self.rows.iter_mut().enumerate() {
                    if matches!(state, RowState::Queued | RowState::Running)
                        && !self.cells[row].done
                    {
                        *state = RowState::Skipped;
                    }
                }
                self.running = None;
                self.running_since = None;
                self.finished = Some(Instant::now());
                self.status = if o.stopped {
                    BoardStatus::Stopped
                } else if self.failed() > 0 {
                    BoardStatus::Failed
                } else {
                    BoardStatus::Passed
                };
            }
            RunEvent::Command { .. } | RunEvent::RetryWait { .. } | RunEvent::Stored { .. } => {}
        }
    }

    fn count(&self, state: RowState) -> usize {
        self.rows.iter().filter(|s| **s == state).count()
    }

    pub fn passed(&self) -> usize {
        self.count(RowState::Passed)
    }

    /// Failed rows and files that did not load.
    pub fn failed(&self) -> usize {
        self.count(RowState::Failed) + self.load_errors.len()
    }

    pub fn queued(&self) -> usize {
        self.count(RowState::Queued)
    }

    pub fn skipped(&self) -> usize {
        self.count(RowState::Skipped)
    }

    /// Requests with a result.
    pub fn done(&self) -> usize {
        self.cells.iter().filter(|c| c.done).count()
    }

    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    fn durations(&self) -> Vec<f64> {
        self.cells.iter().filter_map(|c| c.duration_ms).collect()
    }

    pub fn eta_ms(&self) -> Option<f64> {
        eta_ms(&self.durations(), self.total.saturating_sub(self.done()))
    }

    pub fn average_ms(&self) -> Option<f64> {
        let d = self.durations();
        (!d.is_empty()).then(|| d.iter().sum::<f64>() / d.len() as f64)
    }

    pub fn elapsed(&self) -> Duration {
        self.finished.unwrap_or_else(Instant::now) - self.started
    }

    /// The running request's time so far.
    fn running_ms(&self) -> Option<f64> {
        Some(self.running_since?.elapsed().as_secs_f64() * 1000.)
    }

    /// Retries made, and by how many requests.
    pub fn retries(&self) -> (u32, usize) {
        self.cells
            .iter()
            .filter(|c| c.attempts > 1)
            .fold((0, 0), |(n, rows), c| (n + c.attempts - 1, rows + 1))
    }

    pub fn failed_names(&self) -> Vec<String> {
        let rows = (0..self.rows.len())
            .filter(|&r| self.rows[r] == RowState::Failed)
            .map(|r| self.names[r].clone());
        let files = self.load_errors.iter().map(|(f, _)| label_of(f));
        rows.chain(files).collect()
    }

    /// The files with a failure, in run order: what Re-run failed runs.
    pub fn failed_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        for (g, (file, n)) in self.groups.iter().enumerate() {
            let start = self.offsets[g];
            if self.rows[start..start + n].contains(&RowState::Failed) {
                files.push(file.clone());
            }
        }
        for (file, _) in &self.load_errors {
            if !file.as_os_str().is_empty() && !files.contains(file) {
                files.push(file.clone());
            }
        }
        files
    }

    /// Each group's file and its result count: how the run's results split
    /// for the JUnit suites (a group stopped at a failure has fewer).
    pub fn result_groups(&self) -> Vec<(PathBuf, usize)> {
        self.groups
            .iter()
            .enumerate()
            .map(|(g, (file, n))| {
                let start = self.offsets[g];
                let done = self.cells[start..start + n]
                    .iter()
                    .filter(|c| c.done)
                    .count();
                (file.clone(), done)
            })
            .collect()
    }

    /// `14 of 22 requests · Running webhooks/send-test-event`: the running
    /// request's file without its extension, and its name in a file of many.
    pub fn running_label(&self) -> Option<String> {
        let row = self.running?;
        let g = self.offsets.iter().rposition(|&o| o <= row)?;
        let (file, n) = &self.groups[g];
        let path = file.with_extension("").to_string_lossy().into_owned();
        Some(if *n > 1 {
            format!("{path} · {}", self.names[row])
        } else {
            path
        })
    }

    /// The folder's rows: `(row, state)`.
    fn folder_rows<'a>(&'a self, f: &'a FolderGroup) -> impl Iterator<Item = usize> + 'a {
        f.rows.iter().filter_map(|r| self.row(r.group, r.index))
    }

    /// Open unless every row passed or every row is still queued (the
    /// board's folded `auth` and `payouts`).
    fn open_by_default(&self, f: &FolderGroup) -> bool {
        let states: Vec<RowState> = self.folder_rows(f).map(|r| self.rows[r]).collect();
        !(states.iter().all(|s| *s == RowState::Passed)
            || states.iter().all(|s| *s == RowState::Queued))
    }
}

/// The request count of `file` in `groups`.
fn groups_len(groups: &[(PathBuf, usize)], file: &Path) -> usize {
    groups
        .iter()
        .find(|(f, _)| f == file)
        .map_or(0, |(_, n)| *n)
}

/// A file's label: its name without the YAML extension.
fn label_of(file: &Path) -> String {
    let name = file
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    super::sidebar::request_label(&name).to_string()
}

/// A row's name when the tree does not know the file: the file's label, or
/// `Request n` in a file of many.
fn fallback_name(file: &Path, index: usize, count: usize) -> String {
    if count > 1 {
        format!("Request {}", index + 1)
    } else {
        label_of(file)
    }
}

/// `~6 s left`, rounded up; minutes past one.
pub fn eta_text(ms: f64) -> String {
    let s = (ms / 1000.).ceil().max(1.) as u64;
    if s < 60 {
        format!("~{s} s left")
    } else {
        format!("~{} min left", s.div_ceil(60))
    }
}

/// A folder header's summary: `4 passed · 1 failed`, `1 passed · 1
/// running · 2 queued`.
pub fn folder_summary(states: &[RowState]) -> String {
    let n = |s: RowState| states.iter().filter(|x| **x == s).count();
    [
        (n(RowState::Passed), "passed"),
        (n(RowState::Failed), "failed"),
        (n(RowState::Running), "running"),
        (n(RowState::Queued), "queued"),
        (n(RowState::Skipped), "skipped"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, word)| format!("{count} {word}"))
    .collect::<Vec<_>>()
    .join(" · ")
}

/// A report format of the Export menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Junit,
    Json,
    Markdown,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Junit => "xml",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "md",
        }
    }

    fn label(self) -> &'static str {
        match self {
            ExportFormat::Junit => "JUnit XML",
            ExportFormat::Json => "JSON",
            ExportFormat::Markdown => "Markdown summary",
        }
    }
}

/// `{collection}-{YYYYMMDD-HHMMSS}.{ext}`, with no path separators.
pub fn report_file_name(collection: &str, at: DateTime<Local>, f: ExportFormat) -> String {
    let name: String = collection
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') {
                '-'
            } else {
                c
            }
        })
        .collect();
    format!(
        "{}-{}.{}",
        name.trim(),
        at.format("%Y%m%d-%H%M%S"),
        f.extension()
    )
}

/// The report text, redacted with the run's own redactor.
pub fn report(
    f: ExportFormat,
    o: &RunOutcome,
    groups: &[(PathBuf, usize)],
    collection: &str,
    env: &str,
) -> String {
    let r = o.redactor.lock();
    match f {
        ExportFormat::Junit => to_junit(&o.summary, groups, &r),
        ExportFormat::Json => to_json(&o.summary, &r),
        ExportFormat::Markdown => to_markdown(&o.summary, &format!("Run · {collection}"), env, &r),
    }
}

pub use crate::model::write_atomic;

/// What the run view asks of the workspace.
#[derive(Debug, Clone, PartialEq)]
pub enum RunViewEvent {
    /// Open the file with the inspector on this request's result.
    Open {
        file: PathBuf,
        index: usize,
    },
    Stop,
    RerunFailed(Vec<PathBuf>),
    /// Back to the editor; the run goes on.
    Close,
}

pub struct RunView {
    pub board: Board,
    collection: String,
    ctx: EnvCtx,
    target: RunTarget,
    started_at: DateTime<Local>,
    outcome: Option<RunOutcome>,
    export_open: bool,
    /// When a press outside last closed the menu: the same press landing on
    /// Export report must not open it again.
    export_closed_at: Option<Instant>,
    /// Folders the user opened or closed, by name.
    toggled: HashMap<String, bool>,
    selected: Option<usize>,
    copied: Option<Task<()>>,
    focus: FocusHandle,
    _tick: Task<()>,
}

impl EventEmitter<RunViewEvent> for RunView {}

impl Focusable for RunView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl RunView {
    pub fn new(
        collection: String,
        ctx: EnvCtx,
        target: RunTarget,
        labels: Labels,
        cx: &mut Context<Self>,
    ) -> Self {
        // Elapsed and the ETA move once a second, not every frame.
        let tick = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let going = this
                    .update(cx, |this, cx| {
                        cx.notify();
                        !this.board.is_finished()
                    })
                    .unwrap_or(false);
                if !going {
                    break;
                }
            }
        });
        RunView {
            board: Board::new(labels),
            collection,
            ctx,
            target,
            started_at: Local::now(),
            outcome: None,
            export_open: false,
            export_closed_at: None,
            toggled: HashMap::new(),
            selected: None,
            copied: None,
            focus: cx.focus_handle(),
            _tick: tick,
        }
    }

    /// One event of the run (the workspace never passes `Stored`).
    pub fn apply(&mut self, event: &RunEvent, cx: &mut Context<Self>) {
        self.board.apply(event);
        if let RunEvent::Finished(o) = event {
            self.started_at = o.started_at;
            self.outcome = Some(o.clone());
        }
        cx.notify();
    }

    /// The environment the run used.
    pub fn environment(&self) -> &str {
        &self.ctx.environment
    }

    pub fn target(&self) -> &RunTarget {
        &self.target
    }

    /// The status bar's `Running 14/22`, `Failed 2 of 22`.
    pub fn status_label(&self) -> String {
        let b = &self.board;
        let total = b.total.max(b.done());
        match b.status {
            BoardStatus::Running => format!("Running {}/{}", b.done(), b.total),
            BoardStatus::Passed => format!("Passed {}/{total}", b.passed()),
            BoardStatus::Failed => format!("Failed {} of {total}", b.failed()),
            BoardStatus::Stopped => format!("Stopped {}/{total}", b.done()),
        }
    }

    /// The status bar's `Concurrency 1 · stop on failure off`.
    pub fn config_label(&self) -> String {
        let c = self.board.config;
        let concurrency = c
            .concurrency
            .map_or_else(|| "unlimited".to_string(), |n| n.to_string());
        let stop = if c.continue_on_error { "off" } else { "on" };
        format!("Concurrency {concurrency} · stop on failure {stop}")
    }

    fn env_label(&self) -> String {
        env_label(&self.ctx.environment)
    }

    /// Saves the finished run's report through the save panel.
    pub fn export(&mut self, f: ExportFormat, window: &mut Window, cx: &mut Context<Self>) {
        self.export_open = false;
        cx.notify();
        let Some(o) = &self.outcome else { return };
        let text = report(
            f,
            o,
            &self.board.result_groups(),
            &self.collection,
            &self.env_label(),
        );
        let name = report_file_name(&self.collection, o.started_at, f);
        let picked = cx.prompt_for_new_path(&self.ctx.root, Some(&name));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = picked.await else {
                return;
            };
            if let Err(e) = write_atomic(&path, &text) {
                this.update_in(cx, |_, window, cx| {
                    notify_error(format!("Couldn't save {}: {e}", path.display()), window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// Copies the `curl-runner` command that repeats this run.
    pub fn copy_cli(&mut self, cx: &mut Context<Self>) {
        self.export_open = false;
        let names = self.board.keychain_names.clone();
        let command = cli_command(&self.target, &self.ctx, &names);
        cx.write_to_clipboard(ClipboardItem::new_string(command));
        self.copied = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1500))
                .await;
            this.update(cx, |this, cx| {
                this.copied = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Dev hook (`CURL_RUNNER_STUDIO_SHOW=export`): the Export menu open.
    pub fn open_export_menu(&mut self, cx: &mut Context<Self>) {
        self.export_open = true;
        cx.notify();
    }

    fn is_open(&self, f: &FolderGroup) -> bool {
        self.toggled
            .get(&f.folder)
            .copied()
            .unwrap_or_else(|| self.board.open_by_default(f))
    }

    fn toggle(&mut self, folder: &str, cx: &mut Context<Self>) {
        let open = self
            .board
            .folders
            .iter()
            .find(|f| f.folder == folder)
            .is_some_and(|f| self.is_open(f));
        self.toggled.insert(folder.to_string(), !open);
        cx.notify();
    }
}

impl Render for RunView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("run-view")
            .key_context("RunView")
            .track_focus(&self.focus)
            .role(Role::Main)
            .test_support()
            .aria_label("Collection run")
            .on_action(
                // esc closes the Export menu first, then the screen.
                cx.listener(|this, _: &super::app::CloseRun, _, cx| {
                    if this.export_open {
                        this.export_open = false;
                        cx.notify();
                    } else {
                        cx.emit(RunViewEvent::Close);
                    }
                }),
            )
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(color("bg-100"))
            .child(self.render_header(cx))
            .child(self.render_progress())
            .child(self.render_cards())
            .child(self.render_table(cx))
    }
}

/// A 32px outlined button of the header.
fn header_button(id: &'static str, enabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(32.))
        .px(px(12.))
        .border_1()
        .border_color(color("border-200"))
        .rounded(px(6.))
        .bg(color("bg-100"))
        .text_size(px(13.))
        .whitespace_nowrap()
        .map(|b| {
            if enabled {
                b.text_color(color("text-primary"))
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-100")))
            } else {
                b.text_color(color("text-disabled"))
            }
        })
}

fn key_chip(keys: &'static str) -> Div {
    div()
        .flex_none()
        .px(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(color("border-200"))
        .font_family(mono_font())
        .text_size(px(11.))
        .line_height(px(16.))
        .text_color(color("text-secondary"))
        .child(keys)
}

/// The board's 12px pill: `(text, background, border)` tokens.
pub(super) fn pill(tokens: (&str, &str, &str)) -> Div {
    let (fg, bg, border) = tokens;
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .px(px(8.))
        .py(px(2.))
        .rounded_full()
        .border_1()
        .border_color(color(border))
        .when(!bg.is_empty(), |p| p.bg(color(bg)))
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(fg))
        .whitespace_nowrap()
}

/// `Exports when the run finishes. Same format as curl-runner --output.`,
/// the flag in mono, wrapping as one text.
fn export_note() -> StyledText {
    const HEAD: &str = "Exports when the run finishes. Same format as ";
    const FLAG: &str = "curl-runner --output";
    let run = |len: usize, family: SharedString| TextRun {
        len,
        font: font(family),
        color: color("text-tertiary"),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    StyledText::new(format!("{HEAD}{FLAG}.")).with_runs(vec![
        run(HEAD.len(), super::app::ui_font()),
        run(FLAG.len(), mono_font()),
        run(1, super::app::ui_font()),
    ])
}

fn spinner(token: &str) -> impl IntoElement {
    Spinner::new().with_size(px(11.)).color(color(token))
}

impl RunView {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let b = &self.board;
        let status = b.status;
        let chip = match status {
            BoardStatus::Running => {
                pill(("accent-blue", "info-subtle", "info-border")).child(spinner("accent-blue"))
            }
            BoardStatus::Passed => pill(("success", "success-subtle", "success-border")),
            BoardStatus::Failed => pill(("danger", "danger-subtle", "danger-border")),
            BoardStatus::Stopped => pill(("text-secondary", "", "border-200")),
        };
        // The spinner goes before the label, as on the board.
        let chip = chip
            .child(status.label())
            .id("run-status")
            .role(Role::Label)
            .test_support()
            .aria_label(format!("Run {}", status.label().to_lowercase()));
        let finished = b.is_finished();
        let failures = finished && !b.failed_files().is_empty();
        let stop_or_close = if finished {
            header_button("run-close", true)
                .role(Role::Button)
                .test_support()
                .aria_label("Close")
                .child("Close")
                .child(key_chip("esc"))
                .on_click(cx.listener(|_, _, _, cx| cx.emit(RunViewEvent::Close)))
        } else {
            header_button("run-stop", true)
                .role(Role::Button)
                .test_support()
                .aria_label("Stop")
                .child(
                    div()
                        .size(px(10.))
                        .rounded(px(1.5))
                        .bg(color("text-primary")),
                )
                .child("Stop")
                .child(key_chip("⌘."))
                .on_click(cx.listener(|_, _, _, cx| cx.emit(RunViewEvent::Stop)))
        };
        let rerun = header_button("run-rerun-failed", failures)
            .role(Role::Button)
            .test_support()
            .aria_label("Re-run failed")
            .child(glyph(icons::RERUN).size(px(13.)).flex_none())
            .child("Re-run failed")
            .when(failures, |b| {
                b.on_click(cx.listener(|this, _, _, cx| {
                    let files = this.board.failed_files();
                    cx.emit(RunViewEvent::RerunFailed(files));
                }))
            });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.))
            .h(px(56.))
            .px(px(20.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(format!("Run · {}", self.collection)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(format!(
                                "Started {} · local · curl-runner {}",
                                self.started_at.format("%H:%M:%S"),
                                curl_runner_core::VERSION
                            )),
                    ),
            )
            .child(pill(("text-secondary", "", "border-200")).child(self.env_label()))
            .child(chip)
            .child(div().flex_1())
            .child(stop_or_close)
            .child(rerun)
            .child(self.render_export(cx))
    }

    fn render_export(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let open = self.export_open;
        let ready = self.outcome.is_some();
        let button = header_button("run-export", true)
            .role(Role::Button)
            .test_support()
            .aria_label("Export report")
            .when(open, |b| b.bg(color("surface-300")))
            .child(glyph(icons::DOWNLOAD).size(px(13.)).flex_none())
            .child("Export report")
            .child(
                glyph(icons::CHEVRON_DOWN)
                    .size(px(12.))
                    .text_color(color("text-secondary")),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                let just_closed = this
                    .export_closed_at
                    .is_some_and(|at| at.elapsed() < Duration::from_millis(300));
                this.export_open = !this.export_open && !just_closed;
                this.export_closed_at = None;
                cx.notify();
            }));
        let item = |id: &'static str, f: ExportFormat, cx: &mut Context<Self>| {
            div()
                .id(id)
                .role(Role::MenuItem)
                .test_support()
                .aria_label(f.label())
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(32.))
                .px(px(10.))
                .rounded(px(5.))
                .whitespace_nowrap()
                .map(|d| {
                    if ready {
                        d.text_color(color("text-primary"))
                            .cursor_pointer()
                            .hover(|s| s.bg(color("surface-300")))
                            .on_click(
                                cx.listener(move |this, _, window, cx| this.export(f, window, cx)),
                            )
                    } else {
                        d.text_color(color("text-disabled"))
                    }
                })
                .child(div().flex_1().child(f.label()))
                .child(
                    div()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(format!(".{}", f.extension())),
                )
        };
        let copied = self.copied.is_some();
        let menu = div()
            .id("run-export-menu")
            // The menu is outside the anchor's bounds: a press counts as
            // outside only when it misses the menu.
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.export_open = false;
                this.export_closed_at = Some(Instant::now());
                cx.notify();
            }))
            .role(Role::Menu)
            .test_support()
            .occlude()
            .absolute()
            .top(px(38.))
            .right_0()
            .w(px(232.))
            .p(px(4.))
            .flex()
            .flex_col()
            .bg(color("surface-raised"))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(8.))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.6),
                offset: point(px(0.), px(12.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }])
            .text_size(px(13.))
            .child(item("export-junit", ExportFormat::Junit, cx))
            .child(item("export-json", ExportFormat::Json, cx))
            .child(item("export-md", ExportFormat::Markdown, cx))
            .child(div().h(px(1.)).my(px(4.)).bg(color("border-100")))
            .child(
                div()
                    .id("export-cli")
                    .role(Role::MenuItem)
                    .test_support()
                    .aria_label("Copy CLI command")
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(5.))
                    .text_color(color("text-secondary"))
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-300")))
                    .child(glyph(icons::COPY).size(px(13.)).flex_none())
                    .child(
                        div()
                            .flex_1()
                            .child(if copied { "Copied" } else { "Copy CLI command" }),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.copy_cli(cx))),
            )
            .child(
                div()
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .line_height(relative(1.4))
                    .text_color(color("text-tertiary"))
                    .child(export_note()),
            );
        div()
            .id("run-export-anchor")
            .relative()
            .flex()
            .flex_none()
            .child(button)
            .when(open, |d| d.child(deferred(menu).with_priority(1)))
    }

    fn render_progress(&self) -> impl IntoElement + use<> {
        let b = &self.board;
        let right = if b.is_finished() {
            let verb = if b.status == BoardStatus::Stopped {
                "Stopped after"
            } else {
                "Finished in"
            };
            Some(format!(
                "{verb} {}",
                duration_text(b.elapsed().as_secs_f64() * 1000.)
            ))
        } else {
            b.eta_ms().map(eta_text)
        };
        let color_of = |s: RowState| match s {
            RowState::Passed => color("success"),
            RowState::Failed => color("danger"),
            RowState::Running => color("accent-blue"),
            RowState::Skipped => color("border-200"),
            RowState::Queued => color("surface-300"),
        };
        let bar = div()
            .id("run-progress")
            .role(Role::ProgressIndicator)
            .test_support()
            .aria_label(format!("{} of {}", b.done(), b.total))
            .flex()
            .gap(px(2.))
            .h(px(6.))
            .rounded(px(3.))
            .overflow_hidden();
        let bar = if b.rows.len() <= MAX_SEGMENTS {
            bar.children(
                b.rows
                    .iter()
                    .map(|s| div().flex_1().h_full().bg(color_of(*s))),
            )
        } else {
            // Too many to tell apart: one span per state, in proportion.
            let n = b.rows.len() as f32;
            bar.children(
                [
                    RowState::Passed,
                    RowState::Failed,
                    RowState::Skipped,
                    RowState::Running,
                ]
                .into_iter()
                .map(|s| (s, b.count(s)))
                .filter(|(_, c)| *c > 0)
                .map(|(s, c)| div().h_full().w(relative(c as f32 / n)).bg(color_of(s))),
            )
        };
        let mut line = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .whitespace_nowrap()
            .child(
                div()
                    .text_color(color("text-primary"))
                    .font_weight(FontWeight::MEDIUM)
                    .child(format!("{} of {}", b.done(), b.total)),
            )
            .child("requests");
        if let Some(running) = b.running_label() {
            line = line
                .child(div().text_color(color("text-disabled")).child("·"))
                .child("Running")
                .child(
                    div()
                        .min_w_0()
                        .text_ellipsis()
                        .font_family(mono_font())
                        .text_color(color("text-primary"))
                        .child(running),
                );
        }
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(8.))
            .pt(px(14.))
            .px(px(20.))
            .child(
                line.child(div().flex_1())
                    .children(right.map(|r| div().flex_none().child(r))),
            )
            .child(bar)
    }

    fn render_cards(&self) -> impl IntoElement + use<> {
        let b = &self.board;
        let finished = b.is_finished();
        let card = |label: &'static str, value: String, value_color: &str, sub: String| {
            div()
                .id(label)
                .role(Role::Group)
                .test_support()
                .aria_label(format!("{label} {value}"))
                .flex()
                .flex_col()
                .flex_1()
                .flex_basis(px(0.))
                .min_w_0()
                .gap(px(4.))
                .py(px(12.))
                .px(px(14.))
                .border_1()
                .border_color(color("border-100"))
                .rounded(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color("text-secondary"))
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(22.))
                        .line_height(px(28.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(value_color))
                        .child(value),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(color("text-tertiary"))
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(sub),
                )
        };
        let failed = b.failed();
        let failed_sub = match b.failed_names() {
            names if names.is_empty() => "none".to_string(),
            names => names.join(" · "),
        };
        let (retries, retried) = b.retries();
        let (third_label, third_value, third_sub) = if finished {
            let skipped = b.skipped();
            let why = match b.status {
                BoardStatus::Stopped => "stopped early",
                _ if skipped > 0 => "not run",
                _ => "none left",
            };
            ("Skipped", skipped, why.to_string())
        } else {
            let running = usize::from(b.running.is_some());
            ("Queued", b.queued(), format!("{running} running now"))
        };
        let elapsed = duration_text(b.elapsed().as_secs_f64() * 1000.);
        let avg = b.average_ms().map_or_else(
            || "no requests yet".to_string(),
            |a| format!("avg {} per request", duration_text(a)),
        );
        div()
            .flex()
            .flex_none()
            .gap(px(12.))
            .py(px(14.))
            .px(px(20.))
            .child(card(
                "Passed",
                b.passed().to_string(),
                "success",
                format!("of {} finished", b.done() + b.load_errors.len()),
            ))
            .child(card(
                "Failed",
                failed.to_string(),
                if failed > 0 { "danger" } else { "text-primary" },
                failed_sub,
            ))
            .child(card(
                third_label,
                third_value.to_string(),
                "text-primary",
                third_sub,
            ))
            .child(card(
                "Retries",
                retries.to_string(),
                if retries > 0 {
                    "warning"
                } else {
                    "text-primary"
                },
                format!(
                    "across {retried} {}",
                    if retried == 1 { "request" } else { "requests" }
                ),
            ))
            .child(card("Elapsed", elapsed, "text-primary", avg))
    }

    fn render_table(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let b = &self.board;
        let th = |text: &'static str| {
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color("text-tertiary"))
                .whitespace_nowrap()
                .child(text)
        };
        let head = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.))
            .h(px(32.))
            .pl(px(36.))
            .pr(px(16.))
            .border_b_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .child(div().w(px(16.)).flex_none())
            .child(th("Method").w(px(METHOD_COLUMN)).flex_none())
            .child(th("Request").flex_1())
            .child(th("Status").w(px(56.)).flex_none())
            .child(th("Duration").w(px(150.)).flex_none())
            .child(th("Checks").w(px(64.)).flex_none())
            .child(th("Attempts").w(px(64.)).flex_none());
        let max_ms = b
            .durations()
            .into_iter()
            .chain(b.running_ms())
            .fold(0f64, f64::max)
            .max(1.);
        let mut body = div()
            .id("run-table")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        for (i, (file, message)) in b.load_errors.iter().enumerate() {
            body = body.child(render_load_error_row(i, file, message));
        }
        for f in &b.folders {
            let open = self.is_open(f);
            body = body.child(self.render_folder(f, open, cx));
            if open {
                for r in &f.rows {
                    if let Some(row) = b.row(r.group, r.index) {
                        body =
                            body.child(self.render_row(row, r.file.clone(), r.index, max_ms, cx));
                    }
                }
            }
        }
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .mx(px(20.))
            .mb(px(16.))
            .border_1()
            .border_color(color("border-100"))
            .rounded(px(8.))
            .overflow_hidden()
            .child(head)
            .child(body)
    }

    fn render_folder(
        &self,
        f: &FolderGroup,
        open: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let b = &self.board;
        let states: Vec<RowState> = b.folder_rows(f).map(|r| b.rows[r]).collect();
        let n = states.len();
        let all = |s: RowState| states.iter().all(|x| *x == s);
        let summary = if all(RowState::Passed) {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_color(color("success"))
                .child(
                    glyph(icons::CHECK)
                        .size(px(13.))
                        .text_color(color("success")),
                )
                .child(format!("{n} passed"))
        } else if all(RowState::Queued) {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(state_icon(
                    SharedString::from(format!("run-folder-state-{}", f.folder)),
                    RowState::Queued,
                ))
                .child(format!("{n} queued"))
        } else if !states
            .iter()
            .any(|s| matches!(s, RowState::Queued | RowState::Running))
        {
            // Settled with failures or skips: each part in its color.
            let mut parts = div().flex().items_center().gap(px(4.));
            let mut first = true;
            for (state, word, token) in [
                (RowState::Passed, "passed", "success"),
                (RowState::Failed, "failed", "danger"),
                (RowState::Skipped, "skipped", "text-secondary"),
            ] {
                let c = states.iter().filter(|s| **s == state).count();
                if c == 0 {
                    continue;
                }
                if !first {
                    parts = parts.child("·");
                }
                first = false;
                parts = parts.child(div().text_color(color(token)).child(format!("{c} {word}")));
            }
            parts
        } else {
            div().child(folder_summary(&states))
        };
        let name = if f.folder.is_empty() {
            "(top level)".to_string()
        } else {
            f.folder.clone()
        };
        let folder = f.folder.clone();
        div()
            .id(SharedString::from(format!("run-folder-{}", f.folder)))
            .role(Role::Button)
            .test_support()
            .aria_label(format!("{name} {n}"))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(ROW_HEIGHT))
            .px(px(16.))
            .bg(color("surface-raised"))
            .border_b_1()
            .border_color(color("border-100"))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .whitespace_nowrap()
            .cursor_pointer()
            .child(
                glyph(if open {
                    icons::CHEVRON_DOWN
                } else {
                    icons::CHEVRON_RIGHT
                })
                .size(px(12.))
                .flex_none(),
            )
            .child(glyph(icons::FOLDER).size(px(14.)).flex_none())
            .child(
                div()
                    .text_color(color("text-primary"))
                    .font_weight(FontWeight::MEDIUM)
                    .child(name),
            )
            .child(n.to_string())
            .child(div().flex_1())
            .child(summary)
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(&folder, cx)))
    }

    fn render_row(
        &self,
        row: usize,
        file: PathBuf,
        index: usize,
        max_ms: f64,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let b = &self.board;
        let state = b.rows[row];
        let cell = &b.cells[row];
        let dash = || div().text_color(color("text-disabled")).child("—");
        let queued = matches!(state, RowState::Queued);
        let status = match cell.status {
            Some(code) => div()
                .font_family(mono_font())
                .text_size(px(12.))
                .text_color(color(if code < 400 { "success" } else { "danger" }))
                .child(code.to_string()),
            None => dash(),
        };
        let shown_ms = match state {
            RowState::Running => b.running_ms(),
            _ => cell.duration_ms,
        };
        let duration = match shown_ms {
            Some(ms) => {
                let fill = match state {
                    RowState::Failed => color("danger"),
                    RowState::Running => color("accent-blue"),
                    _ => color("indent-guide-active"),
                };
                let frac = (ms / max_ms) as f32 * BAR_SCALE;
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .h(px(4.))
                            .rounded(px(2.))
                            .overflow_hidden()
                            .bg(color("surface-200"))
                            .child(div().h_full().w(relative(frac.clamp(0.02, 1.))).bg(fill)),
                    )
                    .child(
                        div()
                            .w(px(50.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .font_family(mono_font())
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(duration_text(ms)),
                    )
            }
            None => dash(),
        };
        let checks = match cell.checks {
            Some((p, t)) if cell.done => div()
                .text_color(color(if p < t { "danger" } else { "text-secondary" }))
                .child(format!("{p}/{t}")),
            _ => dash(),
        };
        let attempts = match cell.attempts {
            0 => dash(),
            n => div()
                .text_color(color(if n > 1 { "warning" } else { "text-secondary" }))
                .child(n.to_string()),
        };
        let selected = self.selected == Some(row);
        let clickable = cell.done;
        div()
            .id(("run-row", row))
            .role(Role::Row)
            .test_support()
            .aria_label(b.names[row].clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.))
            .h(px(ROW_HEIGHT))
            .pl(px(36.))
            .pr(px(16.))
            .border_b_1()
            .border_color(color("surface-200"))
            .text_size(px(13.))
            .whitespace_nowrap()
            .when(selected, |r| r.bg(color("surface-200")))
            .when(clickable, |r| {
                r.cursor_pointer()
                    .hover(|s| s.bg(color("surface-100")))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = Some(row);
                        cx.emit(RunViewEvent::Open {
                            file: file.clone(),
                            index,
                        });
                    }))
            })
            .child(
                div()
                    .w(px(16.))
                    .flex_none()
                    .flex()
                    .child(state_icon(("run-state", row), state)),
            )
            .child(
                method_badge(Some(&cell.method))
                    .w(px(METHOD_COLUMN))
                    .whitespace_nowrap(),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_ellipsis()
                    .text_color(color(if queued || state == RowState::Skipped {
                        "text-tertiary"
                    } else {
                        "text-primary"
                    }))
                    .child(b.names[row].clone()),
            )
            .child(div().w(px(56.)).flex_none().child(status))
            .child(div().w(px(150.)).flex_none().child(duration))
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_size(px(12.))
                    .child(checks),
            )
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_size(px(12.))
                    .child(attempts),
            )
    }
}

/// A file that did not load: one failed row with the parser's message.
fn render_load_error_row(i: usize, file: &Path, message: &str) -> impl IntoElement + use<> {
    let name = if file.as_os_str().is_empty() {
        "Run".to_string()
    } else {
        file.display().to_string()
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(12.))
        .h(px(ROW_HEIGHT))
        .pl(px(36.))
        .pr(px(16.))
        .border_b_1()
        .border_color(color("surface-200"))
        .text_size(px(13.))
        .whitespace_nowrap()
        .child(
            div()
                .w(px(16.))
                .flex_none()
                .flex()
                .child(state_icon(("run-load-state", i), RowState::Failed)),
        )
        .child(div().w(px(METHOD_COLUMN)).flex_none())
        .child(
            div()
                .flex_none()
                .text_color(color("text-primary"))
                .child(name),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_ellipsis()
                .text_color(color("danger"))
                .child(message.lines().next().unwrap_or_default().to_string()),
        )
}

/// The row's leading icon, labelled for tests and screen readers.
fn state_icon(id: impl Into<ElementId>, state: RowState) -> AnyElement {
    let (label, el) = match state {
        RowState::Passed => (
            "Passed",
            glyph(icons::CHECK)
                .size(px(13.))
                .text_color(color("success"))
                .into_any_element(),
        ),
        RowState::Failed => (
            "Failed",
            glyph(icons::CROSS)
                .size(px(13.))
                .text_color(color("danger"))
                .into_any_element(),
        ),
        RowState::Running => (
            "Running",
            Spinner::new()
                .with_size(px(13.))
                .color(color("accent-blue"))
                .into_any_element(),
        ),
        RowState::Queued => (
            "Queued",
            glyph(icons::QUEUED)
                .size(px(13.))
                .text_color(color("text-disabled"))
                .into_any_element(),
        ),
        RowState::Skipped => (
            "Skipped",
            glyph(icons::SKIPPED)
                .size(px(13.))
                .text_color(color("text-disabled"))
                .into_any_element(),
        ),
    };
    div()
        .id(id)
        .role(Role::Image)
        .test_support()
        .aria_label(label)
        .flex()
        .child(el)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        Board, BoardStatus, ExportFormat, Labels, Local, PathBuf, RowState, RunConfig, RunEvent,
        RunOutcome, eta_text, folder_summary, report_file_name, write_atomic,
    };
    use crate::model::run::SharedRedactor;
    use curl_runner_core::executor::create_summary;
    use curl_runner_core::model::{ExecutionResult, Metrics, RequestConfig};

    fn started(groups: &[(&str, usize)]) -> RunEvent {
        RunEvent::Started {
            total: groups.iter().map(|(_, n)| n).sum(),
            groups: groups.iter().map(|(f, n)| (PathBuf::from(f), *n)).collect(),
            config: RunConfig {
                concurrency: Some(1),
                continue_on_error: false,
            },
            keychain_names: vec!["API_TOKEN".into()],
        }
    }

    /// One transfer, then its result.
    fn sent(b: &mut Board, group: usize, index: usize, success: bool, ms: f64) {
        b.apply(&RunEvent::Attempt {
            group,
            index,
            attempt: 1,
            outcome: curl_runner_core::executor::AttemptOutcome {
                status: Some(200),
                duration_ms: ms,
                error: None,
            },
        });
        b.apply(&result(group, index, success, ms));
    }

    fn result(group: usize, index: usize, success: bool, ms: f64) -> RunEvent {
        let mut r = ExecutionResult::new(
            RequestConfig {
                url: "https://api.example.com".into(),
                method: Some("POST".into()),
                ..Default::default()
            },
            success,
        );
        r.status = Some(if success { 200 } else { 500 });
        r.metrics = Some(Metrics {
            duration: ms,
            ..Default::default()
        });
        RunEvent::Result {
            group,
            index,
            result: Box::new(r),
        }
    }

    fn finished(stopped: bool) -> RunEvent {
        RunEvent::Finished(RunOutcome {
            summary: create_summary(vec![], 0.),
            stopped,
            stores: vec![],
            attempts: vec![],
            started_at: Local::now(),
            redactor: SharedRedactor::default(),
            keychain_names: vec![],
        })
    }

    fn board() -> Board {
        let labels = Labels::from([(
            PathBuf::from("charges/create.yaml"),
            vec![("POST".to_string(), "create-charge".to_string())],
        )]);
        let mut b = Board::new(labels);
        b.apply(&started(&[
            ("charges/create.yaml", 1),
            ("charges/flow.yaml", 2),
            ("smoke.yaml", 1),
        ]));
        b
    }

    #[test]
    fn rows_follow_the_run() {
        let mut b = board();
        assert_eq!(b.rows, [RowState::Queued; 4]);
        assert_eq!(
            b.names,
            ["create-charge", "Request 1", "Request 2", "smoke"]
        );
        assert_eq!(b.folders.len(), 2);
        b.apply(&RunEvent::RequestStarted {
            group: 0,
            index: 0,
            name: "create-charge".into(),
        });
        assert_eq!(b.rows[0], RowState::Running);
        assert_eq!(b.running_label().as_deref(), Some("charges/create"));
        b.apply(&RunEvent::Attempt {
            group: 0,
            index: 0,
            attempt: 3,
            outcome: curl_runner_core::executor::AttemptOutcome {
                status: Some(503),
                duration_ms: 1.,
                error: None,
            },
        });
        sent(&mut b, 0, 0, true, 100.);
        b.apply(&RunEvent::RequestStarted {
            group: 1,
            index: 1,
            name: "two".into(),
        });
        assert_eq!(
            b.running_label().as_deref(),
            Some("charges/flow · Request 2")
        );
        sent(&mut b, 1, 1, false, 300.);
        assert_eq!((b.passed(), b.failed(), b.queued(), b.done()), (1, 1, 2, 2));
        assert_eq!(b.retries(), (2, 1));
        assert_eq!(b.eta_ms(), Some(400.));
        assert_eq!(b.failed_names(), ["Request 2"]);
        assert_eq!(b.failed_files(), [PathBuf::from("charges/flow.yaml")]);
        b.apply(&finished(false));
        assert_eq!(b.status, BoardStatus::Failed);
        assert_eq!(b.skipped(), 2);
        assert_eq!(
            b.result_groups(),
            [
                (PathBuf::from("charges/create.yaml"), 1),
                (PathBuf::from("charges/flow.yaml"), 1),
                (PathBuf::from("smoke.yaml"), 0),
            ]
        );
    }

    #[test]
    fn a_request_never_sent_shows_no_time_or_attempts() {
        let mut b = board();
        assert_eq!(b.keychain_names, ["API_TOKEN"]);
        // A typing error: a result (with a 0 ms duration) and no attempt.
        b.apply(&result(0, 0, false, 0.));
        assert_eq!(b.rows[0], RowState::Failed);
        assert_eq!((b.cells[0].duration_ms, b.cells[0].attempts), (None, 0));
        assert_eq!(b.average_ms(), None, "not counted in the average");
        // A transport error did send: it keeps its time and attempt.
        sent(&mut b, 1, 0, false, 30.);
        assert_eq!(
            (b.cells[1].duration_ms, b.cells[1].attempts),
            (Some(30.), 1)
        );
    }

    #[test]
    fn stop_marks_the_board_stopped() {
        let mut b = board();
        sent(&mut b, 0, 0, true, 100.);
        b.apply(&finished(true));
        assert_eq!(b.status, BoardStatus::Stopped);
        b = board();
        for (g, i) in [(0, 0), (1, 0), (1, 1), (2, 0)] {
            sent(&mut b, g, i, true, 10.);
        }
        b.apply(&finished(false));
        assert_eq!(b.status, BoardStatus::Passed);
    }

    #[test]
    fn a_load_failure_is_a_failed_row() {
        let mut b = Board::new(Labels::new());
        b.apply(&RunEvent::LoadFailed {
            file: "bad.yaml".into(),
            message: "expected a mapping".into(),
        });
        b.apply(&finished(false));
        assert_eq!(b.failed(), 1);
        assert_eq!(b.status, BoardStatus::Failed);
        assert_eq!(b.failed_files(), [PathBuf::from("bad.yaml")]);
        assert_eq!(b.failed_names(), ["bad"]);
    }

    #[test]
    fn folders_fold_when_settled_or_untouched() {
        let mut b = board();
        assert!(!b.open_by_default(&b.folders[0]), "all queued");
        sent(&mut b, 0, 0, true, 1.);
        assert!(b.open_by_default(&b.folders[0]), "in progress");
        sent(&mut b, 1, 0, true, 1.);
        sent(&mut b, 1, 1, true, 1.);
        assert!(!b.open_by_default(&b.folders[0]), "all passed");
    }

    #[test]
    fn texts_follow_the_board() {
        assert_eq!(eta_text(5_200.), "~6 s left");
        assert_eq!(eta_text(10.), "~1 s left");
        assert_eq!(eta_text(90_000.), "~2 min left");
        use RowState::*;
        assert_eq!(
            folder_summary(&[Passed, Running, Queued, Queued]),
            "1 passed · 1 running · 2 queued"
        );
    }

    #[test]
    fn report_names_and_atomic_writes() {
        let at = chrono::TimeZone::with_ymd_and_hms(&Local, 2026, 10, 30, 9, 41, 7).unwrap();
        assert_eq!(
            report_file_name("Payments API", at, ExportFormat::Junit),
            "Payments API-20261030-094107.xml"
        );
        assert_eq!(
            report_file_name("a/b:c", at, ExportFormat::Markdown),
            "a-b-c-20261030-094107.md"
        );
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("r.json");
        write_atomic(&path, "{}").unwrap();
        write_atomic(&path, "[]").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[]");
        let left: Vec<_> = std::fs::read_dir(d.path()).unwrap().collect();
        assert_eq!(left.len(), 1, "no temp file left");
    }
}

#[cfg(test)]
mod ui_tests {
    use super::super::app::RunCollection;
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_secrets};
    use super::{BoardStatus, RunTarget};
    use crate::model::history::History;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Action as _, AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use testkit::{Response, TestServer};

    /// A collection of `files` with history kept under `base`.
    fn open(
        cx: &mut TestAppContext,
        files: &[(&str, String)],
        base: &Path,
    ) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let root = dir.path().to_path_buf();
        let store = StateStore::at(base);
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
                        store,
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

    fn run_view_has(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("run-view").try_find_by_label(label).is_some()
        })
        .unwrap()
    }

    fn yaml(base: &str, path: &str) -> String {
        format!("request:\n  url: {base}{path}\n  expect:\n    status: 200\n")
    }

    #[gpui_kit::test]
    fn collection_run_fills_table_and_history(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws) = open(
            cx,
            &[
                ("auth/login.yaml", yaml(&srv.base_url, "/status/200")),
                ("charges/get.yaml", yaml(&srv.base_url, "/status/500")),
                ("smoke.yaml", yaml(&srv.base_url, "/status/200")),
            ],
            state.path(),
        );
        cx.update_window(window, |_, window, cx| {
            window.dispatch_action(RunCollection.boxed_clone(), cx);
        })
        .unwrap();
        wait_for_run(cx, &ws);
        assert!(run_view_has(cx, window, "Passed 2"));
        assert!(run_view_has(cx, window, "Failed 1"));
        assert!(run_view_has(cx, window, "Run failed"), "the status chip");
        // The inspector gives the run screen its column.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("inspector").is_none());
        })
        .unwrap();
        let history = History::open(state.path(), dir.path());
        let entries = history.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].collection,
            dir.path().file_name().unwrap().to_string_lossy()
        );
        assert_eq!((entries[0].passed, entries[0].total), (2, 3));
        // Re-run failed runs the failing file alone.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("run-view").click("run-rerun-failed", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, cx| {
            assert!(ws.runs.active.is_some());
            let view = ws.run_view.as_ref().unwrap().read(cx);
            assert_eq!(
                view.target(),
                &RunTarget::Files(vec![PathBuf::from("charges/get.yaml")])
            );
        });
        wait_for_run(cx, &ws);
        assert!(run_view_has(cx, window, "Failed 1"));
        assert!(run_view_has(cx, window, "Passed 0"));
        assert_eq!(history.list().len(), 2);
    }

    #[gpui_kit::test]
    fn stop_marks_stopped(cx: &mut TestAppContext) {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = TestServer::with_handler(move |req| {
            log.lock().unwrap().push(req.url.clone());
            std::thread::sleep(Duration::from_millis(300));
            Response::from_data(b"{}".to_vec())
        });
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws) = open(
            cx,
            &[
                ("a.yaml", yaml(&srv.base_url, "/a")),
                ("b.yaml", yaml(&srv.base_url, "/b")),
            ],
            state.path(),
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.run(RunTarget::Collection, window, cx));
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while seen.lock().unwrap().is_empty() {
            assert!(Instant::now() < deadline, "the run did not start");
            std::thread::sleep(Duration::from_millis(5));
        }
        // ⌘. on the run screen (focused while shown) is Stop.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.press("cmd-.", cx);
        })
        .unwrap();
        wait_for_run(cx, &ws);
        ws.read_with(cx, |ws, cx| {
            let view = ws.run_view.as_ref().unwrap().read(cx);
            assert_eq!(view.board.status, BoardStatus::Stopped);
        });
        assert!(run_view_has(cx, window, "Run stopped"));
        assert_eq!(*seen.lock().unwrap(), ["/a"], "b never ran");
    }

    #[gpui_kit::test]
    fn a_row_opens_its_result_and_esc_closes(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws) = open(
            cx,
            &[
                ("ok.yaml", yaml(&srv.base_url, "/status/200")),
                ("bad.yaml", yaml(&srv.base_url, "/status/500")),
            ],
            state.path(),
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.run(RunTarget::Collection, window, cx));
        })
        .unwrap();
        wait_for_run(cx, &ws);
        // Files run sorted: bad.yaml is row 0 (its folder is open: it failed).
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("run-view").click(("run-row", 0usize), cx);
            window.render_frame(cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            assert!(!ws.run_shown());
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("bad.yaml"));
            assert_eq!(
                ws.inspector_tab,
                super::super::inspector::InspectorTab::Checks
            );
        });
        // The status bar brings the screen back; esc leaves it.
        cx.update_window(window, |_, window, cx| {
            window.click("status-run", cx);
            window.render_frame(cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(ws.run_shown()));
        cx.update_window(window, |_, window, cx| {
            window.press("escape", cx);
            window.render_frame(cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(!ws.run_shown()));
    }

    fn export_open(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> bool {
        ws.read_with(cx, |ws, cx| {
            ws.run_view.as_ref().unwrap().read(cx).export_open
        })
    }

    #[gpui_kit::test]
    fn the_export_menu_closes_on_its_button_and_on_esc_first(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws) = open(
            cx,
            &[("ok.yaml", yaml(&srv.base_url, "/status/200"))],
            state.path(),
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.run(RunTarget::Collection, window, cx));
        })
        .unwrap();
        wait_for_run(cx, &ws);
        let click_export = |cx: &mut TestAppContext| {
            cx.update_window(window, |_, window, cx| {
                window.render_frame(cx);
                window.within("run-view").click("run-export", cx);
                window.render_frame(cx);
            })
            .unwrap();
        };
        click_export(cx);
        assert!(export_open(cx, &ws));
        // The press outside the menu lands on its button: it stays closed.
        click_export(cx);
        assert!(!export_open(cx, &ws), "the button closes the open menu");
        click_export(cx);
        assert!(export_open(cx, &ws), "and opens it again");
        let esc = |cx: &mut TestAppContext| {
            cx.update_window(window, |_, window, cx| {
                window.press("escape", cx);
                window.render_frame(cx);
            })
            .unwrap();
        };
        esc(cx);
        assert!(!export_open(cx, &ws));
        ws.read_with(cx, |ws, _| {
            assert!(ws.run_shown(), "esc closed only the menu")
        });
        esc(cx);
        ws.read_with(cx, |ws, _| assert!(!ws.run_shown()));
    }

    #[gpui_kit::test]
    fn history_keeps_the_environment_the_run_used(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws) = open(
            cx,
            &[
                ("ok.yaml", yaml(&srv.base_url, "/status/200")),
                (".env.staging", "A=1\n".to_string()),
            ],
            state.path(),
        );
        ws.update(cx, |ws, cx| ws.set_environment("staging", cx));
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.run(RunTarget::Collection, window, cx));
            // Picked again while the run goes.
            ws.update(cx, |ws, cx| ws.set_environment("Default", cx));
        })
        .unwrap();
        wait_for_run(cx, &ws);
        let entries = History::open(state.path(), dir.path()).list();
        assert_eq!(entries[0].environment, "staging");
    }

    #[gpui_kit::test]
    fn export_saves_the_report_through_the_save_panel(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws) = open(
            cx,
            &[("ok.yaml", yaml(&srv.base_url, "/status/200"))],
            state.path(),
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.run(RunTarget::Collection, window, cx));
        })
        .unwrap();
        wait_for_run(cx, &ws);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("run-view").click("run-export", cx);
            window.render_frame(cx);
            window.within("run-view").click("export-junit", cx);
        })
        .unwrap();
        assert!(cx.did_prompt_for_new_path());
        let out = dir.path().join("report.xml");
        let suggested = Arc::new(Mutex::new(PathBuf::new()));
        let seen = suggested.clone();
        let to = out.clone();
        cx.simulate_new_path_selection(move |dir| {
            *seen.lock().unwrap() = dir.to_path_buf();
            Some(to)
        });
        cx.run_until_parked();
        assert_eq!(*suggested.lock().unwrap(), dir.path());
        let xml = std::fs::read_to_string(&out).unwrap();
        assert!(xml.contains("<testsuite name=\"ok.yaml\""), "{xml}");
    }
}
