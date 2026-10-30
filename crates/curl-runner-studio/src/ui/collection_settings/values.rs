//! Saving the drawer: where a value goes (a `.env` file or the Keychain),
//! the confirms before a move out of the Keychain or a plain copy left
//! behind, and deleting a variable.

use super::*;

impl CollectionSettings {
    /// Saves the drawer. Moving a value out of the Keychain, leaving a
    /// plain copy behind, or unmarking a name other environments keep secret
    /// asks first (`confirmed` once it did).
    pub fn save(&mut self, confirmed: Confirmed, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.drawer_name(cx) else {
            return;
        };
        let Some(d) = &self.drawer else { return };
        if name.is_empty() {
            notify_error("Name the variable first.".into(), window, cx);
            return;
        }
        let new = d.name.is_none();
        let received = self.received(cx);
        if new && received {
            return;
        }
        if new
            && self
                .rows
                .iter()
                .any(|r| r.name == name && !matches!(r.source, RowSource::Captured { .. }))
        {
            notify_error(
                format!("{name} already exists in {}.", env_label(&self.environment)),
                window,
                cx,
            );
            return;
        }
        let secret = d.secret || is_secret(&name, &[]);
        let own_file = self.own_file();
        let plan = plan_value(secret, &d.source, d.edited);
        // Every plain copy the environment reads; the own file's goes with
        // the move, the others are asked about.
        let copies: Vec<PathBuf> = match plan {
            ValueWrite::Keychain => plain_copies(&self.root, &self.environment, &name),
            _ => Vec::new(),
        };
        let asks_copies = copies.iter().any(|p| *p != own_file);
        let unmarking = !secret && self.meta(cx).secret_names.contains(&name);
        let elsewhere = if unmarking {
            self.keychain_elsewhere(&name, cx)
        } else {
            Vec::new()
        };
        if confirmed == Confirmed::No {
            match &plan {
                ValueWrite::Keychain if asks_copies => {
                    self.confirm_copy(copies, window, cx);
                    return;
                }
                ValueWrite::KeychainToFile => {
                    self.confirm_plain_text(&name, elsewhere, true, window, cx);
                    return;
                }
                _ if !elsewhere.is_empty() => {
                    self.confirm_plain_text(&name, elsewhere, false, window, cx);
                    return;
                }
                _ => {}
            }
        }
        let value = d.value.read(cx).value().to_string();
        let (root, env, service) = (
            self.root.clone(),
            self.environment.clone(),
            self.service(cx),
        );
        let secrets: &dyn SecretStore = &*self.secrets;
        // A copy that could not be removed once the Keychain has the value:
        // the name is still marked secret, then the error is shown.
        let mut leftover: Option<String> = None;
        let written = match &plan {
            ValueWrite::Nothing => Ok(()),
            ValueWrite::Keychain if asks_copies && confirmed == Confirmed::KeepCopy => secrets
                .set(&service, &name, &value)
                .map_err(std::io::Error::other),
            ValueWrite::Keychain => move_to_keychain(&root, &env, &name, &value, secrets, &service)
                .inspect(|()| {
                    let failed: Vec<String> = copies
                        .iter()
                        .filter(|p| **p != own_file && !p.to_string_lossy().ends_with(".local"))
                        .filter_map(|p| {
                            remove_env_var_at(&root, p, &name)
                                .err()
                                .map(|e| e.to_string())
                        })
                        .collect();
                    if !failed.is_empty() {
                        leftover = Some(failed.join("; "));
                    }
                }),
            ValueWrite::File => write_env_var(&root, &env, &name, &value),
            ValueWrite::KeychainToFile => {
                let value = (d.edited || d.read_error.is_none()).then_some(value.as_str());
                move_to_file(&root, &env, &name, value, secrets, &service)
            }
        };
        if let Err(e) = written {
            notify_error(format!("Couldn't save {name}: {e}"), window, cx);
            return;
        }
        let description = d.description.read(cx).value().trim().to_string();
        let required = d.required;
        // A received copy's notes are the owner's: only the value changed.
        let edit_meta = !received;
        // Into the shared notes, read as they are now (the share may have
        // ticked a flag, an import marked a secret, since the drawer
        // opened); the workspace saves them, once.
        edit_notes(&self.notes, cx, |meta| {
            if !edit_meta {
                return;
            }
            // Unmarked here, a name other environments keep in the Keychain
            // stays secret collection-wide.
            if secret || elsewhere.is_empty() {
                meta.set_secret(&name, secret);
            }
            // Share default is the Share dialog's; a secret drops it.
            let notes = VarMeta {
                description,
                required_for_shared: required,
                share_default: !secret && meta.vars.get(&name).is_some_and(|v| v.share_default),
                share_locked: false,
            };
            if notes == VarMeta::default() {
                meta.vars.remove(&name);
            } else {
                meta.vars.insert(name.clone(), notes);
            }
        });
        self.drawer = None;
        self.reload(cx);
        cx.emit(SettingsEvent::Changed);
        match leftover {
            Some(e) => notify_error(
                format!("{name} is in the Keychain, but a plain copy could not be removed: {e}"),
                window,
                cx,
            ),
            None => window.push_notification(Notification::success(format!("Saved {name}")), cx),
        }
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Before a Keychain value goes into a file, or a name other
    /// environments keep secret is unmarked.
    pub(super) fn confirm_plain_text(
        &self,
        name: &str,
        elsewhere: Vec<String>,
        to_file: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut text = Vec::new();
        if to_file {
            text.push(format!(
                "This value will be stored in plain text in {}",
                self.file_name()
            ));
        }
        if !elsewhere.is_empty() {
            text.push(format!("{name} stays secret in {}.", elsewhere.join(", ")));
        }
        let description = text.join(". ");
        let (title, ok) = if to_file {
            ("Store this value in a file?", "Store in file")
        } else {
            ("Keep it secret elsewhere?", "OK")
        };
        let this = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let this = this.clone();
            alert
                .title(title)
                .description(description.clone())
                .ok_text(ok)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    this.update(cx, |this, cx| this.save(Confirmed::Yes, window, cx))
                        .ok();
                    true
                })
        });
    }

    /// Before a value moves into the Keychain while plain copies remain in
    /// files the environment reads: Remove them, Keep them, or Cancel.
    pub(super) fn confirm_copy(
        &self,
        files: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let description = copies_text(&files);
        // `.local` overrides are the person's own: never written.
        let removable = files
            .iter()
            .any(|p| !p.to_string_lossy().ends_with(".local"));
        let this = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let (keep, remove) = (this.clone(), this.clone());
            dialog
                .w(px(480.))
                .title("Move to the Keychain?")
                .child(
                    div()
                        .text_color(color("text-secondary"))
                        .child(description.clone()),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("copy-cancel")
                                .ghost()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("copy-keep")
                                .outline()
                                .label("Keep copy")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    keep.update(cx, |this, cx| {
                                        this.save(Confirmed::KeepCopy, window, cx)
                                    })
                                    .ok();
                                }),
                        )
                        .when(removable, |f| {
                            f.child(
                                Button::new("copy-remove")
                                    .primary()
                                    .label("Remove")
                                    .on_click(move |_, window, cx| {
                                        window.close_dialog(cx);
                                        remove
                                            .update(cx, |this, cx| {
                                                this.save(Confirmed::Yes, window, cx)
                                            })
                                            .ok();
                                    }),
                            )
                        }),
                )
        });
    }

    pub(super) fn confirm_delete(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.received(cx) {
            return;
        }
        let Some(name) = self.drawer.as_ref().and_then(|d| d.name.clone()) else {
            // A new variable has nothing to delete.
            return;
        };
        let file = self.file_name();
        let env = env_label(&self.environment);
        let this = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (this, name) = (this.clone(), name.clone());
            alert
                .title(format!("Delete {name}?"))
                .description(format!(
                    "It is removed from {file} and from your Keychain for {env}."
                ))
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    this.update(cx, |this, cx| this.delete(&name, window, cx))
                        .ok();
                    true
                })
        });
    }

    pub(super) fn delete(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.received(cx) {
            return;
        }
        let service = self.service(cx);
        let removed = remove_env_var(&self.root, &self.environment, name)
            .map_err(|e| e.to_string())
            .and_then(|()| self.secrets.delete(&service, name));
        if let Err(e) = removed {
            notify_error(format!("Couldn't delete {name}: {e}"), window, cx);
        }
        // The mark goes once no environment's file or Keychain (names only)
        // has the name; an unreadable Keychain keeps it.
        let left = self.environments.iter().any(|e| {
            name_in_env_files(&self.root, e, name)
                || self
                    .secrets
                    .try_names(&self.service_of(e, cx))
                    .map_or(true, |n| n.iter().any(|n| n == name))
        });
        if !left {
            edit_notes(&self.notes, cx, |meta| meta.set_secret(name, false));
        }
        self.drawer = None;
        self.reload(cx);
        cx.emit(SettingsEvent::Changed);
        cx.notify();
    }
}
