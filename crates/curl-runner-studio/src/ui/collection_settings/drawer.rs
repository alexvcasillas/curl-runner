//! The Environments page's drawer: opening it on a variable, its toggles
//! and how it is drawn.

use super::*;

impl CollectionSettings {
    /// Opens the drawer on `name` (its row), or on a new variable. A
    /// Keychain value is read here, once, through the cached store.
    pub fn open_variable(
        &mut self,
        name: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A received copy's list is the owner's: no new names.
        if name.is_none() && self.received(cx) {
            return;
        }
        let row = name
            .and_then(|n| self.rows.iter().find(|r| r.name == n))
            .cloned();
        // The owner's shared default is read only here.
        if row
            .as_ref()
            .is_some_and(|r| matches!(r.source, RowSource::Shared(_)))
        {
            return;
        }
        let (value, read_error) = match &row {
            Some(r) => self.read_value(r, cx),
            None => (String::new(), None),
        };
        let secret = match &row {
            Some(r) => r.secret,
            None => name.is_some_and(|n| is_secret(n, &self.meta(cx).secret_names)),
        };
        let notes = name
            .and_then(|n| self.meta(cx).vars.get(n))
            .cloned()
            .unwrap_or_default();
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("API_TOKEN"));
        let value_input = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(secret)
                .default_value(value)
        });
        let description = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(3)
                .placeholder("What it is and where to get one")
                .default_value(notes.description.clone())
        });
        let edited = cx.subscribe(&value_input, |this, _, e: &InputEvent, cx| {
            if matches!(e, InputEvent::Change)
                && let Some(d) = &mut this.drawer
            {
                d.edited = true;
                cx.notify();
            }
        });
        // A typed `SECRET_` name is a secret: its value is masked.
        let typed = cx.subscribe_in(
            &name_input,
            window,
            |this, input, e: &InputEvent, window, cx| {
                if !matches!(e, InputEvent::Change) {
                    return;
                }
                let prefixed = input.read(cx).value().trim().starts_with("SECRET_");
                if let Some(d) = &mut this.drawer
                    && prefixed
                    && !d.secret
                {
                    d.secret = true;
                    d.revealed = false;
                    d.value.update(cx, |s, cx| s.set_masked(true, window, cx));
                }
                cx.notify();
            },
        );
        if name.is_none() {
            name_input.read(cx).focus_handle(cx).focus(window, cx);
        }
        self.drawer = Some(Drawer {
            // A name without a row yet (the usage scan is still running) is
            // edited as an unset variable.
            name: name.map(String::from),
            name_input,
            value: value_input,
            description,
            secret,
            required: notes.required_for_shared,
            revealed: false,
            edited: false,
            source: row.map_or(RowSource::NotSet, |r| r.source),
            read_error,
            _subscriptions: vec![edited, typed],
        });
        cx.notify();
    }

    /// The value the drawer starts with: the Keychain's for a Keychain row
    /// (read now), the file's for a file row, nothing for the others.
    pub(super) fn read_value(&mut self, row: &EnvRow, cx: &App) -> (String, Option<String>) {
        match &row.source {
            RowSource::Keychain => match self.secrets.try_get(&self.service(cx), &row.name) {
                Ok(v) => (v.unwrap_or_default(), None),
                Err(e) => {
                    log::warn!("Couldn't read {} from the Keychain: {e}", row.name);
                    self.keychain_locked = true;
                    (
                        String::new(),
                        Some(
                            "The Keychain is locked. Unlock it to see or change this value.".into(),
                        ),
                    )
                }
            },
            RowSource::File(p) => {
                let value = std::fs::read_to_string(p)
                    .ok()
                    .and_then(|t| parse_env_content(&t).get(&row.name).cloned())
                    .unwrap_or_default();
                (value, None)
            }
            RowSource::Process | RowSource::Shared(_) => {
                (row.value.clone().unwrap_or_default(), None)
            }
            RowSource::Captured { .. } | RowSource::NotSet => (String::new(), None),
        }
    }

    pub(super) fn drawer_name(&self, cx: &App) -> Option<String> {
        let d = self.drawer.as_ref()?;
        Some(match &d.name {
            Some(n) => n.clone(),
            None => d.name_input.read(cx).value().trim().to_string(),
        })
    }

    pub(super) fn toggle_reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = &mut self.drawer else { return };
        d.revealed = !d.revealed;
        let masked = d.secret && !d.revealed;
        d.value.update(cx, |s, cx| s.set_masked(masked, window, cx));
        cx.notify();
    }

    pub(super) fn toggle_secret(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.received(cx) {
            return;
        }
        let name = self.drawer_name(cx).unwrap_or_default();
        let Some(d) = &mut self.drawer else { return };
        // A `SECRET_` name always is one.
        if name.starts_with("SECRET_") {
            return;
        }
        d.secret = !d.secret;
        d.revealed = false;
        let masked = d.secret;
        d.value.update(cx, |s, cx| s.set_masked(masked, window, cx));
        cx.notify();
    }

    pub(super) fn toggle_required(&mut self, cx: &mut Context<Self>) {
        if self.received(cx) {
            return;
        }
        if let Some(d) = &mut self.drawer {
            d.required = !d.required;
            cx.notify();
        }
    }

    /// The file environment writes go to, as the confirmation names it.
    pub(super) fn file_name(&self) -> String {
        env_file(&self.root, &self.environment)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| ".env".into())
    }

    pub(super) fn own_file(&self) -> PathBuf {
        env_file(&self.root, &self.environment).unwrap_or_else(|_| self.root.join(".env"))
    }

    /// The other environments with a Keychain item for `name` (names only).
    pub(super) fn keychain_elsewhere(&self, name: &str, cx: &App) -> Vec<String> {
        self.environments
            .iter()
            .filter(|e| **e != self.environment)
            .filter(|e| {
                let service = self.service_of(e, cx);
                // Unreadable counts as holding it: the mark stays.
                self.secrets
                    .try_names(&service)
                    .map_or(true, |n| n.iter().any(|n| n == name))
            })
            .map(|e| env_label(e))
            .collect()
    }

    pub(super) fn render_drawer(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let d = self.drawer.as_ref()?;
        let name = self.drawer_name(cx).unwrap_or_default();
        let secret = d.secret || name.starts_with("SECRET_");
        let prefix_secret = name.starts_with("SECRET_");
        // A received copy's notes are the owner's: shown, not changed.
        let received = self.received(cx);
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .child(match &d.name {
                Some(n) => div()
                    .font_family(mono_font())
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(n.clone())
                    .into_any_element(),
                None => div()
                    .flex_1()
                    .font_family(mono_font())
                    .text_size(px(13.))
                    .child(Input::new(&d.name_input).h(px(32.)))
                    .into_any_element(),
            })
            .when(secret, |h| {
                h.child(
                    tinted_pill("accent-purple")
                        .child(key_icon(12.))
                        .child("Secret"),
                )
            })
            .when(d.name.is_some(), |h| h.child(div().flex_1()))
            .child(
                div()
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child(env_label(&self.environment)),
            );
        let reveal = secret.then(|| {
            div()
                .id("reveal-value")
                .role(Role::Button)
                .test_support()
                .aria_label(if d.revealed {
                    "Hide value"
                } else {
                    "Reveal value"
                })
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(24.))
                .rounded(px(4.))
                .text_color(color("text-secondary"))
                .cursor_pointer()
                .hover(|s| s.text_color(color("text-primary")))
                .child(
                    glyph(if d.revealed {
                        icons::EYE_OFF
                    } else {
                        icons::EYE
                    })
                    .size(px(14.)),
                )
                .on_click(cx.listener(|this, _, window, cx| this.toggle_reveal(window, cx)))
        });
        // Secrets are never copied: a synced clipboard would carry them off
        // this device.
        let copy = (!secret).then(|| {
            div()
                .id("copy-value")
                .role(Role::Button)
                .test_support()
                .aria_label("Copy")
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(24.))
                .rounded(px(4.))
                .text_color(color("text-secondary"))
                .cursor_pointer()
                .hover(|s| s.text_color(color("text-primary")))
                .child(glyph(icons::COPY).size(px(13.)))
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(d) = &this.drawer {
                        let v = d.value.read(cx).value().to_string();
                        cx.write_to_clipboard(ClipboardItem::new_string(v));
                    }
                }))
        });
        let where_stored = where_stored(&d.source, secret, &self.own_file(), &self.service(cx));
        let value = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(label_text("Value"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .h(px(32.))
                    .pl(px(2.))
                    .pr(px(6.))
                    .border_1()
                    .border_color(color("border-200"))
                    .rounded(px(6.))
                    .bg(color("bg-100"))
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .text_color(color(if secret && !d.revealed {
                        "text-secondary"
                    } else {
                        "text-primary"
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&d.value).appearance(false).h(px(30.))),
                    )
                    .children(reveal)
                    .children(copy),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.45))
                    .text_color(match &d.read_error {
                        Some(_) => color("warning"),
                        None => color("text-tertiary"),
                    })
                    .child(d.read_error.clone().unwrap_or(where_stored)),
            );
        let description = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(label_text("Description shown to people you share with"))
            // The Kit's textarea draws its own field.
            .child(
                div()
                    .line_height(relative(1.45))
                    .child(Textarea::new(&d.description).disabled(received)),
            );
        let toggle =
            |id: &'static str, on: bool, enabled: bool, title: &'static str, note: &'static str| {
                div()
                    .id(id)
                    .role(Role::Switch)
                    .test_support()
                    .aria_label(title)
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .text_size(px(13.))
                    .when(enabled, |d| d.cursor_pointer())
                    .child(switch(on, enabled))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.))
                            .child(title)
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(relative(1.45))
                                    .text_color(color("text-tertiary"))
                                    .child(note),
                            ),
                    )
            };
        let groups = self.usage.of(&name);
        let used_by = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(label_text(used_by_text(self.usage.total(&name))))
            .child(
                div().flex().flex_wrap().gap(px(6.)).children(
                    groups
                        .iter()
                        .map(|(g, n)| neutral_pill().child(format!("{g} · {n}"))),
                ),
            );
        let footer = div()
            .flex()
            .flex_none()
            .gap(px(8.))
            .when(d.name.is_some() && !received, |f| {
                f.child(
                    small_button("delete-variable", "Delete variable")
                        .text_color(color("danger"))
                        .child("Delete variable")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.confirm_delete(window, cx)),
                        ),
                )
            })
            .child(div().flex_1())
            .child(
                small_button("drawer-cancel", "Cancel")
                    .child("Cancel")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.drawer = None;
                        this.focus.focus(window, cx);
                        cx.notify();
                    })),
            )
            .child(
                primary_button("drawer-save", "Save")
                    .child("Save")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.save(Confirmed::No, window, cx)),
                    ),
            );
        Some(
            div()
                .id("variable-drawer")
                .role(Role::Complementary)
                .test_support()
                .aria_label("Variable detail")
                .flex()
                .flex_col()
                .flex_none()
                .gap(px(18.))
                .w(px(DRAWER_WIDTH))
                .h_full()
                .p(px(20.))
                .border_l_1()
                .border_color(color("border-100"))
                .bg(color("bg-100"))
                .child(header)
                .child(value)
                .child(description)
                .child(
                    toggle(
                        "secret-toggle",
                        secret,
                        !prefix_secret && !received,
                        "Secret",
                        "Masked in the UI, logs and exported reports. Never leaves this device.",
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_secret(window, cx))),
                )
                .child(
                    toggle(
                        "required-toggle",
                        d.required,
                        !received,
                        "Required for shared users",
                        "People opening a shared link are asked for it before their first run.",
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_required(cx))),
                )
                .child(used_by)
                .child(div().flex_1())
                .child(footer),
        )
    }
}
