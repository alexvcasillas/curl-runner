//! The last run's failed checks in the editor: a `got 2000` hint at the end
//! of each failing `expect` line, with the row tinted and a cross in the
//! gutter. Painted with the diagnostic marks (`diagnostic_marks`); a line
//! with a diagnostic keeps the diagnostic.
//!
//! A hint goes away once its line is edited; the rest follow edits above
//! them, and the next run replaces them all.

use super::diagnostic_marks::LineMark;
use super::diagnostics_view::minimal_edit;
use super::runs::FileRun;
use crate::model::result_view::expect_hints;
use curl_runner_core::lint::Severity;
use curl_runner_core::lint::spans::locate;

/// One failing expectation's line.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ResultHint {
    /// Zero-based line number.
    pub line: u32,
    /// Byte offsets of the line, without its newline.
    pub start: usize,
    pub end: usize,
    /// The line as it was when the hint was made: an edit that changes it
    /// drops the hint.
    pub line_text: String,
    /// `got 2000`.
    pub text: String,
}

/// A tab's hints and the line count the gutter is sized for.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ResultMarks {
    pub hints: Vec<ResultHint>,
    pub total_lines: usize,
}

/// The line holding byte `at`: its number and span.
fn line_at(text: &str, at: usize) -> (u32, usize, usize) {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    (text[..start].matches('\n').count() as u32, start, end)
}

impl ResultMarks {
    /// The hints for `run`'s failed checks in `text`, the file that ran;
    /// one per line, the first check's.
    pub fn new(text: &str, run: &FileRun) -> Self {
        let single = locate(text, "request").is_some();
        let mut hints: Vec<ResultHint> = Vec::new();
        for (&index, view) in &run.views {
            let at = (!single).then_some(index);
            for (range, hint) in expect_hints(text, at, &view.checks) {
                let (line, start, end) = line_at(text, range.start.min(text.len()));
                if hints.iter().any(|h| h.line == line) {
                    continue;
                }
                hints.push(ResultHint {
                    line,
                    start,
                    end,
                    line_text: text[start..end].to_string(),
                    text: hint,
                });
            }
        }
        hints.sort_by_key(|h| h.line);
        ResultMarks {
            hints,
            total_lines: text.matches('\n').count() + 1,
        }
    }

    /// Follows the edit from `old` to `new`: hints below it move with their
    /// line, a hint whose line changed is dropped.
    pub fn edited(&mut self, old: &str, new: &str) {
        self.total_lines = new.matches('\n').count() + 1;
        if self.hints.is_empty() {
            return;
        }
        let (range, replacement) = minimal_edit(old, new);
        let first = old[..range.start].matches('\n').count() as u32;
        let last = first + old[range.clone()].matches('\n').count() as u32;
        let delta = replacement.matches('\n').count() as i64 - i64::from(last - first);
        let starts: Vec<usize> = std::iter::once(0)
            .chain(new.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let line_span = |line: u32| {
            let start = *starts.get(line as usize)?;
            let end = new[start..].find('\n').map_or(new.len(), |i| start + i);
            Some((start, end))
        };
        self.hints.retain_mut(|h| {
            let moved = u32::try_from(i64::from(h.line) + delta).ok();
            // A touched line may also be the one an inserted newline pushed
            // down: try it in both places.
            let candidates = if h.line < first {
                [Some(h.line), None]
            } else if h.line > last {
                [moved, None]
            } else {
                [Some(h.line), moved]
            };
            for line in candidates.into_iter().flatten() {
                if let Some((start, end)) = line_span(line)
                    && new[start..end] == h.line_text
                {
                    (h.line, h.start, h.end) = (line, start, end);
                    return true;
                }
            }
            false
        });
    }

    /// The hints as marks to paint.
    pub fn line_marks(&self) -> Vec<LineMark> {
        self.hints
            .iter()
            .map(|h| LineMark {
                line: h.line,
                start: h.start,
                end: h.end,
                severity: Severity::Error,
                ghost: (h.text.clone(), Severity::Error),
                failed_check: true,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{ResultHint, ResultMarks};

    fn marks(text: &str, line: u32, hint: &str) -> ResultMarks {
        let start = text
            .split_inclusive('\n')
            .take(line as usize)
            .map(str::len)
            .sum::<usize>();
        let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        ResultMarks {
            hints: vec![ResultHint {
                line,
                start,
                end,
                line_text: text[start..end].to_string(),
                text: hint.into(),
            }],
            total_lines: 4,
        }
    }

    const TEXT: &str = "a: 1\nb: 2\namount: 2500\nc: 3\n";

    #[test]
    fn an_edit_on_the_line_drops_its_hint() {
        let mut m = marks(TEXT, 2, "got 2000");
        m.edited(TEXT, &TEXT.replace("2500", "2000"));
        assert!(m.hints.is_empty());
    }

    #[test]
    fn hints_follow_edits_above_and_ignore_edits_below() {
        let mut m = marks(TEXT, 2, "got 2000");
        let above = format!("# note\n{TEXT}");
        m.edited(TEXT, &above);
        assert_eq!(m.hints[0].line, 3);
        assert_eq!(&above[m.hints[0].start..m.hints[0].end], "amount: 2500");
        let below = above.replace("c: 3", "c: 4");
        m.edited(&above, &below);
        assert_eq!(m.hints[0].line, 3);
        // Enter at the end of the line above: the hint moves down a line.
        let at = below.find("b: 2").unwrap() + 4;
        let split = format!("{}\n{}", &below[..at], &below[at..]);
        m.edited(&below, &split);
        assert_eq!(m.hints[0].line, 4);
        assert_eq!(m.total_lines, 7);
    }

    #[test]
    fn new_reads_each_request_of_the_run() {
        use super::super::inspector::ResultView;
        use super::super::runs::FileRun;
        use curl_runner_core::model::{ExecutionResult, RequestConfig};
        let text = "requests:\n  - url: a\n    expect:\n      status: 200\n  - url: b\n    expect:\n      status: 200\n      body:\n        amount: 2500\n";
        let mut run = FileRun::new("Default");
        for (index, status, amount) in [(0, 200, 2500), (1, 503, 2000)] {
            let mut request = RequestConfig {
                url: "https://api.example.com".into(),
                ..Default::default()
            };
            let expect = if index == 0 {
                serde_json::json!({"status": 200})
            } else {
                serde_json::json!({"status": 200, "body": {"amount": 2500}})
            };
            request.expect = serde_json::from_value(expect).ok();
            let mut r = ExecutionResult::new(request, false);
            r.status = Some(status);
            r.body = Some(serde_json::json!({ "amount": amount }));
            run.views.insert(index, ResultView::new(&r));
        }
        let m = ResultMarks::new(text, &run);
        let got: Vec<(u32, &str)> = m.hints.iter().map(|h| (h.line, h.text.as_str())).collect();
        assert_eq!(got, [(6, "got 503"), (8, "got 2000")]);
        assert_eq!(m.hints[1].line_text, "        amount: 2500");
        assert_eq!(m.total_lines, 10);
        assert!(m.line_marks().iter().all(|l| l.failed_check));
    }

    #[test]
    fn deleting_lines_above_moves_hints_up() {
        let mut m = marks(TEXT, 2, "got 2000");
        let fewer = TEXT.replacen("a: 1\n", "", 1);
        m.edited(TEXT, &fewer);
        assert_eq!(m.hints[0].line, 1);
        assert_eq!(m.line_marks()[0].ghost.0, "got 2000");
    }
}
