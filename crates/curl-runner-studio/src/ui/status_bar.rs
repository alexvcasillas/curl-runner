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
/// order: `BASE_URL · API_TOKEN from local keychain`. Names without a value
/// are left out (they are `missing`, counted apart); so is everything when
/// nothing is referenced.
pub fn source_groups(vars: &[ResolvedVar], missing: &[String]) -> Vec<SharedString> {
    let mut groups: Vec<(String, Vec<&str>)> = Vec::new();
    for v in vars {
        if missing.contains(&v.name) {
            continue;
        }
        let Some(source) = &v.source else { continue };
        let from = match source {
            VarSource::Keychain => "local keychain".to_string(),
            VarSource::Process => "shell environment".to_string(),
            VarSource::Captured => "last run".to_string(),
            VarSource::EnvFile(p) => p
                .file_name()
                .map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into()),
        };
        match groups.iter_mut().find(|(f, _)| *f == from) {
            Some((_, names)) => names.push(&v.name),
            None => groups.push((from, vec![&v.name])),
        }
    }
    groups
        .into_iter()
        .map(|(from, names)| format!("{} from {from}", names.join(" · ")).into())
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
    use super::{Position, cursor_label, source_groups};
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
        let groups = source_groups(&vars, &["ACCOUNT_ID".into()]);
        assert_eq!(
            groups,
            [
                "BASE_URL · API_TOKEN from local keychain",
                "REGION from .env.staging",
                "store.id from last run",
            ]
        );
        assert!(source_groups(&[], &[]).is_empty());
    }

    #[test]
    fn cursor_label_is_one_based() {
        assert_eq!(cursor_label(Position::new(14, 4)), "Ln 15, Col 5");
    }
}
