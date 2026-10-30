//! The status bar: collection or environment, where the active file's
//! variables come from, language, cursor and curl-runner version.
//!
//! A view of its own: it follows the active editor's cursor, so moving it
//! repaints only this bar.

use super::color;
use crate::model::environments::{ResolvedVar, VarSource};
use gpui_kit::component::input::{EditorState, Position};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashMap;

pub const STATUS_BAR_HEIGHT: f32 = 28.;

#[derive(Default)]
pub struct StatusBar {
    collection: Option<SharedString>,
    editor: Option<Entity<EditorState>>,
    cursor: Option<Position>,
    /// The picked environment, shown instead of the collection.
    environment: Option<SharedString>,
    /// `BASE_URL · API_TOKEN from local keychain`, one per source.
    sources: Vec<SharedString>,
    missing: usize,
    _cursor_watch: Option<Subscription>,
}

/// The active file's resolved variables grouped by source, in first-use
/// order: `BASE_URL · API_TOKEN from local keychain`. A captured value whose
/// request is known reads `CHARGE_ID captured from create-charge`
/// (`captured_by`: value name to request name). Names without a value are
/// left out (they are `missing`, counted apart); so is everything when
/// nothing is referenced.
pub fn source_groups(
    vars: &[ResolvedVar],
    missing: &[String],
    captured_by: &HashMap<String, String>,
) -> Vec<SharedString> {
    // (the text after the names, the names)
    let mut groups: Vec<(String, Vec<&str>)> = Vec::new();
    for v in vars {
        if missing.contains(&v.name) {
            continue;
        }
        let Some(source) = &v.source else { continue };
        let mut name = v.name.as_str();
        let tail = match source {
            VarSource::Keychain => "from local keychain".to_string(),
            VarSource::Process => "from shell environment".to_string(),
            VarSource::Captured => {
                let key = name.strip_prefix("store.").unwrap_or(name);
                match captured_by.get(key) {
                    Some(by) => {
                        name = key;
                        format!("captured from {by}")
                    }
                    None => "from last run".to_string(),
                }
            }
            VarSource::EnvFile(p) => format!(
                "from {}",
                p.file_name()
                    .map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into())
            ),
        };
        match groups.iter_mut().find(|(t, _)| *t == tail) {
            Some((_, names)) => names.push(name),
            None => groups.push((tail, vec![name])),
        }
    }
    groups
        .into_iter()
        .map(|(tail, names)| format!("{} {tail}", names.join(" · ")).into())
        .collect()
}

impl StatusBar {
    /// Shows `collection` and follows `editor`'s cursor (none: no file open).
    pub fn set(
        &mut self,
        collection: Option<SharedString>,
        editor: Option<Entity<EditorState>>,
        cx: &mut Context<Self>,
    ) {
        self.collection = collection;
        if self.editor != editor {
            self.cursor = editor.as_ref().map(|e| e.read(cx).cursor_position());
            self._cursor_watch = editor.as_ref().map(|e| {
                cx.observe(e, |this, editor, cx| {
                    let cursor = Some(editor.read(cx).cursor_position());
                    if this.cursor != cursor {
                        this.cursor = cursor;
                        cx.notify();
                    }
                })
            });
            self.editor = editor;
        }
        cx.notify();
    }
}

impl StatusBar {
    /// Shows the environment, the variable sources and the missing count.
    pub fn set_vars(
        &mut self,
        environment: Option<SharedString>,
        sources: Vec<SharedString>,
        missing: usize,
        cx: &mut Context<Self>,
    ) {
        if self.environment != environment || self.sources != sources || self.missing != missing {
            self.environment = environment;
            self.sources = sources;
            self.missing = missing;
            cx.notify();
        }
    }
}

/// `Ln 15, Col 5`, one-based like every editor shows it.
fn cursor_label(p: Position) -> String {
    format!("Ln {}, Col {}", p.line + 1, p.character + 1)
}

impl Render for StatusBar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("status-bar")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(16.))
            .h(px(STATUS_BAR_HEIGHT))
            .px(px(16.))
            .border_t_1()
            .border_color(color("border-100"))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .children(self.environment.clone().or_else(|| self.collection.clone()))
            .children(
                self.sources
                    .iter()
                    .map(|s| div().flex_none().text_ellipsis().child(s.clone())),
            )
            .when(self.missing > 0, |bar| {
                bar.child(
                    div()
                        .id("status-missing")
                        .text_color(color("warning"))
                        .child(format!("{} missing", self.missing)),
                )
            })
            .child(div().flex_1())
            .when_some(self.cursor, |bar, cursor| {
                bar.child("YAML").child(cursor_label(cursor))
            })
            .child(format!("curl-runner {}", curl_runner_core::VERSION))
    }
}

#[cfg(test)]
mod tests {
    use super::{HashMap, Position, cursor_label, source_groups};
    use crate::model::environments::{ResolvedVar, VarSource};

    fn var(name: &str, source: Option<VarSource>) -> ResolvedVar {
        ResolvedVar {
            name: name.into(),
            value: source.as_ref().map(|_| "v".into()),
            source,
            secret: false,
        }
    }

    #[test]
    fn sources_group_names_by_where_they_come_from() {
        let vars = [
            var("BASE_URL", Some(VarSource::Keychain)),
            var("REGION", Some(VarSource::EnvFile("/c/.env.staging".into()))),
            var("API_TOKEN", Some(VarSource::Keychain)),
            var("store.id", Some(VarSource::Captured)),
            var("ACCOUNT_ID", None),
        ];
        let groups = source_groups(&vars, &["ACCOUNT_ID".into()], &HashMap::new());
        assert_eq!(
            groups,
            [
                "BASE_URL · API_TOKEN from local keychain",
                "REGION from .env.staging",
                "store.id from last run",
            ]
        );
        assert!(source_groups(&[], &[], &HashMap::new()).is_empty());
        let by = HashMap::from([("CHARGE_ID".to_string(), "create-charge".to_string())]);
        let vars = [
            var("BASE_URL", Some(VarSource::Keychain)),
            var("store.CHARGE_ID", Some(VarSource::Captured)),
            var("store.other", Some(VarSource::Captured)),
        ];
        assert_eq!(
            source_groups(&vars, &[], &by),
            [
                "BASE_URL from local keychain",
                "CHARGE_ID captured from create-charge",
                "store.other from last run",
            ]
        );
    }

    #[test]
    fn cursor_label_is_one_based() {
        assert_eq!(cursor_label(Position::new(14, 4)), "Ln 15, Col 5");
    }
}
