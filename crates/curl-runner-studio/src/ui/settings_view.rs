//! The app's Settings (board: Settings): General, Editor, Appearance,
//! Accounts & sync, Keyboard shortcuts and Updates, each a title, a line
//! of description and cards. It takes the whole area under the title bar,
//! like the collection settings page, and leaves on Esc or `Back to
//! workspace`.
//!
//! Accounts & sync: the Server, Account and Sync cards speak to the
//! [`Backend`]; Credentials lists Keychain item names (never their values)
//! and the environment files' values.

use super::app::{CloseSettings, Shortcut, bindings, mono_font};
use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::settings_kit::{
    MASK, back_link, card, card_body, card_footer, card_text, card_title, kbd_key, nav_item,
    primary_button, segment, segments, setting_row, small_button, switch, th,
};
use super::sign_in::SETUP_GUIDE_URL;
use super::workspace::notify_error;
use crate::model::backend::{Account, Backend, CollectionKey, Server, SyncStatus};
use crate::model::environments::{env_file, is_secret, list_environments};
use crate::model::history::relative_time;
use crate::model::secrets::{SecretStore, service_name};
use crate::model::settings::{FONT_SIZES, Settings, TAB_SIZES};
use crate::ui::sync::SyncLine;
use chrono::{DateTime, Local, Utc};
use curl_runner_core::env::parse_env_content;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const NAV_WIDTH: f32 = 248.;
/// The pages' column.
const CONTENT_WIDTH: f32 = 760.;
/// The studio's own version, and the CLI's it runs requests with.
pub const STUDIO_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The CLI's changelog (the studio ships in the same release train).
pub const CHANGELOG_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/packages/cli/CHANGELOG.md"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPage {
    General,
    Editor,
    Appearance,
    Accounts,
    Shortcuts,
    Updates,
}

impl AppPage {
    pub const ALL: [AppPage; 6] = [
        AppPage::General,
        AppPage::Editor,
        AppPage::Appearance,
        AppPage::Accounts,
        AppPage::Shortcuts,
        AppPage::Updates,
    ];

    pub fn title(self) -> &'static str {
        match self {
            AppPage::General => "General",
            AppPage::Editor => "Editor",
            AppPage::Appearance => "Appearance",
            AppPage::Accounts => "Accounts & sync",
            AppPage::Shortcuts => "Keyboard shortcuts",
            AppPage::Updates => "Updates",
        }
    }

    fn description(self) -> &'static str {
        match self {
            AppPage::General => "How the studio starts and where it keeps its data.",
            AppPage::Editor => {
                "How the YAML editor looks and indents. Changes apply to open files right away."
            }
            AppPage::Appearance => "The studio's colors.",
            AppPage::Accounts => {
                "Which server your collections sync with, who you're signed in as, and where secrets are kept."
            }
            AppPage::Shortcuts => {
                "Every shortcut the studio binds. They work anywhere in the window, the editor included."
            }
            AppPage::Updates => "The versions you are running.",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            AppPage::General => icons::GENERAL,
            AppPage::Editor => icons::CODE,
            AppPage::Appearance => icons::APPEARANCE,
            AppPage::Accounts => icons::USER,
            AppPage::Shortcuts => icons::KEYBOARD,
            AppPage::Updates => icons::UPDATES,
        }
    }

    fn id(self) -> &'static str {
        match self {
            AppPage::General => "app-settings-general",
            AppPage::Editor => "app-settings-editor",
            AppPage::Appearance => "app-settings-appearance",
            AppPage::Accounts => "app-settings-accounts",
            AppPage::Shortcuts => "app-settings-shortcuts",
            AppPage::Updates => "app-settings-updates",
        }
    }

    /// `CURL_RUNNER_STUDIO_PAGE`: `general`, `editor`, `appearance`,
    /// `accounts`, `shortcuts` or `updates`.
    pub fn parse(s: &str) -> Option<AppPage> {
        AppPage::ALL
            .into_iter()
            .find(|p| p.id().strip_prefix("app-settings-") == Some(s))
    }
}

/// What the screen asks of the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppSettingsEvent {
    /// Back to the editor.
    Close,
    /// A setting changed (and was saved): apply it.
    Changed,
    /// A Credentials row's Edit: the variable drawer on it.
    EditVariable { environment: String, name: String },
    /// The Credentials card's Add variable.
    AddVariable,
    /// `Sign in` or `Switch account`: the sign-in screen.
    OpenSignIn,
    /// Signed out or switched servers: what shows the account repaints.
    AccountChanged,
    /// `Sync now`, for the open collection.
    SyncNow,
}

/// What Accounts & sync reads: the backend, the open collection as the
/// server knows it, and whether the studio was set to work offline.
pub struct Accounts {
    pub backend: Arc<dyn Backend>,
    pub key: Option<CollectionKey>,
    pub offline: bool,
    /// The open collection's sync, as the workspace (and its footer) has
    /// it; kept current with [`SettingsView::set_sync`].
    pub sync: SyncLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerPick {
    Cloud,
    SelfHosted,
}

/// Where a credential is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stored {
    Keychain,
    /// The environment's `.env` file in the collection folder.
    File,
}

/// One row of the Credentials table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub name: String,
    pub environment: String,
    pub stored: Stored,
    /// A plain file value; `None` reads as the mask. Keychain values are
    /// never read for the table.
    pub value: Option<String>,
}

/// Every credential of the collection at `root`: each environment's
/// Keychain item names, then its own file's values (a secret's masked).
/// The second value is set when the Keychain could not be listed.
pub fn credential_rows(
    root: &Path,
    collection: &str,
    id: &str,
    secrets: &dyn SecretStore,
    marked: &[String],
) -> (Vec<Credential>, bool) {
    let mut rows = Vec::new();
    let mut locked = false;
    for env in list_environments(root) {
        match secrets.try_names(&service_name(collection, &env, id)) {
            Ok(names) => rows.extend(names.into_iter().map(|name| Credential {
                name,
                environment: env.clone(),
                stored: Stored::Keychain,
                value: None,
            })),
            Err(e) => {
                log::warn!("Couldn't list Keychain items: {e}");
                locked = true;
            }
        }
        let text = env_file(root, &env)
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let mut values: Vec<(String, String)> = parse_env_content(&text).into_iter().collect();
        values.sort();
        rows.extend(values.into_iter().map(|(name, value)| Credential {
            value: (!is_secret(&name, marked)).then_some(value),
            name,
            environment: env.clone(),
            stored: Stored::File,
        }));
    }
    (rows, locked)
}

/// A binding's keys as macOS shows them: `⇧⌘N`, `⌥⌘↵`, `Esc`.
pub fn keys_text(binding: &KeyBinding) -> String {
    binding
        .keystrokes()
        .iter()
        .map(|k| {
            let m = k.modifiers();
            let mut s = String::new();
            for (on, glyph) in [
                (m.control, '⌃'),
                (m.alt, '⌥'),
                (m.shift, '⇧'),
                (m.platform, '⌘'),
            ] {
                if on {
                    s.push(glyph);
                }
            }
            s.push_str(&match k.key() {
                "enter" => "↵".to_string(),
                "escape" => "Esc".to_string(),
                "up" => "↑".to_string(),
                "down" => "↓".to_string(),
                "left" => "←".to_string(),
                "right" => "→".to_string(),
                "backspace" => "⌫".to_string(),
                "tab" => "⇥".to_string(),
                "space" => "Space".to_string(),
                key => {
                    let mut c = key.chars();
                    c.next().map_or_else(String::new, |f| {
                        f.to_uppercase().collect::<String>() + c.as_str()
                    })
                }
            });
            s
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `(label, keys)` for each of `shortcuts`, once: bindings that differ
/// only by context read as one row.
fn rows_of(shortcuts: &[Shortcut]) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    for s in shortcuts {
        let row = (s.label.to_string(), keys_text(&s.binding));
        if !rows.contains(&row) {
            rows.push(row);
        }
    }
    rows
}

/// The Keyboard shortcuts table, from the list the app registers.
pub fn shortcut_rows() -> Vec<(String, String)> {
    rows_of(&bindings())
}

/// The open collection, for the Credentials card.
pub struct CollectionInfo {
    pub root: PathBuf,
    pub name: String,
    /// Its `CollectionMeta::id`, for the Keychain services.
    pub id: String,
    /// Names the collection marks secret.
    pub marked: Vec<String>,
    /// Where its Keychain items are listed from.
    pub secrets: Arc<dyn SecretStore>,
}

pub struct SettingsView {
    page: AppPage,
    settings: Settings,
    /// Where settings are saved (`None` in tests that keep nothing).
    base: Option<PathBuf>,
    /// The open collection's name, if any.
    collection: Option<String>,
    credentials: Vec<Credential>,
    keychain_locked: bool,
    shortcuts: Vec<(String, String)>,
    accounts: Accounts,
    /// The Server card's draft, saved by its Save.
    server_pick: ServerPick,
    server_url: Entity<InputState>,
    /// A backend call in flight (Save, Sign out).
    busy: bool,
    /// Why the last Save failed, under the server row.
    server_error: Option<String>,
    task: Option<Task<()>>,
    focus: FocusHandle,
}

impl EventEmitter<AppSettingsEvent> for SettingsView {}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsView {
    pub fn new(
        page: AppPage,
        settings: Settings,
        base: Option<PathBuf>,
        collection: Option<CollectionInfo>,
        accounts: Accounts,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (server_pick, url) = match accounts.backend.server() {
            Server::Cloud => (ServerPick::Cloud, String::new()),
            Server::SelfHosted { url } => (ServerPick::SelfHosted, url),
        };
        let server_url = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("https://curl-runner.example.com")
                .default_value(url)
        });
        // Save's enabled state follows the draft; an edit clears the error.
        cx.subscribe(
            &server_url,
            |this: &mut Self, _, e: &gpui_kit::component::input::InputEvent, cx| {
                if matches!(e, gpui_kit::component::input::InputEvent::Change) {
                    this.server_error = None;
                }
                cx.notify()
            },
        )
        .detach();
        let (credentials, keychain_locked) = collection
            .as_ref()
            .map(|c| credential_rows(&c.root, &c.name, &c.id, &*c.secrets, &c.marked))
            .unwrap_or_default();
        SettingsView {
            page,
            settings,
            base,
            collection: collection.map(|c| c.name),
            credentials,
            keychain_locked,
            shortcuts: shortcut_rows(),
            accounts,
            server_pick,
            server_url,
            busy: false,
            server_error: None,
            task: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn page(&self) -> AppPage {
        self.page
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    #[cfg(test)]
    pub fn credentials(&self) -> &[Credential] {
        &self.credentials
    }

    /// The workspace's sync changed: the Sync card follows.
    pub fn set_sync(&mut self, line: SyncLine, cx: &mut Context<Self>) {
        if self.accounts.sync != line {
            self.accounts.sync = line;
            cx.notify();
        }
    }

    pub fn set_page(&mut self, page: AppPage, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    /// Changes, saves and announces a setting.
    pub fn update_settings(
        &mut self,
        change: impl FnOnce(&mut Settings),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self.settings.clone();
        change(&mut self.settings);
        if self.settings == before {
            return;
        }
        if let Some(base) = &self.base
            && let Err(e) = self.settings.save(base)
        {
            notify_error(format!("Couldn't save settings: {e}"), window, cx);
        }
        cx.emit(AppSettingsEvent::Changed);
        cx.notify();
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = self.page;
        let body: Vec<AnyElement> = match page {
            AppPage::General => self.render_general(cx),
            AppPage::Editor => self.render_editor(cx),
            AppPage::Appearance => vec![self.render_appearance().into_any_element()],
            AppPage::Accounts => self.render_accounts(cx),
            AppPage::Shortcuts => vec![self.render_shortcuts().into_any_element()],
            AppPage::Updates => vec![render_updates().into_any_element()],
        };
        let header = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(24.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(page.title()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(color("text-secondary"))
                    .child(page.description()),
            );
        div()
            .id("app-settings")
            .key_context("AppSettings")
            .track_focus(&self.focus)
            .test_support()
            .on_action(cx.listener(|_, _: &CloseSettings, _, cx| cx.emit(AppSettingsEvent::Close)))
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(color("bg-100"))
            .child(self.render_nav(cx))
            .child(
                div()
                    .id("app-settings-scroll")
                    .role(Role::Main)
                    .test_support()
                    .aria_label(page.title())
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .child(
                        div().flex().justify_center().px(px(24.)).child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(20.))
                                .w_full()
                                .max_w(px(CONTENT_WIDTH))
                                .pt(px(32.))
                                .pb(px(40.))
                                .child(header)
                                .children(body),
                        ),
                    ),
            )
    }
}

/// A link in `accent-blue`.
fn link(id: &'static str, text: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Link)
        .text_color(color("accent-blue"))
        .cursor_pointer()
        .hover(|s| s.text_color(color("primary")))
        .child(text)
}

impl SettingsView {
    fn render_nav(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let items = AppPage::ALL.map(|p| {
            nav_item(p.id(), p.title(), 32., Some(self.page == p))
                .gap(px(10.))
                .px(px(10.))
                .child(glyph(p.icon()).size(px(14.)).flex_none())
                .child(p.title())
                .on_click(cx.listener(move |this, _, _, cx| this.set_page(p, cx)))
        });
        let footer = div()
            .flex()
            .flex_col()
            .p(px(10.))
            .text_size(px(12.))
            .line_height(relative(1.5))
            .text_color(color("text-tertiary"))
            .child(format!("curl-runner studio {STUDIO_VERSION}"))
            .child(
                div()
                    .flex()
                    .child(format!("CLI {} ·\u{a0}", curl_runner_core::VERSION))
                    .child(
                        link("app-settings-changelog", "Changelog")
                            .on_click(|_, _, cx| cx.open_url(CHANGELOG_URL)),
                    ),
            );
        div()
            .id("app-settings-nav")
            .role(Role::Navigation)
            .test_support()
            .aria_label("Settings")
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(2.))
            .w(px(NAV_WIDTH))
            .h_full()
            .px(px(12.))
            .py(px(16.))
            .border_r_1()
            .border_color(color("border-100"))
            .bg(color("bg-100"))
            .child(
                back_link("app-settings-back", "Back to workspace", 13.)
                    .h(px(32.))
                    .px(px(10.))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(AppSettingsEvent::Close))),
            )
            .child(
                div()
                    .px(px(10.))
                    .pt(px(12.))
                    .pb(px(14.))
                    .text_size(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Settings"),
            )
            .children(items)
            .child(div().flex_1())
            .child(footer)
    }

    // General.

    fn render_general(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let reopen = self.settings.general.reopen_last_collection;
        let startup = card("general-startup")
            .aria_label("Startup")
            .child(card_body().child(card_title("Startup")))
            .child(
                setting_row(
                    "reopen-last-collection",
                    "Open the last collection on launch",
                    "Reopens the folder you had open when you quit.",
                    false,
                    switch(reopen, true),
                )
                .role(Role::Switch)
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| {
                    this.update_settings(
                        |s| s.general.reopen_last_collection = !s.general.reopen_last_collection,
                        window,
                        cx,
                    )
                })),
            );
        let data = self.base.clone().map(|base| {
            let shown = base.display().to_string();
            card("general-data")
                .aria_label("Data")
                .child(
                    card_body()
                        .child(card_title("Data"))
                        .child(card_text(
                            "Run history, collection notes and these settings stay on this Mac. Variable values stay in the macOS Keychain.",
                        )),
                )
                .child(setting_row(
                    "data-folder",
                    "Data folder",
                    div().font_family(mono_font()).child(shown),
                    false,
                    small_button("reveal-data-folder", "Show in Finder")
                        .child("Show in Finder")
                        .on_click(move |_, _, cx| cx.reveal_path(&base)),
                ))
                .into_any_element()
        });
        std::iter::once(startup.into_any_element())
            .chain(data)
            .collect()
    }

    // Editor.

    fn render_editor(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let e = self.settings.editor.clone();
        let step = |id: &'static str, label: &'static str, delta: i8, enabled: bool| {
            small_button(id, label)
                .size(px(28.))
                .px_0()
                .justify_center()
                .text_size(px(14.))
                .when(!enabled, |b| b.opacity(0.4).cursor_default())
                .child(if delta < 0 { "−" } else { "+" })
                .when(enabled, |b| {
                    b.on_click(cx.listener(move |this, _, window, cx| {
                        this.update_settings(
                            |s| {
                                let size = s.editor.font_size.saturating_add_signed(delta);
                                s.editor.font_size =
                                    size.clamp(*FONT_SIZES.start(), *FONT_SIZES.end());
                            },
                            window,
                            cx,
                        )
                    }))
                })
        };
        let font_size = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(step(
                "font-size-down",
                "Smaller",
                -1,
                e.font_size > *FONT_SIZES.start(),
            ))
            .child(
                div()
                    .id("font-size-value")
                    .test_support()
                    .aria_label(format!("{} px", e.font_size))
                    .w(px(48.))
                    .flex()
                    .justify_center()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .child(format!("{} px", e.font_size)),
            )
            .child(step(
                "font-size-up",
                "Larger",
                1,
                e.font_size < *FONT_SIZES.end(),
            ));
        let tab_size = segments("tab-size")
            .aria_label("Tab size")
            .children(TAB_SIZES.map(|n| {
                segment(
                    ("tab-size", n as usize),
                    format!("{n} spaces"),
                    e.tab_size == n,
                )
                .child(format!("{n} spaces"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.update_settings(|s| s.editor.tab_size = n, window, cx)
                }))
            }));
        let settings = card("editor-settings")
            .aria_label("Editor")
            .child(setting_row(
                "editor-font-size",
                "Font size",
                "The YAML editor's text. Lines grow with it.",
                true,
                font_size,
            ))
            .child(setting_row(
                "editor-tab-size",
                "Tab size",
                "Spaces per indent level. YAML never uses tabs.",
                false,
                tab_size,
            ))
            .child(
                setting_row(
                    "editor-word-wrap",
                    "Word wrap",
                    "Wrap long lines at the editor's edge instead of scrolling sideways.",
                    false,
                    switch(e.word_wrap, true),
                )
                .role(Role::Switch)
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| {
                    this.update_settings(|s| s.editor.word_wrap = !s.editor.word_wrap, window, cx)
                })),
            );
        // A sample at the chosen size.
        let indent = " ".repeat(e.tab_size as usize);
        let line = |parts: Vec<(String, &'static str)>| {
            div()
                .flex()
                .h(px(e.line_height()))
                .items_center()
                .whitespace_nowrap()
                .children(
                    parts
                        .into_iter()
                        .map(|(text, tone)| div().text_color(color(tone)).child(text)),
                )
        };
        let preview = card("editor-preview").aria_label("Preview").child(
            card_body().child(card_title("Preview")).child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .border_1()
                    .border_color(color("border-100"))
                    .rounded(px(6.))
                    .font_family(mono_font())
                    .text_size(px(f32::from(e.font_size)))
                    .child(line(vec![("request:".into(), "syntax-key")]))
                    .child(line(vec![
                        (format!("{indent}name: "), "syntax-key"),
                        ("Create charge".into(), "syntax-string"),
                    ]))
                    .child(line(vec![
                        (format!("{indent}url: "), "syntax-key"),
                        ("${BASE_URL}".into(), "syntax-variable"),
                        ("/charges".into(), "syntax-string"),
                    ]))
                    .child(line(vec![
                        (format!("{indent}method: "), "syntax-key"),
                        ("POST".into(), "syntax-string"),
                    ])),
            ),
        );
        vec![settings.into_any_element(), preview.into_any_element()]
    }

    // Appearance.

    fn render_appearance(&self) -> impl IntoElement + use<> {
        let option = |id: &'static str, label: &'static str, note: &'static str, on: bool| {
            div()
                .id(id)
                .role(Role::RadioButton)
                .test_support()
                .aria_label(label)
                .flex()
                .flex_1()
                .min_w_0()
                .gap(px(12.))
                .p(px(16.))
                .border_1()
                .rounded(px(8.))
                .map(|d| {
                    if on {
                        d.border_color(color("primary")).bg(color("surface-raised"))
                    } else {
                        d.border_color(color("border-200"))
                    }
                })
                .child(
                    div()
                        .flex_none()
                        .mt(px(2.))
                        .size(px(16.))
                        .rounded_full()
                        .map(|r| {
                            if on {
                                r.border(px(5.)).border_color(color("primary"))
                            } else {
                                r.border_1().border_color(color("text-disabled"))
                            }
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(if on {
                                    "text-primary"
                                } else {
                                    "text-secondary"
                                }))
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-tertiary"))
                                .child(note),
                        ),
                )
        };
        card("appearance-theme").aria_label("Theme").child(
            card_body()
                .child(card_title("Theme"))
                .child(card_text(
                    "Dark is the studio's theme. Light and following the system arrive in a later build.",
                ))
                .child(
                    div()
                        .id("theme-options")
                        .role(Role::RadioGroup)
                        .flex()
                        .gap(px(12.))
                        .child(option("theme-dark", "Dark", "Calm and dense", true))
                        .child(option("theme-light", "Light", "Coming soon", false))
                        .child(option("theme-system", "System", "Coming soon", false)),
                ),
        )
    }

    // Accounts & sync.

    fn render_accounts(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![
            self.render_server(cx).into_any_element(),
            self.render_account(cx).into_any_element(),
            self.render_sync(cx).into_any_element(),
            self.render_credentials(cx).into_any_element(),
        ]
    }

    fn render_credentials(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let head = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(px(32.))
            .pl(px(24.))
            .pr(px(12.))
            .border_t_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .child(th("Name").w(px(140.)).flex_none())
            .child(th("Value").flex_1())
            .child(th("Environment").w(px(80.)).flex_none())
            .child(th("Stored in").w(px(110.)).flex_none())
            .child(div().w(px(44.)).flex_none());
        let rows = self.credentials.iter().enumerate().map(|(i, c)| {
            let (environment, name) = (c.environment.clone(), c.name.clone());
            let value = c.value.clone().unwrap_or_else(|| MASK.to_string());
            div()
                .id(("credential", i))
                .role(Role::Row)
                .test_support()
                .aria_label(format!("{} {}", c.name, env_label(&c.environment)))
                .flex()
                .items_center()
                .gap(px(12.))
                .h(px(40.))
                .pl(px(24.))
                .pr(px(12.))
                .border_t_1()
                .border_color(color("border-100"))
                .text_size(px(12.))
                .child(
                    div()
                        .w(px(140.))
                        .flex_none()
                        .text_ellipsis()
                        .font_family(mono_font())
                        .child(c.name.clone()),
                )
                .child(
                    div()
                        .id(("credential-value", i))
                        .test_support()
                        .aria_label(value.clone())
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis()
                        .font_family(mono_font())
                        .text_color(color("text-secondary"))
                        .child(value),
                )
                .child(
                    div()
                        .w(px(80.))
                        .flex_none()
                        .text_ellipsis()
                        .text_color(color("text-secondary"))
                        .child(env_label(&c.environment)),
                )
                .child(
                    div()
                        .w(px(110.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_color(color("text-secondary"))
                        .child(glyph(icons::LOCK).size(px(14.)).flex_none())
                        .child(match c.stored {
                            Stored::Keychain => "Keychain",
                            Stored::File => "Local file",
                        }),
                )
                .child(
                    div().w(px(44.)).flex_none().flex().justify_end().child(
                        small_button(("credential-edit", i), "Edit")
                            .h(px(24.))
                            .child("Edit")
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.emit(AppSettingsEvent::EditVariable {
                                    environment: environment.clone(),
                                    name: name.clone(),
                                })
                            })),
                    ),
                )
        });
        let note = |text: String, tone: &'static str| {
            div()
                .flex()
                .items_center()
                .h(px(40.))
                .px(px(24.))
                .border_t_1()
                .border_color(color("border-100"))
                .text_size(px(12.))
                .text_color(color(tone))
                .child(text)
        };
        let empty = match &self.collection {
            None => Some(note(
                "Open a collection to see its credentials.".into(),
                "text-tertiary",
            )),
            Some(name) if self.credentials.is_empty() && !self.keychain_locked => Some(note(
                format!("{name} has no variables yet."),
                "text-tertiary",
            )),
            _ => None,
        };
        let locked = self.keychain_locked.then(|| {
            note(
                "The Keychain is locked. Unlock it to list the values stored there.".into(),
                "warning",
            )
        });
        card("credentials-card")
            .aria_label("Credentials")
            .child(
                card_body()
                    .pb_0()
                    .child(card_title("Credentials"))
                    .child(card_text(
                        "Variable values stay in the macOS Keychain on this machine. They are never synced or shared; collections only carry the names.",
                    ))
                    .child(
                        div()
                            .id("credentials-table")
                            .role(Role::Table)
                            .test_support()
                            .mt(px(4.))
                            .mx(px(-24.))
                            .child(head)
                            .children(rows)
                            .children(empty)
                            .children(locked),
                    ),
            )
            .child(
                card_footer()
                    .child(div().flex_1().child(
                        "Customers opening a shared collection are asked for their own values on first run.",
                    ))
                    .when(self.collection.is_some(), |f| {
                        f.child(
                            small_button("credentials-add", "Add variable")
                                .child("Add variable")
                                .on_click(cx.listener(|_, _, _, cx| {
                                    cx.emit(AppSettingsEvent::AddVariable)
                                })),
                        )
                    }),
            )
    }

    // Keyboard shortcuts.

    fn render_shortcuts(&self) -> impl IntoElement + use<> {
        let rows = self.shortcuts.iter().enumerate().map(|(i, (label, keys))| {
            div()
                .id(("shortcut", i))
                .role(Role::Row)
                .test_support()
                .aria_label(format!("{label} {keys}"))
                .flex()
                .items_center()
                .gap(px(12.))
                .h(px(40.))
                .px(px(24.))
                .border_t_1()
                .border_color(color("border-100"))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.))
                        .child(label.clone()),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .children(keys.split(' ').map(|k| kbd_key(k.to_string()))),
                )
        });
        card("shortcuts-card")
            .aria_label("Shortcuts")
            .child(
                div()
                    .id("shortcuts-table")
                    .role(Role::Table)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .h(px(32.))
                            .px(px(24.))
                            .bg(color("surface-raised"))
                            .child(th("Action").flex_1())
                            .child(th("Shortcut")),
                    )
                    .children(rows),
            )
            .child(card_footer().child("Shortcuts can't be changed in this build."))
    }
}

/// The Account card's avatar: the board's blue-to-purple disc.
fn avatar() -> Div {
    div()
        .flex_none()
        .size(px(40.))
        .rounded_full()
        .bg(linear_gradient(
            135.,
            linear_color_stop(color("accent-blue"), 0.),
            linear_color_stop(color("accent-purple"), 1.),
        ))
}

/// `jamie@acme.dev · Owner of Acme · signed in with GitHub`, the org in
/// `text-primary`.
fn account_line(a: &Account) -> Div {
    let line = div()
        .flex()
        .flex_wrap()
        .text_color(color("text-secondary"))
        .child(format!("{} · {}", a.email, a.role));
    match &a.org {
        Some(org) => line
            .child("\u{a0}of\u{a0}")
            .child(div().text_color(color("text-primary")).child(org.clone()))
            .child(format!(" · signed in with {}", a.provider.label())),
        None => line.child(format!(" · signed in with {}", a.provider.label())),
    }
}

/// The Sync card's status line: its dot's token and text. What waits,
/// whether this client is offline and its conflicts come from `line`, the
/// workspace's own state (the footer's); the server adds only what it knows
/// once synced: how many collections and requests it has.
pub fn sync_line(
    line: &SyncLine,
    server: &SyncStatus,
    now: DateTime<Utc>,
) -> (&'static str, String) {
    let changes = |n: usize| {
        if n == 1 {
            "1 change".to_string()
        } else {
            format!("{n} changes")
        }
    };
    let when =
        |at: &DateTime<Utc>| relative_time(at.with_timezone(&Local), now.with_timezone(&Local));
    match line {
        SyncLine::LocalOnly => (
            "text-tertiary",
            "Not synced · collections stay on this Mac".into(),
        ),
        SyncLine::NotSynced => ("text-tertiary", "Not synced yet".into()),
        SyncLine::NotLinked => ("text-tertiary", "Not linked to a share".into()),
        SyncLine::Syncing => ("text-tertiary", "Syncing…".into()),
        SyncLine::Synced(at) => match server {
            SyncStatus::Synced {
                collections,
                requests,
                ..
            } => {
                let plural = |n: usize, one: &str| {
                    if n == 1 {
                        format!("1 {one}")
                    } else {
                        format!("{n} {one}s")
                    }
                };
                (
                    "success",
                    format!(
                        "Synced {} · {} · {}",
                        when(at),
                        plural(*collections, "collection"),
                        plural(*requests, "request")
                    ),
                )
            }
            SyncStatus::Off => ("success", format!("Synced {}", when(at))),
        },
        SyncLine::Waiting(n) => ("warning", format!("{} waiting to sync", changes(*n))),
        SyncLine::Offline(0) => ("text-tertiary", "Offline".into()),
        SyncLine::Offline(n) => (
            "text-tertiary",
            format!("Offline · {} waiting to sync", changes(*n)),
        ),
        SyncLine::Conflicts(n) => (
            "danger",
            if *n == 1 {
                "1 conflict to resolve".into()
            } else {
                format!("{n} conflicts to resolve")
            },
        ),
        SyncLine::CantRead => ("danger", "Can't read the collection folder".into()),
        SyncLine::AllRemoved => (
            "warning",
            "All files removed · Sync now to delete them".into(),
        ),
    }
}

impl SettingsView {
    /// The server the Server card describes.
    fn server_draft(&self, cx: &App) -> Server {
        match self.server_pick {
            ServerPick::Cloud => Server::Cloud,
            ServerPick::SelfHosted => Server::SelfHosted {
                url: self.server_url.read(cx).value().trim().to_string(),
            },
        }
    }

    fn pick_server(&mut self, pick: ServerPick, cx: &mut Context<Self>) {
        self.server_pick = pick;
        self.server_error = None;
        cx.notify();
    }

    /// Save on the Server card: tests a self-hosted address, then switches
    /// servers (which signs out). A failure shows under the row, as on the
    /// sign-in screen.
    pub fn save_server(&mut self, cx: &mut Context<Self>) {
        let server = self.server_draft(cx);
        if self.busy || server == self.accounts.backend.server() {
            return;
        }
        self.busy = true;
        self.server_error = None;
        let call = backend_call(cx, &self.accounts.backend, move |b| {
            Box::pin(async move {
                if matches!(server, Server::SelfHosted { .. }) {
                    b.test_connection(&server).await?;
                }
                b.set_server(server).await
            })
        });
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => cx.emit(AppSettingsEvent::AccountChanged),
                    Err(e) => this.server_error = Some(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    #[cfg(test)]
    pub fn set_self_hosted(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.server_pick = ServerPick::SelfHosted;
        let url = url.to_string();
        self.server_url
            .update(cx, |s, cx| s.set_value(url, window, cx));
        cx.notify();
    }

    pub fn sign_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        let call = backend_call(cx, &self.accounts.backend, |b| b.sign_out());
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(()) => cx.emit(AppSettingsEvent::AccountChanged),
                    Err(e) => notify_error(format!("Couldn't sign out: {e}"), window, cx),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// The board's Server card.
    fn render_server(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let cloud = self.server_pick == ServerPick::Cloud;
        let kind = segments("server-kind")
            .child(
                segment("server-cloud", "curl-runner Cloud", cloud)
                    .on_click(cx.listener(|this, _, _, cx| this.pick_server(ServerPick::Cloud, cx)))
                    .child(glyph(icons::CLOUD).size(px(16.)))
                    .child("curl-runner Cloud"),
            )
            .child(
                segment("server-self-hosted", "Self-hosted", !cloud)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.pick_server(ServerPick::SelfHosted, cx)),
                    )
                    .child(glyph(icons::SERVER).size(px(16.)))
                    .child("Self-hosted"),
            );
        let url = if cloud {
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .h(px(32.))
                .px(px(10.))
                .border_1()
                .border_color(color("border-100"))
                .rounded(px(6.))
                .font_family(mono_font())
                .text_size(px(12.))
                .text_color(color("text-tertiary"))
                .child(Server::Cloud.url().to_string())
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .font_family(mono_font())
                .child(
                    Input::new(&self.server_url)
                        .aria_label("Server URL")
                        .min_h(px(32.))
                        .text_size(px(12.)),
                )
                .into_any_element()
        };
        let draft = self.server_draft(cx);
        let changed = !self.busy
            && draft != self.accounts.backend.server()
            && !matches!(&draft, Server::SelfHosted { url } if url.is_empty());
        let save = primary_button("server-save", "Save")
            .when(!changed, |b| b.opacity(0.4).cursor_default())
            .when(changed, |b| {
                b.on_click(cx.listener(|this, _, _, cx| this.save_server(cx)))
            })
            .child("Save");
        card("server-card")
            .aria_label("Server")
            .child(
                card_body()
                    .child(card_title("Server"))
                    .child(card_text(
                        "Collections and sharing links live on this server. Switching servers signs you out.",
                    ))
                    .child(div().flex().items_center().gap(px(8.)).child(kind).child(url))
                    .children(self.server_error.clone().map(|e| {
                        div()
                            .id("server-error")
                            .role(Role::Alert)
                            .test_support()
                            .aria_label(e.clone())
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .text_color(color("danger"))
                            .child(glyph(icons::CROSS).size(px(13.)))
                            .child(e)
                    })),
            )
            .child(
                card_footer()
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .child("Self-hosting? See the\u{a0}")
                            .child(
                                div()
                                    .id("deployment-guide")
                                    .role(Role::Link)
                                    .text_color(color("accent-blue"))
                                    .cursor_pointer()
                                    .on_click(|_, _, cx| cx.open_url(SETUP_GUIDE_URL))
                                    .child("deployment guide"),
                            )
                            .child("."),
                    )
                    .child(save),
            )
    }

    /// The board's Account card: who is signed in, or `Sign in`.
    fn render_account(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let body = card_body()
            .flex_row()
            .items_center()
            .gap(px(14.))
            .child(avatar());
        let body = match self.accounts.backend.account() {
            Some(a) => body
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .id("account-name")
                                .test_support()
                                .aria_label(a.name.clone())
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(a.name.clone()),
                        )
                        .child(account_line(&a)),
                )
                .child(
                    small_button("switch-account", "Switch account")
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(AppSettingsEvent::OpenSignIn)))
                        .child("Switch account"),
                )
                .child(
                    small_button("sign-out", "Sign out")
                        .when(self.busy, |b| b.opacity(0.4))
                        .on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx)))
                        .child("Sign out"),
                ),
            None => body
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .id("account-name")
                                .test_support()
                                .aria_label("Not signed in")
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child("Not signed in"),
                        )
                        .child(div().text_color(color("text-secondary")).child(
                            if self.accounts.offline {
                                "Working offline. Sign in to sync your collections and share them."
                            } else {
                                "Sign in to sync your collections and share them."
                            },
                        )),
                )
                .child(
                    small_button("sign-in", "Sign in")
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(AppSettingsEvent::OpenSignIn)))
                        .child("Sign in"),
                ),
        };
        card("account-card").aria_label("Account").child(body)
    }

    /// The board's Sync card: the open collection's status, `Sync now` and
    /// the two switches.
    fn render_sync(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let backend = &self.accounts.backend;
        let server = self
            .accounts
            .key
            .as_ref()
            .map_or(SyncStatus::Off, |k| backend.sync_status(k));
        let (dot, text) = sync_line(&self.accounts.sync, &server, Utc::now());
        let can_sync =
            backend.account().is_some() && self.accounts.key.is_some() && !self.accounts.offline;
        let sync = self.settings.sync.clone();
        let toggle = |id: &'static str,
                      label: &'static str,
                      on: bool,
                      flip: fn(&mut Settings),
                      cx: &mut Context<Self>| {
            div()
                .id(id)
                .role(Role::Switch)
                .test_support()
                .aria_label(label)
                .flex()
                .items_center()
                .gap(px(12.))
                .cursor_pointer()
                .on_click(
                    cx.listener(move |this, _, window, cx| this.update_settings(flip, window, cx)),
                )
                .child(switch(on, true))
                .child(div().flex_1().child(label))
        };
        card("sync-card")
            .aria_label("Sync")
            .child(
                card_body()
                    .child(card_title("Sync"))
                    .child(card_text(
                        "Edits save locally first, then sync. Shared collections update for viewers when you sync.",
                    ))
                    .child(
                        div()
                            .id("sync-status")
                            .role(Role::Status)
                            .test_support()
                            .aria_label(text.clone())
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .px(px(12.))
                            .py(px(10.))
                            .border_1()
                            .border_color(color("border-100"))
                            .rounded(px(6.))
                            .child(div().flex_none().size(px(6.)).rounded_full().bg(color(dot)))
                            .child(div().flex_1().child(text))
                            .child(
                                small_button("sync-now", "Sync now")
                                    .when(!can_sync, |b| b.opacity(0.4).cursor_default())
                                    .when(can_sync, |b| {
                                        b.on_click(cx.listener(|_, _, _, cx| {
                                            cx.emit(AppSettingsEvent::SyncNow)
                                        }))
                                    })
                                    .child(glyph(icons::SYNC).size(px(14.)))
                                    .child("Sync now"),
                            ),
                    )
                    .child(toggle(
                        "sync-automatically",
                        "Sync automatically",
                        sync.automatic,
                        |s| s.sync.automatic = !s.sync.automatic,
                        cx,
                    ))
                    .child(toggle(
                        "sync-metered",
                        "Sync over metered connections",
                        sync.metered,
                        |s| s.sync.metered = !s.sync.metered,
                        cx,
                    )),
            )
            .child(card_footer().child(div().flex_1().child(
                "On conflict, the studio shows both versions and asks you. It never merges silently.",
            )))
    }
}

/// Updates: the versions and the changelog.
fn render_updates() -> impl IntoElement {
    let version = |id: &'static str, label: &'static str, v: &'static str, first: bool| {
        setting_row(
            id,
            label,
            match id {
                "studio-version" => "This app.",
                _ => {
                    "The engine the studio runs requests with, the same as the curl-runner command."
                }
            },
            first,
            div()
                .id((id, 1usize))
                .test_support()
                .aria_label(v)
                .font_family(mono_font())
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child(v),
        )
    };
    card("updates-card")
        .aria_label("Versions")
        .child(version(
            "studio-version",
            "curl-runner studio",
            STUDIO_VERSION,
            true,
        ))
        .child(version(
            "cli-version",
            "curl-runner CLI",
            curl_runner_core::VERSION,
            false,
        ))
        .child(
            card_footer()
                .child(div().flex_1().child("Updates arrive with new builds."))
                .child(
                    small_button("updates-changelog", "Changelog")
                        .child("Changelog")
                        .on_click(|_, _, cx| cx.open_url(CHANGELOG_URL)),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::{AppPage, Credential, Stored, credential_rows, keys_text, rows_of, shortcut_rows};
    use crate::model::secrets::{MemorySecrets, SecretStore, service_for, service_name};
    use gpui_kit::KeyBinding;

    #[test]
    fn shortcut_rows_cover_every_binding() {
        let bindings = super::bindings();
        let rows = shortcut_rows();
        // One row per (label, keys): context variants fold into one.
        let mut distinct: Vec<(String, String)> = bindings
            .iter()
            .map(|b| (b.label.to_string(), keys_text(&b.binding)))
            .collect();
        distinct.dedup();
        distinct.sort();
        distinct.dedup();
        assert_eq!(rows.len(), distinct.len());
        for b in &bindings {
            let row = (b.label.to_string(), keys_text(&b.binding));
            assert!(rows.contains(&row), "{row:?} missing");
        }
        // The same list the app registers.
        assert_eq!(super::super::app::keymap().len(), bindings.len());
        assert!(rows.contains(&("Settings".into(), "⌘,".into())));
        assert!(rows.contains(&("New collection".into(), "⇧⌘N".into())));
        assert!(rows.contains(&("Run request under cursor".into(), "⌥⌘↵".into())));
        assert_eq!(rows_of(&[]), Vec::<(String, String)>::new());
    }

    #[test]
    fn keys_read_as_macos_shows_them() {
        let k = |keys| keys_text(&KeyBinding::new(keys, gpui_kit::NoAction, None));
        assert_eq!(k("cmd-k"), "⌘K");
        assert_eq!(k("ctrl-alt-shift-cmd-x"), "⌃⌥⇧⌘X");
        assert_eq!(k("escape"), "Esc");
        assert_eq!(k("cmd-k cmd-s"), "⌘K ⌘S");
    }

    #[test]
    fn pages_parse_from_the_dev_hook() {
        assert_eq!(AppPage::parse("accounts"), Some(AppPage::Accounts));
        assert_eq!(AppPage::parse("shortcuts"), Some(AppPage::Shortcuts));
        assert_eq!(AppPage::parse("nope"), None);
        assert!(super::CHANGELOG_URL.starts_with("https://github.com/"));
        assert!(super::CHANGELOG_URL.ends_with("/CHANGELOG.md"));
    }

    #[test]
    fn credentials_list_names_and_file_values_without_reading_secrets() {
        /// Fails every value read: the table must not make one.
        struct NamesOnly(MemorySecrets);
        impl SecretStore for NamesOnly {
            fn get(&self, _: &str, name: &str) -> Option<String> {
                panic!("read {name}")
            }
            fn try_get(&self, _: &str, name: &str) -> Result<Option<String>, String> {
                panic!("read {name}")
            }
            fn set(&self, s: &str, n: &str, v: &str) -> Result<(), String> {
                self.0.set(s, n, v)
            }
            fn delete(&self, s: &str, n: &str) -> Result<(), String> {
                self.0.delete(s, n)
            }
            fn names(&self, s: &str) -> Vec<String> {
                self.0.names(s)
            }
        }
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".env"), "BASE_URL=https://api.example.com\n").unwrap();
        std::fs::write(
            d.path().join(".env.production"),
            "BASE_URL=https://prod.api.example.com\nSECRET_KEY=plain\nTOKEN=t\n",
        )
        .unwrap();
        std::fs::write(d.path().join(".env.staging"), "").unwrap();
        let secrets = NamesOnly(MemorySecrets::default());
        for env in ["staging", "production"] {
            secrets
                .set(
                    &service_name("Payments API", env, "0a1b2c3d"),
                    "API_TOKEN",
                    "tok_1234",
                )
                .unwrap();
        }
        let (rows, locked) = credential_rows(
            d.path(),
            "Payments API",
            "0a1b2c3d",
            &secrets,
            &["TOKEN".to_string()],
        );
        assert!(!locked);
        let row = |name: &str, env: &str, stored, value: Option<&str>| Credential {
            name: name.into(),
            environment: env.into(),
            stored,
            value: value.map(String::from),
        };
        assert_eq!(
            rows,
            [
                row(
                    "BASE_URL",
                    "Default",
                    Stored::File,
                    Some("https://api.example.com")
                ),
                row("API_TOKEN", "production", Stored::Keychain, None),
                row(
                    "BASE_URL",
                    "production",
                    Stored::File,
                    Some("https://prod.api.example.com")
                ),
                row("SECRET_KEY", "production", Stored::File, None),
                row("TOKEN", "production", Stored::File, None),
                row("API_TOKEN", "staging", Stored::Keychain, None),
            ]
        );
    }

    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use crate::model::collection_meta::CollectionMeta;
    use crate::model::settings::Settings;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;
    use std::sync::Arc;

    /// A window on a collection with `a.yaml` open: BASE_URL in `.env`,
    /// API_TOKEN (marked secret) in the staging Keychain.
    fn open(
        cx: &mut TestAppContext,
        base: &Path,
    ) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "BASE_URL=https://api.example.com\n",
        )
        .unwrap();
        std::fs::write(dir.path().join(".env.staging"), "").unwrap();
        std::fs::write(
            dir.path().join("a.yaml"),
            "request:\n  url: ${BASE_URL}/a\n  headers:\n    Authorization: Bearer ${API_TOKEN}\n",
        )
        .unwrap();
        let mut meta = CollectionMeta::default();
        meta.set_secret("API_TOKEN", true);
        meta.save(base, dir.path()).unwrap();
        let secrets = Arc::new(MemorySecrets::default());
        secrets
            .set(
                &service_for(base, dir.path(), "staging"),
                "API_TOKEN",
                "tok_live_9f3a",
            )
            .unwrap();
        let store = StateStore::at(base);
        let root = dir.path().to_path_buf();
        let shared: Arc<dyn SecretStore> = secrets;
        let (window, ws) = cx.update(|cx| {
            gpui_kit::init(cx);
            super::super::app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    // Tall enough to show every Accounts card unscrolled.
                    size: size(px(1440.), px(1400.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    let mut ws = Workspace::new(
                        Some(root),
                        store,
                        shared,
                        super::super::workspace::test_backend(),
                        window,
                        cx,
                    );
                    ws.open_file(Path::new("a.yaml"), window, cx);
                    ws
                })
            })
            .unwrap()
        });
        (dir, window, ws)
    }

    fn shows(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("app-settings")
                .try_find_by_label(label)
                .is_some()
        })
        .unwrap()
    }

    fn line_height(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
    ) -> f32 {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            let editor = ws.read(cx).tabs[0].editor.clone();
            f32::from(editor.read(cx).line_height().expect("laid out"))
        })
        .unwrap()
    }

    #[gpui_kit::test]
    fn editor_font_size_applies(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws) = open(cx, state.path());
        assert_eq!(line_height(cx, window, &ws), 18.);
        // ⌘, opens Settings; the Editor page; three steps up.
        cx.update_window(window, |_, window, cx| {
            window.press("cmd-,", cx);
            window.render_frame(cx);
            window
                .within("app-settings-nav")
                .click("app-settings-editor", cx);
            window.render_frame(cx);
            for _ in 0..3 {
                window.within("app-settings").click("font-size-up", cx);
                window.render_frame(cx);
            }
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, cx| {
            let view = ws.app_settings.clone().expect("Settings is open");
            assert_eq!(view.read(cx).page(), AppPage::Editor);
            assert_eq!(ws.settings.editor.font_size, 16);
        });
        assert!(shows(cx, window, "16 px"));
        assert_eq!(Settings::load(state.path()).editor.font_size, 16);
        // Esc goes back to the editor, at the new size.
        cx.update_window(window, |_, window, cx| window.press("escape", cx))
            .unwrap();
        ws.read_with(cx, |ws, _| assert!(!ws.app_settings_shown()));
        assert_eq!(line_height(cx, window, &ws), 22.);
    }

    #[gpui_kit::test]
    fn accounts_lists_credentials_masked_and_edits_in_the_drawer(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws) = open(cx, state.path());
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_app_settings(AppPage::Accounts, window, cx)
            });
        })
        .unwrap();
        assert!(shows(cx, window, "API_TOKEN Staging"));
        assert!(shows(cx, window, "BASE_URL Default"));
        assert!(shows(cx, window, "https://api.example.com"));
        assert!(shows(cx, window, super::MASK));
        for card in ["Server", "Account", "Sync", "Sync now", "Sign in"] {
            assert!(shows(cx, window, card), "{card}");
        }
        assert!(shows(
            cx,
            window,
            "Not synced · collections stay on this Mac"
        ));
        // The Keychain value never reaches the table, nor its tail.
        assert!(!shows(cx, window, "tok_live_9f3a"));
        assert!(!shows(cx, window, &format!("{}9f3a", super::MASK)));
        let rows = ws.read_with(cx, |ws, cx| {
            ws.app_settings
                .as_ref()
                .unwrap()
                .read(cx)
                .credentials()
                .len()
        });
        assert_eq!(rows, 2);
        // Edit: the collection's variable drawer on that environment.
        let row = ws.read_with(cx, |ws, cx| {
            ws.app_settings
                .as_ref()
                .unwrap()
                .read(cx)
                .credentials()
                .iter()
                .position(|c| c.name == "API_TOKEN")
                .unwrap()
        });
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("app-settings")
                .click(("credential-edit", row), cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, cx| {
            assert!(!ws.app_settings_shown());
            let view = ws.settings_view.clone().expect("the Environments page");
            assert_eq!(view.read(cx).status().environment, "Staging");
            assert!(view.read(cx).drawer_open_on("API_TOKEN"));
        });
        // Esc closes the drawer, then returns to Settings › Accounts & sync.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window
                    .within("settings-nav")
                    .try_find_by_label("Back to settings")
                    .is_some()
            );
            window.press("escape", cx);
            window.render_frame(cx);
            window.press("escape", cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, cx| {
            assert!(!ws.settings_shown());
            let view = ws.app_settings.clone().expect("back in Settings");
            assert_eq!(view.read(cx).page(), AppPage::Accounts);
        });
        // Edit again; this time the collection nav's App row.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("app-settings")
                .click(("credential-edit", row), cx);
        })
        .unwrap();
        cx.run_until_parked();
        // Its App rows open Settings on their page.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("settings-nav")
                .click("settings-shortcuts", cx);
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, cx| {
            assert!(!ws.settings_shown());
            let view = ws.app_settings.clone().expect("Settings is open");
            assert_eq!(view.read(cx).page(), AppPage::Shortcuts);
        });
        assert!(shows(cx, window, "Settings ⌘,"));
    }
}
