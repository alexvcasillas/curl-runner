//! The History screen (board: History): this device's runs of the
//! collection, grouped by day and filtered by kind and environment, and the
//! picked run's detail: stats, a duration chart of its last runs and what
//! changed since the run before it. It takes the main area and the
//! inspector's column while it is open.
//!
//! The workspace loads the entries (off the main thread) and acts on what
//! the screen emits.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::run_view::{ExportFormat, pill, report_file_name, write_atomic};
use super::workspace::notify_error;
use crate::model::duration_text;
use crate::model::environments::EnvCtx;
use crate::model::export::{cli_command, to_markdown};
use crate::model::history::{
    EntryKind, Filter, HistoryEntry, KindFilter, StoredResult, day_bucket, filter_entries,
    previous_for, target_of,
};
use crate::model::run::RunTarget;
use chrono::{DateTime, Local};
use curl_runner_core::executor::create_summary;
use curl_runner_core::model::{ExecutionResult, Metrics, RequestConfig};
use curl_runner_core::security::redact::Redactor;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// The detail pane, apart for size; a child module, so it shares this
// module's private state.
#[path = "history_detail.rs"]
mod detail;

const ROW_HEIGHT: f32 = 40.;
/// The Passed column's segment bar; above this many requests it is drawn
/// as proportions.
const BAR_WIDTH: f32 = 88.;
const MAX_SEGMENTS: usize = 44;

/// A row of the list: a day heading or an entry (index into `entries`).
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Day(String),
    Entry(usize),
}

/// The day headings and entries the list shows for `shown` (newest first).
fn list_items(entries: &[HistoryEntry], shown: &[usize], now: DateTime<Local>) -> Vec<Item> {
    let mut out = Vec::with_capacity(shown.len() + 4);
    let mut last: Option<String> = None;
    for &i in shown {
        let day = day_bucket(entries[i].at, now);
        if last.as_deref() != Some(&day) {
            out.push(Item::Day(day.clone()));
            last = Some(day);
        }
        out.push(Item::Entry(i));
    }
    out
}

/// The row's two lines: `Payments API` / `Collection run`, or
/// `charges/get-charge` / `Request`.
pub fn entry_title(e: &HistoryEntry) -> (String, String) {
    let stem = |f: &Path| f.with_extension("").to_string_lossy().into_owned();
    match target_of(e) {
        RunTarget::Collection => (collection_name(e), "Collection run".into()),
        RunTarget::Files(fs) => (
            collection_name(e),
            match fs.len() {
                1 => "Collection run · 1 file".into(),
                n => format!("Collection run · {n} files"),
            },
        ),
        RunTarget::File(f) => (stem(&f), "Request".into()),
        RunTarget::Request { file, .. } => {
            let name = e.results.first().map(|r| r.name.as_str()).unwrap_or("");
            let title = if name.is_empty() {
                stem(&file)
            } else {
                format!("{} · {name}", stem(&file))
            };
            (title, "Request".into())
        }
    }
}

fn collection_name(e: &HistoryEntry) -> String {
    if e.collection.is_empty() {
        "Collection".into()
    } else {
        e.collection.clone()
    }
}

/// `today 09:41`, `yesterday 18:12`, `Oct 27 19:55`.
fn when_text(at: DateTime<Local>, now: DateTime<Local>) -> String {
    let day = day_bucket(at, now);
    let day = match day.as_str() {
        "Today" | "Yesterday" => day.to_lowercase(),
        _ => day,
    };
    format!("{day} {}", at.format("%H:%M"))
}

/// `Yesterday 18:12 · Local`: the compared run, as its picker reads.
fn compare_label(at: DateTime<Local>, now: DateTime<Local>) -> String {
    format!("{} {} · Local", day_bucket(at, now), at.format("%H:%M"))
}

/// `3.1 MB`, `48 KB`, `512 B`.
pub fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{} KB", (b / KB).round() as u64)
    } else {
        format!("{:.1} MB", b / KB / KB)
    }
}

/// `142 runs stored · 3.1 MB`.
pub fn usage_text((runs, bytes): (usize, u64)) -> String {
    let noun = if runs == 1 { "run" } else { "runs" };
    format!("{runs} {noun} stored · {}", size_text(bytes))
}

/// `109 → 212 ms`, `1.2 → 2.0 s`.
fn change_text(prev: f64, cur: f64) -> String {
    if prev.max(cur) < 1000. {
        format!("{} → {} ms", prev.round() as u64, cur.round() as u64)
    } else {
        format!("{:.1} → {:.1} s", prev / 1000., cur / 1000.)
    }
}

/// `+94%`.
fn slower_text(prev: f64, cur: f64) -> String {
    let pct = if prev > 0. {
        ((cur / prev - 1.) * 100.).round() as i64
    } else {
        0
    };
    format!("+{pct}%")
}

/// The board's footnote, with the unchanged count.
pub fn unchanged_note(unchanged: usize) -> String {
    let rest = match unchanged {
        1 => "1 other request unchanged.".to_string(),
        n => format!("{n} other requests unchanged."),
    };
    format!("Slower means at least 25% over the previous run and over 50 ms. {rest}")
}

/// What a newly failing request failed on: its failed checks, the status
/// check as `status 409`.
fn failure_detail(r: &StoredResult) -> String {
    let parts: Vec<String> = r
        .failed_checks
        .iter()
        .map(|c| match (c.as_str(), r.status) {
            ("status", Some(code)) => format!("status {code}"),
            _ => c.clone(),
        })
        .collect();
    match (parts.is_empty(), r.status) {
        (false, _) => parts.join(", "),
        (true, Some(code)) => format!("status {code}"),
        (true, None) => "no response".into(),
    }
}

fn checks_text(r: &StoredResult) -> String {
    format!("{}/{} checks", r.checks_passed, r.checks_total)
}

/// A Markdown summary of a stored run, in the run screen's format. History
/// keeps results, not full responses, so this is the one report a past run
/// can give faithfully. Stored text is already redacted.
pub fn history_markdown(e: &HistoryEntry) -> String {
    let results = e
        .results
        .iter()
        .map(|r| {
            let request = RequestConfig {
                name: Some(r.name.clone()),
                method: Some(r.method.clone()),
                source_file: Some(r.file.to_string_lossy().into_owned()),
                ..Default::default()
            };
            let mut x = ExecutionResult::new(request, r.success);
            x.status = r.status;
            x.metrics = Some(Metrics {
                duration: r.duration_ms,
                ..Default::default()
            });
            if !r.success {
                x.error = Some(failure_detail(r));
            }
            x
        })
        .collect();
    let summary = create_summary(results, e.duration_ms);
    let (title, _) = entry_title(e);
    to_markdown(
        &summary,
        &format!("Run · {title}"),
        &env_label(&e.environment),
        &Redactor::default(),
    )
}

/// What the History screen asks of the workspace.
#[derive(Debug, Clone, PartialEq)]
pub enum HistoryViewEvent {
    /// Back to the editor.
    Close,
    /// Run this entry's target again.
    Rerun(Box<HistoryEntry>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Menu {
    Environment,
    Export,
    Compare,
}

pub struct HistoryView {
    root: PathBuf,
    collection: String,
    /// The collection's `CollectionMeta::id`, for the CLI command it copies.
    collection_id: String,
    /// The collection's secret names, for the CLI command it copies.
    marked: Vec<String>,
    /// Newest first.
    entries: Vec<HistoryEntry>,
    loaded: bool,
    usage: (usize, u64),
    now: DateTime<Local>,
    filter: Filter,
    items: Vec<Item>,
    /// The picked entry, by id.
    selected: Option<String>,
    /// The run it is compared with when not the one right before it.
    compare_with: Option<String>,
    menu: Option<Menu>,
    /// When a press outside last closed a menu: the same press landing on
    /// its button must not open it again.
    menu_closed_at: Option<Instant>,
    list: ListState,
    copied: Option<Task<()>>,
    focus: FocusHandle,
}

impl EventEmitter<HistoryViewEvent> for HistoryView {}

impl Focusable for HistoryView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl HistoryView {
    pub fn new(
        root: PathBuf,
        collection: String,
        collection_id: String,
        marked: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        HistoryView {
            root,
            collection,
            collection_id,
            marked,
            entries: Vec::new(),
            loaded: false,
            usage: (0, 0),
            now: Local::now(),
            filter: Filter::default(),
            items: Vec::new(),
            selected: None,
            compare_with: None,
            menu: None,
            menu_closed_at: None,
            list: ListState::new(0, ListAlignment::Top, px(400.)),
            copied: None,
            focus: cx.focus_handle(),
        }
    }

    /// The stored runs, newest first, and their footprint.
    pub fn set_entries(
        &mut self,
        entries: Vec<HistoryEntry>,
        usage: (usize, u64),
        cx: &mut Context<Self>,
    ) {
        // A run that just arrived on top is picked, as it was just made.
        let newest = |e: &[HistoryEntry]| e.first().map(|e| e.id.clone());
        if self.loaded && newest(&entries) != newest(&self.entries) {
            self.selected = newest(&entries);
            self.compare_with = None;
        }
        self.entries = entries;
        self.usage = usage;
        self.loaded = true;
        self.now = Local::now();
        // A filter on an environment no run has any more shows nothing.
        if let Some(env) = &self.filter.environment
            && !self.entries.iter().any(|e| &e.environment == env)
        {
            self.filter.environment = None;
        }
        self.rebuild();
        cx.notify();
    }

    pub fn set_marked(&mut self, marked: Vec<String>) {
        self.marked = marked;
    }

    pub fn usage(&self) -> (usize, u64) {
        self.usage
    }

    /// Re-filters the list, keeping the pick when it is still shown.
    fn rebuild(&mut self) {
        let shown = filter_entries(&self.entries, &self.filter);
        self.items = list_items(&self.entries, &shown, self.now);
        self.list.reset(self.items.len());
        let kept = self
            .selected
            .as_ref()
            .is_some_and(|id| shown.iter().any(|&i| &self.entries[i].id == id));
        if !kept {
            self.selected = shown.first().map(|&i| self.entries[i].id.clone());
            self.compare_with = None;
        }
    }

    fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        self.menu = None;
        if self.filter != filter {
            self.filter = filter;
            self.rebuild();
        }
        cx.notify();
    }

    pub fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected.as_deref() != Some(id) {
            self.selected = Some(id.to_string());
            self.compare_with = None;
        }
        self.menu = None;
        cx.notify();
    }

    fn selected_entry(&self) -> Option<&HistoryEntry> {
        let id = self.selected.as_ref()?;
        self.entries.iter().find(|e| &e.id == id)
    }

    /// Earlier runs of `e`'s target and environment, newest first.
    fn earlier(&self, e: &HistoryEntry) -> impl Iterator<Item = &HistoryEntry> {
        let (target, env, at) = (e.target.clone(), e.environment.clone(), e.at);
        self.entries
            .iter()
            .filter(move |o| o.target == target && o.environment == env && o.at < at)
    }

    /// The run `e` is compared with: the picked one, else the one before.
    fn compared(&self, e: &HistoryEntry) -> Option<&HistoryEntry> {
        let picked = self
            .compare_with
            .as_ref()
            .and_then(|id| self.earlier(e).find(|o| &o.id == id));
        picked.or_else(|| previous_for(&self.entries, e))
    }

    /// Opens or closes `menu` from its button.
    fn toggle_menu(&mut self, menu: Menu, cx: &mut Context<Self>) {
        let just_closed = self
            .menu_closed_at
            .is_some_and(|at| at.elapsed() < Duration::from_millis(300));
        self.menu = if self.menu == Some(menu) || just_closed {
            None
        } else {
            Some(menu)
        };
        self.menu_closed_at = None;
        cx.notify();
    }

    fn close_menu_outside(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        self.menu_closed_at = Some(Instant::now());
        cx.notify();
    }

    /// Saves the picked run's Markdown summary through the save panel.
    fn export_markdown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        cx.notify();
        let Some(e) = self.selected_entry() else {
            return;
        };
        let text = history_markdown(e);
        let name = report_file_name(&self.collection, e.at, ExportFormat::Markdown);
        let picked = cx.prompt_for_new_path(&self.root, Some(&name));
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

    /// Copies the `curl-runner` command that repeats the picked run.
    fn copy_cli(&mut self, cx: &mut Context<Self>) {
        let Some(e) = self.selected_entry() else {
            return;
        };
        let ctx = EnvCtx {
            root: self.root.clone(),
            collection: self.collection.clone(),
            collection_id: self.collection_id.clone(),
            environment: e.environment.clone(),
            marked_secret: self.marked.clone(),
        };
        let command = cli_command(&target_of(e), &ctx, &e.keychain_names);
        cx.write_to_clipboard(ClipboardItem::new_string(command));
        self.menu = None;
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

    /// The environments the stored runs used, sorted.
    fn environments(&self) -> Vec<String> {
        let mut envs: Vec<String> = self.entries.iter().map(|e| e.environment.clone()).collect();
        envs.sort();
        envs.dedup();
        envs
    }
}

impl Render for HistoryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("history-view")
            .key_context("HistoryView")
            .track_focus(&self.focus)
            .role(Role::Main)
            .test_support()
            .aria_label("Run history")
            .on_action(
                // esc closes an open menu first, then the screen.
                cx.listener(|this, _: &super::app::CloseHistory, _, cx| {
                    if this.menu.is_some() {
                        this.menu = None;
                        cx.notify();
                    } else {
                        cx.emit(HistoryViewEvent::Close);
                    }
                }),
            )
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(color("bg-100"))
            .child(self.render_list(cx))
            .child(self.render_detail(cx))
    }
}

/// An element the smoke tests can find.
type Observed = gpui_kit::base::ObservedElement<Stateful<Div>>;

/// The board's `.btn.sm`: a 28px outlined button.
fn small_button(id: impl Into<ElementId>) -> Observed {
    div()
        .id(id)
        .role(Role::Button)
        .test_support()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(10.))
        .border_1()
        .border_color(color("border-200"))
        .rounded(px(6.))
        .bg(color("bg-100"))
        .text_size(px(12.))
        .text_color(color("text-primary"))
        .whitespace_nowrap()
        .cursor_pointer()
        .hover(|s| s.bg(color("surface-100")))
}

fn chevron() -> impl IntoElement {
    glyph(icons::CHEVRON_DOWN)
        .size(px(12.))
        .text_color(color("text-secondary"))
}

/// An environment's pill: Production in warning colors, others neutral.
fn env_pill(env: &str) -> Div {
    let production = env.eq_ignore_ascii_case("production") || env.eq_ignore_ascii_case("prod");
    let tokens = if production {
        ("warning", "warning-subtle", "warning-border")
    } else {
        ("text-secondary", "", "border-200")
    };
    pill(tokens)
        .font_weight(FontWeight::NORMAL)
        .child(env_label(env))
}

/// A popup menu under its button, right-aligned to it.
fn menu_panel(id: &'static str, width: f32) -> Observed {
    div()
        .id(id)
        .role(Role::Menu)
        .test_support()
        .occlude()
        .absolute()
        .top(px(32.))
        .right_0()
        .w(px(width))
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
}

fn menu_item(id: impl Into<ElementId>, label: impl Into<SharedString>, on: bool) -> Observed {
    let label = label.into();
    div()
        .id(id)
        .role(Role::MenuItem)
        .test_support()
        .aria_label(label.clone())
        .flex()
        .items_center()
        .gap(px(10.))
        .h(px(32.))
        .px(px(10.))
        .rounded(px(5.))
        .whitespace_nowrap()
        .map(|d| {
            if on {
                d.text_color(color("text-primary"))
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-300")))
            } else {
                d.text_color(color("text-disabled"))
            }
        })
        .child(div().flex_1().min_w_0().text_ellipsis().child(label))
}

impl HistoryView {
    fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let th = |text: &'static str| {
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color("text-tertiary"))
                .whitespace_nowrap()
                .child(text)
        };
        let columns = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.))
            .h(px(32.))
            .px(px(16.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(div().w(px(16.)).flex_none())
            .child(th("Time").w(px(44.)).flex_none())
            .child(th("Run").flex_1().min_w_0())
            .child(th("Environment").w(px(92.)).flex_none())
            .child(th("Where").w(px(76.)).flex_none())
            .child(th("Passed").w(px(140.)).flex_none())
            .child(th("Duration").w(px(60.)).flex_none().flex().justify_end());
        let body = if self.items.is_empty() {
            let text = if !self.loaded {
                ""
            } else if self.entries.is_empty() {
                "No runs yet. Runs you start here are kept on this device for 30 days."
            } else {
                "No runs match these filters."
            };
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .justify_center()
                .pt(px(48.))
                .px(px(20.))
                .text_size(px(13.))
                .text_color(color("text-tertiary"))
                .child(text)
                .into_any_element()
        } else {
            div()
                .id("history-list")
                .role(Role::List)
                .test_support()
                .flex_1()
                .min_h_0()
                .child(
                    list(
                        self.list.clone(),
                        cx.processor(|this, ix: usize, _, cx| this.render_item(ix, cx)),
                    )
                    .size_full(),
                )
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(self.render_list_header(cx))
            .child(columns)
            .child(body)
    }

    fn render_list_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let tab =
            |id: &'static str, label: &'static str, kind: KindFilter, cx: &mut Context<Self>| {
                let on = self.filter.kind == kind;
                div()
                    .id(id)
                    .role(Role::Tab)
                    .test_support()
                    .aria_label(label)
                    .flex()
                    .items_center()
                    .h(px(26.))
                    .px(px(10.))
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .map(|t| {
                        if on {
                            t.bg(color("surface-300")).text_color(color("text-primary"))
                        } else {
                            t.text_color(color("text-secondary"))
                                .hover(|s| s.text_color(color("text-primary")))
                        }
                    })
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let filter = Filter {
                            kind,
                            ..this.filter.clone()
                        };
                        this.set_filter(filter, cx);
                    }))
            };
        let tabs = div()
            .flex()
            .flex_none()
            .gap(px(2.))
            .p(px(2.))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(6.))
            .child(tab("history-all", "All", KindFilter::All, cx))
            .child(tab(
                "history-collection-runs",
                "Collection runs",
                KindFilter::CollectionRuns,
                cx,
            ))
            .child(tab(
                "history-requests",
                "Requests",
                KindFilter::Requests,
                cx,
            ))
            .child(tab("history-failed", "Failed", KindFilter::Failed, cx));
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
                    .flex_none()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("History"),
            )
            .child(
                div()
                    .min_w_0()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    // Wraps as the board's does when the tabs need the room.
                    .line_height(relative(1.3))
                    .child("Kept on this device · last 30 days"),
            )
            .child(div().flex_1())
            .child(tabs)
            .child(self.render_env_filter(cx))
    }

    fn render_env_filter(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let label = self
            .filter
            .environment
            .as_deref()
            .map_or_else(|| "All environments".to_string(), env_label);
        let button = small_button("history-env")
            .aria_label(label.clone())
            .when(self.menu == Some(Menu::Environment), |b| {
                b.bg(color("surface-300"))
            })
            .child(label)
            .child(chevron())
            .on_click(cx.listener(|this, _, _, cx| this.toggle_menu(Menu::Environment, cx)));
        let mut menu = menu_panel("history-env-menu", 200.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_menu_outside(cx)));
        let options = std::iter::once(None).chain(self.environments().into_iter().map(Some));
        for (i, env) in options.enumerate() {
            let label = env
                .as_deref()
                .map_or_else(|| "All environments".to_string(), env_label);
            let on = self.filter.environment == env;
            menu = menu.child(
                menu_item(("history-env-option", i), label, true)
                    .when(on, |d| d.bg(color("surface-200")))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let filter = Filter {
                            environment: env.clone(),
                            ..this.filter.clone()
                        };
                        this.set_filter(filter, cx);
                    })),
            );
        }
        div()
            .relative()
            .flex()
            .flex_none()
            .child(button)
            .when(self.menu == Some(Menu::Environment), |d| {
                d.child(deferred(menu).with_priority(1))
            })
    }

    fn render_item(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        match self.items.get(ix) {
            Some(Item::Day(day)) => div()
                .w_full()
                .flex()
                .items_center()
                .h(px(28.))
                .px(px(16.))
                .bg(color("surface-raised"))
                .border_b_1()
                .border_color(color("border-100"))
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child(day.clone())
                .into_any_element(),
            Some(Item::Entry(i)) => self.render_row(*i, cx).into_any_element(),
            None => div().into_any_element(),
        }
    }

    fn render_row(&self, i: usize, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let e = &self.entries[i];
        let failed = e.passed < e.total;
        let selected = self.selected.as_ref() == Some(&e.id);
        let (title, kind) = entry_title(e);
        let icon = if failed {
            glyph(icons::CROSS)
                .size(px(13.))
                .text_color(color("danger"))
        } else {
            glyph(icons::CHECK)
                .size(px(13.))
                .text_color(color("success"))
        };
        let segments = if e.kind == EntryKind::CollectionRun && !e.results.is_empty() {
            let bar = div()
                .flex()
                .flex_none()
                .gap(px(1.))
                .w(px(BAR_WIDTH))
                .h(px(8.))
                .rounded(px(2.))
                .overflow_hidden();
            let fill = |ok: bool| color(if ok { "success" } else { "danger" });
            if e.results.len() <= MAX_SEGMENTS {
                bar.children(
                    e.results
                        .iter()
                        .map(|r| div().flex_1().h_full().bg(fill(r.success))),
                )
            } else {
                let n = e.results.len() as f32;
                let ok = e.results.iter().filter(|r| r.success).count() as f32;
                bar.child(div().h_full().w(relative(ok / n)).bg(fill(true)))
                    .child(div().h_full().flex_1().bg(fill(false)))
            }
        } else {
            div().w(px(BAR_WIDTH)).flex_none()
        };
        let id = e.id.clone();
        div()
            .id(SharedString::from(format!("history-row-{}", e.id)))
            .role(Role::Row)
            .test_support()
            .aria_label(format!("{title} {}", e.at.format("%H:%M")))
            .relative()
            .w_full()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(px(ROW_HEIGHT))
            .px(px(16.))
            .border_b_1()
            .border_color(color("surface-200"))
            .text_size(px(13.))
            .whitespace_nowrap()
            .cursor_pointer()
            .map(|r| {
                if selected {
                    r.bg(color("surface-200")).child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .bottom_0()
                            .w(px(2.))
                            .bg(color("text-primary")),
                    )
                } else {
                    r.hover(|s| s.bg(color("surface-100")))
                }
            })
            .on_click(cx.listener(move |this, _, _, cx| this.select(&id, cx)))
            .child(div().w(px(16.)).flex_none().flex().child(icon))
            .child(
                div()
                    .w(px(44.))
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(e.at.format("%H:%M").to_string()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .line_height(relative(1.3))
                    .child(div().text_ellipsis().child(title))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(color("text-tertiary"))
                            .child(kind),
                    ),
            )
            .child(
                div()
                    .w(px(92.))
                    .flex_none()
                    .flex()
                    .child(env_pill(&e.environment)),
            )
            .child(
                div()
                    .w(px(76.))
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child("Local"),
            )
            .child(
                div()
                    .w(px(140.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(segments)
                    .child(
                        div()
                            .font_family(mono_font())
                            .text_size(px(12.))
                            .text_color(color(if failed { "danger" } else { "text-secondary" }))
                            .child(format!("{}/{}", e.passed, e.total)),
                    ),
            )
            .child(
                div()
                    .w(px(60.))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(duration_text(e.duration_ms)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DateTime, EntryKind, HistoryEntry, Item, Local, StoredResult, change_text, compare_label,
        entry_title, failure_detail, history_markdown, list_items, size_text, slower_text,
        unchanged_note, usage_text, when_text,
    };
    use chrono::TimeZone;

    fn at(day: u32, h: u32, m: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, day, h, m, 0).unwrap()
    }

    fn stored(name: &str, ok: bool) -> StoredResult {
        StoredResult {
            file: "charges/get-charge.yaml".into(),
            name: name.into(),
            method: "GET".into(),
            status: Some(if ok { 200 } else { 409 }),
            success: ok,
            duration_ms: 120.,
            attempts: 1,
            checks_passed: if ok { 1 } else { 0 },
            checks_total: 1,
            failed_checks: if ok { vec![] } else { vec!["status".into()] },
            body: None,
        }
    }

    fn entry(id: &str, when: DateTime<Local>, target: &str, kind: EntryKind) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            at: when,
            collection: "Payments API".into(),
            target: target.into(),
            kind,
            environment: "staging".into(),
            passed: 1,
            total: 2,
            duration_ms: 846.,
            retries: 0,
            results: vec![stored("get-charge", true), stored("refund", false)],
            version: "2.1.1".into(),
            keychain_names: Vec::new(),
        }
    }

    #[test]
    fn titles_follow_the_board() {
        let c = entry("a", at(30, 9, 41), "collection", EntryKind::CollectionRun);
        assert_eq!(
            entry_title(&c),
            ("Payments API".to_string(), "Collection run".to_string())
        );
        let f = entry(
            "b",
            at(30, 9, 38),
            "charges/get-charge.yaml",
            EntryKind::Request,
        );
        assert_eq!(
            entry_title(&f),
            ("charges/get-charge".to_string(), "Request".to_string())
        );
        let r = entry(
            "c",
            at(30, 9, 38),
            "charges/list.yaml#1",
            EntryKind::Request,
        );
        assert_eq!(entry_title(&r).0, "charges/list · get-charge");
        let files = entry(
            "d",
            at(30, 9, 38),
            "a.yaml, b.yaml",
            EntryKind::CollectionRun,
        );
        assert_eq!(entry_title(&files).1, "Collection run · 2 files");
    }

    #[test]
    fn list_groups_by_day() {
        let now = at(30, 12, 0);
        let list = [
            entry("a", at(30, 9, 41), "collection", EntryKind::CollectionRun),
            entry("b", at(30, 9, 38), "collection", EntryKind::CollectionRun),
            entry("c", at(29, 18, 12), "collection", EntryKind::CollectionRun),
            entry("d", at(27, 19, 55), "collection", EntryKind::CollectionRun),
        ];
        assert_eq!(
            list_items(&list, &[0, 1, 2, 3], now),
            [
                Item::Day("Today".into()),
                Item::Entry(0),
                Item::Entry(1),
                Item::Day("Yesterday".into()),
                Item::Entry(2),
                Item::Day("Oct 27".into()),
                Item::Entry(3),
            ]
        );
        assert_eq!(when_text(at(30, 9, 41), now), "today 09:41");
        assert_eq!(when_text(at(27, 19, 55), now), "Oct 27 19:55");
        assert_eq!(
            compare_label(at(29, 18, 12), now),
            "Yesterday 18:12 · Local"
        );
    }

    #[test]
    fn texts() {
        assert_eq!(size_text(512), "512 B");
        assert_eq!(size_text(48 * 1024), "48 KB");
        assert_eq!(size_text(3_250_000), "3.1 MB");
        assert_eq!(usage_text((142, 3_250_000)), "142 runs stored · 3.1 MB");
        assert_eq!(usage_text((1, 10)), "1 run stored · 10 B");
        assert_eq!(change_text(109., 212.), "109 → 212 ms");
        assert_eq!(change_text(900., 2000.), "0.9 → 2.0 s");
        assert_eq!(slower_text(109., 212.), "+94%");
        assert_eq!(
            unchanged_note(17),
            "Slower means at least 25% over the previous run and over 50 ms. 17 other requests unchanged."
        );
        assert!(unchanged_note(1).ends_with("1 other request unchanged."));
        let bad = stored("x", false);
        assert_eq!(failure_detail(&bad), "status 409");
        let mut body = bad.clone();
        body.failed_checks = vec!["body.amount".into(), "body.status".into()];
        assert_eq!(failure_detail(&body), "body.amount, body.status");
    }

    #[test]
    fn markdown_from_a_stored_run() {
        let e = entry("a", at(30, 9, 41), "collection", EntryKind::CollectionRun);
        let md = history_markdown(&e);
        assert!(md.starts_with("# Run · Payments API\n"), "{md}");
        assert!(
            md.contains("Environment: Staging · Passed 1 · Failed 1"),
            "{md}"
        );
        assert!(md.contains("| GET | refund | 409 |"), "{md}");
        assert!(md.contains("- **refund**: status 409"), "{md}");
    }
}

#[cfg(test)]
mod ui_tests {
    use super::super::app::OpenHistory;
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_secrets};
    use crate::model::history::{EntryKind, History, HistoryEntry, StoredResult};
    use crate::model::run::RunTarget;
    use chrono::{Duration as Days, Local};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Action as _, AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;
    use std::time::{Duration, Instant};
    use testkit::TestServer;

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

    fn result(name: &str, ok: bool, ms: f64) -> StoredResult {
        StoredResult {
            file: "charges/get-charge.yaml".into(),
            name: name.into(),
            method: "GET".into(),
            status: Some(if ok { 200 } else { 500 }),
            success: ok,
            duration_ms: ms,
            attempts: 1,
            checks_passed: usize::from(ok),
            checks_total: 1,
            failed_checks: if ok { vec![] } else { vec!["status".into()] },
            body: None,
        }
    }

    fn entry(id: &str, ago_min: i64, results: Vec<StoredResult>) -> HistoryEntry {
        let passed = results.iter().filter(|r| r.success).count() as u64;
        HistoryEntry {
            id: id.into(),
            at: Local::now() - Days::minutes(ago_min),
            collection: "acme".into(),
            target: "collection".into(),
            kind: EntryKind::CollectionRun,
            environment: "Default".into(),
            passed,
            total: results.len() as u64,
            duration_ms: 100.,
            retries: 0,
            results,
            version: String::new(),
            keychain_names: Vec::new(),
        }
    }

    fn shows(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("history-view")
                .try_find_by_label(label)
                .is_some()
        })
        .unwrap()
    }

    fn open_history(cx: &mut TestAppContext, window: AnyWindowHandle) {
        cx.update_window(window, |_, window, cx| {
            window.dispatch_action(OpenHistory.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn history_shows_comparison(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws) = open(
            cx,
            &[(
                "charges/get-charge.yaml",
                "request:\n  url: https://api.example.com\n".into(),
            )],
            state.path(),
        );
        let history = History::open(state.path(), dir.path());
        let before = entry(
            "aaaa0001",
            60,
            vec![
                result("get-charge", true, 100.),
                result("refund", true, 100.),
            ],
        );
        let now = entry(
            "aaaa0002",
            1,
            vec![
                result("get-charge", false, 100.),
                result("refund", true, 100.),
            ],
        );
        history.record(&before).unwrap();
        history.record(&now).unwrap();
        // ⌘Y opens it.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.press("cmd-y", cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| assert!(ws.history_shown()));
        assert!(shows(cx, window, "Newly failing 1"));
        assert!(shows(cx, window, "Passed 1"), "the stats");
        assert!(shows(cx, window, "Failed 1"));
        // The inspector gives the screen its column.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("inspector").is_none());
        })
        .unwrap();
        // The older run has nothing before it.
        cx.update_window(window, |_, window, cx| {
            window
                .within("history-view")
                .click("history-row-aaaa0001", cx);
        })
        .unwrap();
        assert!(!shows(cx, window, "Newly failing 1"));
        // Failed keeps only the failing run.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("history-view").click("history-failed", cx);
        })
        .unwrap();
        assert!(shows(cx, window, "Newly failing 1"), "the pick moves");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window
                    .within("history-view")
                    .try_find("history-row-aaaa0001")
                    .is_none()
            );
            // esc leaves it (the screen has focus).
            window.press("escape", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(!ws.history_shown()));
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

    #[gpui_kit::test]
    fn copied_cli_command_names_the_keychain_secrets(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let yaml = format!(
            "request:\n  url: {}/status/200\n  headers:\n    X-Key: ${{SECRET_TOKEN}}\n",
            srv.base_url
        );
        let (dir, window, ws) = open(cx, &[("a.yaml", yaml)], state.path());
        let service = crate::model::secrets::service_for(state.path(), dir.path(), "Default");
        ws.read_with(cx, |ws, _| {
            ws.secrets.set(&service, "SECRET_TOKEN", "kc-tok-55")
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.invalidate_secrets();
                ws.run(RunTarget::File("a.yaml".into()), window, cx)
            });
        })
        .unwrap();
        wait_for_run(cx, &ws);
        let entries = History::open(state.path(), dir.path()).list();
        assert_eq!(entries[0].keychain_names, ["SECRET_TOKEN"]);
        open_history(cx, window);
        let view = ws.read_with(cx, |ws, _| ws.history_view.clone().unwrap());
        view.update(cx, |v, cx| {
            v.select(&entries[0].id, cx);
            v.copy_cli(cx);
        });
        let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
        assert!(copied.contains("SECRET_TOKEN"), "{copied}");
        assert!(copied.contains("(keychain)"), "{copied}");
        assert!(!copied.contains("kc-tok-55"), "{copied}");
    }

    #[gpui_kit::test]
    fn request_runs_are_recorded_and_rerun_from_history(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let yaml = format!(
            "requests:\n  - name: one\n    url: {0}/status/200\n  - name: two\n    url: {0}/status/200\n",
            srv.base_url
        );
        let (dir, window, ws) = open(cx, &[("charges/list.yaml", yaml)], state.path());
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.run(
                    RunTarget::Request {
                        file: "charges/list.yaml".into(),
                        index: 1,
                    },
                    window,
                    cx,
                )
            });
        })
        .unwrap();
        wait_for_run(cx, &ws);
        let history = History::open(state.path(), dir.path());
        let entries = history.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, EntryKind::Request);
        assert_eq!(entries[0].target, "charges/list.yaml#1");
        open_history(cx, window);
        assert!(shows(
            cx,
            window,
            &format!("charges/list · two {}", entries[0].at.format("%H:%M"))
        ));
        // Re-run finds the request by name, though it moved.
        std::fs::write(
            dir.path().join("charges/list.yaml"),
            format!(
                "requests:\n  - name: two\n    url: {0}/status/200\n  - name: one\n    url: {0}/status/200\n",
                srv.base_url
            ),
        )
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("history-view").click("history-rerun", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            assert!(!ws.history_shown());
            assert_eq!(
                ws.runs.target(),
                Some(&RunTarget::Request {
                    file: "charges/list.yaml".into(),
                    index: 0
                })
            );
        });
        wait_for_run(cx, &ws);
        assert_eq!(history.list().len(), 2);
    }

    /// Runs `target` and waits; returns the history entries, newest first.
    fn run_and_list(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        target: RunTarget,
        history: &History,
    ) -> Vec<HistoryEntry> {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.run(target, window, cx));
        })
        .unwrap();
        wait_for_run(cx, ws);
        history.list()
    }

    fn rerun(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        e: &HistoryEntry,
    ) {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.rerun_history(e, window, cx));
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn a_one_request_file_entry_reruns(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let yaml = format!("request:\n  url: {}/status/200\n", srv.base_url);
        let (dir, window, ws) = open(cx, &[("single.yaml", yaml)], state.path());
        let history = History::open(state.path(), dir.path());
        // As ⌥⌘↵ runs it: request 0 of the file.
        let target = RunTarget::Request {
            file: "single.yaml".into(),
            index: 0,
        };
        let entries = run_and_list(cx, window, &ws, target, &history);
        assert_eq!(entries[0].target, "single.yaml#0");
        rerun(cx, window, &ws, &entries[0]);
        ws.read_with(cx, |ws, _| {
            assert_eq!(
                ws.runs.target(),
                Some(&RunTarget::File("single.yaml".into()))
            )
        });
        wait_for_run(cx, &ws);
        assert_eq!(history.list().len(), 2);
    }

    #[gpui_kit::test]
    fn an_unnamed_request_reruns_by_index(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        // The stored name is the sent URL; the tree shows the raw one.
        let yaml = "requests:\n  - url: ${BASE}/status/200\n  - url: ${BASE}/status/201\n";
        let (dir, window, ws) = open(
            cx,
            &[
                ("list.yaml", yaml.to_string()),
                (".env", format!("BASE={}\n", srv.base_url)),
            ],
            state.path(),
        );
        let history = History::open(state.path(), dir.path());
        let target = RunTarget::Request {
            file: "list.yaml".into(),
            index: 1,
        };
        let entries = run_and_list(cx, window, &ws, target.clone(), &history);
        assert!(entries[0].results[0].name.contains("/status/201"));
        rerun(cx, window, &ws, &entries[0]);
        ws.read_with(cx, |ws, _| assert_eq!(ws.runs.target(), Some(&target)));
        wait_for_run(cx, &ws);
    }

    #[gpui_kit::test]
    fn rerun_uses_the_entry_environment(cx: &mut TestAppContext) {
        let srv = TestServer::start();
        let state = tempfile::tempdir().unwrap();
        let yaml = format!("request:\n  url: {}/status/200\n", srv.base_url);
        let (dir, window, ws) = open(
            cx,
            &[("a.yaml", yaml), (".env.staging", "A=1\n".into())],
            state.path(),
        );
        let history = History::open(state.path(), dir.path());
        ws.update(cx, |ws, cx| ws.set_environment("staging", cx));
        let target = RunTarget::File("a.yaml".into());
        let mut entries = run_and_list(cx, window, &ws, target, &history);
        ws.update(cx, |ws, cx| ws.set_environment("Default", cx));
        rerun(cx, window, &ws, &entries[0]);
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.environment, "staging", "the picker follows the entry");
            assert_eq!(ws.runs.env(), "staging");
        });
        wait_for_run(cx, &ws);
        // An environment that is gone: a notice, no run.
        entries[0].environment = "qa".into();
        rerun(cx, window, &ws, &entries[0]);
        ws.read_with(cx, |ws, _| {
            assert!(ws.runs.active.is_none());
            assert_eq!(ws.environment, "staging");
        });
        assert_eq!(history.list().len(), 2);
    }
}
