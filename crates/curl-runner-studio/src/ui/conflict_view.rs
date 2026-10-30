//! The sync conflict screen (board: SyncConflict): a file changed on both
//! sides, your version beside the server's as a line diff, and `Keep mine`,
//! `Take theirs` or `Keep both`. It takes the main area and the inspector's
//! column while it is open (the footer's `Resolve` and the status bar open
//! it; esc leaves it, the conflicts stay). With several, it steps through
//! them (`Conflict N of M`).
//!
//! What each choice does is [`crate::model::conflict`]'s; the workspace
//! checks unsaved edits first, pushes, settles the sync base and, once the
//! last conflict is gone, syncs again. A received copy only takes theirs:
//! the owner's version wins.

use super::app::{CloseConflicts, mono_font};
use super::color;
use super::icons::{self, glyph};
use super::workspace::{Then, Workspace};
use crate::model::backend::{BackendError, Conflict, SyncFile};
use crate::model::conflict::{
    Choice, DiffRow, Push, ResolveError, line_diff, resolve_on_disk, server_copy_name,
};
use crate::model::history::relative_time;
use chrono::{DateTime, Local, Utc};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(super) enum ConflictEvent {
    Close,
    Resolve(Choice),
}

pub(super) struct ConflictView {
    root: PathBuf,
    conflicts: Vec<Conflict>,
    ix: usize,
    /// A received copy: only `Take theirs`.
    read_only: bool,
    /// A resolution or a sync is in flight: the choices wait.
    busy: bool,
    /// Why the last resolution did not happen.
    pub(super) error: Option<String>,
    /// The shown conflict's rows, and what they were made from (or are
    /// being made from: the diff runs off the UI thread).
    rows: Rc<Vec<DiffRow>>,
    rows_of: Option<(PathBuf, u64, u64)>,
    diff: Option<Task<()>>,
    /// When the local file was last written (`edited … ago`).
    local_at: Option<DateTime<Utc>>,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
}

impl EventEmitter<ConflictEvent> for ConflictView {}

impl Focusable for ConflictView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// `rates/get.yaml` → `get.yaml`.
fn file_name(rel: &Path) -> String {
    rel.file_name().map_or_else(
        || rel.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The line under the diff: what Keep both leaves behind (a file that
/// runs with the collection like any other), or why there is one choice.
fn note_text(c: &Conflict, read_only: bool) -> String {
    if read_only {
        format!(
            "Shared with you: {}'s version is the one kept.",
            c.server_author
        )
    } else if c.server_deleted {
        String::new()
    } else {
        let copy = server_copy_name(&c.rel, 1);
        format!(
            "Keep both saves {}'s version as {} next to it. It runs with the \
             collection until you remove it.",
            c.server_author,
            file_name(&copy)
        )
    }
}

fn ago(at: DateTime<Utc>) -> String {
    relative_time(at.with_timezone(&Local), Local::now())
}

impl ConflictView {
    pub(super) fn new(root: PathBuf, cx: &mut Context<Self>) -> Self {
        ConflictView {
            root,
            conflicts: Vec::new(),
            ix: 0,
            read_only: false,
            busy: false,
            error: None,
            rows: Rc::default(),
            rows_of: None,
            diff: None,
            local_at: None,
            scroll: UniformListScrollHandle::new(),
            focus: cx.focus_handle(),
        }
    }

    /// The workspace's conflicts, as they are now. The shown one stays
    /// shown while it is there.
    pub(super) fn set_conflicts(
        &mut self,
        conflicts: Vec<Conflict>,
        read_only: bool,
        busy: bool,
        cx: &mut Context<Self>,
    ) {
        let shown = self.current().map(|c| c.rel.clone());
        self.ix = shown
            .and_then(|rel| conflicts.iter().position(|c| c.rel == rel))
            .unwrap_or(self.ix)
            .min(conflicts.len().saturating_sub(1));
        self.conflicts = conflicts;
        self.read_only = read_only;
        self.busy = busy;
        self.refresh_rows(cx);
        cx.notify();
    }

    pub(super) fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        if busy {
            self.error = None;
        }
        cx.notify();
    }

    pub(super) fn set_error(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        self.error = error;
        cx.notify();
    }

    pub(super) fn current(&self) -> Option<&Conflict> {
        self.conflicts.get(self.ix)
    }

    /// `Conflict N of M`.
    pub(super) fn position(&self) -> (usize, usize) {
        (self.ix + 1, self.conflicts.len())
    }

    /// Next (`1`) or previous (`-1`), wrapping.
    pub(super) fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let n = self.conflicts.len();
        if n > 1 {
            self.ix = (self.ix as isize + by).rem_euclid(n as isize) as usize;
            self.error = None;
            self.refresh_rows(cx);
            cx.notify();
        }
    }

    /// Re-diffs only when the shown texts changed, in the background:
    /// the rows show once the diff is done.
    fn refresh_rows(&mut self, cx: &mut Context<Self>) {
        let Some(c) = self.conflicts.get(self.ix) else {
            self.rows = Rc::default();
            self.rows_of = None;
            self.diff = None;
            return;
        };
        let key = (
            c.rel.clone(),
            SyncFile::hash_of(&c.local),
            SyncFile::hash_of(&c.server),
        );
        if self.rows_of.as_ref() == Some(&key) {
            return;
        }
        self.rows = Rc::default();
        self.local_at = None;
        self.rows_of = Some(key.clone());
        self.scroll = UniformListScrollHandle::new();
        let (local, server, path) = (c.local.clone(), c.server.clone(), self.root.join(&c.rel));
        let work = cx.background_spawn(async move {
            let at = std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .map(DateTime::<Utc>::from);
            (line_diff(&local, &server), at)
        });
        self.diff = Some(cx.spawn(async move |this, cx| {
            let (rows, at) = work.await;
            this.update(cx, |this, cx| {
                // Another conflict (or text) may be shown by now.
                if this.rows_of.as_ref() == Some(&key) {
                    this.rows = Rc::new(rows);
                    this.local_at = at;
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    fn offered(&self, choice: Choice) -> bool {
        self.current()
            .is_some_and(|c| choice.offered(c, self.read_only))
    }
}

/// The board's `.btn`: 32px, 13px text.
fn button(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .h(px(32.))
        .px(px(12.))
        .text_size(px(13.))
        .label(label)
}

/// A YAML line in the editor's token colors: key, `:`, then a number,
/// `${VAR}`s or a string; comments whole.
fn yaml_line(text: &str) -> StyledText {
    let style = |token: &str| HighlightStyle {
        color: Some(color(token)),
        ..Default::default()
    };
    let mut marks: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    let body = text.trim_start();
    let mut at = text.len() - body.len();
    if body.starts_with('#') {
        marks.push((at..text.len(), style("syntax-comment")));
        return StyledText::new(SharedString::from(text.to_string())).with_highlights(marks);
    }
    if let Some(rest) = text[at..].strip_prefix("- ") {
        marks.push((at..at + 1, style("syntax-punctuation")));
        at = text.len() - rest.len();
    }
    let colon = text[at..]
        .find(": ")
        .map(|i| at + i)
        .or_else(|| text.ends_with(':').then(|| text.len() - 1));
    let value_at = match colon {
        Some(c) if c > at => {
            marks.push((at..c, style("syntax-key")));
            marks.push((c..c + 1, style("syntax-punctuation")));
            c + 1
        }
        _ => at,
    };
    let value = text[value_at..].trim();
    if !value.is_empty() {
        let start = value_at + text[value_at..].find(value).unwrap_or(0);
        let end = start + value.len();
        let number = value.parse::<f64>().is_ok() || matches!(value, "true" | "false" | "null");
        if number {
            marks.push((start..end, style("syntax-number")));
        } else {
            let mut from = start;
            while let Some(open) = text[from..end].find("${") {
                let open = from + open;
                let Some(close) = text[open..end].find('}') else {
                    break;
                };
                let close = open + close + 1;
                if open > from {
                    marks.push((from..open, style("syntax-string")));
                }
                marks.push((open..close, style("syntax-variable")));
                from = close;
            }
            if from < end {
                marks.push((from..end, style("syntax-string")));
            }
        }
    }
    StyledText::new(SharedString::from(text.to_string())).with_highlights(marks)
}

/// One side of a diff row: number, `−`/`+`, the line.
fn diff_cell(cell: Option<&crate::model::conflict::DiffCell>, left: bool) -> Div {
    let side = div()
        .flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .items_center()
        .overflow_hidden();
    let Some(c) = cell else {
        return side.bg(color("surface-raised"));
    };
    let (mark, token, bg) = if left {
        ("−", "danger", "danger-subtle")
    } else {
        ("+", "success", "success-subtle")
    };
    side.when(c.changed, |d| d.bg(color(bg)))
        .child(
            div()
                .flex_none()
                .w(px(36.))
                .pr(px(10.))
                .text_right()
                .text_color(color("text-gutter"))
                .child(c.line.to_string()),
        )
        .child(
            div()
                .flex_none()
                .w(px(14.))
                .text_color(color(token))
                .when(c.changed, |d| d.child(mark)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pl(px(14.))
                .whitespace_nowrap()
                .overflow_hidden()
                // Unchanged lines step back.
                .when(!c.changed, |d| d.opacity(0.6))
                .child(yaml_line(&c.text)),
        )
}

impl Render for ConflictView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(c) = self.current().cloned() else {
            return div().id("conflict-view").flex_1().into_any_element();
        };
        let name = file_name(&c.rel);
        let (n, m) = self.position();
        let theirs_at = ago(c.server_at);
        let title = format!("{name} changed on both sides");
        let subtitle = format!(
            "You edited it offline, and {} pushed a change {theirs_at}. Pick which version to keep.",
            c.server_author
        );
        let count = format!("Conflict {n} of {m}");
        let local_meta = if c.local_deleted {
            "deleted offline".to_string()
        } else {
            match self.local_at {
                Some(at) => format!("edited offline · {}", ago(at)),
                None => "edited offline".to_string(),
            }
        };
        let server_meta = if c.server_deleted {
            format!("deleted {theirs_at}")
        } else {
            format!("pushed {theirs_at}")
        };
        let header = |id: &'static str, title: String, meta: String| {
            div()
                .id(id)
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(px(8.))
                .h(px(36.))
                .px(px(14.))
                .text_size(px(12.))
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-title")))
                        .test_support()
                        .aria_label(title.clone())
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-meta")))
                        .test_support()
                        .aria_label(meta.clone())
                        .text_color(color("text-tertiary"))
                        .child(meta),
                )
        };
        let rows = self.rows.clone();
        let list = uniform_list("conflict-rows", rows.len(), move |range, _, _| {
            range
                .map(|i| {
                    let r = &rows[i];
                    div()
                        .flex()
                        .w_full()
                        .h(px(20.))
                        .child(diff_cell(r.left.as_ref(), true))
                        .child(div().flex_none().w(px(1.)))
                        .child(diff_cell(r.right.as_ref(), false))
                })
                .collect()
        })
        .track_scroll(&self.scroll)
        .flex_1()
        .py(px(8.))
        .font_family(mono_font())
        .text_size(px(12.))
        .line_height(px(20.))
        .text_color(color("syntax-punctuation"));
        let empty_side = |deleted: bool| {
            deleted.then(|| {
                div()
                    .px(px(14.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child("Deleted")
            })
        };
        let diff = div()
            .id("conflict-diff")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .mx(px(24.))
            .border_1()
            .border_color(color("border-100"))
            .rounded(px(8.))
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .border_b_1()
                    .border_color(color("border-100"))
                    .bg(color("surface-raised"))
                    .child(header("conflict-mine", "Your version".into(), local_meta))
                    .child(div().flex_none().w(px(1.)).bg(color("border-100")))
                    .child(header(
                        "conflict-theirs",
                        format!("{}'s version", c.server_author),
                        server_meta,
                    )),
            )
            .when(c.local_deleted || c.server_deleted, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_none()
                        .child(div().flex_1().children(empty_side(c.local_deleted)))
                        .child(div().flex_1().children(empty_side(c.server_deleted))),
                )
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(list)
                    // The column rule, the full height however short the file.
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(0.5))
                            .w(px(1.))
                            .bg(color("border-100")),
                    ),
            );
        let busy = self.busy;
        let choice = |id: &'static str, label: &'static str, which: Choice| {
            button(id, label)
                .disabled(busy)
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(ConflictEvent::Resolve(which))))
        };
        let keep_both = self
            .offered(Choice::KeepBoth)
            .then(|| choice("conflict-keep-both", "Keep both", Choice::KeepBoth).outline());
        let keep_mine = self
            .offered(Choice::KeepMine)
            .then(|| choice("conflict-keep-mine", "Keep mine", Choice::KeepMine).primary());
        let theirs = choice("conflict-take-theirs", "Take theirs", Choice::TakeTheirs);
        let theirs = if self.read_only {
            theirs.primary()
        } else {
            theirs.outline()
        };
        let note = note_text(&c, self.read_only);
        let nav = |id: &'static str, svg: &'static str, by: isize| {
            div()
                .id(id)
                .role(Role::Button)
                .test_support()
                .aria_label(if by < 0 {
                    "Previous conflict"
                } else {
                    "Next conflict"
                })
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(24.))
                .rounded(px(6.))
                .text_color(color("text-secondary"))
                .hover(|s| s.bg(color("surface-100")))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.step(by, cx)))
                .child(glyph(svg).size(px(14.)))
        };
        let stepper = (m > 1).then(|| {
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .child(nav("conflict-prev", icons::CHEVRON_LEFT, -1))
                .child(nav("conflict-next", icons::CHEVRON_RIGHT, 1))
        });
        div()
            .id("conflict-view")
            .key_context("ConflictView")
            .track_focus(&self.focus)
            .role(Role::Main)
            .test_support()
            .aria_label("Sync conflict")
            .on_action(cx.listener(|_, _: &CloseConflicts, _, cx| cx.emit(ConflictEvent::Close)))
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(color("bg-100"))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_start()
                    .gap(px(12.))
                    .px(px(24.))
                    .pt(px(20.))
                    .pb(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(32.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(color("warning-border"))
                            .bg(color("warning-subtle"))
                            .text_color(color("warning"))
                            .child(glyph(icons::WARNING).size(px(13.))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(px(6.))
                            .child(
                                div()
                                    .id("conflict-title")
                                    .role(Role::Heading)
                                    .test_support()
                                    .aria_label(title.clone())
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .id("conflict-subtitle")
                                    .test_support()
                                    .aria_label(subtitle.clone())
                                    .text_size(px(13.))
                                    .line_height(relative(1.5))
                                    .text_color(color("text-secondary"))
                                    .child(subtitle),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(8.))
                            .pt(px(4.))
                            .child(
                                div()
                                    .id("conflict-count")
                                    .test_support()
                                    .aria_label(count.clone())
                                    .text_size(px(12.))
                                    .text_color(color("text-secondary"))
                                    .child(count),
                            )
                            .children(stepper)
                            .child(
                                div()
                                    .id("conflict-close")
                                    .role(Role::Button)
                                    .test_support()
                                    .aria_label("Close")
                                    .flex()
                                    .flex_none()
                                    .items_center()
                                    .justify_center()
                                    .size(px(28.))
                                    .rounded(px(6.))
                                    .text_color(color("text-secondary"))
                                    .hover(|s| s.bg(color("surface-100")))
                                    .cursor_pointer()
                                    .on_click(
                                        cx.listener(|_, _, _, cx| cx.emit(ConflictEvent::Close)),
                                    )
                                    .child(glyph(icons::CROSS).size(px(14.))),
                            ),
                    ),
            )
            .child(diff)
            .child(
                div()
                    .id("conflict-note")
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(16.))
                    .px(px(24.))
                    .pt(px(14.))
                    .pb(px(18.))
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .map(|d| match &self.error {
                        Some(e) => d.child(
                            div()
                                .id("conflict-error")
                                .role(Role::Alert)
                                .test_support()
                                .aria_label(e.clone())
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(color("danger"))
                                .child(glyph(icons::WARNING).size(px(13.)).flex_none())
                                .child(e.clone()),
                        ),
                        None => d.child(note),
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .pl(px(24.))
                    .pr(px(16.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(color("border-100"))
                    .bg(color("surface-raised"))
                    .children(keep_both)
                    .child(div().flex_1())
                    .child(theirs)
                    .children(keep_mine),
            )
            .into_any_element()
    }
}

impl Workspace {
    pub(super) fn conflicts_shown(&self) -> bool {
        self.show_conflicts && self.conflict_view.is_some() && !self.sync.conflicts.is_empty()
    }

    /// Shows the conflict screen over the editor (the footer's `Resolve`,
    /// the status bar).
    pub(super) fn open_conflicts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.state.collection.as_ref().map(|c| c.root.clone()) else {
            return;
        };
        if self.sync.conflicts.is_empty() {
            return;
        }
        let view = match &self.conflict_view {
            Some(v) => v.clone(),
            None => {
                let view = cx.new(|cx| ConflictView::new(root, cx));
                self._conflict_events =
                    Some(cx.subscribe_in(&view, window, Self::on_conflict_event));
                self.conflict_view = Some(view.clone());
                view
            }
        };
        self.show_conflicts = true;
        self.show_history = false;
        self.show_run = false;
        self.settings_view = None;
        self.close_app_settings();
        self.close_hover(cx);
        self.conflicts_changed(cx);
        view.read(cx).focus_handle(cx).focus(window, cx);
        self.sync_chrome(window, cx);
    }

    /// Esc: back to the editor; the conflicts stay pending.
    pub(super) fn hide_conflicts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.show_conflicts {
            return;
        }
        self.show_conflicts = false;
        if self.state.active.is_some() {
            self.focus_active_editor(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
        self.sync_chrome(window, cx);
    }

    /// The screen follows the workspace's conflicts.
    pub(super) fn conflicts_changed(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.conflict_view.clone() else {
            return;
        };
        let conflicts = self.sync.conflicts.clone();
        let read_only = self.pull_only();
        let busy = self.sync.running;
        view.update(cx, |v, cx| v.set_conflicts(conflicts, read_only, busy, cx));
    }

    fn on_conflict_event(
        &mut self,
        _: &Entity<ConflictView>,
        event: &ConflictEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ConflictEvent::Close => self.hide_conflicts(window, cx),
            ConflictEvent::Resolve(choice) => self.resolve_shown(*choice, false, window, cx),
        }
    }

    fn conflict_error(&mut self, error: String, cx: &mut Context<Self>) {
        if let Some(view) = &self.conflict_view {
            view.update(cx, |v, cx| v.set_error(Some(error), cx));
        }
    }

    /// Resolves the shown conflict with `choice`: unsaved edits to the file
    /// are saved or dropped first, a local delete asks first, then the disk
    /// part and the push run off the UI thread. One at a time, never during
    /// a sync.
    pub(super) fn resolve_shown(
        &mut self,
        choice: Choice,
        confirmed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.conflict_view.clone() else {
            return;
        };
        let Some(c) = view.read(cx).current().cloned() else {
            return;
        };
        let (Some(root), Some(key)) = (
            self.state.collection.as_ref().map(|c| c.root.clone()),
            self.collection_key(cx).filter(|_| self.sync_on()),
        ) else {
            return;
        };
        let read_only = self.pull_only();
        if self.sync.running || !choice.offered(&c, read_only) {
            return;
        }
        let again: Then =
            Rc::new(move |this, window, cx| this.resolve_shown(choice, confirmed, window, cx));
        if let Some(ix) = self.state.position(&c.rel)
            && self.state.open[ix].is_dirty()
        {
            self.settle_file(ix, again, window, cx);
            return;
        }
        if choice == Choice::TakeTheirs && c.server_deleted && !confirmed {
            self.confirm_delete_local(&c, window, cx);
            return;
        }
        self.sync.running = true;
        view.update(cx, |v, cx| v.set_busy(true, cx));
        self.sync_changed(cx);
        let backend = self.backend.clone();
        let base = self.store.base().map(Path::to_path_buf);
        let id = key.id.clone();
        // Another collection opened meanwhile: its sync state is not ours.
        let same =
            move |this: &Workspace, cx: &App| this.collection_key(cx).is_some_and(|k| k.id == id);
        cx.spawn_in(window, async move |this, cx| {
            let (disk_root, disk_c) = (root.clone(), c.clone());
            let disk = cx
                .background_spawn(async move {
                    let base = base.as_deref();
                    resolve_on_disk(&disk_root, &disk_c, choice, read_only, base)
                })
                .await;
            let resolved = match disk {
                Ok(r) => r,
                Err(e) => {
                    this.update_in(cx, |this, window, cx| {
                        if !same(this, cx) {
                            return;
                        }
                        this.resolve_failed(e.to_string(), window, cx);
                        if e == ResolveError::ChangedSince {
                            this.run_sync(window, cx);
                        }
                    })
                    .ok();
                    return;
                }
            };
            let pushed = match resolved.push.clone() {
                None => Ok(None),
                Some(push) => {
                    let (files, gone) = match push {
                        Push::File(f) => (vec![f], Vec::new()),
                        Push::Delete(d) => (Vec::new(), vec![d]),
                    };
                    let Ok(call) = this.update(cx, |_, cx| {
                        super::backend_call(cx, &backend, move |b| b.sync(&key, &files, &gone))
                    }) else {
                        return;
                    };
                    call.await.map(Some)
                }
            };
            this.update_in(cx, |this, window, cx| match pushed {
                _ if !same(this, cx) => {}
                Ok(Some(report)) => {
                    match report.conflicts.into_iter().find(|x| x.rel == c.rel) {
                        // Changed on the server again meanwhile: shown anew.
                        Some(newer) => {
                            let author = newer.server_author.clone();
                            this.replace_conflict(newer, cx);
                            this.resolve_failed(
                                format!("{author} changed it again. Pick once more."),
                                window,
                                cx,
                            );
                        }
                        None => this.settle(&c, resolved.base, resolved.touched, window, cx),
                    }
                }
                Ok(None) => {
                    let removed = c.server_deleted && choice == Choice::TakeTheirs;
                    if removed {
                        for ix in this.state.tabs_under(&c.rel) {
                            this.state.close(ix);
                            this.tabs.remove(ix);
                        }
                    }
                    this.settle(&c, resolved.base, resolved.touched, window, cx);
                }
                // Keep both's copy stays on disk: it holds the server's text,
                // so a retry finds it and reuses it rather than adding a
                // `.server-2.yaml`; left alone, it syncs like any new file.
                Err(e) => {
                    let text = match e {
                        BackendError::Offline => {
                            "You're offline. The conflict stays until you're back.".to_string()
                        }
                        e => format!("Couldn't resolve it: {e}"),
                    };
                    this.resolve_failed(text, window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The resolution did not happen: say why; the conflict stays.
    fn resolve_failed(&mut self, error: String, window: &mut Window, cx: &mut Context<Self>) {
        self.sync.running = false;
        self.conflicts_changed(cx);
        self.conflict_error(error, cx);
        if std::mem::take(&mut self.sync.again) {
            self.run_sync(window, cx);
        }
        self.sync_changed(cx);
    }

    fn replace_conflict(&mut self, newer: Conflict, cx: &mut Context<Self>) {
        for list in [&mut self.sync.conflicts, &mut self.sync.base.conflicts] {
            if let Some(slot) = list.iter_mut().find(|x| x.rel == newer.rel) {
                *slot = newer.clone();
            }
        }
        self.save_base(cx);
    }

    /// The conflict on `c.rel` is settled: the base takes `base` (or drops
    /// the file), the conflict and its hold go, open tabs re-read what was
    /// written. The last one closes the screen and syncs again.
    fn settle(
        &mut self,
        c: &Conflict,
        base: Option<u64>,
        touched: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync.running = false;
        let sync = &mut self.sync;
        match base {
            Some(hash) => sync.base.files.insert(c.rel.clone(), hash),
            None => sync.base.files.remove(&c.rel),
        };
        sync.conflicts.retain(|x| x.rel != c.rel);
        sync.base.conflicts.retain(|x| x.rel != c.rel);
        sync.base.held.retain(|r| *r != c.rel);
        self.save_base(cx);
        self.conflicts_changed(cx);
        if let Some(view) = &self.conflict_view {
            view.update(cx, |v, cx| v.set_busy(false, cx));
        }
        if touched.is_empty() {
            // The waiting count follows the new base.
            self.mark_changed(window, cx);
        } else {
            self.disk_changed(&touched, window, cx);
        }
        let last = self.sync.conflicts.is_empty();
        if last {
            self.hide_conflicts(window, cx);
        }
        if last || std::mem::take(&mut self.sync.again) {
            self.sync.again = false;
            self.run_sync(window, cx);
        }
        self.sync_changed(cx);
    }

    /// Take theirs when the server deleted the file removes it here: asked
    /// first.
    fn confirm_delete_local(&mut self, c: &Conflict, window: &mut Window, cx: &mut Context<Self>) {
        let name = file_name(&c.rel);
        let ws = cx.entity().downgrade();
        let author = c.server_author.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            let ws = ws.clone();
            dialog
                .w(px(480.))
                .title(format!("Delete {name}?"))
                .child(div().text_color(color("text-secondary")).child(format!(
                    "{author} deleted it on the server. Your copy is removed too."
                )))
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("conflict-delete-cancel")
                                .ghost()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("conflict-delete")
                                .danger()
                                .label("Delete")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    ws.update(cx, |this, cx| {
                                        this.resolve_shown(Choice::TakeTheirs, true, window, cx)
                                    })
                                    .ok();
                                }),
                        ),
                )
        });
    }
}

impl Workspace {
    /// Development aid (`CURL_RUNNER_STUDIO_SHOW=conflict`): the screen on
    /// the conflicts kept from the last sync; with none, the mock conflicts
    /// on the next sync, a sync runs, and the screen opens once it is in.
    pub(super) fn dev_show_conflict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // One kept from the last launch: at once (a background window
        // draws only its first frame).
        if !self.sync.conflicts.is_empty() {
            return self.open_conflicts(window, cx);
        }
        if let Some(dev) = self.backend.dev() {
            dev.conflict_next_sync(true);
        }
        self.sync_now(window, cx);
        cx.spawn_in(window, async move |this, cx| {
            for _ in 0..100 {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                let done = this
                    .update_in(cx, |this, window, cx| {
                        let ready = !this.sync.conflicts.is_empty() && !this.sync.running;
                        if ready {
                            this.open_conflicts(window, cx);
                        }
                        ready
                    })
                    .unwrap_or(true);
                if done {
                    break;
                }
            }
        })
        .detach();
    }
}

#[cfg(test)]
mod note_tests {
    use super::{Conflict, note_text};

    #[test]
    fn keep_both_says_the_copy_runs_with_the_collection() {
        let c = Conflict {
            rel: "rates/get.yaml".into(),
            local: "a: 1\n".into(),
            server: "a: 2\n".into(),
            server_author: "Sam".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        assert_eq!(
            note_text(&c, false),
            "Keep both saves Sam's version as get.server.yaml next to it. It runs with the \
             collection until you remove it."
        );
        assert_eq!(
            note_text(&c, true),
            "Shared with you: Sam's version is the one kept."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::StateStore;
    use super::super::sync::{SYNC_DELAY, SyncLine};
    use super::super::workspace::{Workspace, test_backend, test_secrets};
    use crate::model::backend::{Backend, Conflict, ShareUpdate, SignInMethod, SyncFile};
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;
    use std::sync::Arc;

    const BASE: &str = "request:\n  url: https://api.example.com/a\n  timeout: 30\n";
    const MINE: &str = "request:\n  url: https://api.example.com/a\n  timeout: 40\n";

    struct Studio {
        dir: tempfile::TempDir,
        _state: tempfile::TempDir,
        backend: Arc<dyn Backend>,
        window: AnyWindowHandle,
        ws: Entity<Workspace>,
    }

    impl Studio {
        fn read(&self, rel: &str) -> String {
            std::fs::read_to_string(self.dir.path().join(rel)).unwrap()
        }

        fn update<R>(
            &self,
            cx: &mut TestAppContext,
            f: impl FnOnce(
                &mut Workspace,
                &mut gpui_kit::Window,
                &mut gpui_kit::Context<Workspace>,
            ) -> R,
        ) -> R {
            let ws = self.ws.clone();
            let out = cx
                .update_window(self.window, |_, window, cx| {
                    ws.update(cx, |ws, cx| f(ws, window, cx))
                })
                .unwrap();
            cx.run_until_parked();
            out
        }

        fn click(&self, cx: &mut TestAppContext, id: &'static str) {
            cx.update_window(self.window, |_, window, cx| {
                window.render_frame(cx);
                window.click(id, cx);
            })
            .unwrap();
            cx.run_until_parked();
        }

        fn shows(&self, cx: &mut TestAppContext, label: &str) -> bool {
            cx.update_window(self.window, |_, window, cx| {
                window.render_frame(cx);
                window.try_find_by_label(label).is_some()
            })
            .unwrap()
        }

        fn has(&self, cx: &mut TestAppContext, id: &'static str) -> bool {
            cx.update_window(self.window, |_, window, cx| {
                window.render_frame(cx);
                window.try_find(id).is_some()
            })
            .unwrap()
        }

        fn conflicts(&self, cx: &mut TestAppContext) -> Vec<Conflict> {
            self.ws.read_with(cx, |ws, _| ws.sync.conflicts.clone())
        }

        fn line(&self, cx: &mut TestAppContext) -> SyncLine {
            self.ws.read_with(cx, |ws, _| ws.sync_line())
        }

        /// The server's copy of each file.
        fn server(&self, cx: &mut TestAppContext) -> Vec<SyncFile> {
            let key = self
                .ws
                .read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
            futures::executor::block_on(self.backend.update_share(&key, ShareUpdate::default()))
                .unwrap();
            futures::executor::block_on(self.backend.fetch_shared(&key.id))
                .unwrap()
                .files
        }

        fn server_text(&self, cx: &mut TestAppContext, rel: &str) -> Option<String> {
            self.server(cx)
                .into_iter()
                .find(|f| f.rel == Path::new(rel))
                .map(|f| f.text)
        }

        fn syncs(&self) -> usize {
            let dev = self.backend.dev().unwrap();
            dev.recorded()
                .iter()
                .filter(|c| c.starts_with("sync("))
                .count()
        }

        /// Opens `rel`, replaces its text and saves it.
        fn save(&self, cx: &mut TestAppContext, rel: &str, text: &str) {
            self.update(cx, |ws, window, cx| {
                ws.open_file(Path::new(rel), window, cx);
                let ix = ws.state.active.unwrap();
                ws.state.open[ix].set_text(text.into());
                assert!(ws.save(ix, window, cx));
            });
        }

        fn sync_now(&self, cx: &mut TestAppContext) {
            self.update(cx, |ws, window, cx| ws.sync_now(window, cx));
        }

        fn open_conflicts(&self, cx: &mut TestAppContext) {
            self.click(cx, "sidebar-resolve");
            assert!(self.ws.read_with(cx, |ws, _| ws.conflicts_shown()));
        }
    }

    /// A signed-in studio on a collection of `files`, its state on disk.
    fn studio(cx: &mut TestAppContext, files: &[(&str, &str)], automatic: bool) -> Studio {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let state = tempfile::tempdir().unwrap();
        if !automatic {
            std::fs::write(
                state.path().join("settings.json"),
                r#"{"sync":{"automatic":false}}"#,
            )
            .unwrap();
        }
        let backend = test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (root, store, b) = (
            dir.path().to_path_buf(),
            StateStore::at(state.path()),
            backend.clone(),
        );
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
                cx.new(|cx| Workspace::new(Some(root), store, test_secrets(), b, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        let s = Studio {
            dir,
            _state: state,
            backend,
            window,
            ws,
        };
        if !automatic {
            // The first sync seeds the server and the base.
            s.sync_now(cx);
        }
        s
    }

    /// One conflict on `a.yaml`: saved here as [`MINE`], Sam's on the
    /// server. Returns Sam's text.
    fn conflicted(cx: &mut TestAppContext) -> (Studio, String) {
        let s = studio(cx, &[("a.yaml", BASE), ("b.yaml", BASE)], true);
        s.backend.dev().unwrap().conflict_next_sync(true);
        s.save(cx, "a.yaml", MINE);
        cx.executor().advance_clock(SYNC_DELAY);
        cx.run_until_parked();
        assert_eq!(s.line(cx), SyncLine::Conflicts(1));
        let theirs = s.conflicts(cx)[0].server.clone();
        assert_ne!(theirs, MINE);
        (s, theirs)
    }

    /// No conflict comes back, and the server and disk agree.
    fn settled(s: &Studio, cx: &mut TestAppContext) {
        assert!(s.conflicts(cx).is_empty());
        assert!(!s.ws.read_with(cx, |ws, _| ws.conflicts_shown()));
        assert!(
            matches!(s.line(cx), SyncLine::Synced(_)),
            "{:?}",
            s.line(cx)
        );
        s.sync_now(cx);
        assert!(s.conflicts(cx).is_empty(), "nothing conflicts next time");
        assert!(matches!(s.line(cx), SyncLine::Synced(_)));
    }

    #[gpui_kit::test]
    fn conflict_view_reads_like_the_board(cx: &mut TestAppContext) {
        let (s, _) = conflicted(cx);
        s.open_conflicts(cx);
        assert!(s.shows(cx, "a.yaml changed on both sides"));
        assert!(s.shows(
            cx,
            "You edited it offline, and Sam pushed a change 1 h ago. Pick which version to keep."
        ));
        assert!(s.shows(cx, "Conflict 1 of 1"));
        assert!(s.shows(cx, "Your version"));
        assert!(s.shows(cx, "Sam's version"));
        assert!(s.shows(cx, "pushed 1 h ago"));
        for id in [
            "conflict-keep-mine",
            "conflict-take-theirs",
            "conflict-keep-both",
        ] {
            assert!(s.has(cx, id), "{id}");
        }
        assert!(!s.has(cx, "conflict-next"), "one conflict: no stepper");
        // The diff marks the changed line on both sides.
        let rows = s.ws.read_with(cx, |ws, cx| {
            ws.conflict_view.as_ref().unwrap().read(cx).rows.clone()
        });
        let changed: Vec<_> = rows
            .iter()
            .filter(|r| r.left.as_ref().is_some_and(|c| c.changed))
            .collect();
        assert_eq!(changed.len(), 1);
        assert!(
            changed[0]
                .left
                .as_ref()
                .unwrap()
                .text
                .contains("timeout: 40")
        );
        // Esc leaves it; the conflict stays.
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.press("escape", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!s.ws.read_with(cx, |ws, _| ws.conflicts_shown()));
        assert_eq!(s.line(cx), SyncLine::Conflicts(1));
        // The status bar opens it too.
        s.click(cx, "status-sync");
        assert!(s.ws.read_with(cx, |ws, _| ws.conflicts_shown()));
    }

    #[gpui_kit::test]
    fn a_tree_click_leaves_the_conflicts_for_the_file(cx: &mut TestAppContext) {
        let (s, _) = conflicted(cx);
        s.open_conflicts(cx);
        assert!(s.ws.read_with(cx, |ws, _| ws.conflicts_shown()));
        // The rows: a.yaml, b.yaml.
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("collection-tree")
                .click(("tree-row", 1usize), cx);
        })
        .unwrap();
        cx.run_until_parked();
        s.ws.read_with(cx, |ws, _| {
            assert!(!ws.conflicts_shown());
            let active = ws.state.active_document().unwrap();
            assert_eq!(active.rel_path, Path::new("b.yaml"));
        });
        assert_eq!(s.line(cx), SyncLine::Conflicts(1), "still pending");
    }

    #[gpui_kit::test]
    fn keep_both_offline_keeps_its_copy_and_a_retry_reuses_it(cx: &mut TestAppContext) {
        let (s, theirs) = conflicted(cx);
        s.open_conflicts(cx);
        s.backend.dev().unwrap().set_offline(true);
        s.click(cx, "conflict-keep-both");
        assert!(s.shows(cx, "You're offline. The conflict stays until you're back."));
        assert_eq!(s.read("a.server.yaml"), theirs, "written before the push");
        assert_eq!(s.line(cx), SyncLine::Conflicts(1));
        s.backend.dev().unwrap().set_offline(false);
        s.click(cx, "conflict-keep-both");
        assert_eq!(s.read("a.server.yaml"), theirs);
        assert!(!s.dir.path().join("a.server-2.yaml").exists(), "reused");
        assert_eq!(s.server_text(cx, "a.yaml").as_deref(), Some(MINE));
        settled(&s, cx);
        assert_eq!(s.server_text(cx, "a.server.yaml"), Some(theirs));
    }

    #[gpui_kit::test]
    fn keep_mine_pushes_local(cx: &mut TestAppContext) {
        let (s, _) = conflicted(cx);
        s.open_conflicts(cx);
        let before = s.syncs();
        s.click(cx, "conflict-keep-mine");
        assert_eq!(s.server_text(cx, "a.yaml").as_deref(), Some(MINE));
        assert_eq!(s.read("a.yaml"), MINE);
        assert!(s.syncs() >= before + 2, "the push, then a sync");
        let key = s.ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        let base = crate::model::sync_base::load(Some(s._state.path()), &key.id);
        assert_eq!(base.files[Path::new("a.yaml")], SyncFile::hash_of(MINE));
        assert!(base.conflicts.is_empty() && base.held.is_empty());
        settled(&s, cx);
    }

    #[gpui_kit::test]
    fn take_theirs_writes_server_text_exactly(cx: &mut TestAppContext) {
        let (s, theirs) = conflicted(cx);
        s.open_conflicts(cx);
        s.click(cx, "conflict-take-theirs");
        assert_eq!(s.read("a.yaml"), theirs);
        assert_eq!(s.server_text(cx, "a.yaml"), Some(theirs.clone()));
        // The open tab took it.
        s.ws.read_with(cx, |ws, _| {
            let ix = ws.state.position(Path::new("a.yaml")).unwrap();
            assert_eq!(ws.state.open[ix].text, theirs);
            assert!(!ws.state.open[ix].is_dirty());
        });
        settled(&s, cx);
    }

    #[gpui_kit::test]
    fn keep_both_writes_server_copy_next_to_local(cx: &mut TestAppContext) {
        let (s, theirs) = conflicted(cx);
        // Taken already: never overwritten.
        std::fs::write(s.dir.path().join("a.server.yaml"), "kept: 1\n").unwrap();
        s.open_conflicts(cx);
        s.click(cx, "conflict-keep-both");
        assert_eq!(s.read("a.yaml"), MINE);
        assert_eq!(s.read("a.server.yaml"), "kept: 1\n");
        assert_eq!(s.read("a.server-2.yaml"), theirs);
        assert_eq!(s.server_text(cx, "a.yaml").as_deref(), Some(MINE));
        assert_eq!(s.server_text(cx, "a.server-2.yaml"), Some(theirs));
        settled(&s, cx);
    }

    /// Both files edited here and by another device: two conflicts.
    fn two_conflicts(cx: &mut TestAppContext) -> Studio {
        let s = studio(cx, &[("a.yaml", BASE), ("b.yaml", BASE)], false);
        let key = s.ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        let theirs: Vec<SyncFile> = ["a.yaml", "b.yaml"]
            .iter()
            .map(|rel| {
                SyncFile::new(*rel, format!("{BASE}  # {rel} theirs\n"))
                    .with_base(Some(SyncFile::hash_of(BASE)))
            })
            .collect();
        futures::executor::block_on(s.backend.sync(&key, &theirs, &[])).unwrap();
        for rel in ["a.yaml", "b.yaml"] {
            std::fs::write(s.dir.path().join(rel), format!("{BASE}  # {rel} mine\n")).unwrap();
        }
        s.sync_now(cx);
        assert_eq!(s.line(cx), SyncLine::Conflicts(2));
        s
    }

    #[gpui_kit::test]
    fn multiple_conflicts_step_through(cx: &mut TestAppContext) {
        let s = two_conflicts(cx);
        s.open_conflicts(cx);
        assert!(s.shows(cx, "Conflict 1 of 2"));
        assert!(s.shows(cx, "a.yaml changed on both sides"));
        s.click(cx, "conflict-next");
        assert!(s.shows(cx, "Conflict 2 of 2"));
        assert!(s.shows(cx, "b.yaml changed on both sides"));
        s.click(cx, "conflict-next");
        assert!(s.shows(cx, "Conflict 1 of 2"), "wraps");
        s.click(cx, "conflict-prev");
        assert!(s.shows(cx, "Conflict 2 of 2"));
        let before = s.syncs();
        s.click(cx, "conflict-take-theirs");
        assert_eq!(s.read("b.yaml"), format!("{BASE}  # b.yaml theirs\n"));
        assert_eq!(s.syncs(), before, "not the last one: no sync yet");
        assert!(s.shows(cx, "Conflict 1 of 1"));
        assert!(s.shows(cx, "a.yaml changed on both sides"));
        s.click(cx, "conflict-keep-mine");
        assert_eq!(
            s.server_text(cx, "a.yaml"),
            Some(format!("{BASE}  # a.yaml mine\n"))
        );
        assert!(s.syncs() >= before + 2, "the push, then the sync");
        settled(&s, cx);
    }

    #[gpui_kit::test]
    fn conflict_never_includes_env_files(cx: &mut TestAppContext) {
        let s = studio(
            cx,
            &[
                (".env", "API_TOKEN=sk_live_dotenv\n"),
                (".env.staging", "API_TOKEN=sk_live_staging\n"),
                ("a.yaml", BASE),
            ],
            true,
        );
        // Nothing changed: the forced conflict takes a synced file.
        s.backend.dev().unwrap().conflict_next_sync(true);
        s.sync_now(cx);
        let conflicts = s.conflicts(cx);
        assert_eq!(conflicts.len(), 1);
        for c in &conflicts {
            assert!(!c.rel.to_string_lossy().contains(".env"), "{:?}", c.rel);
            assert!(!c.local.contains("sk_live") && !c.server.contains("sk_live"));
        }
        // A conflict on a `.env` path (a bad server) is never written.
        let bad = Conflict {
            rel: ".env".into(),
            local: "API_TOKEN=sk_live_dotenv\n".into(),
            server: "API_TOKEN=evil\n".into(),
            ..conflicts[0].clone()
        };
        s.update(cx, |ws, _, _| ws.sync.conflicts.insert(0, bad));
        s.open_conflicts(cx);
        for id in [
            "conflict-take-theirs",
            "conflict-keep-both",
            "conflict-keep-mine",
        ] {
            s.click(cx, id);
        }
        assert_eq!(s.read(".env"), "API_TOKEN=sk_live_dotenv\n");
        assert!(!s.dir.path().join(".env.server").exists());
        assert!(!s.dir.path().join(".env.server.yaml").exists());
        assert!(s.shows(cx, "This file can't be written here"));
        for call in s.backend.dev().unwrap().recorded() {
            assert!(!call.contains(".env"), "{call}");
        }
    }

    #[gpui_kit::test]
    fn changed_since_conflict_refuses_and_syncs_again(cx: &mut TestAppContext) {
        let (s, theirs) = conflicted(cx);
        s.open_conflicts(cx);
        for (i, id) in [
            "conflict-take-theirs",
            "conflict-keep-mine",
            "conflict-keep-both",
        ]
        .into_iter()
        .enumerate()
        {
            // Edited outside the studio since, no watcher event yet.
            let newer = format!("request:\n  url: https://api.example.com/z{i}\n");
            std::fs::write(s.dir.path().join("a.yaml"), &newer).unwrap();
            let before = s.syncs();
            s.click(cx, id);
            assert_eq!(s.read("a.yaml"), newer, "{id}: not overwritten");
            assert!(!s.dir.path().join("a.server.yaml").exists());
            assert_eq!(
                s.server_text(cx, "a.yaml"),
                Some(theirs.clone()),
                "not pushed"
            );
            assert!(s.shows(cx, "This file changed since the conflict — sync again"));
            assert!(s.syncs() > before, "synced again");
            // The fresh conflict has the newer text.
            assert_eq!(s.conflicts(cx)[0].local, newer);
        }
    }

    #[gpui_kit::test]
    fn local_deleted_keep_mine_deletes_on_the_server(cx: &mut TestAppContext) {
        let s = studio(cx, &[("a.yaml", BASE), ("b.yaml", BASE)], false);
        let key = s.ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        let theirs = format!("{BASE}  # theirs\n");
        let pushed =
            SyncFile::new("a.yaml", theirs.clone()).with_base(Some(SyncFile::hash_of(BASE)));
        futures::executor::block_on(s.backend.sync(&key, &[pushed], &[])).unwrap();
        std::fs::remove_file(s.dir.path().join("a.yaml")).unwrap();
        s.sync_now(cx);
        let c = s.conflicts(cx);
        assert!(c.len() == 1 && c[0].local_deleted, "{c:?}");
        s.open_conflicts(cx);
        assert!(s.shows(cx, "deleted offline"));
        s.click(cx, "conflict-keep-mine");
        assert_eq!(s.server_text(cx, "a.yaml"), None, "deleted there too");
        assert!(!s.dir.path().join("a.yaml").exists());
        settled(&s, cx);
    }

    #[gpui_kit::test]
    fn local_deleted_keep_both_restores_theirs_beside(cx: &mut TestAppContext) {
        let s = studio(cx, &[("a.yaml", BASE), ("b.yaml", BASE)], false);
        let key = s.ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        let theirs = format!("{BASE}  # theirs\n");
        let pushed =
            SyncFile::new("a.yaml", theirs.clone()).with_base(Some(SyncFile::hash_of(BASE)));
        futures::executor::block_on(s.backend.sync(&key, &[pushed], &[])).unwrap();
        std::fs::remove_file(s.dir.path().join("a.yaml")).unwrap();
        s.sync_now(cx);
        s.open_conflicts(cx);
        s.click(cx, "conflict-keep-both");
        assert_eq!(s.read("a.server.yaml"), theirs);
        assert!(!s.dir.path().join("a.yaml").exists());
        assert_eq!(s.server_text(cx, "a.yaml"), None);
        assert_eq!(s.server_text(cx, "a.server.yaml"), Some(theirs));
        settled(&s, cx);
    }

    #[gpui_kit::test]
    fn server_deleted_take_theirs_asks_then_removes(cx: &mut TestAppContext) {
        let s = studio(cx, &[("a.yaml", BASE), ("b.yaml", BASE)], false);
        let c = Conflict {
            rel: "a.yaml".into(),
            local: BASE.into(),
            server: String::new(),
            server_author: "Sam".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: true,
        };
        s.update(cx, |ws, window, cx| {
            ws.sync.conflicts = vec![c];
            ws.open_file(Path::new("a.yaml"), window, cx);
        });
        s.open_conflicts(cx);
        assert!(
            !s.has(cx, "conflict-keep-both"),
            "one text: nothing to keep both of"
        );
        s.click(cx, "conflict-take-theirs");
        assert!(s.dir.path().join("a.yaml").exists(), "asked first");
        s.click(cx, "conflict-delete");
        assert!(!s.dir.path().join("a.yaml").exists());
        s.ws.read_with(cx, |ws, _| {
            assert!(!ws.sync.base.files.contains_key(Path::new("a.yaml")));
            assert!(
                ws.state.position(Path::new("a.yaml")).is_none(),
                "its tab closed"
            );
        });
    }

    #[gpui_kit::test]
    fn dirty_buffer_asks_before_resolving(cx: &mut TestAppContext) {
        let (s, _) = conflicted(cx);
        // Typed after the conflict, not saved.
        s.update(cx, |ws, _, _| {
            let ix = ws.state.position(Path::new("a.yaml")).unwrap();
            ws.state.open[ix].set_text("request:\n  url: https://typed\n".into());
        });
        s.open_conflicts(cx);
        s.click(cx, "conflict-keep-mine");
        assert!(
            cx.update_window(s.window, |_, window, cx| window.has_active_dialog(cx))
                .unwrap(),
            "save or discard first"
        );
        assert_eq!(s.conflicts(cx).len(), 1, "nothing resolved yet");
        s.click(cx, "unsaved-discard");
        assert_eq!(s.server_text(cx, "a.yaml").as_deref(), Some(MINE));
        s.ws.read_with(cx, |ws, _| {
            let ix = ws.state.position(Path::new("a.yaml")).unwrap();
            assert_eq!(ws.state.open[ix].text, MINE, "the typing was dropped");
        });
        settled(&s, cx);
    }

    #[gpui_kit::test]
    fn dirty_buffer_saved_counts_as_changed_since(cx: &mut TestAppContext) {
        let (s, theirs) = conflicted(cx);
        let typed = "request:\n  url: https://typed\n";
        s.update(cx, |ws, _, _| {
            let ix = ws.state.position(Path::new("a.yaml")).unwrap();
            ws.state.open[ix].set_text(typed.into());
        });
        s.open_conflicts(cx);
        s.click(cx, "conflict-take-theirs");
        s.click(cx, "unsaved-save");
        assert_eq!(
            s.read("a.yaml"),
            typed,
            "the saved typing is not overwritten"
        );
        assert_eq!(s.server_text(cx, "a.yaml"), Some(theirs));
    }
}
