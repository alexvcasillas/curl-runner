//! The root view: owns the [`AppState`], the editors and the file watcher,
//! and lays out title bar, sidebar, editor and status bar.

use super::app::{
    CloseTab, Import, NewCollection, NewRequest, OpenFolder, Quit, SaveFile, ui_font,
};
use super::state::{self, AppState};
use super::status_bar::StatusBar;
use super::watch;
use super::{color, sidebar::request_label};
use crate::model::collection::{Collection, TreeNode};
use crate::model::document::CloseDecision;
use futures::StreamExt as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{EditorState, Input, InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

/// A tab's editor; `AppState::open[i]` is the document of `tabs[i]`.
pub(super) struct EditorTab {
    pub editor: Entity<EditorState>,
    _changes: Subscription,
}

/// What to do once unsaved changes are settled.
type Then = Rc<dyn Fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>;

pub struct Workspace {
    pub(super) state: AppState,
    pub(super) tabs: Vec<EditorTab>,
    /// Folders the user collapsed, by path relative to the root.
    pub(super) collapsed: HashSet<PathBuf>,
    pub(super) filter: Entity<InputState>,
    status: Entity<StatusBar>,
    focus: FocusHandle,
    /// Set once the user agreed to close the window with unsaved changes.
    closing: bool,
    _watcher: Option<notify::RecommendedWatcher>,
    _watch_task: Option<Task<()>>,
    _filter_changes: Subscription,
}

impl Workspace {
    pub fn new(folder: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            filter,
            status: cx.new(|_| StatusBar::default()),
            focus,
            closing: false,
            _watcher: None,
            _watch_task: None,
            _filter_changes: filter_changes,
        };
        if let Some(folder) = folder {
            this.open_collection(&folder, window, cx);
        }
        this.sync_chrome(window, cx);
        this
    }

    /// Opens `root` as the collection, replacing the current one and its
    /// tabs. Callers settle unsaved changes first.
    fn open_collection(
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
        cx.notify();
    }

    // Tabs and editors.

    pub(super) fn open_file(&mut self, rel: &Path, window: &mut Window, cx: &mut Context<Self>) {
        match self.state.open_document(rel) {
            Ok((ix, true)) => {
                let text = self.state.open[ix].text.clone();
                let tab = self.new_editor(text, window, cx);
                self.tabs.push(tab);
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
                .default_value(text)
        });
        let changes = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.editor_changed(&editor, cx);
            }
        });
        EditorTab {
            editor,
            _changes: changes,
        }
    }

    /// Every edit: the document takes the buffer (and re-diagnoses).
    fn editor_changed(&mut self, editor: &Entity<EditorState>, cx: &mut Context<Self>) {
        if let Some(ix) = self.tabs.iter().position(|t| &t.editor == editor) {
            let text = editor.read(cx).value().to_string();
            self.state.open[ix].set_text(text);
            cx.notify();
        }
    }

    pub(super) fn activate_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix < self.tabs.len() {
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
    fn save(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
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
    fn settle_unsaved(&mut self, then: Then, window: &mut Window, cx: &mut Context<Self>) {
        let dirty = self.dirty_count();
        if dirty == 0 {
            return then(self, window, cx);
        }
        let ws = cx.entity().downgrade();
        let title = match dirty {
            1 => "Save changes to 1 file?".to_string(),
            n => format!("Save changes to {n} files?"),
        };
        window.open_dialog(cx, move |dialog, _, _| {
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
        }
        cx.notify();
    }

    // Tree actions.

    /// Placeholder until the New dialog exists: `folder` is where the
    /// request would go.
    pub(super) fn new_request(&mut self, folder: Option<PathBuf>, _: &mut Context<Self>) {
        log::debug!("new request in {folder:?}: the New dialog is not built yet");
    }

    /// Placeholder until the New collection dialog exists.
    pub(super) fn new_collection(&mut self, _: &mut Context<Self>) {
        log::debug!("new collection: the New dialog is not built yet");
    }

    /// Placeholder until the New dialog's From curl template exists.
    pub(super) fn import(&mut self, _: &mut Context<Self>) {
        log::debug!("import: the New dialog is not built yet");
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
                            state::remember_folder(&path);
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
                    self.collapsed.insert(to);
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let main = if self.state.collection.is_some() {
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
            .on_action(cx.listener(|this, _: &Quit, window, cx| this.quit(window, cx)))
            .on_action(cx.listener(|this, _: &NewRequest, _, cx| this.new_request(None, cx)))
            .on_action(cx.listener(|this, _: &NewCollection, _, cx| this.new_collection(cx)))
            .on_action(cx.listener(|this, _: &Import, _, cx| this.import(cx)))
            .size_full()
            .flex()
            .flex_col()
            .bg(color("bg-100"))
            .text_color(color("text-primary"))
            .font_family(ui_font())
            .text_size(px(13.))
            .child(self.render_title_bar())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(main),
            )
            .child(self.status.clone())
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

fn notify_error(message: String, window: &mut Window, cx: &mut App) {
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
    use super::super::app::Quit;
    use super::Workspace;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Action as _, AnyWindowHandle, AppContext as _, Bounds, Entity, Point, Role, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;

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
                cx.new(|cx| Workspace::new(Some(root), window, cx))
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
