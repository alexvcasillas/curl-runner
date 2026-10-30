//! The tree's actions: opening a folder, renaming and deleting files, and
//! the problems panel toggle.

use super::*;

impl Workspace {
    // Tree actions.

    pub(in crate::ui) fn toggle_problems(&mut self, cx: &mut Context<Self>) {
        self.problems_open = !self.problems_open;
        cx.notify();
    }

    pub(in crate::ui) fn open_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                            this.remember_folder(&path);
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

    pub(in crate::ui) fn prompt_rename(
        &mut self,
        rel: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.refuse_read_only(window, cx) {
            return;
        }
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
    pub(super) fn rename(
        &mut self,
        from: &Path,
        new_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.refuse_read_only(window, cx) || new_name.is_empty() || new_name.contains('/') {
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

    pub(in crate::ui) fn prompt_delete(
        &mut self,
        rel: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.refuse_read_only(window, cx) {
            return;
        }
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

    pub(super) fn delete(&mut self, rel: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if self.refuse_read_only(window, cx) {
            return;
        }
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
    pub(in crate::ui) fn method_of(&self, rel: &Path) -> Option<String> {
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
