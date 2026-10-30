//! The editor's diagnostic marks: gutter icon and colored line number,
//! full-row tint, squiggle and inline hint, painted over the Kit editor.
//!
//! The Kit has no gutter or per-line hooks, and its own squiggles come with
//! a popover we replace, so the studio keeps the Kit's diagnostic set empty
//! and paints everything here from the editor's fresh layout (see
//! `SPIKE.md`, "Diagnostic marks").

use super::app::{mono_font, ui_font};
use super::color;
use super::diagnostics_view::{byte_to_position, severity_color};
use super::icons;
use crate::model::diagnostics::{StudioDiagnostic, ghost_text, rank};
use curl_runner_core::lint::Severity;
use gpui_kit::component::input::EditorState;
use gpui_kit::*;

/// A line with diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LineMark {
    /// Zero-based line number.
    pub line: u32,
    /// Byte offset of the line's start.
    pub start: usize,
    /// Byte offset of the line's end, before its newline.
    pub end: usize,
    /// The most severe diagnostic on the line.
    pub severity: Severity,
    /// The inline hint: the first diagnostic on the line.
    pub ghost: (String, Severity),
    /// A failed check of the last run (`result_marks`), not a diagnostic:
    /// a cross in the gutter.
    pub failed_check: bool,
}

/// One [`LineMark`] per line holding a diagnostic start, in line order.
pub(super) fn line_marks(text: &str, diagnostics: &[StudioDiagnostic]) -> Vec<LineMark> {
    let mut by_start: Vec<&StudioDiagnostic> = diagnostics.iter().collect();
    by_start.sort_by_key(|d| d.range.start);
    let mut marks: Vec<LineMark> = Vec::new();
    for d in by_start {
        let at = d.range.start.min(text.len());
        let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        match marks.last_mut() {
            Some(m) if m.start == start => {
                if rank(d.severity) < rank(m.severity) {
                    m.severity = d.severity;
                }
            }
            _ => marks.push(LineMark {
                line: byte_to_position(text, start).0,
                start,
                end: text[start..].find('\n').map_or(text.len(), |i| start + i),
                severity: d.severity,
                ghost: (ghost_text(&d.message).to_string(), d.severity),
                failed_check: false,
            }),
        }
    }
    marks
}

/// What the overlay paints for a tab, refreshed with its diagnostics.
#[derive(Default)]
pub(super) struct Marks {
    pub lines: Vec<LineMark>,
    /// The diagnostics shown, matching the editor's text.
    pub shown: Vec<StudioDiagnostic>,
    /// Lines in the text, for the gutter's number width.
    pub total_lines: usize,
}

impl Marks {
    pub fn new(text: &str, diagnostics: &[StudioDiagnostic]) -> Self {
        Marks {
            lines: line_marks(text, diagnostics),
            shown: diagnostics.to_vec(),
            total_lines: text.bytes().filter(|&b| b == b'\n').count() + 1,
        }
    }
}

// The Kit gutter (`gpui-base` `input/base/element.rs`): line numbers of at
// least 3 digits from the editor's left edge, a 6px margin, then an 18px
// fold column.
const NUMBER_MARGIN: f32 = 6.;
const FOLD_COLUMN: f32 = 18.;
const GUTTER_ICON: f32 = 11.;
/// The editor's text size (`Editor::text_size` in `editor_pane.rs`).
pub(super) const EDITOR_TEXT: f32 = 13.;
/// Space between a line's end and its inline hint.
const GHOST_GAP: f32 = 16.;
const GHOST_TEXT: f32 = 12.;
/// The hint's severity color over the editor background (the mockup's
/// `#9a3e41` and `#8a6a1f`).
const GHOST_OPACITY: f32 = 0.55;
/// The squiggle's distance above the row's bottom.
const SQUIGGLE_LIFT: f32 = 2.;

/// Digits the Kit reserves for line numbers.
fn number_digits(total_lines: usize) -> usize {
    (total_lines.max(1).ilog10() as usize + 1).clamp(3, 7)
}

pub(super) fn severity_glyph(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => icons::ERROR,
        Severity::Warning => icons::WARNING,
        Severity::Info => icons::INFO,
    }
}

/// The row tint (`diag-error-line`, `diag-warning-line`); info is not tinted.
fn tint(severity: Severity) -> Option<Hsla> {
    match severity {
        Severity::Error => Some(color("diag-error-line")),
        Severity::Warning => Some(color("diag-warning-line")),
        Severity::Info => None,
    }
}

fn run(font: Font, len: usize, color: Hsla) -> TextRun {
    TextRun {
        len,
        font,
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// Where one marked line sits this frame.
struct Placed {
    top: Pixels,
    end_x: Pixels,
    current: bool,
}

/// Paints `marks` over `editor`, which must be painted already this frame:
/// call it from a `canvas` placed after the editor, so `range_to_bounds`
/// and `input_bounds` read the layout the editor just painted. Lines
/// scrolled out or folded away have no bounds and are skipped. The line
/// `hide_ghost` (its popover is open) gets no inline hint. `checks` are the
/// last run's failed-check lines (`result_marks`), for a text of
/// `total_lines`; a line with a diagnostic shows the diagnostic.
pub(super) fn paint_marks(
    editor: &Entity<EditorState>,
    marks: &Marks,
    checks: &[LineMark],
    total_lines: usize,
    hide_ghost: Option<u32>,
    window: &mut Window,
    cx: &mut App,
) {
    let lines: Vec<&LineMark> = marks
        .lines
        .iter()
        .chain(
            checks
                .iter()
                .filter(|c| !marks.lines.iter().any(|m| m.line == c.line)),
        )
        .collect();
    if lines.is_empty() {
        return;
    }
    let (line_height, input, placed, squiggles) = {
        let state = editor.read(cx);
        let Some(line_height) = state.line_height() else {
            return;
        };
        let cursor_line = state.cursor_position().line;
        let placed: Vec<Option<Placed>> = lines
            .iter()
            .map(|m| {
                let start = state.range_to_bounds(&(m.start..m.start))?;
                let end = state.range_to_bounds(&(m.end..m.end))?;
                Some(Placed {
                    top: start.origin.y,
                    end_x: end.origin.x,
                    current: m.line == cursor_line,
                })
            })
            .collect();
        let squiggles: Vec<(Bounds<Pixels>, Severity)> = marks
            .shown
            .iter()
            .filter_map(|d| Some((state.range_to_bounds(&d.range)?, d.severity)))
            .collect();
        (line_height, state.input_bounds(), placed, squiggles)
    };
    let mono = font(mono_font());
    let digits = number_digits(marks.total_lines.max(total_lines));
    let number_width = window
        .text_system()
        .shape_line(
            "0".repeat(digits).into(),
            px(EDITOR_TEXT),
            &[run(mono.clone(), digits, color("text-gutter"))],
            None,
        )
        .width;
    let gutter_width = number_width + px(NUMBER_MARGIN + FOLD_COLUMN);
    let text_area = Bounds::from_corners(
        point(input.origin.x + gutter_width, input.origin.y),
        input.bottom_right(),
    );

    window.with_content_mask(Some(ContentMask { bounds: input }), |window| {
        for (m, p) in lines.iter().zip(&placed) {
            let Some(p) = p else { continue };
            // The Kit's gutter cell (number and fold chevron), repainted.
            let under = if p.current { "surface-100" } else { "bg-100" };
            let gutter = Bounds::new(
                point(input.origin.x, p.top),
                size(gutter_width, line_height),
            );
            window.paint_quad(fill(gutter, color(under)));
            if let Some(tint) = tint(m.severity) {
                let row = Bounds::new(
                    point(input.origin.x, p.top),
                    size(input.size.width, line_height),
                );
                window.paint_quad(fill(row, tint));
            }
            let fg = severity_color(m.severity);
            let number = format!("{:>digits$}", m.line + 1);
            let len = number.len();
            let number = window.text_system().shape_line(
                number.into(),
                px(EDITOR_TEXT),
                &[run(mono.clone(), len, fg)],
                None,
            );
            number
                .paint(
                    point(input.origin.x, p.top),
                    line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
            let inset = (FOLD_COLUMN - GUTTER_ICON) / 2.;
            let icon = Bounds::new(
                point(
                    input.origin.x + number_width + px(NUMBER_MARGIN + inset),
                    p.top + (line_height - px(GUTTER_ICON)) / 2.,
                ),
                size(px(GUTTER_ICON), px(GUTTER_ICON)),
            );
            let glyph = if m.failed_check {
                icons::CROSS
            } else {
                severity_glyph(m.severity)
            };
            window
                .paint_svg(
                    icon,
                    SharedString::new_static(glyph),
                    Some(glyph.as_bytes()),
                    TransformationMatrix::unit(),
                    fg,
                    cx,
                )
                .ok();
        }
    });

    let mut ghost_font = font(ui_font());
    ghost_font.style = FontStyle::Italic;
    window.with_content_mask(Some(ContentMask { bounds: text_area }), |window| {
        for (bounds, severity) in &squiggles {
            let style = UnderlineStyle {
                thickness: px(1.),
                color: Some(severity_color(*severity)),
                wavy: true,
            };
            let origin = point(
                bounds.origin.x,
                bounds.origin.y + line_height - px(SQUIGGLE_LIFT),
            );
            window.paint_underline(origin, bounds.size.width.max(px(6.)), &style);
        }
        for (m, p) in lines.iter().zip(&placed) {
            let Some(p) = p else { continue };
            if hide_ghost == Some(m.line) {
                continue;
            }
            let (text, severity) = &m.ghost;
            let fg = severity_color(*severity).opacity(GHOST_OPACITY);
            let ghost = window.text_system().shape_line(
                SharedString::from(text.clone()),
                px(GHOST_TEXT),
                &[run(ghost_font.clone(), text.len(), fg)],
                None,
            );
            ghost
                .paint(
                    point(p.end_x + px(GHOST_GAP), p.top),
                    line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{LineMark, Marks, line_marks, number_digits};
    use crate::model::diagnostics::StudioDiagnostic;
    use curl_runner_core::lint::Severity;
    use std::ops::Range;

    fn diag(range: Range<usize>, severity: Severity, message: &str) -> StudioDiagnostic {
        StudioDiagnostic {
            range,
            severity,
            message: message.into(),
            path: String::new(),
            fix: None,
            issue: None,
        }
    }

    #[test]
    fn line_marks_take_the_most_severe_and_the_first_hint() {
        let text = "a: 1\nb: 2 3\nc: 4";
        let marks = line_marks(
            text,
            &[
                diag(10..11, Severity::Error, "Later. Must be: x"),
                diag(5..6, Severity::Warning, "Unknown key \"b\"."),
                diag(12..13, Severity::Info, "info"),
            ],
        );
        assert_eq!(
            marks,
            [
                LineMark {
                    line: 1,
                    start: 5,
                    end: 11,
                    severity: Severity::Error,
                    ghost: ("Unknown key \"b\"".into(), Severity::Warning),
                    failed_check: false,
                },
                LineMark {
                    line: 2,
                    start: 12,
                    end: 16,
                    severity: Severity::Info,
                    ghost: ("info".into(), Severity::Info),
                    failed_check: false,
                },
            ]
        );
    }

    #[test]
    fn line_marks_on_the_last_line_and_past_the_end() {
        let text = "a: 1";
        let marks = line_marks(text, &[diag(9..9, Severity::Error, "x")]);
        assert_eq!((marks[0].line, marks[0].start, marks[0].end), (0, 0, 4));
        assert!(line_marks(text, &[]).is_empty());
    }

    #[test]
    fn marks_count_lines() {
        let marks = Marks::new("a\nb\n", &[diag(0..1, Severity::Error, "x")]);
        assert_eq!(marks.total_lines, 3);
        assert_eq!(marks.shown.len(), 1);
    }

    #[test]
    fn number_digits_match_the_kit() {
        assert_eq!(number_digits(0), 3);
        assert_eq!(number_digits(42), 3);
        assert_eq!(number_digits(1000), 4);
        assert_eq!(number_digits(usize::MAX), 7);
    }
}
