//! Diagnostics in the editor: squiggles with their hover text, line tints,
//! scrollbar ticks and the quick-fix code actions.

use super::color;
use super::workspace::Workspace;
use crate::model::diagnostics::StudioDiagnostic;
use crate::model::document::FixError;
use curl_runner_core::lint::Severity;
use gpui_kit::base::Scrollbar;
use gpui_kit::base::input::EditorScrollbarContext;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::highlighter::{Diagnostic, DiagnosticSeverity};
use gpui_kit::component::input::{
    CodeActionProvider, EditorState, Position, RangeDecoration, RangeDecorationCollection,
    RangeDecorationStyle,
};
use gpui_kit::component::notification::Notification;
use gpui_kit::*;
use lsp_types::{CodeAction, CodeActionKind};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

/// `(line, character)` of byte `offset`, both zero-based. `character` counts
/// chars, the unit the editor's `Position` uses (see `SPIKE.md`). Offsets
/// past the end or inside a character clamp back.
pub fn byte_to_position(text: &str, offset: usize) -> (u32, u32) {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let line = before.bytes().filter(|&b| b == b'\n').count();
    (line as u32, before[line_start..].chars().count() as u32)
}

fn position(text: &str, offset: usize) -> Position {
    let (line, character) = byte_to_position(text, offset);
    Position::new(line, character)
}

/// The severity's color: `danger`, `warning` or `accent-blue`.
pub fn severity_color(severity: Severity) -> Hsla {
    color(match severity {
        Severity::Error => "danger",
        Severity::Warning => "warning",
        Severity::Info => "accent-blue",
    })
}

/// The hover text: the message, then where it comes from.
fn hover_text(d: &StudioDiagnostic) -> String {
    if d.path.is_empty() {
        format!("{}\n\ncurl-runner", d.message)
    } else {
        format!("{}\n\ncurl-runner · {}", d.message, d.path)
    }
}

fn editor_diagnostic(text: &str, d: &StudioDiagnostic) -> Diagnostic {
    let severity = match d.severity {
        Severity::Error => DiagnosticSeverity::Error,
        Severity::Warning => DiagnosticSeverity::Warning,
        Severity::Info => DiagnosticSeverity::Info,
    };
    // The editor's built-in diagnostic popover is the hover: it opens on the
    // squiggle and shows the message as markdown.
    Diagnostic::new(
        position(text, d.range.start)..position(text, d.range.end),
        hover_text(d),
    )
    .with_severity(severity)
    .with_source("curl-runner")
}

/// Most severe first.
fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

/// Byte range of the line holding `offset`, with its newline.
fn line_range(text: &str, offset: usize) -> Range<usize> {
    let offset = offset.min(text.len());
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = text[start..]
        .find('\n')
        .map_or(text.len(), |i| start + i + 1);
    start..end
}

/// One tint per line with an error or warning, in the most severe color
/// (`diag-error-line`, `diag-warning-line`; info lines are not tinted).
fn line_tints(text: &str, diagnostics: &[StudioDiagnostic]) -> Vec<(Range<usize>, Severity)> {
    let mut lines: Vec<(Range<usize>, Severity)> = Vec::new();
    for d in diagnostics.iter().filter(|d| d.severity != Severity::Info) {
        let range = line_range(text, d.range.start);
        match lines.iter_mut().find(|(r, _)| *r == range) {
            Some((_, s)) if rank(d.severity) < rank(*s) => *s = d.severity,
            Some(_) => {}
            None => lines.push((range, d.severity)),
        }
    }
    lines
}

/// Scrollbar ticks: each diagnostic's line as a fraction of the document.
fn ticks(text: &str, diagnostics: &[StudioDiagnostic]) -> Vec<(f32, Severity)> {
    let lines = text.bytes().filter(|&b| b == b'\n').count() + 1;
    diagnostics
        .iter()
        .map(|d| {
            let (line, _) = byte_to_position(text, d.range.start);
            (line as f32 / lines as f32, d.severity)
        })
        .collect()
}

/// A tab's diagnostics presentation: what the editor shows beyond its own
/// squiggles. Created with the tab's editor.
pub(super) struct DiagnosticsView {
    lines: RangeDecorationCollection,
    ticks: Rc<RefCell<Vec<(f32, Hsla)>>>,
}

impl DiagnosticsView {
    /// Installs the quick-fix provider and the ticked scrollbar on `editor`.
    pub fn attach(
        editor: &Entity<EditorState>,
        workspace: WeakEntity<Workspace>,
        cx: &mut App,
    ) -> Self {
        let ticks: Rc<RefCell<Vec<(f32, Hsla)>>> = Rc::default();
        let painted = ticks.clone();
        let lines = editor.update(cx, |state, cx| {
            state.lsp_mut().code_action_providers = vec![Rc::new(QuickFixes { workspace })];
            state.set_scrollbar_renderer(Some(Rc::new(move |context, _, _| {
                ticked_scrollbar(context, &painted.borrow())
            })));
            state.create_range_decorations_collection(Vec::new(), cx)
        });
        DiagnosticsView { lines, ticks }
    }

    /// Shows `diagnostics` of `text`, which must be the editor's text.
    pub fn show(
        &self,
        editor: &Entity<EditorState>,
        text: &str,
        diagnostics: &[StudioDiagnostic],
        cx: &mut App,
    ) {
        editor.update(cx, |state, cx| {
            if let Some(set) = state.diagnostics_mut() {
                set.clear();
                set.extend(diagnostics.iter().map(|d| editor_diagnostic(text, d)));
            }
            cx.notify();
        });
        let tints = line_tints(text, diagnostics)
            .into_iter()
            .map(|(range, severity)| {
                let token = match severity {
                    Severity::Error => "diag-error-line",
                    _ => "diag-warning-line",
                };
                RangeDecoration::new(range)
                    .with_style(RangeDecorationStyle::Fill)
                    .with_color(color(token))
            })
            .collect();
        self.lines.set(tints, cx);
        *self.ticks.borrow_mut() = ticks(text, diagnostics)
            .into_iter()
            .map(|(at, severity)| (at, severity_color(severity)))
            .collect();
    }
}

/// The editor's own scrollbar with a tick per diagnostic at the right edge.
fn ticked_scrollbar(context: EditorScrollbarContext, ticks: &[(f32, Hsla)]) -> AnyElement {
    div()
        .size_full()
        .relative()
        .child(Scrollbar::new(&context))
        .children(ticks.iter().map(|&(at, tick)| {
            div()
                .absolute()
                .right_0()
                .top(relative(at))
                .w(px(4.))
                .h(px(3.))
                .bg(tick)
        }))
        .into_any_element()
}

/// The quick fixes at the cursor, as code actions (⌘. menu). Each action
/// names its diagnostic by path and fix; the edit is computed when applied.
struct QuickFixes {
    workspace: WeakEntity<Workspace>,
}

impl CodeActionProvider for QuickFixes {
    fn id(&self) -> SharedString {
        "curl-runner".into()
    }

    fn code_actions(
        &self,
        state: Entity<EditorState>,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let actions = self
            .workspace
            .update(cx, |ws, cx| ws.code_actions(&state, range, cx))
            .unwrap_or_default();
        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<EditorState>,
        action: CodeAction,
        _: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let field = |key: &str| {
            action
                .data
                .as_ref()
                .and_then(|d| d.get(key))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let (Some(path), Some(fix)) = (field("path"), field("fix")) else {
            return Task::ready(Ok(()));
        };
        let workspace = self.workspace.clone();
        // This runs while the editor is being updated; edit it afterwards.
        window.defer(cx, move |window, cx| {
            workspace
                .update(cx, |ws, cx| {
                    if let Some(ix) = ws.tab_of(&state) {
                        ws.apply_named_fix(ix, &path, &fix, window, cx);
                    }
                })
                .ok();
        });
        Task::ready(Ok(()))
    }
}

/// Does `d` cover the selection `range` (touching counts)?
fn covers(d: &StudioDiagnostic, range: &Range<usize>) -> bool {
    d.range.start <= range.end && range.start <= d.range.end
}

/// The smallest edit turning `old` into `new`: a byte range of `old` and
/// its replacement, so the editor keeps its undo history and cursor.
fn minimal_edit<'a>(old: &str, new: &'a str) -> (Range<usize>, &'a str) {
    let mut start = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(start) || !new.is_char_boundary(start) {
        start -= 1;
    }
    let max_suffix = old.len().min(new.len()) - start;
    let mut suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    (start..old.len() - suffix, &new[start..new.len() - suffix])
}

impl Workspace {
    pub(super) fn tab_of(&self, editor: &Entity<EditorState>) -> Option<usize> {
        self.tabs.iter().position(|t| &t.editor == editor)
    }

    /// Pushes tab `ix`'s diagnostics to its editor. Skipped while the editor
    /// is ahead of the document: its next change event pushes again.
    pub(super) fn show_diagnostics(&self, ix: usize, cx: &mut App) {
        let (tab, doc) = (&self.tabs[ix], &self.state.open[ix]);
        if tab.editor.read(cx).value().as_ref() != doc.text {
            return;
        }
        tab.diagnostics
            .show(&tab.editor, &doc.text, &doc.diagnostics, cx);
    }

    /// Diagnoses tab `ix` now if its text changed since the last diagnosis.
    pub(super) fn flush_diagnostics(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.state.open[ix].is_stale() {
            self.state.open[ix].rediagnose();
            self.tabs[ix].diagnose = Task::ready(());
            self.show_diagnostics(ix, cx);
            cx.notify();
        }
    }

    pub(super) fn code_actions(
        &mut self,
        editor: &Entity<EditorState>,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<CodeAction> {
        let Some(ix) = self.tab_of(editor) else {
            return Vec::new();
        };
        self.flush_diagnostics(ix, cx);
        self.state.open[ix]
            .diagnostics
            .iter()
            .filter(|d| covers(d, &range))
            .filter_map(|d| {
                let fix = d.fix.as_ref()?;
                Some(CodeAction {
                    title: fix.clone(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    data: Some(serde_json::json!({ "path": d.path, "fix": fix })),
                    ..Default::default()
                })
            })
            .collect()
    }

    /// ⌘.: applies the first quick fix at the active editor's cursor.
    pub(super) fn quick_fix(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.state.active else { return };
        self.flush_diagnostics(ix, cx);
        let cursor = self.tabs[ix].editor.read(cx).cursor();
        let found = self.state.open[ix]
            .diagnostics
            .iter()
            .position(|d| d.fix.is_some() && covers(d, &(cursor..cursor)));
        if let Some(idx) = found {
            self.apply_fix(ix, idx, window, cx);
        }
    }

    /// Applies the fix `fix` of the diagnostic at `path` in tab `ix`, if the
    /// current diagnostics still have it.
    fn apply_named_fix(
        &mut self,
        ix: usize,
        path: &str,
        fix: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_diagnostics(ix, cx);
        let found = self.state.open[ix]
            .diagnostics
            .iter()
            .position(|d| d.path == path && d.fix.as_deref() == Some(fix));
        match found {
            Some(idx) => self.apply_fix(ix, idx, window, cx),
            None => cant_apply(window, cx),
        }
    }

    /// Applies diagnostic `idx`'s fix to tab `ix`: the document computes the
    /// edit, the editor takes it as one undoable change.
    fn apply_fix(&mut self, ix: usize, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        let doc = &mut self.state.open[ix];
        let before = doc.text.clone();
        match doc.apply_fix(idx) {
            Ok(()) => {
                let (range, replacement) = minimal_edit(&before, &doc.text);
                let replacement = replacement.to_string();
                self.tabs[ix].editor.update(cx, |state, cx| {
                    state.set_selected_range(range, cx);
                    state.replace(replacement, window, cx);
                });
                // The change event re-pushes the (already current) diagnostics.
                cx.notify();
            }
            Err(FixError::NotApplicable) => cant_apply(window, cx),
            Err(FixError::NoFix) => {}
        }
    }

    /// Moves the active editor's cursor to diagnostic `idx`.
    pub(super) fn reveal_problem(
        &mut self,
        idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.state.active else { return };
        let doc = &self.state.open[ix];
        let Some(d) = doc.diagnostics.get(idx) else {
            return;
        };
        let at = position(&doc.text, d.range.start);
        self.tabs[ix]
            .editor
            .update(cx, |state, cx| state.set_cursor_position(at, window, cx));
    }
}

fn cant_apply(window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::warning("Can't apply this fix here"), cx);
}

#[cfg(test)]
mod tests {
    use super::{
        Severity, StudioDiagnostic, byte_to_position, hover_text, line_tints, minimal_edit, ticks,
    };
    use std::ops::Range;

    #[test]
    fn byte_to_position_handles_multibyte() {
        let text = "a: 😀\nbc: 1\n";
        let line1 = text.find("bc").unwrap();
        assert_eq!(byte_to_position(text, line1 + 2), (1, 2));
        // The emoji is 4 bytes but one character.
        assert_eq!(byte_to_position(text, text.find('\n').unwrap()), (0, 4));
        assert_eq!(byte_to_position(text, text.len() + 9), (2, 0));
        // Inside the emoji: clamps back to its start.
        assert_eq!(byte_to_position(text, 4), (0, 3));
    }

    fn diag(range: Range<usize>, severity: Severity) -> StudioDiagnostic {
        StudioDiagnostic {
            range,
            severity,
            message: "Unknown property".into(),
            path: "request.timout".into(),
            fix: None,
            issue: None,
        }
    }

    #[test]
    fn hover_text_names_curl_runner_and_path() {
        let d = diag(0..1, Severity::Warning);
        assert_eq!(
            hover_text(&d),
            "Unknown property\n\ncurl-runner · request.timout"
        );
        let parse = StudioDiagnostic {
            path: String::new(),
            ..d
        };
        assert_eq!(hover_text(&parse), "Unknown property\n\ncurl-runner");
    }

    #[test]
    fn line_tints_take_the_most_severe_per_line() {
        let text = "a: 1\nb: 2\nc: 3";
        let tints = line_tints(
            text,
            &[
                diag(5..6, Severity::Warning),
                diag(8..9, Severity::Error),
                diag(0..1, Severity::Info),
                diag(10..11, Severity::Warning),
            ],
        );
        assert_eq!(
            tints,
            [(5..10, Severity::Error), (10..14, Severity::Warning)]
        );
    }

    #[test]
    fn ticks_sit_at_the_diagnostic_line() {
        let text = "a\nb\nc\nd\n";
        assert_eq!(
            ticks(text, &[diag(4..5, Severity::Error)]),
            [(2. / 5., Severity::Error)]
        );
    }

    #[test]
    fn minimal_edit_covers_only_the_change() {
        let old = "  timout: 5000\n";
        let new = "  timeout: 5000\n";
        let (range, text) = minimal_edit(old, new);
        assert_eq!((range.clone(), text), (5..5, "e"));
        let mut applied = old.to_string();
        applied.replace_range(range, text);
        assert_eq!(applied, new);
        assert_eq!(minimal_edit("aé", "aè"), (1..3, "è"));
        assert_eq!(minimal_edit("same", "same"), (4..4, ""));
    }
}
