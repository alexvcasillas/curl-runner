//! The root view: owns the [`AppState`], the editors and the file watcher,
//! and lays out title bar, sidebar, editor and status bar.

use super::app::{
    CloseTab, CopyAsCurl, Import, NewCollection, NewRequest, OpenFolder, OpenPalette, QuickFix,
    Rerun, RunActive, RunCollection, RunRequestUnderCursor, SaveFile, ToggleProblems, ui_font,
};
use super::diagnostic_popover::DiagnosticHover;
use super::diagnostics_view::DiagnosticsView;
use super::inspector::{InspectorTab, env_label};
use super::problems_panel::PanelTab;
use super::result_marks::ResultMarks;
use super::runs::Runs;
use super::state::{AppState, StateStore};
use super::status_bar::{StatusBar, source_groups};
use super::watch;
use super::{color, sidebar::request_label};
use crate::model::collection::{Collection, TreeNode, request_at_line, request_lines};
use crate::model::document::CloseDecision;
use crate::model::environments::{
    DEFAULT_ENVIRONMENT, EnvCtx, ResolvedVar, list_environments, missing_vars, referenced_vars,
    resolve_vars,
};
use crate::model::secrets::{CachedSecrets, SecretStore};
use futures::StreamExt as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{EditorState, Input, InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::select::{SelectEvent, SelectState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// A tab's editor; `AppState::open[i]` is the document of `tabs[i]`.
pub(super) struct EditorTab {
    pub editor: Entity<EditorState>,
    pub diagnostics: DiagnosticsView,
    /// The pending re-diagnosis, replaced by every keystroke.
    pub diagnose: Task<()>,
    /// Each request's first line, as of the last diagnosis.
    pub request_lines: Vec<Option<u32>>,
    /// The request holding the cursor (zero-based), which the inspector shows.
    pub cursor_request: usize,
    /// The last run's failed checks, shown on their `expect` lines.
    pub results: ResultMarks,
    _changes: Subscription,
    _cursor: Subscription,
}

/// Quiet time after the last keystroke before the file is re-diagnosed.
const DIAGNOSE_DELAY: Duration = Duration::from_millis(150);

/// What to do once unsaved changes are settled.
type Then = Rc<dyn Fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>;

pub struct Workspace {
    pub(super) state: AppState,
    pub(super) tabs: Vec<EditorTab>,
    /// Folders the user collapsed, by path relative to the root.
    pub(super) collapsed: HashSet<PathBuf>,
    /// Multi-request files the user expanded (collapsed by default).
    pub(super) expanded: HashSet<PathBuf>,
    /// The request row last clicked inside a multi-request file.
    pub(super) selected_request: Option<(PathBuf, usize)>,
    pub(super) filter: Entity<InputState>,
    /// The Problems panel under the editor (⇧⌘M).
    pub(super) problems_open: bool,
    /// The panel's tab: Problems or Output.
    pub(super) panel_tab: PanelTab,
    /// The Problems row last clicked, by diagnostic index.
    pub(super) selected_problem: Option<usize>,
    /// The diagnostic popover's squiggle.
    pub(super) hover: Option<DiagnosticHover>,
    /// The popover's pending close, while the pointer is off its squiggle.
    pub(super) hover_close: Option<Task<()>>,
    /// The pointer is on the popover.
    pub(super) on_popover: bool,
    /// Keeps the popover open without a pointer (`CURL_RUNNER_STUDIO_SHOW=problem`).
    pub(super) hover_pinned: bool,
    /// Where the last folder is remembered.
    pub(super) store: StateStore,
    /// Each file's last run and the run in flight.
    pub(super) runs: Runs,
    /// The picked environment: `Default` or a `.env.{name}` name.
    pub(super) environment: String,
    /// `Default`, then the collection's `.env.{name}` names.
    pub(super) environments: Vec<String>,
    pub(super) env_select: Option<Entity<SelectState<Vec<SharedString>>>>,
    pub(super) inspector_tab: InspectorTab,
    /// The Checks tab's body panel shows the raw body, not the diff.
    pub(super) body_raw: bool,
    /// The active file's variables, resolved for the picked environment.
    pub(super) vars: Vec<ResolvedVar>,
    /// Its references with no value anywhere.
    pub(super) missing: Vec<String>,
    pub(super) secrets: Arc<dyn SecretStore>,
    /// `secrets` read once per item, for resolving variables while typing.
    secret_cache: CachedSecrets,
    status: Entity<StatusBar>,
    focus: FocusHandle,
    /// Set once the user agreed to close the window with unsaved changes.
    closing: bool,
    /// Alive while the unsaved-changes dialog is open, so a repeated ⌘Q or
    /// close does not stack another one.
    asking: std::rc::Weak<()>,
    _watcher: Option<notify::RecommendedWatcher>,
    _watch_task: Option<Task<()>>,
    _filter_changes: Subscription,
    _env_changes: Option<Subscription>,
}

impl Workspace {
    pub fn new(
        folder: Option<PathBuf>,
        store: StateStore,
        secrets: Arc<dyn SecretStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter"));
        let filter_changes = cx.subscribe(&filter, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let ws = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            ws.update(cx, |this, cx| this.should_close(window, cx))
                .unwrap_or(true)
        });
        let mut this = Workspace {
            state: AppState::default(),
            tabs: Vec::new(),
            collapsed: HashSet::new(),
            expanded: HashSet::new(),
            selected_request: None,
            filter,
            problems_open: true,
            panel_tab: PanelTab::Problems,
            selected_problem: None,
            hover: None,
            hover_close: None,
            on_popover: false,
            hover_pinned: false,
            store,
            runs: Runs::default(),
            environment: DEFAULT_ENVIRONMENT.to_string(),
            environments: Vec::new(),
            env_select: None,
            inspector_tab: InspectorTab::Response,
            body_raw: false,
            vars: Vec::new(),
            missing: Vec::new(),
            secret_cache: CachedSecrets::new(secrets.clone()),
            secrets,
            status: cx.new(|_| StatusBar::default()),
            focus,
            closing: false,
            asking: std::rc::Weak::new(),
            _watcher: None,
            _watch_task: None,
            _filter_changes: filter_changes,
            _env_changes: None,
        };
        if let Some(folder) = folder {
            this.open_collection(&folder, window, cx);
        }
        this.sync_chrome(window, cx);
        this
    }

    /// Opens `root` as the collection, replacing the current one and its
    /// tabs. Callers settle unsaved changes first.
    pub(super) fn open_collection(
        &mut self,
        root: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let collection = match Collection::open(root) {
            Ok(c) => c,
            Err(e) => {
                notify_error(format!("Couldn't open {}: {e}", root.display()), window, cx);
                return false;
            }
        };
        self.state = AppState {
            collection: Some(collection),
            ..AppState::default()
        };
        self.tabs.clear();
        self.collapsed.clear();
        self.expanded.clear();
        self.selected_request = None;
        self.runs = Runs::default();
        self.secret_cache.invalidate();
        self.environment = self
            .store
            .environment(root)
            .unwrap_or_else(|| DEFAULT_ENVIRONMENT.to_string());
        self.load_environments(window, cx);
        self.watch(root, window, cx);
        self.sync_chrome(window, cx);
        true
    }

    /// Window title and status bar, after the collection or tab changed.
    fn sync_chrome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.state.collection.as_ref().map(|c| c.name.clone());
        window.set_window_title(&match &name {
            Some(name) => format!("curl-runner studio — {name}"),
            None => "curl-runner studio".to_string(),
        });
        let editor = self.state.active.map(|i| self.tabs[i].editor.clone());
        self.status
            .update(cx, |s, cx| s.set(name.map(Into::into), editor, cx));
        self.refresh_vars(cx);
        cx.notify();
    }

    // Environments and variables.

    /// Reads the collection's environments and rebuilds the picker; a
    /// remembered environment that is gone falls back to `Default`.
    fn load_environments(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(collection) = &self.state.collection else {
            return;
        };
        self.environments = list_environments(&collection.root);
        if !self.environments.contains(&self.environment) {
            self.environment = DEFAULT_ENVIRONMENT.to_string();
        }
        let labels: Vec<SharedString> = self
            .environments
            .iter()
            .map(|e| env_label(e).into())
            .collect();
        let selected = self
            .environments
            .iter()
            .position(|e| e == &self.environment)
            .map(gpui_kit::component::IndexPath::new);
        let select = cx.new(|cx| SelectState::new(labels, selected, window, cx));
        self._env_changes = Some(cx.subscribe(
            &select,
            |this, _, event: &SelectEvent<Vec<SharedString>>, cx| {
                let SelectEvent::Confirm(Some(label)) = event else {
                    return;
                };
                let picked = this
                    .environments
                    .iter()
                    .find(|e| env_label(e) == label.as_ref())
                    .cloned();
                if let Some(name) = picked {
                    this.set_environment(&name, cx);
                }
            },
        ));
        self.env_select = Some(select);
    }

    /// Picks `name` for the collection's runs and remembers it.
    pub(super) fn set_environment(&mut self, name: &str, cx: &mut Context<Self>) {
        if self.environment == name {
            return;
        }
        self.environment = name.to_string();
        self.invalidate_secrets();
        if let Some(c) = &self.state.collection {
            self.store.remember_environment(&c.root, name);
        }
        self.refresh_vars(cx);
        cx.notify();
    }

    /// Drops cached Keychain values, so the next lookup reads them again
    /// (after an environment switch, or once secrets are edited).
    pub(super) fn invalidate_secrets(&mut self) {
        self.secret_cache.invalidate();
    }

    /// The active file's variables for the picked environment, for the
    /// status bar and the inspector's missing-variable warning.
    pub(super) fn refresh_vars(&mut self, cx: &mut Context<Self>) {
        let (vars, missing) = match (&self.state.collection, self.state.active_document()) {
            (Some(c), Some(doc)) => {
                let ctx = EnvCtx {
                    root: c.root.clone(),
                    collection: c.name.clone(),
                    environment: self.environment.clone(),
                    marked_secret: Vec::new(),
                };
                let captured = self
                    .runs
                    .last
                    .get(&doc.rel_path)
                    .map(|r| r.store.clone())
                    .unwrap_or_default();
                let names = referenced_vars(&doc.text);
                let vars = resolve_vars(
                    &names,
                    &captured,
                    &self.secret_cache,
                    &ctx,
                    std::slice::from_ref(&doc.rel_path),
                );
                let missing = missing_vars(&doc.text, &vars);
                (vars, missing)
            }
            _ => (Vec::new(), Vec::new()),
        };
        let env = self
            .state
            .collection
            .as_ref()
            .map(|_| SharedString::from(env_label(&self.environment)));
        let captured_by = self
            .state
            .active_document()
            .and_then(|doc| self.runs.last.get(&doc.rel_path))
            .map(|r| r.captured_by.clone())
            .unwrap_or_default();
        let groups = source_groups(&vars, &missing, &captured_by);
        let missing_count = missing.len();
        self.vars = vars;
        self.missing = missing;
        self.status
            .update(cx, |s, cx| s.set_vars(env, groups, missing_count, cx));
    }

    /// The file's request lines changed: re-read them and follow the cursor.
    fn refresh_request_lines(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.tabs[ix].request_lines = request_lines(&self.state.open[ix].text);
        self.cursor_moved(ix, cx);
    }

    /// Keeps the inspector on the request under the cursor.
    fn cursor_moved(&mut self, ix: usize, cx: &mut Context<Self>) {
        let tab = &mut self.tabs[ix];
        let line = tab.editor.read(cx).cursor_position().line;
        let request = request_at_line(&tab.request_lines, line);
        if tab.cursor_request != request {
            tab.cursor_request = request;
            cx.notify();
        }
    }

    // Tabs and editors.

    pub(super) fn open_file(&mut self, rel: &Path, window: &mut Window, cx: &mut Context<Self>) {
        match self.state.open_document(rel) {
            Ok((ix, true)) => {
                let text = self.state.open[ix].text.clone();
                let tab = self.new_editor(text, window, cx);
                self.tabs.push(tab);
                self.show_diagnostics(ix, cx);
                self.refresh_request_lines(ix, cx);
                self.refresh_result_marks(ix);
            }
            Ok((_, false)) => {}
            Err(e) => {
                notify_error(format!("Couldn't open {}: {e}", rel.display()), window, cx);
                return;
            }
        }
        self.focus_active_editor(window, cx);
        self.sync_chrome(window, cx);
    }

    fn new_editor(&self, text: String, window: &mut Window, cx: &mut Context<Self>) -> EditorTab {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("yaml")
                .line_number(true)
                .folding(true)
                .default_value(text.clone())
        });
        let changes = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.editor_changed(&editor, cx);
            }
        });
        let cursor = cx.observe(&editor, |this, editor, cx| {
            if let Some(ix) = this.tab_of(&editor) {
                this.cursor_moved(ix, cx);
            }
        });
        let diagnostics = DiagnosticsView::attach(&editor, cx.entity().downgrade(), cx);
        EditorTab {
            request_lines: request_lines(&text),
            editor,
            diagnostics,
            diagnose: Task::ready(()),
            cursor_request: 0,
            results: ResultMarks::default(),
            _changes: changes,
            _cursor: cursor,
        }
    }

    /// Every edit: the document takes the buffer and is re-diagnosed once
    /// typing pauses. The editor dropped its diagnostics on the change, so
    /// an edit that leaves the document current (a quick fix) gets them back
    /// right away.
    fn editor_changed(&mut self, editor: &Entity<EditorState>, cx: &mut Context<Self>) {
        let Some(ix) = self.tab_of(editor) else {
            return;
        };
        let text = editor.read(cx).value().to_string();
        // The marks and popover point into the old text.
        self.tabs[ix].diagnostics.clear();
        self.close_hover(cx);
        self.selected_problem = None;
        let doc = &mut self.state.open[ix];
        self.tabs[ix].results.edited(&doc.text, &text);
        doc.set_buffer(text);
        if doc.is_stale() {
            let editor = editor.clone();
            self.tabs[ix].diagnose = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(DIAGNOSE_DELAY).await;
                this.update(cx, |this, cx| {
                    if let Some(ix) = this.tab_of(&editor) {
                        this.flush_diagnostics(ix, cx);
                        this.refresh_request_lines(ix, cx);
                        this.refresh_vars(cx);
                    }
                })
                .ok();
            });
        } else {
            self.show_diagnostics(ix, cx);
            self.refresh_request_lines(ix, cx);
        }
        cx.notify();
    }

    pub(super) fn activate_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix < self.tabs.len() {
            self.close_hover(cx);
            self.selected_problem = None;
            self.state.active = Some(ix);
            self.focus_active_editor(window, cx);
            self.sync_chrome(window, cx);
        }
    }

    fn focus_active_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(i) = self.state.active {
            self.tabs[i]
                .editor
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
        }
    }

    /// Closes tab `ix`, asking Save / Don't save / Cancel when it is dirty.
    pub(super) fn request_close_tab(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.state.open.get(ix) else {
            return;
        };
        match doc.close_decision() {
            CloseDecision::Close => self.close_tab(ix, window, cx),
            CloseDecision::Ask => self.confirm_close(doc.rel_path.clone(), window, cx),
        }
    }

    fn close_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix < self.tabs.len() {
            self.state.close(ix);
            self.tabs.remove(ix);
            self.focus_active_editor(window, cx);
            self.sync_chrome(window, cx);
        }
    }

    fn confirm_close(&mut self, rel: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let name = rel
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let ws = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let (save_ws, discard_ws) = (ws.clone(), ws.clone());
            let (save_rel, discard_rel) = (rel.clone(), rel.clone());
            dialog
                .w(px(480.))
                .title(format!("Save changes to {name}?"))
                .child(secondary("Your changes are lost if you don't save them."))
                .footer(three_buttons(
                    "Save",
                    move |window, cx| {
                        save_ws
                            .update(cx, |this, cx| {
                                if let Some(ix) = this.state.position(&save_rel)
                                    && this.save(ix, window, cx)
                                {
                                    this.close_tab(ix, window, cx);
                                }
                            })
                            .ok();
                    },
                    move |window, cx| {
                        discard_ws
                            .update(cx, |this, cx| {
                                if let Some(ix) = this.state.position(&discard_rel) {
                                    this.close_tab(ix, window, cx);
                                }
                            })
                            .ok();
                    },
                ))
        });
    }

    /// Saves tab `ix`; reports and returns `false` on failure.
    pub(super) fn save(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(collection) = &mut self.state.collection else {
            return false;
        };
        let doc = &mut self.state.open[ix];
        if let Err(e) = doc.save(&collection.root) {
            notify_error(
                format!("Couldn't save {}: {e}", doc.rel_path.display()),
                window,
                cx,
            );
            return false;
        }
        // The method badge may have changed.
        collection.refresh();
        cx.notify();
        true
    }

    fn dirty_count(&self) -> usize {
        self.state.open.iter().filter(|d| d.is_dirty()).count()
    }

    /// Runs `then` once unsaved changes in every tab are saved or dropped,
    /// asking first when there are any.
    pub(super) fn settle_unsaved(
        &mut self,
        then: Then,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dirty = self.dirty_count();
        if dirty == 0 {
            return then(self, window, cx);
        }
        // The token lives in the dialog's builder, which the Kit drops when
        // the dialog goes; while one is closing it is no longer active.
        if self.asking.upgrade().is_some() && window.has_active_dialog(cx) {
            return;
        }
        let token = Rc::new(());
        self.asking = Rc::downgrade(&token);
        let ws = cx.entity().downgrade();
        let title = match dirty {
            1 => "Save changes to 1 file?".to_string(),
            n => format!("Save changes to {n} files?"),
        };
        window.open_dialog(cx, move |dialog, _, _| {
            let _asking = &token;
            let (save_ws, discard_ws) = (ws.clone(), ws.clone());
            let (save_then, discard_then) = (then.clone(), then.clone());
            dialog
                .w(px(480.))
                .title(title.clone())
                .child(secondary("Your changes are lost if you don't save them."))
                .footer(three_buttons(
                    "Save all",
                    move |window, cx| {
                        save_ws
                            .update(cx, |this, cx| {
                                let all_saved = (0..this.state.open.len())
                                    .filter(|&i| this.state.open[i].is_dirty())
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .all(|i| this.save(i, window, cx));
                                if all_saved {
                                    save_then(this, window, cx);
                                }
                            })
                            .ok();
                    },
                    move |window, cx| {
                        discard_ws
                            .update(cx, |this, cx| discard_then(this, window, cx))
                            .ok();
                    },
                ))
        });
    }

    fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.closing || self.dirty_count() == 0 {
            return true;
        }
        self.settle_unsaved(
            Rc::new(|this, window, _| {
                this.closing = true;
                window.remove_window();
            }),
            window,
            cx,
        );
        false
    }

    /// Quits once unsaved changes are saved or dropped.
    fn quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settle_unsaved(
            Rc::new(|this, _, cx| {
                this.closing = true;
                cx.quit();
            }),
            window,
            cx,
        );
    }

    // Disk changes.

    fn watch(&mut self, root: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let (watcher, mut rx) = watch::start(root);
        self._watcher = watcher;
        self._watch_task = Some(cx.spawn_in(window, async move |this, cx| {
            while let Some(mut changed) = rx.next().await {
                // Coalesce a burst (a save, a checkout) into one refresh.
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                while let Ok(more) = rx.try_recv() {
                    changed.extend(more);
                }
                if this
                    .update_in(cx, |this, window, cx| {
                        this.disk_changed(&changed, window, cx)
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    /// YAML files or folders under the root changed: refresh the tree,
    /// reload clean tabs whose file is in `changed`, flag dirty ones.
    fn disk_changed(&mut self, changed: &[PathBuf], window: &mut Window, cx: &mut Context<Self>) {
        let Some(collection) = &mut self.state.collection else {
            return;
        };
        collection.refresh();
        let root = collection.root.clone();
        if list_environments(&root) != self.environments {
            self.load_environments(window, cx);
        }
        let reread = watch::tabs_to_reread(
            self.state.open.iter().map(|d| d.rel_path.as_path()),
            changed,
        );
        for ix in reread {
            let doc = &mut self.state.open[ix];
            let Ok(disk) = std::fs::read_to_string(root.join(&doc.rel_path)) else {
                continue;
            };
            let reload = !doc.is_dirty() && disk != doc.saved_text;
            doc.on_disk_changed(disk);
            if reload {
                set_editor_text(&self.tabs[ix].editor, &doc.text, window, cx);
                self.show_diagnostics(ix, cx);
            }
        }
        cx.notify();
    }

    pub(super) fn resolve_conflict(
        &mut self,
        keep_mine: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.state.active else { return };
        let doc = &mut self.state.open[ix];
        doc.resolve_conflict(keep_mine);
        if !keep_mine {
            set_editor_text(&self.tabs[ix].editor, &doc.text, window, cx);
            self.show_diagnostics(ix, cx);
        }
        cx.notify();
    }

    // Tree actions.

    pub(super) fn toggle_problems(&mut self, cx: &mut Context<Self>) {
        self.problems_open = !self.problems_open;
        cx.notify();
    }

    pub(super) fn open_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settle_unsaved(
            Rc::new(|_, window, cx| {
                let picked = cx.prompt_for_paths(PathPromptOptions {
                    files: false,
                    directories: true,
                    multiple: false,
                    prompt: Some("Open".into()),
                });
                cx.spawn_in(window, async move |this, cx| {
                    let Ok(Ok(Some(paths))) = picked.await else {
                        return;
                    };
                    let Some(path) = paths.into_iter().next() else {
                        return;
                    };
                    this.update_in(cx, |this, window, cx| {
                        if this.open_collection(&path, window, cx) {
                            this.store.remember_folder(&path);
                        }
                    })
                    .ok();
                })
                .detach();
            }),
            window,
            cx,
        );
    }

    pub(super) fn prompt_rename(
        &mut self,
        rel: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = rel
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name.clone()));
        let first_build = Cell::new(true);
        let ws = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, window, cx| {
            if first_build.replace(false) {
                input.read(cx).focus_handle(cx).focus(window, cx);
            }
            let (ws, rel, input) = (ws.clone(), rel.clone(), input.clone());
            dialog
                .w(px(480.))
                .title(format!("Rename {name}"))
                .child(Input::new(&input))
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("rename-cancel")
                                .ghost()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(Button::new("rename-ok").primary().label("Rename").on_click(
                            move |_, window, cx| {
                                let to = input.read(cx).value().trim().to_string();
                                window.close_dialog(cx);
                                ws.update(cx, |this, cx| this.rename(&rel, &to, window, cx))
                                    .ok();
                            },
                        )),
                )
        });
    }

    /// Renames `from` to `new_name` in the same folder. A file keeps its
    /// YAML extension when the new name has none.
    fn rename(&mut self, from: &Path, new_name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if new_name.is_empty() || new_name.contains('/') {
            return;
        }
        let Some(collection) = &mut self.state.collection else {
            return;
        };
        let mut file_name = new_name.to_string();
        let is_file = collection.root.join(from).is_file();
        if is_file && request_label(&file_name) == file_name {
            let ext = from
                .extension()
                .map_or("yaml".into(), |e| e.to_string_lossy());
            file_name = format!("{file_name}.{ext}");
        }
        let to = from.with_file_name(&file_name);
        if to == from {
            return;
        }
        match collection.rename(from, &to) {
            Ok(()) => {
                self.state.renamed(from, &to);
                if self.collapsed.remove(from) {
                    self.collapsed.insert(to.clone());
                }
                if self.expanded.remove(from) {
                    self.expanded.insert(to);
                }
                self.sync_chrome(window, cx);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => notify_error(
                format!("“{file_name}” already exists here. Pick another name."),
                window,
                cx,
            ),
            Err(e) => notify_error(
                format!("Couldn't rename {}: {e}", from.display()),
                window,
                cx,
            ),
        }
    }

    pub(super) fn prompt_delete(
        &mut self,
        rel: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = rel
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let ws = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (ws, rel) = (ws.clone(), rel.clone());
            alert
                .title(format!("Delete {name}?"))
                .description("It is removed from disk. You can't undo this here.")
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    ws.update(cx, |this, cx| this.delete(&rel, window, cx)).ok();
                    true
                })
        });
    }

    fn delete(&mut self, rel: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let Some(collection) = &mut self.state.collection else {
            return;
        };
        if let Err(e) = collection.delete(rel) {
            notify_error(
                format!("Couldn't delete {}: {e}", rel.display()),
                window,
                cx,
            );
            return;
        }
        for ix in self.state.tabs_under(rel) {
            self.state.close(ix);
            self.tabs.remove(ix);
        }
        self.sync_chrome(window, cx);
    }

    /// The method badge for `rel`, from the tree.
    pub(super) fn method_of(&self, rel: &Path) -> Option<String> {
        fn find(nodes: &[TreeNode], rel: &Path) -> Option<Option<String>> {
            nodes.iter().find_map(|n| match n {
                TreeNode::Folder { children, .. } => find(children, rel),
                TreeNode::File {
                    rel_path, method, ..
                } => (rel_path == rel).then(|| method.clone()),
            })
        }
        find(&self.state.collection.as_ref()?.tree, rel).flatten()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.state.collection.is_some();
        let main = if open {
            self.render_editor_pane(cx).into_any_element()
        } else {
            self.render_welcome(cx).into_any_element()
        };
        div()
            .id("workspace")
            .key_context("Workspace")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &OpenFolder, window, cx| this.open_folder(window, cx)))
            .on_action(cx.listener(|this, _: &SaveFile, window, cx| {
                if let Some(ix) = this.state.active {
                    this.save(ix, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CloseTab, window, cx| {
                if let Some(ix) = this.state.active {
                    this.request_close_tab(ix, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &NewRequest, window, cx| {
                this.new_request(None, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NewCollection, window, cx| {
                this.new_collection(window, cx);
            }))
            .on_action(cx.listener(|this, _: &Import, window, cx| {
                this.import(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenPalette, window, cx| {
                this.open_palette(window, cx);
            }))
            .on_action(cx.listener(|this, _: &QuickFix, window, cx| this.quick_fix(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleProblems, _, cx| this.toggle_problems(cx)))
            .on_action(cx.listener(|this, _: &RunActive, window, cx| this.run_active(window, cx)))
            .on_action(cx.listener(|this, _: &RunRequestUnderCursor, window, cx| {
                this.run_request_under_cursor(window, cx)
            }))
            .on_action(cx.listener(|this, _: &RunCollection, window, cx| {
                this.run(crate::model::run::RunTarget::Collection, window, cx)
            }))
            .on_action(cx.listener(|this, _: &CopyAsCurl, _, cx| this.copy_as_curl(cx)))
            .on_action(cx.listener(|this, _: &Rerun, window, cx| this.rerun(window, cx)))
            .size_full()
            .flex()
            .flex_col()
            .bg(color("bg-100"))
            .text_color(color("text-primary"))
            .font_family(ui_font())
            .text_size(px(13.))
            .child(self.render_title_bar(window, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(main)
                    .when(open, |row| row.child(self.render_inspector(cx))),
            )
            .child(self.status.clone())
    }
}

/// Quits through the window's workspace, so unsaved changes are settled
/// even when focus is in a dialog or the palette: the Kit draws those beside
/// the workspace, so the action never reaches it. With no workspace open
/// there is nothing unsaved, so it quits directly.
pub(super) fn quit_app(cx: &mut App) {
    let found = cx.windows().into_iter().find_map(|handle| {
        let root = handle.downcast::<gpui_kit::base::Root>()?;
        let view = root.read(cx).ok()?.view().clone();
        Some((handle, view.downcast::<Workspace>().ok()?))
    });
    match found {
        Some((handle, ws)) => {
            let _ = handle.update(cx, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.quit(window, cx))
            });
        }
        None => cx.quit(),
    }
}

/// Replaces an editor's text from disk, keeping the cursor where it was.
fn set_editor_text(editor: &Entity<EditorState>, text: &str, window: &mut Window, cx: &mut App) {
    let text = text.to_string();
    editor.update(cx, |e, cx| {
        let cursor = e.cursor_position();
        e.set_value(text, window, cx);
        e.set_cursor_position(cursor, window, cx);
    });
}

/// Secrets for tests: in memory, never the user's Keychain.
#[cfg(test)]
pub(super) fn test_secrets() -> Arc<dyn SecretStore> {
    Arc::new(crate::model::secrets::MemorySecrets::default())
}

pub(super) fn notify_error(message: String, window: &mut Window, cx: &mut App) {
    log::warn!("{message}");
    window.push_notification(Notification::error(message), cx);
}

fn secondary(text: &'static str) -> impl IntoElement {
    div().text_color(color("text-secondary")).child(text)
}

/// Cancel, Don't save and the primary `save_label` button.
fn three_buttons(
    save_label: &'static str,
    on_save: impl Fn(&mut Window, &mut App) + 'static,
    on_discard: impl Fn(&mut Window, &mut App) + 'static,
) -> DialogFooter {
    DialogFooter::new()
        .child(
            Button::new("unsaved-cancel")
                .ghost()
                .label("Cancel")
                .on_click(|_, window, cx| window.close_dialog(cx)),
        )
        .child(
            Button::new("unsaved-discard")
                .outline()
                .label("Don't save")
                .on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    on_discard(window, cx);
                }),
        )
        .child(
            Button::new("unsaved-save")
                .primary()
                .label(save_label)
                .on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    on_save(window, cx);
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::super::app::{QuickFix, Quit};
    use super::super::state::StateStore;
    use super::Workspace;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::component::input::{CodeActionItem, Position};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Action as _, AnyWindowHandle, AppContext as _, Bounds, Entity, Point, Role, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;
    use std::time::Duration;

    /// A window on a collection of 2 folders and 3 files.
    fn open(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        for rel in [
            "auth/login.yaml",
            "charges/create-charge.yaml",
            "smoke.yaml",
        ] {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "request:\n  url: https://api.example.com\n").unwrap();
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
                        super::super::workspace::test_secrets(),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        (dir, window, ws)
    }

    #[gpui_kit::test]
    fn workspace_renders_tree_rows(cx: &mut TestAppContext) {
        let (_dir, window, _) = open(cx);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            let rows = window
                .within("collection-tree")
                .find_all_by_role(Role::TreeItem);
            // auth/, login, charges/, create-charge, smoke
            assert_eq!(rows.len(), 5);
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn clicking_a_row_opens_a_tab_and_typing_dirties_it(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("collection-tree")
                .click(("tree-row", 4usize), cx);
            window.render_frame(cx);
            window.input("# x\n", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.open.len(), 1);
            assert_eq!(ws.state.open[0].rel_path, Path::new("smoke.yaml"));
            assert!(ws.state.open[0].text.contains("# x"));
            assert!(ws.state.open[0].is_dirty());
        });
    }

    #[gpui_kit::test]
    fn quit_and_window_close_ask_when_a_file_is_dirty(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("collection-tree")
                .click(("tree-row", 4usize), cx);
            window.render_frame(cx);
            window.input("# x\n", cx);
            // Dispatch is deferred to the end of this update.
            window.dispatch_action(Quit.boxed_clone(), cx);
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.has_active_dialog(cx), "quit must ask first");
            window.close_dialog(cx);
            let closes = ws.update(cx, |ws, cx| ws.should_close(window, cx));
            assert!(!closes, "window close must ask first");
            assert!(window.has_active_dialog(cx));
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(!ws.closing));
    }

    /// Opens `smoke.yaml` and types into it, so it is dirty.
    fn dirty_smoke(window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
        window.render_frame(cx);
        window
            .within("collection-tree")
            .click(("tree-row", 4usize), cx);
        window.render_frame(cx);
        window.input("# x\n", cx);
    }

    #[gpui_kit::test]
    fn quit_from_the_palette_asks_when_a_file_is_dirty(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            dirty_smoke(window, cx);
            ws.update(cx, |ws, cx| drop(ws.open_palette(window, cx)));
            // The palette takes focus as it renders.
            window.render_frame(cx);
            window.dispatch_action(Quit.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            assert!(ws.asking.upgrade().is_some(), "quit must ask first");
            assert!(!ws.closing);
        });
    }

    #[gpui_kit::test]
    fn repeated_quit_does_not_stack_dialogs(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            dirty_smoke(window, cx);
            window.dispatch_action(Quit.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
        let first = ws.read_with(cx, |ws, _| ws.asking.clone());
        assert!(first.upgrade().is_some(), "quit must ask first");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            // From inside the dialog, as a second ⌘Q would come.
            window.dispatch_action(Quit.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            assert!(ws.asking.ptr_eq(&first), "a second dialog was opened");
        });
        cx.update_window(window, |_, window, cx| {
            window.close_dialog(cx);
            window.render_frame(cx);
            assert!(!window.has_active_dialog(cx), "only one dialog was open");
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn typing_rediagnoses_after_a_pause(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("collection-tree")
                .click(("tree-row", 4usize), cx);
            window.render_frame(cx);
            window.input("[", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            let doc = &ws.state.open[0];
            assert!(doc.is_stale(), "a keystroke must not diagnose");
            assert!(doc.diagnostics.is_empty());
        });
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            let doc = &ws.state.open[0];
            assert!(!doc.is_stale());
            assert_eq!(doc.diagnostics.len(), 1, "{:?}", doc.diagnostics);
            let shown = ws.tabs[0].diagnostics.marks.borrow().shown.len();
            assert_eq!(shown, 1, "the editor shows the new diagnostics");
        });
    }

    #[gpui_kit::test]
    fn quick_fix_applies_the_fix_at_the_cursor(cx: &mut TestAppContext) {
        let (dir, window, ws) = open(cx);
        std::fs::write(
            dir.path().join("typo.yaml"),
            "request:\n  url: https://api.example.com\n  timout: 5000\n",
        )
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("typo.yaml"), window, cx);
                ws.tabs[0].editor.update(cx, |e, cx| {
                    e.set_cursor_position(Position::new(2, 4), window, cx)
                });
            });
            window.render_frame(cx);
            assert_eq!(
                ws.read(cx).tabs[0].diagnostics.marks.borrow().shown.len(),
                1
            );
            window.dispatch_action(QuickFix.boxed_clone(), cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, cx| {
            let doc = &ws.state.open[0];
            assert!(doc.text.contains("  timeout: 5000"), "{}", doc.text);
            assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
            assert_eq!(ws.tabs[0].editor.read(cx).value().as_ref(), doc.text);
            assert!(doc.is_dirty());
        });
    }

    #[gpui_kit::test]
    fn code_action_lists_the_fix_and_applies_it(cx: &mut TestAppContext) {
        let (dir, window, ws) = open(cx);
        let text = "request:\n  url: https://api.example.com\n  timout: 5000\n";
        std::fs::write(dir.path().join("typo.yaml"), text).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("typo.yaml"), window, cx)
            });
            let editor = ws.read(cx).tabs[0].editor.clone();
            let at = text.find("timout").unwrap() + 2;
            let actions = ws.update(cx, |ws, cx| ws.code_actions(&editor, at..at, cx));
            let fix = ws.read(cx).state.open[0].diagnostics[0].fix.clone();
            let titles: Vec<_> = actions.iter().map(|a| Some(a.title.clone())).collect();
            assert_eq!(titles, [fix]);
            let item = CodeActionItem {
                provider_id: "curl-runner".into(),
                action: actions[0].clone(),
            };
            // As the menu does: from inside the editor's update.
            editor.update(cx, |e, cx| e.perform_code_action(&item, window, cx));
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            let doc = &ws.state.open[0];
            assert!(doc.text.contains("  timeout: 5000"), "{}", doc.text);
        });
    }

    #[gpui_kit::test]
    fn clean_window_closes_without_asking(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            assert!(ws.update(cx, |ws, cx| ws.should_close(window, cx)));
            assert!(!window.has_active_dialog(cx));
        })
        .unwrap();
    }
}
