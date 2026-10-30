//! The status bar: collection or environment, where the active file's
//! variables come from, language, cursor and curl-runner version.
//!
//! A view of its own: it follows the active editor's cursor, so moving it
//! repaints only this bar.

use super::collection_settings::EnvStatus;
use super::color;
use super::sync::{SyncLine, sync_dot};
use crate::model::environments::{ResolvedVar, VarSource};
use gpui_kit::component::Sizable as _;
use gpui_kit::component::input::{EditorState, Position};
use gpui_kit::component::spinner::Spinner;
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
    /// The collection run, while there is one to show.
    run: Option<RunLine>,
    /// `142 runs stored · 3.1 MB` while the History screen is shown: the
    /// bar then reads as the board's history footer.
    history: Option<SharedString>,
    /// The Environments page's footer, while it is shown.
    environments: Option<EnvStatus>,
    /// The sidebar footer's sync state, when it asks for attention.
    sync: Option<SyncLine>,
    /// A received copy: `YAML · read only` in place of the cursor.
    read_only: bool,
    _cursor_watch: Option<Subscription>,
}

/// The collection run's part of the bar: `Running 14/22` on the left
/// (clicking it shows the run screen) and, while the screen is shown, its
/// `Concurrency 1 · stop on failure on` on the right.
#[derive(Debug, Clone, PartialEq)]
pub struct RunLine {
    pub label: SharedString,
    pub running: bool,
    pub config: Option<SharedString>,
}

/// What the bar asks of the workspace.
pub enum StatusBarEvent {
    OpenRun,
    /// The environment name was clicked.
    OpenEnvironments,
    /// The sync item was clicked: resolve conflicts, else sync now.
    Sync,
}

impl EventEmitter<StatusBarEvent> for StatusBar {}

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

impl StatusBar {
    /// Shows (or with `None` hides) the collection run's line.
    pub fn set_run(&mut self, run: Option<RunLine>, cx: &mut Context<Self>) {
        if self.run != run {
            self.run = run;
            cx.notify();
        }
    }
}

impl StatusBar {
    /// Shows (or with `None` hides) the history footer.
    pub fn set_history(&mut self, usage: Option<SharedString>, cx: &mut Context<Self>) {
        if self.history != usage {
            self.history = usage;
            cx.notify();
        }
    }
}

impl StatusBar {
    /// Shows (or with `None` hides) the Environments page's footer.
    pub fn set_environments(&mut self, status: Option<EnvStatus>, cx: &mut Context<Self>) {
        if self.environments != status {
            self.environments = status;
            cx.notify();
        }
    }
}

impl StatusBar {
    /// A received copy is open (the ReadOnly board's `YAML · read only`).
    pub fn set_read_only(&mut self, read_only: bool, cx: &mut Context<Self>) {
        if self.read_only != read_only {
            self.read_only = read_only;
            cx.notify();
        }
    }
}

impl StatusBar {
    /// Shows (or with `None` hides) the sync item.
    pub fn set_sync(&mut self, line: Option<SyncLine>, cx: &mut Context<Self>) {
        if self.sync != line {
            self.sync = line;
            cx.notify();
        }
    }
}

/// `Ln 15, Col 5`, one-based like every editor shows it.
fn cursor_label(p: Position) -> String {
    format!("Ln {}, Col {}", p.line + 1, p.character + 1)
}

impl Render for StatusBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(env) = &self.environments {
            // The Environments board's footer.
            return div()
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
                .when(!env.environment.is_empty(), |bar| {
                    bar.child(env.environment.clone())
                })
                .child(
                    div()
                        .id("status-summary")
                        .test_support()
                        .aria_label(env.summary.clone())
                        .child(env.summary.clone()),
                )
                .child(div().flex_1())
                .when(!env.keychain.is_empty(), |bar| {
                    bar.child(env.keychain.clone())
                })
                .child(format!("curl-runner {}", curl_runner_core::VERSION));
        }
        // The history footer reads as the board's: its two strings only.
        let run = self.run.clone().filter(|_| self.history.is_none());
        let config = run.as_ref().and_then(|r| r.config.clone());
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
            .when_some(run, |bar, run| {
                bar.child(
                    div()
                        .id("status-run")
                        .role(Role::Button)
                        .test_support()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .hover(|s| s.text_color(color("text-primary")))
                        .when(run.running, |d| {
                            d.child(
                                Spinner::new()
                                    .with_size(px(11.))
                                    .color(color("accent-blue")),
                            )
                        })
                        .child(run.label)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(StatusBarEvent::OpenRun))),
                )
            })
            .when_some(self.history.clone(), |bar, usage| {
                bar.child(div().flex_none().child(usage))
                    .child("History stays on this device unless you share a run")
            })
            .when(self.history.is_none(), |bar| self.render_vars(bar, cx))
            .child(div().flex_1())
            .when_some(
                self.sync.clone().filter(|_| self.history.is_none()),
                |bar, line| bar.child(render_sync(&line, cx)),
            )
            .when(self.history.is_none(), |bar| {
                bar.children(config).when_some(self.cursor, |bar, cursor| {
                    if self.read_only {
                        bar.child(
                            div()
                                .id("status-language")
                                .test_support()
                                .aria_label("YAML · read only")
                                .child("YAML · read only"),
                        )
                    } else {
                        bar.child("YAML").child(cursor_label(cursor))
                    }
                })
            })
            .child(format!("curl-runner {}", curl_runner_core::VERSION))
    }
}

impl StatusBar {
    /// The environment, where variables come from and the missing count.
    fn render_vars(&self, bar: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        let env = self.environment.clone().map(|env| {
            div()
                .id("status-environment")
                .role(Role::Button)
                .test_support()
                .flex_none()
                .cursor_pointer()
                .hover(|s| s.text_color(color("text-primary")))
                .child(env)
                .on_click(cx.listener(|_, _, _, cx| cx.emit(StatusBarEvent::OpenEnvironments)))
                .into_any_element()
        });
        let name = env.or_else(|| {
            self.collection
                .clone()
                .map(|c| div().child(c).into_any_element())
        });
        bar.children(name)
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
    }
}

/// The footer's sync state, compact: `● Offline · 3 changes waiting`.
fn render_sync(line: &SyncLine, cx: &mut Context<StatusBar>) -> impl IntoElement + use<> {
    let text = line.text(chrono::Utc::now());
    div()
        .id("status-sync")
        .role(Role::Button)
        .test_support()
        .aria_label(text.clone())
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .cursor_pointer()
        .text_color(color(line.text_color()))
        .hover(|s| s.text_color(color("text-primary")))
        .children(line.dot().map(|(token, ring)| sync_dot(token, ring)))
        .child(text)
        .on_click(cx.listener(|_, _, _, cx| cx.emit(StatusBarEvent::Sync)))
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
