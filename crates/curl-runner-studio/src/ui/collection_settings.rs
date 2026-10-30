//! The collection settings page (board: Environments): its own left nav,
//! the General page (name and folder, read only), the Environments page:
//! one tab per environment, its variables with where each value comes from,
//! and the drawer that edits one, and the Sharing page (the Share dialog's
//! controls, board: Share). Secret values live in the Keychain only: the
//! table never shows them, and the drawer reads one when it opens. A
//! received collection's shared defaults read `Shared by {org}`, locked.
//!
//! It takes the sidebar's, the main area's and the inspector's place. The
//! workspace re-reads what changed when the page emits [`SettingsEvent`].

use super::app::{CloseSettings, mono_font, ui_font};
use super::color;
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::run_view::pill;
use super::settings_kit::{
    MASK, back_link, label_text, nav_item, neutral_pill, primary_button, small_button, soon_chip,
    switch, tinted_pill,
};
use super::settings_view::AppPage;
use super::share_dialog::{SUBTITLE as SHARE_SUBTITLE, ShareMode, ShareView};
use super::share_sync::ShareSync;
use super::workspace::{edit_notes, notify_error};
use crate::model::backend::Backend;
use crate::model::collection_meta::{CollectionMeta, VarMeta};
use crate::model::environments::{
    CapturedVar, DEFAULT_ENVIRONMENT, EnvRow, RowSource, UsageIndex, create_environment, env_file,
    environment_rows, file_readers, import_dotenv, is_secret, list_environments, move_to_file,
    move_to_keychain, name_in_env_files, plain_copies, remove_env_var, remove_env_var_at,
    write_env_var,
};
use crate::model::secrets::{CachedSecrets, SecretStore, service_name};
use curl_runner_core::env::parse_env_content;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::notification::Notification;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod drawer;
mod values;

pub const NAV_WIDTH: f32 = 240.;
pub const DRAWER_WIDTH: f32 = 380.;
const ROW_HEIGHT: f32 = 40.;
/// The table's fixed columns.
const NAME_COL: f32 = 180.;
const SOURCE_COL: f32 = 150.;
const SCOPE_COL: f32 = 110.;

/// The board's callout, verbatim.
const CALLOUT_HEAD: &str = "How customers bring their own values.";
const CALLOUT_BODY: &str = " When you share this collection, people get the variable names, descriptions and non-secret defaults. Secrets are never shared or synced: each person enters their own, and it stays in their device keychain. Values marked ";
const CALLOUT_SHARED: &str = "Shared by Acme";
const CALLOUT_TAIL: &str = " are locked for them.";
const LEGEND_SHARED: &str = "Set by the collection owner, read only for others";
/// The largest `.env` file Import reads.
const IMPORT_MAX_BYTES: u64 = 1 << 20;

/// A process-environment lookup.
type ProcessEnv = std::rc::Rc<dyn Fn(&str) -> Option<String>>;

/// What the page asks of the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsEvent {
    /// Back to the editor.
    Close,
    /// Files, Keychain items or the collection's notes changed: re-read
    /// environments, secret names and variables.
    Changed,
    /// Only the counts changed (the usage scan finished): re-read the
    /// status bar.
    Status,
    /// One of the `App` rows: the app's Settings on that page.
    OpenApp(AppPage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    General,
    Environments,
    Sharing,
}

/// What the Sharing page needs beyond the page's own notes.
pub struct Sharing {
    pub backend: Arc<dyn Backend>,
    /// The open collection's share, which the workspace keeps.
    pub sync: Entity<ShareSync>,
    /// People with access, as the title bar last counted them.
    pub people: Option<usize>,
}

/// The status bar while the page is shown.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvStatus {
    pub environment: SharedString,
    /// `6 of 7 variables set · 1 missing`.
    pub summary: SharedString,
    /// `Keychain unlocked`, or `Keychain locked` once a read failed.
    pub keychain: SharedString,
}

/// `6 of 7 variables set · 1 missing`.
pub fn summary_text(rows: &[EnvRow]) -> String {
    let set = rows
        .iter()
        .filter(|r| r.source != RowSource::NotSet)
        .count();
    let missing = rows.len() - set;
    let noun = if rows.len() == 1 {
        "variable"
    } else {
        "variables"
    };
    let mut text = format!("{set} of {} {noun} set", rows.len());
    if missing > 0 {
        text.push_str(&format!(" · {missing} missing"));
    }
    text
}

/// The Source pill's text.
pub fn source_label(source: &RowSource, root: &Path) -> String {
    match source {
        RowSource::File(p) if p == &root.join(".env") => "Collection default".into(),
        RowSource::File(p) => p
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        RowSource::Keychain => "My keychain".into(),
        RowSource::Process => "Shell environment".into(),
        RowSource::Captured { .. } => "Captured at run".into(),
        RowSource::Shared(org) => format!("Shared by {org}"),
        RowSource::Suggested(org) => format!("Suggested by {org}"),
        RowSource::NotSet => "Not set".into(),
    }
}

/// `from create-charge → body.id`.
fn captured_text(request: Option<&str>, path: Option<&str>) -> String {
    match (request, path) {
        (Some(r), Some(p)) => format!("from {r} → {p}"),
        (Some(r), None) => format!("from {r}"),
        (None, Some(p)) => format!("from {p}"),
        (None, None) => "from the last run".into(),
    }
}

/// `Used by 3 requests`.
fn used_by_text(n: usize) -> String {
    match n {
        0 => "Not used by any request".into(),
        1 => "Used by 1 request".into(),
        n => format!("Used by {n} requests"),
    }
}

/// What Save does with the drawer's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueWrite {
    Nothing,
    /// Into the Keychain, and out of this environment's own file. Plain
    /// copies in the other files it reads are asked about first.
    Keychain,
    /// Into this environment's file.
    File,
    /// Out of the Keychain into this environment's file: asks first.
    KeychainToFile,
}

/// The write a Save makes. A value is written only when typed into, or
/// when it moves between the file and the Keychain: a never-edited value
/// is never written (an empty Keychain item would count as set, and win
/// over the shell).
pub fn plan_value(secret: bool, source: &RowSource, edited: bool) -> ValueWrite {
    match (secret, source) {
        (true, RowSource::File(_)) => ValueWrite::Keychain,
        (true, _) if edited => ValueWrite::Keychain,
        (false, RowSource::Keychain) => ValueWrite::KeychainToFile,
        (false, _) if edited => ValueWrite::File,
        _ => ValueWrite::Nothing,
    }
}

/// The drawer's note under the value: where it is and where Save puts it.
pub fn where_stored(source: &RowSource, secret: bool, own_file: &Path, service: &str) -> String {
    let file = |p: &Path| {
        p.file_name().map_or_else(
            || p.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    };
    let keychain = format!("macOS Keychain under “{service}”");
    match (source, secret) {
        (RowSource::Keychain, true) => format!("Stored in {keychain}."),
        (RowSource::Keychain, false) => format!(
            "In the Keychain now. Save moves it to {}, in plain text.",
            file(own_file)
        ),
        (RowSource::Process, true) => format!(
            "Set in your shell environment. A value saved here goes to the {keychain} and wins over the shell."
        ),
        (RowSource::Process, false) => format!(
            "Overridden by your shell environment. Edits are saved to {}, but the shell value wins.",
            file(own_file)
        ),
        (RowSource::File(p), true) if file(p).ends_with(".local") => format!(
            "In plain text in {}. The studio never edits .local files — remove it there yourself.",
            file(p)
        ),
        (RowSource::File(p), true) => {
            format!(
                "In plain text in {}. Save moves it to the Keychain.",
                file(p)
            )
        }
        (RowSource::File(p), false) if p == own_file => {
            format!("Stored in {} in the collection folder.", file(p))
        }
        (RowSource::File(p), false) => {
            format!("From {}. Edits are saved to {}.", file(p), file(own_file))
        }
        (_, true) => format!("Saved to the {keychain}."),
        (_, false) => format!("Saved to {} in the collection folder.", file(own_file)),
    }
}

/// The copy confirmation: `A copy remains in .env, which every environment
/// reads. Remove it there too?`, or `Copies remain in .env.staging and .env,
/// …`. `.local` files are named apart: the studio never edits them.
pub fn copies_text(files: &[PathBuf]) -> String {
    let name = |p: &PathBuf| {
        p.file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    };
    let (local, removable): (Vec<String>, Vec<String>) =
        files.iter().map(name).partition(|n| n.ends_with(".local"));
    let readers = |names: &[String]| {
        let envs: Vec<Option<String>> = names.iter().map(|n| file_readers(n)).collect();
        match envs.as_slice() {
            [Some(e), rest @ ..] if !e.is_empty() && rest.iter().all(|r| r.as_ref() == Some(e)) => {
                format!("the {} environment reads", env_label(e))
            }
            _ => "every environment reads".to_string(),
        }
    };
    let and = |names: &[String]| match names {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    };
    let mut text = Vec::new();
    match removable.len() {
        0 => {}
        1 => text.push(format!(
            "A copy remains in {}, which {}. Remove it there too?",
            and(&removable),
            readers(&removable)
        )),
        _ => text.push(format!(
            "Copies remain in {}, which {}. Remove them there too?",
            and(&removable),
            readers(&removable)
        )),
    }
    if !local.is_empty() {
        let verb = if local.len() == 1 { "is" } else { "are" };
        text.push(format!(
            "{} {verb} your local override, so the studio leaves it; remove it there yourself.",
            and(&local)
        ));
    }
    text.join(" ")
}

/// How the person answered a Save question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmed {
    /// Not asked yet.
    No,
    /// Store in file / Remove the copy / OK.
    Yes,
    /// Write the Keychain, leave the plain copy where it is.
    KeepCopy,
}

/// `Plain copy remains in .env`.
fn plain_copy_text(p: &Path) -> String {
    let file = p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    format!("Plain copy remains in {file}")
}

/// The variable being edited.
struct Drawer {
    /// `None`: a new variable, named in `name_input`.
    name: Option<String>,
    name_input: Entity<InputState>,
    value: Entity<InputState>,
    description: Entity<TextareaState>,
    secret: bool,
    required: bool,
    revealed: bool,
    /// The value was typed into since the drawer opened.
    edited: bool,
    /// Where the value was when the drawer opened.
    source: RowSource,
    /// Why the value could not be read (a locked Keychain).
    read_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

pub struct CollectionSettings {
    root: PathBuf,
    collection: String,
    secrets: Arc<CachedSecrets>,
    environments: Vec<String>,
    environment: String,
    /// The collection's notes, shared with the workspace (which saves
    /// them); the page reads and writes them there, never a copy.
    notes: Entity<CollectionMeta>,
    _notes: Subscription,
    /// This environment's Keychain item names (no values).
    keychain: Vec<String>,
    keychain_locked: bool,
    usage: Arc<UsageIndex>,
    captured: Vec<CapturedVar>,
    /// The process environment rows resolve through (tests swap it).
    process_env: ProcessEnv,
    rows: Vec<EnvRow>,
    page: Page,
    drawer: Option<Drawer>,
    /// Opened from the app's Settings: Esc and Back return there.
    back_to_settings: bool,
    /// The Sharing page's server and counts (`None`: no sharing here).
    sharing: Option<Sharing>,
    /// The Sharing page's controls, while it is shown.
    share_view: Option<Entity<ShareView>>,
    _share_events: Option<Subscription>,
    focus: FocusHandle,
    _scan: Task<()>,
}

impl EventEmitter<SettingsEvent> for CollectionSettings {}

impl Focusable for CollectionSettings {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl CollectionSettings {
    /// The page on `environment`, with the last run's `captured` values.
    pub fn new(
        root: PathBuf,
        collection: String,
        notes: Entity<CollectionMeta>,
        secrets: Arc<CachedSecrets>,
        environment: String,
        captured: Vec<CapturedVar>,
        cx: &mut Context<Self>,
    ) -> Self {
        // An import (or the share) changing the notes shows at once.
        let notes_changed = cx.observe(&notes, |this, _, cx| {
            this.reload_rows(cx);
            cx.notify();
        });
        // Reading every request file is the slow part: off the main thread.
        let scan_root = root.clone();
        let scan = cx.background_spawn(async move { UsageIndex::scan(&scan_root) });
        let scan = cx.spawn(async move |this, cx| {
            let usage = scan.await;
            this.update(cx, |this, cx| {
                this.usage = Arc::new(usage);
                this.reload_rows(cx);
                cx.emit(SettingsEvent::Status);
                cx.notify();
            })
            .ok();
        });
        let mut this = CollectionSettings {
            environments: list_environments(&root),
            root,
            collection,
            secrets,
            environment,
            notes,
            _notes: notes_changed,
            keychain: Vec::new(),
            keychain_locked: false,
            usage: Arc::default(),
            captured,
            process_env: std::rc::Rc::new(|k: &str| std::env::var(k).ok()),
            rows: Vec::new(),
            page: Page::Environments,
            drawer: None,
            back_to_settings: false,
            sharing: None,
            share_view: None,
            _share_events: None,
            focus: cx.focus_handle(),
            _scan: scan,
        };
        if !this.environments.contains(&this.environment) {
            this.environment = DEFAULT_ENVIRONMENT.to_string();
        }
        this.reload(cx);
        this
    }

    fn service(&self, cx: &App) -> String {
        self.service_of(&self.environment, cx)
    }

    /// The Keychain service of environment `env` of this collection.
    fn service_of(&self, env: &str, cx: &App) -> String {
        service_name(&self.collection, env, &self.notes.read(cx).id)
    }

    /// Re-reads the Keychain's item names and the rows.
    fn reload(&mut self, cx: &App) {
        match self.secrets.try_names(&self.service(cx)) {
            Ok(names) => self.keychain = names,
            Err(e) => {
                log::warn!("Couldn't list Keychain items: {e}");
                self.keychain.clear();
                self.keychain_locked = true;
            }
        }
        self.reload_rows(cx);
    }

    fn reload_rows(&mut self, cx: &App) {
        let meta = self.notes.read(cx);
        self.rows = environment_rows(
            &self.root,
            &self.environment,
            &meta.secret_names,
            &self.keychain,
            &self.usage.referenced(),
            &self.captured,
            &*self.process_env,
        );
        // A received copy: the owner's defaults are listed (and count as
        // set) in every environment, as a run reads them. A locked one is
        // `Shared by {org}` over any local value; a suggested one is
        // `Suggested by {org}` only where nothing local sets the name.
        let shared: Vec<(EnvRow, bool)> = meta
            .received_vars
            .iter()
            .filter_map(|v| {
                let (locked, (org, value)) = match meta.shared_by(&v.name) {
                    Some(l) => (true, l),
                    None => (false, meta.suggested_by(&v.name)?),
                };
                let source = if locked {
                    RowSource::Shared(org.to_string())
                } else {
                    RowSource::Suggested(org.to_string())
                };
                let row = EnvRow {
                    name: v.name.clone(),
                    value: Some(value.to_string()),
                    source,
                    secret: false,
                    plain_copy: None,
                };
                Some((row, locked))
            })
            .collect();
        for (row, locked) in shared {
            match self.rows.iter_mut().find(|r| r.name == row.name) {
                Some(r) if locked || r.source == RowSource::NotSet => *r = row,
                Some(_) => {}
                None => self.rows.push(row),
            }
        }
    }

    /// The Sharing page's server and counts.
    pub fn set_sharing(&mut self, sharing: Sharing, cx: &mut Context<Self>) {
        self.sharing = Some(sharing);
        cx.notify();
    }

    /// A new share (or none: signed out, or nothing open) for the Sharing
    /// page: the old one's controls go, and a shown page reopens on it.
    pub fn replace_sharing(
        &mut self,
        sharing: Option<Sharing>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sharing = sharing;
        self.share_view = None;
        self._share_events = None;
        if self.page == Page::Sharing {
            self.open_sharing(window, cx);
        }
        cx.notify();
    }

    /// People with access, for the nav's count: the share's own once it
    /// answered, else the title bar's.
    pub fn sharing_count(&self, cx: &App) -> Option<usize> {
        let s = self.sharing.as_ref()?;
        s.sync.read(cx).people().or(s.people)
    }

    /// The Sharing page's controls, when it is shown.
    #[cfg(test)]
    pub fn share_view(&self) -> Option<&Entity<ShareView>> {
        self.share_view.as_ref()
    }

    #[cfg(test)]
    pub fn set_process_env(&mut self, env: impl Fn(&str) -> Option<String> + 'static, cx: &App) {
        self.process_env = std::rc::Rc::new(env);
        self.reload_rows(cx);
    }

    #[cfg(test)]
    pub fn drawer_open_on(&self, name: &str) -> bool {
        self.drawer.as_ref().and_then(|d| d.name.as_deref()) == Some(name)
    }

    #[cfg(test)]
    pub fn rows(&self) -> &[EnvRow] {
        &self.rows
    }

    /// Back and Esc return to the app's Settings, not the workspace.
    pub fn set_back_to_settings(&mut self, on: bool, cx: &mut Context<Self>) {
        self.back_to_settings = on;
        cx.notify();
    }

    pub fn status(&self) -> EnvStatus {
        EnvStatus {
            environment: env_label(&self.environment).into(),
            summary: summary_text(&self.rows).into(),
            keychain: if self.keychain_locked {
                "Keychain locked"
            } else {
                "Keychain unlocked"
            }
            .into(),
        }
    }

    /// The collection's notes (the workspace's).
    /// A received copy: its variables are the owner's (no new, deleted or
    /// re-described ones); only their values are the customer's.
    fn received(&self, cx: &App) -> bool {
        self.meta(cx).read_only_from.is_some()
    }

    pub fn meta<'a>(&self, cx: &'a App) -> &'a CollectionMeta {
        self.notes.read(cx)
    }

    pub fn select_environment(&mut self, name: &str, cx: &mut Context<Self>) {
        if self.environment == name {
            return;
        }
        self.environment = name.to_string();
        self.drawer = None;
        self.reload(cx);
        cx.emit(SettingsEvent::Changed);
        cx.notify();
    }

    pub fn set_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
        self.drawer = None;
        self.share_view = None;
        self._share_events = None;
        if page == Page::Sharing {
            self.open_sharing(window, cx);
        }
        cx.notify();
    }

    /// The Share dialog's controls in page form, on the notes as they are
    /// now; only for an owner who is signed in.
    fn open_sharing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = &self.sharing else { return };
        if s.backend.account().is_none() || self.meta(cx).read_only_from.is_some() {
            return;
        }
        let sync = s.sync.clone();
        let view = cx.new(|cx| ShareView::new(sync.clone(), ShareMode::Page, window, cx));
        // The nav's count follows the share.
        self._share_events = Some(cx.observe(&sync, |_, _, cx| cx.notify()));
        self.share_view = Some(view);
    }

    /// Esc: the drawer first, then the page.
    fn escape(&mut self, cx: &mut Context<Self>) {
        if self.drawer.take().is_some() {
            cx.notify();
        } else {
            cx.emit(SettingsEvent::Close);
        }
    }

    // New environment and Import.

    fn prompt_new_environment(&self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("staging"));
        let first_build = Cell::new(true);
        let this = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, window, cx| {
            if first_build.replace(false) {
                input.read(cx).focus_handle(cx).focus(window, cx);
            }
            let (this, input) = (this.clone(), input.clone());
            dialog
                .w(px(420.))
                .title("New environment")
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(Input::new(&input))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-tertiary"))
                                .child("Creates an empty .env.{name} file in the collection."),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("new-env-cancel")
                                .ghost()
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("new-env-create")
                                .primary()
                                .label("Create")
                                .on_click(move |_, window, cx| {
                                    let name = input.read(cx).value().trim().to_string();
                                    window.close_dialog(cx);
                                    this.update(cx, |this, cx| {
                                        this.new_environment(&name, window, cx)
                                    })
                                    .ok();
                                }),
                        ),
                )
        });
    }

    fn new_environment(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = create_environment(&self.root, name) {
            notify_error(format!("Couldn't create the environment: {e}"), window, cx);
            return;
        }
        self.environments = list_environments(&self.root);
        self.select_environment(name, cx);
        cx.emit(SettingsEvent::Changed);
    }

    fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.received(cx) {
            return;
        }
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            this.update_in(cx, |this, window, cx| this.import_file(&path, window, cx))
                .ok();
        })
        .detach();
    }

    fn import_file(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let too_big = std::fs::metadata(path).is_ok_and(|m| m.len() > IMPORT_MAX_BYTES);
        let content = match std::fs::read_to_string(path) {
            _ if too_big => {
                notify_error(
                    format!("{} is too large to import.", path.display()),
                    window,
                    cx,
                );
                return;
            }
            Ok(c) => c,
            Err(e) => {
                notify_error(format!("Couldn't read {}: {e}", path.display()), window, cx);
                return;
            }
        };
        let service = self.service(cx);
        let marked = self.meta(cx).secret_names.clone();
        match import_dotenv(
            &self.root,
            &self.environment,
            &content,
            &*self.secrets,
            &service,
            &marked,
        ) {
            Ok(n) => {
                let noun = if n == 1 { "variable" } else { "variables" };
                window.push_notification(
                    Notification::success(format!(
                        "Imported {n} {noun} into {}",
                        env_label(&self.environment)
                    )),
                    cx,
                );
            }
            Err(e) => notify_error(
                format!("Couldn't import {}: {e}", path.display()),
                window,
                cx,
            ),
        }
        self.reload(cx);
        cx.emit(SettingsEvent::Changed);
        cx.notify();
    }
}

impl Render for CollectionSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer = (self.page == Page::Environments)
            .then(|| self.render_drawer(cx))
            .flatten();
        div()
            .id("collection-settings")
            .key_context("CollectionSettings")
            .track_focus(&self.focus)
            .test_support()
            .on_action(cx.listener(|this, _: &CloseSettings, _, cx| this.escape(cx)))
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(color("bg-100"))
            .child(self.render_nav(cx))
            .child(match self.page {
                Page::General => self.render_general().into_any_element(),
                Page::Environments => self.render_environments(cx).into_any_element(),
                Page::Sharing => self.render_sharing(cx).into_any_element(),
            })
            .children(drawer)
    }
}

/// The secret key glyph in `size`.
fn key_icon(size: f32) -> impl IntoElement {
    glyph(icons::KEY).size(px(size))
}

impl CollectionSettings {
    fn render_nav(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let section = |text: &'static str, enabled: bool, first: bool| {
            label_text(text)
                .px(px(8.))
                .pt(px(if first { 4. } else { 14. }))
                .pb(px(6.))
                .when(!enabled, |d| d.text_color(color("text-disabled")))
        };
        let item = |id: &'static str, label: &'static str, on: Option<bool>| {
            nav_item(id, label, 28., on)
                .gap(px(8.))
                .px(px(8.))
                .child(div().flex_1().child(label))
        };
        let page_item =
            |id: &'static str, label: &'static str, page: Page, cx: &mut Context<Self>| {
                item(id, label, Some(self.page == page)).on_click(
                    cx.listener(move |this, _, window, cx| this.set_page(page, window, cx)),
                )
            };
        // The app's own settings, on the Settings screen.
        let app_item =
            |id: &'static str, label: &'static str, page: AppPage, cx: &mut Context<Self>| {
                item(id, label, Some(false))
                    .on_click(cx.listener(move |_, _, _, cx| cx.emit(SettingsEvent::OpenApp(page))))
            };
        let disabled =
            |id: &'static str, label: &'static str| item(id, label, None).child(soon_chip());
        div()
            .id("settings-nav")
            .role(Role::Navigation)
            .test_support()
            .aria_label("Settings")
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(2.))
            .w(px(NAV_WIDTH))
            .h_full()
            .px(px(8.))
            .py(px(12.))
            .border_r_1()
            .border_color(color("border-100"))
            .bg(color("bg-100"))
            .child(
                back_link(
                    "settings-back",
                    if self.back_to_settings {
                        "Back to settings"
                    } else {
                        "Back to workspace"
                    },
                    12.,
                )
                .px(px(8.))
                .pt(px(4.))
                .pb(px(12.))
                .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close))),
            )
            .child(
                label_text(self.collection.clone())
                    .px(px(8.))
                    .pt(px(4.))
                    .pb(px(6.))
                    .text_ellipsis(),
            )
            .child(page_item("settings-general", "General", Page::General, cx))
            .child(
                page_item(
                    "settings-environments",
                    "Environments",
                    Page::Environments,
                    cx,
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(self.environments.len().to_string()),
                ),
            )
            .child(
                page_item("settings-sharing", "Sharing", Page::Sharing, cx).children(
                    self.sharing_count(cx).map(|n| {
                        div()
                            .text_size(px(11.))
                            .text_color(color("text-tertiary"))
                            .child(n.to_string())
                    }),
                ),
            )
            .child(section("Workspace", false, false))
            .child(disabled("settings-members", "Members"))
            .child(disabled("settings-sync", "Sync & hosting"))
            .child(disabled("settings-billing", "Billing"))
            .child(section("App", true, false))
            .child(app_item(
                "settings-appearance",
                "Appearance",
                AppPage::Appearance,
                cx,
            ))
            .child(app_item("settings-editor", "Editor", AppPage::Editor, cx))
            .child(app_item(
                "settings-shortcuts",
                "Keyboard shortcuts",
                AppPage::Shortcuts,
                cx,
            ))
            .child(app_item(
                "settings-updates",
                "Updates",
                AppPage::Updates,
                cx,
            ))
    }

    fn render_header(&self, title: &'static str, subtitle: impl IntoElement) -> Div {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(4.))
            .px(px(24.))
            .pt(px(20.))
            .pb(px(12.))
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(title),
            )
            .child(subtitle)
    }

    fn render_general(&self) -> impl IntoElement + use<> {
        let field = |label: &'static str, value: String, mono: bool| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .h(px(ROW_HEIGHT))
                .px(px(16.))
                .border_b_1()
                .border_color(color("surface-200"))
                .child(
                    div()
                        .w(px(NAME_COL))
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(color("text-tertiary"))
                        .child(label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis()
                        .when(mono, |d| d.font_family(mono_font()).text_size(px(12.)))
                        .child(value),
                )
        };
        div()
            .id("settings-general-page")
            .role(Role::Main)
            .test_support()
            .aria_label("General")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                self.render_header(
                    "General",
                    div()
                        .text_size(px(13.))
                        .text_color(color("text-secondary"))
                        .child("The collection is the folder: rename or move it in Finder."),
                ),
            )
            .child(
                div()
                    .mx(px(24.))
                    .mt(px(4.))
                    .border_1()
                    .border_color(color("border-100"))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .child(field("Name", self.collection.clone(), false))
                    .child(field("Folder", self.root.display().to_string(), true)),
            )
    }

    /// Some row is a received copy's `Shared by {org}` default.
    fn shared_rows(&self) -> bool {
        self.rows
            .iter()
            .any(|r| matches!(r.source, RowSource::Shared(_)))
    }

    fn render_sharing(&self, cx: &App) -> impl IntoElement + use<> {
        let subtitle = div()
            .text_size(px(13.))
            .line_height(relative(1.5))
            .text_color(color("text-secondary"))
            .child(SHARE_SUBTITLE);
        // Why there is nothing to change: received, or not signed in.
        let note = match (&self.meta(cx).read_only_from, &self.sharing) {
            (Some(org), _) => Some(format!(
                "Shared with you by {org}. Only the owner can change who has access."
            )),
            (None, Some(s)) if s.backend.account().is_some() => None,
            (None, _) => Some("Sign in to share this collection.".to_string()),
        };
        div()
            .id("sharing-page")
            .role(Role::Main)
            .test_support()
            .aria_label("Sharing")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(self.render_header("Sharing", subtitle))
            .child(
                div()
                    .id("sharing-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(24.))
                    .pt(px(8.))
                    .pb(px(24.))
                    .child(
                        div()
                            .max_w(px(640.))
                            .children(note.map(|n| {
                                div()
                                    .id("sharing-note")
                                    .test_support()
                                    .aria_label(n.clone())
                                    .px(px(16.))
                                    .py(px(14.))
                                    .border_1()
                                    .border_color(color("border-100"))
                                    .rounded(px(8.))
                                    .text_size(px(13.))
                                    .text_color(color("text-secondary"))
                                    .child(n)
                            }))
                            .children(self.share_view.clone()),
                    ),
            )
    }

    fn render_environments(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let subtitle = div()
            .flex()
            .items_center()
            .text_size(px(13.))
            .text_color(color("text-secondary"))
            .child("Variables fill\u{a0}")
            .child(
                div()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .child("${NAME}"),
            )
            .child("\u{a0}placeholders in every request of this collection.");
        div()
            .id("environments-page")
            .role(Role::Main)
            .test_support()
            .aria_label("Environments")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(self.render_header("Environments", subtitle))
            .child(self.render_tabs(cx))
            .child(
                div()
                    .id("environments-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb(px(24.))
                    .child(render_callout())
                    .child(self.render_table(cx))
                    .child(render_legend(self.shared_rows())),
            )
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let tabs = self.environments.iter().enumerate().map(|(i, env)| {
            let on = *env == self.environment;
            let dot = match env.to_ascii_lowercase().as_str() {
                "production" | "prod" => "warning",
                "staging" | "stage" => "success",
                _ => "text-tertiary",
            };
            let name = env.clone();
            div()
                .id(("environment-tab", i))
                .role(Role::Tab)
                .test_support()
                .aria_label(env_label(env))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.))
                .h(px(30.))
                .px(px(12.))
                .rounded(px(6.))
                .text_size(px(13.))
                .whitespace_nowrap()
                .cursor_pointer()
                .map(|t| {
                    if on {
                        t.bg(color("surface-300")).text_color(color("text-primary"))
                    } else {
                        t.text_color(color("text-secondary"))
                            .hover(|s| s.bg(color("surface-100")))
                    }
                })
                .child(div().size(px(6.)).rounded_full().bg(color(dot)))
                .child(env_label(env))
                .on_click(cx.listener(move |this, _, _, cx| this.select_environment(&name, cx)))
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.))
            .px(px(24.))
            .pb(px(12.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(
                div()
                    .id("environment-tabs")
                    .role(Role::TabList)
                    .flex()
                    .gap(px(4.))
                    .children(tabs),
            )
            .child(
                small_button("new-environment", "New environment")
                    .border_dashed()
                    .text_color(color("text-secondary"))
                    .child(glyph(icons::PLUS).size(px(13.)))
                    .child("New environment")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.prompt_new_environment(window, cx)),
                    ),
            )
            .child(div().flex_1())
            .when(!self.received(cx), |bar| {
                bar.child(
                    small_button("import-env", "Import .env")
                        .child("Import .env")
                        .on_click(cx.listener(|this, _, window, cx| this.import(window, cx))),
                )
                .child(
                    primary_button("add-variable", "Add variable")
                        .ml(px(4.))
                        .child(glyph(icons::PLUS).size(px(13.)))
                        .child("Add variable")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_variable(None, window, cx)),
                        ),
                )
            })
    }

    fn render_table(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let th = |text: &'static str| {
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color("text-tertiary"))
                .whitespace_nowrap()
                .child(text)
        };
        let head = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(px(32.))
            .px(px(16.))
            .border_b_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .child(th("Name").w(px(NAME_COL)).flex_none())
            .child(th("Value").flex_1().min_w_0())
            .child(th("Source").w(px(SOURCE_COL)).flex_none())
            .child(th("Scope").w(px(SCOPE_COL)).flex_none());
        let selected = self.drawer.as_ref().and_then(|d| d.name.clone());
        let rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| self.render_row(i, r, selected.as_deref() == Some(&r.name), cx));
        let empty = self.rows.is_empty().then(|| {
            div()
                .flex()
                .items_center()
                .h(px(ROW_HEIGHT))
                .px(px(16.))
                .border_b_1()
                .border_color(color("surface-200"))
                .text_size(px(13.))
                .text_color(color("text-tertiary"))
                .child(format!(
                    "No variables in {} yet.",
                    env_label(&self.environment)
                ))
        });
        div()
            .id("variables-table")
            .role(Role::Table)
            .test_support()
            .m(px(16.))
            .mx(px(24.))
            .border_1()
            .border_color(color("border-100"))
            .rounded(px(8.))
            .overflow_hidden()
            .child(head)
            .children(rows)
            .children(empty)
            .when(!self.received(cx), |table| {
                table.child(
                    div()
                        .id("add-variable-row")
                        .role(Role::Button)
                        .test_support()
                        .aria_label("Add variable")
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .h(px(ROW_HEIGHT))
                        .px(px(16.))
                        .text_size(px(13.))
                        .text_color(color("text-secondary"))
                        .cursor_pointer()
                        .hover(|s| s.text_color(color("text-primary")))
                        .child(glyph(icons::PLUS).size(px(13.)))
                        .child("Add variable")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_variable(None, window, cx)),
                        ),
                )
            })
    }

    fn render_row(
        &self,
        ix: usize,
        r: &EnvRow,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let missing = r.source == RowSource::NotSet;
        let mono = |text: String| {
            div()
                .min_w_0()
                .text_ellipsis()
                .font_family(mono_font())
                .text_size(px(12.))
                .child(text)
        };
        // A captured value is used as `${store.NAME}`.
        let used_as = match &r.source {
            RowSource::Captured { .. } => format!("store.{}", r.name),
            _ => r.name.clone(),
        };
        let uses = self.usage.total(&used_as);
        let value = match (&r.source, r.secret) {
            (RowSource::NotSet, _) => div()
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.))
                .text_color(color("warning"))
                .child(glyph(icons::WARNING).size(px(12.)).flex_none())
                .child(div().whitespace_nowrap().child(used_by_text(uses)))
                .child({
                    let name = r.name.clone();
                    small_button(("set-value", ix), "Set value")
                        .h(px(24.))
                        .child("Set value")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.open_variable(Some(&name), window, cx)
                        }))
                })
                .into_any_element(),
            (RowSource::Captured { request, path }, _) => {
                mono(captured_text(request.as_deref(), path.as_deref()))
                    .text_color(color("text-tertiary"))
                    .into_any_element()
            }
            (_, true) => div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(mono(MASK.into()).text_color(color("text-secondary")))
                .child(
                    // Opens the drawer, where the value can be revealed.
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.))
                        .text_color(color("text-secondary"))
                        .child(glyph(icons::EYE).size(px(14.))),
                )
                .children(r.plain_copy.as_ref().map(|p| {
                    div()
                        .flex()
                        .min_w_0()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(color("warning"))
                        .child(glyph(icons::WARNING).size(px(12.)).flex_none())
                        .child(div().text_ellipsis().child(plain_copy_text(p)))
                }))
                .into_any_element(),
            (_, false) => mono(r.value.clone().unwrap_or_default()).into_any_element(),
        };
        // What the smoke tests read: never a secret's value.
        let value_label = match (&r.source, r.secret) {
            (RowSource::NotSet, _) => used_by_text(uses),
            (RowSource::Captured { request, path }, _) => {
                captured_text(request.as_deref(), path.as_deref())
            }
            (_, true) => match &r.plain_copy {
                Some(p) => format!("Masked · {}", plain_copy_text(p)),
                None => "Masked".into(),
            },
            (_, false) => r.value.clone().unwrap_or_default(),
        };
        let source_text = source_label(&r.source, &self.root);
        let source = match &r.source {
            // A received copy's owner default: locked.
            RowSource::Shared(_) => neutral_pill()
                .gap(px(6.))
                .child(glyph(icons::LOCK).size(px(12.)))
                .child(source_text.clone()),
            RowSource::Keychain => tinted_pill("accent-purple")
                .child(key_icon(12.))
                .child("My keychain"),
            RowSource::Captured { .. } => tinted_pill("accent-blue").child("Captured at run"),
            RowSource::NotSet => pill(("warning", "warning-subtle", "warning-border"))
                .font_weight(FontWeight::NORMAL)
                .child("Not set"),
            other => neutral_pill().child(source_label(other, &self.root)),
        };
        let scope = self
            .usage
            .scope(&self.root, &used_as)
            .unwrap_or_else(|| "—".into());
        let name = r.name.clone();
        let clickable = !matches!(r.source, RowSource::Captured { .. } | RowSource::Shared(_));
        div()
            .id(("env-row", ix))
            .role(Role::Row)
            .test_support()
            .aria_label(r.name.clone())
            .relative()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(px(ROW_HEIGHT))
            .px(px(16.))
            .border_b_1()
            .border_color(color("surface-200"))
            .text_size(px(13.))
            .when(missing, |d| d.bg(color("warning-subtle")))
            .when(selected, |d| {
                d.bg(color("surface-200")).child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(2.))
                        .bg(color("primary")),
                )
            })
            .when(clickable, |d| {
                d.cursor_pointer()
                    .when(!selected && !missing, |d| {
                        d.hover(|s| s.bg(color("surface-100")))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_variable(Some(&name), window, cx)
                    }))
            })
            .child(
                div()
                    .w(px(NAME_COL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(mono(r.name.clone()).text_color(color(if missing {
                        "warning"
                    } else {
                        "text-primary"
                    })))
                    .when(r.secret, |d| {
                        d.child(
                            neutral_pill()
                                .text_size(px(11.))
                                .px(px(6.))
                                .py_0()
                                .child("secret"),
                        )
                    }),
            )
            .child(
                div()
                    .id(("env-value", ix))
                    .test_support()
                    .aria_label(value_label)
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .child(value),
            )
            .child(
                div()
                    .id(("env-source", ix))
                    .test_support()
                    .aria_label(source_text)
                    .w(px(SOURCE_COL))
                    .flex_none()
                    .flex()
                    .child(source),
            )
            .child(
                div()
                    .id(("env-scope", ix))
                    .test_support()
                    .aria_label(scope.clone())
                    .w(px(SCOPE_COL))
                    .flex_none()
                    .text_ellipsis()
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(scope),
            )
    }
}

/// The board's info callout, wrapping as one text.
fn render_callout() -> impl IntoElement {
    let run = |len: usize, weight: FontWeight, tone: &str| TextRun {
        len,
        font: Font {
            weight,
            ..font(ui_font())
        },
        color: color(tone),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let text = format!("{CALLOUT_HEAD}{CALLOUT_BODY}{CALLOUT_SHARED}{CALLOUT_TAIL}");
    let styled = StyledText::new(text).with_runs(vec![
        run(CALLOUT_HEAD.len(), FontWeight::MEDIUM, "text-primary"),
        run(CALLOUT_BODY.len(), FontWeight::NORMAL, "text-primary"),
        run(CALLOUT_SHARED.len(), FontWeight::NORMAL, "text-secondary"),
        run(CALLOUT_TAIL.len(), FontWeight::NORMAL, "text-primary"),
    ]);
    div()
        .id("environments-callout")
        .role(Role::Note)
        .test_support()
        .flex()
        .gap(px(10.))
        .mx(px(24.))
        .mt(px(16.))
        .px(px(14.))
        .py(px(12.))
        .border_1()
        .border_color(color("info-border"))
        .bg(color("info-subtle"))
        .rounded(px(8.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .child(
            glyph(icons::INFO)
                .size(px(14.))
                .flex_none()
                .mt(px(2.))
                .text_color(color("accent-blue")),
        )
        .child(div().flex_1().min_w_0().child(styled))
}

/// `shared`: the lock's `Set by the collection owner` joins the legend.
fn render_legend(shared: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap(px(16.))
        .px(px(24.))
        .text_size(px(12.))
        .text_color(color("text-tertiary"))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_color(color("accent-purple"))
                        .child(key_icon(12.)),
                )
                .child("Secret — stored in local keychain, never synced"),
        )
        .when(shared, |d| {
            d.child(
                div()
                    .id("legend-shared")
                    .test_support()
                    .aria_label(LEGEND_SHARED)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(glyph(icons::LOCK).size(px(12.)))
                    .child(LEGEND_SHARED),
            )
        })
        .child(
            div().w_full().child(
                "Resolution order: captured → my keychain → environment → collection default",
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        CollectionMeta, CollectionSettings, Confirmed, EnvRow, Path, RowSource, SecretStore,
        ValueWrite, captured_text, copies_text, is_secret, plan_value, source_label, summary_text,
        used_by_text, where_stored,
    };
    use crate::model::secrets::service_for;
    use gpui_kit::{AppContext as _, Entity, Focusable as _, px, size};
    use std::sync::Arc;

    fn row(name: &str, source: RowSource) -> EnvRow {
        EnvRow {
            name: name.into(),
            value: None,
            source,
            secret: false,
            plain_copy: None,
        }
    }

    #[test]
    fn summary_counts_set_and_missing() {
        let rows = [
            row("A", RowSource::Keychain),
            row("B", RowSource::NotSet),
            row(
                "C",
                RowSource::Captured {
                    request: None,
                    path: None,
                },
            ),
        ];
        assert_eq!(summary_text(&rows), "2 of 3 variables set · 1 missing");
        assert_eq!(summary_text(&rows[..1]), "1 of 1 variable set");
    }

    #[test]
    fn sources_read_as_the_board() {
        let root = Path::new("/c");
        assert_eq!(
            source_label(&RowSource::File(root.join(".env")), root),
            "Collection default"
        );
        assert_eq!(
            source_label(&RowSource::File(root.join(".env.staging")), root),
            ".env.staging"
        );
        assert_eq!(source_label(&RowSource::Keychain, root), "My keychain");
        assert_eq!(
            captured_text(Some("create-charge"), Some("body.id")),
            "from create-charge → body.id"
        );
        assert_eq!(used_by_text(1), "Used by 1 request");
    }

    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use crate::model::secrets::MemorySecrets;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Action as _, AnyWindowHandle, Bounds, Point, TestAppContext, WindowBounds, WindowOptions,
    };

    /// A window on a staging collection: BASE_URL in `.env`, API_TOKEN
    /// marked secret in the Keychain, ACCOUNT_ID used but unset.
    fn open(
        cx: &mut TestAppContext,
        base: &Path,
    ) -> (
        tempfile::TempDir,
        AnyWindowHandle,
        Entity<Workspace>,
        Arc<MemorySecrets>,
    ) {
        open_with(cx, base, super::super::workspace::test_backend())
    }

    /// [`open`] on `backend`.
    fn open_with(
        cx: &mut TestAppContext,
        base: &Path,
        backend: Arc<dyn crate::model::backend::Backend>,
    ) -> (
        tempfile::TempDir,
        AnyWindowHandle,
        Entity<Workspace>,
        Arc<MemorySecrets>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let files = [
            (".env", "BASE_URL=https://api.example.com\n"),
            (".env.staging", "BASE_URL=https://staging.api.example.com\n"),
            (
                "charges/create-charge.yaml",
                "request:\n  url: ${BASE_URL}/charges\n  headers:\n    Authorization: Bearer ${API_TOKEN}\n",
            ),
            (
                "customers/get.yaml",
                "request:\n  url: ${BASE_URL}/customers/${ACCOUNT_ID}\n",
            ),
        ];
        for (rel, text) in files {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        let mut meta = CollectionMeta::default();
        meta.set_secret("API_TOKEN", true);
        meta.save(base, dir.path()).unwrap();
        let secrets = Arc::new(MemorySecrets::default());
        secrets
            .set(
                &service_for(base, dir.path(), "staging"),
                "API_TOKEN",
                "tok_live_1",
            )
            .unwrap();
        let store = StateStore::at(base);
        store.remember_environment(dir.path(), "staging");
        let root = dir.path().to_path_buf();
        let shared: Arc<dyn SecretStore> = secrets.clone();
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
                cx.new(|cx| Workspace::new(Some(root), store, shared, backend, window, cx))
            })
            .unwrap()
        });
        (dir, window, ws, secrets)
    }

    fn shows(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("collection-settings")
                .try_find_by_label(label)
                .is_some()
        })
        .unwrap()
    }

    #[gpui_kit::test]
    fn environments_screen_lists_sources(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws, _) = open(cx, state.path());
        cx.update_window(window, |_, window, cx| {
            window.dispatch_action(super::super::app::OpenEnvironments.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
        let rows = ws.read_with(cx, |ws, cx| {
            let view = ws.settings_view.clone().expect("the page is open");
            let rows: Vec<(String, RowSource)> = view
                .read(cx)
                .rows()
                .iter()
                .map(|r| (r.name.clone(), r.source.clone()))
                .collect();
            rows
        });
        let names: Vec<&str> = rows.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["BASE_URL", "API_TOKEN", "ACCOUNT_ID"]);
        assert_eq!(rows[1].1, RowSource::Keychain);
        assert_eq!(rows[2].1, RowSource::NotSet);
        assert!(shows(cx, window, ".env.staging"));
        assert!(shows(cx, window, "My keychain"));
        assert!(shows(cx, window, "Not set"));
        assert!(shows(cx, window, "Used by 1 request"));
        assert!(shows(cx, window, "All requests"), "BASE_URL's scope");
        assert!(shows(cx, window, "customers/*"), "ACCOUNT_ID's scope");
        // The secret never reaches the table.
        assert!(!shows(cx, window, "tok_live_1"));
        ws.read_with(cx, |ws, cx| {
            let status = ws.settings_view.as_ref().unwrap().read(cx).status();
            assert_eq!(status.environment, "Staging");
            assert_eq!(status.summary, "2 of 3 variables set · 1 missing");
            assert_eq!(status.keychain, "Keychain unlocked");
        });
        // Esc leaves for the editor.
        cx.update_window(window, |_, window, cx| window.press("escape", cx))
            .unwrap();
        ws.read_with(cx, |ws, _| assert!(!ws.settings_shown()));
    }

    #[gpui_kit::test]
    fn drawer_names_the_full_keychain_service(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, _) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        let id = CollectionMeta::load(state.path(), dir.path()).id;
        assert!(
            service.ends_with(&format!(" · staging · {id}")),
            "{service}"
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_settings(window, cx);
                ws.settings_on(None, Some("API_TOKEN"), window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        let note = ws.read_with(cx, |ws, cx| {
            let view = ws.settings_view.clone().unwrap();
            let v = view.read(cx);
            let d = v.drawer.as_ref().expect("the drawer is open");
            where_stored(&d.source, true, &v.own_file(), &v.service(cx))
        });
        assert_eq!(note, format!("Stored in macOS Keychain under “{service}”."));
    }

    #[gpui_kit::test]
    fn drawer_saves_a_secret_to_the_keychain_only(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_settings(window, cx);
                ws.settings_on(None, Some("ACCOUNT_ID"), window, cx);
            });
            window.render_frame(cx);
            // Mark it secret, type a value, save.
            window.within("variable-drawer").click("secret-toggle", cx);
            let view = ws.read(cx).settings_view.clone().unwrap();
            let value = view.read(cx).drawer.as_ref().unwrap().value.clone();
            // Typed, as a person would: only an edited value is written.
            value.read(cx).focus_handle(cx).focus(window, cx);
            window.render_frame(cx);
            window.input("acct_123", cx);
        })
        .unwrap();
        cx.run_until_parked();
        let (edited, typed) = ws.read_with(cx, |ws, cx| {
            let view = ws.settings_view.clone().unwrap();
            let d = view.read(cx).drawer.as_ref().unwrap();
            (d.edited, d.value.read(cx).value().to_string())
        });
        assert_eq!((edited, typed.as_str()), (true, "acct_123"));
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("variable-drawer").click("drawer-save", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            secrets.get(&service, "ACCOUNT_ID").as_deref(),
            Some("acct_123")
        );
        let staging = std::fs::read_to_string(dir.path().join(".env.staging")).unwrap();
        assert!(!staging.contains("acct_123"), "{staging}");
        let meta = CollectionMeta::load(state.path(), dir.path());
        assert!(meta.secret_names.contains(&"ACCOUNT_ID".to_string()));
        ws.read_with(cx, |ws, cx| {
            assert!(ws.meta(cx).secret_names.contains(&"ACCOUNT_ID".to_string()));
        });
    }

    #[gpui_kit::test]
    fn keychain_to_file_asks_first(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_settings(window, cx);
                ws.settings_on(None, Some("API_TOKEN"), window, cx);
            });
            window.render_frame(cx);
            let view = ws.read(cx).settings_view.clone().unwrap();
            // Opening the drawer read the Keychain value into a masked field.
            let d = view.read(cx).drawer.as_ref().unwrap();
            assert!(d.secret && !d.revealed, "masked");
            let value = d.value.clone();
            assert_eq!(value.read(cx).value().as_ref(), "tok_live_1");
            window.within("variable-drawer").click("secret-toggle", cx);
            window.render_frame(cx);
            window.within("variable-drawer").click("drawer-save", cx);
            window.render_frame(cx);
            assert!(
                window.has_active_dialog(cx),
                "asks before writing plain text"
            );
        })
        .unwrap();
        // Nothing moved yet.
        assert_eq!(
            secrets.get(&service, "API_TOKEN").as_deref(),
            Some("tok_live_1")
        );
        cx.update_window(window, |_, window, cx| {
            let view = ws.read(cx).settings_view.clone().unwrap();
            window.close_dialog(cx);
            view.update(cx, |v, cx| v.save(Confirmed::Yes, window, cx));
        })
        .unwrap();
        let staging = std::fs::read_to_string(dir.path().join(".env.staging")).unwrap();
        assert!(staging.contains("API_TOKEN=tok_live_1"), "{staging}");
        assert_eq!(secrets.get(&service, "API_TOKEN"), None);
    }

    #[test]
    fn save_writes_only_edited_or_moving_values() {
        let staging = RowSource::File("/c/.env.staging".into());
        let default = RowSource::File("/c/.env".into());
        // Never-edited: nothing written, whatever the source.
        for source in [RowSource::Process, RowSource::NotSet, RowSource::Keychain] {
            assert_eq!(
                plan_value(true, &source, false),
                ValueWrite::Nothing,
                "{source:?}"
            );
        }
        assert_eq!(
            plan_value(false, &RowSource::NotSet, false),
            ValueWrite::Nothing
        );
        assert_eq!(plan_value(false, &staging, false), ValueWrite::Nothing);
        assert_eq!(
            plan_value(false, &RowSource::Process, false),
            ValueWrite::Nothing
        );
        // Edited or moving.
        assert_eq!(
            plan_value(true, &RowSource::Process, true),
            ValueWrite::Keychain
        );
        assert_eq!(plan_value(true, &staging, false), ValueWrite::Keychain);
        assert_eq!(plan_value(true, &default, false), ValueWrite::Keychain);
        assert_eq!(
            plan_value(false, &RowSource::Keychain, false),
            ValueWrite::KeychainToFile
        );
        assert_eq!(
            plan_value(false, &RowSource::NotSet, true),
            ValueWrite::File
        );
    }

    #[test]
    fn copies_text_lists_every_copy() {
        let p = |n: &str| std::path::PathBuf::from(format!("/c/{n}"));
        assert_eq!(
            copies_text(&[p(".env")]),
            "A copy remains in .env, which every environment reads. Remove it there too?"
        );
        assert_eq!(
            copies_text(&[p(".env.staging"), p(".env")]),
            "Copies remain in .env.staging and .env, which every environment reads. Remove them there too?"
        );
        assert_eq!(
            copies_text(&[p(".env.staging")]),
            "A copy remains in .env.staging, which the Staging environment reads. Remove it there too?"
        );
        assert_eq!(
            copies_text(&[p(".env.staging.local"), p(".env")]),
            "A copy remains in .env, which every environment reads. Remove it there too? .env.staging.local is your local override, so the studio leaves it; remove it there yourself."
        );
    }

    #[test]
    fn where_stored_follows_the_source() {
        let own = Path::new("/c/.env.staging");
        let svc = "curl-runner studio · Payments API · staging · 0a1b2c3d";
        assert_eq!(
            where_stored(
                &RowSource::File("/c/.env.staging.local".into()),
                true,
                own,
                svc
            ),
            "In plain text in .env.staging.local. The studio never edits .local files — remove it there yourself."
        );
        let text = |s: &RowSource, secret| where_stored(s, secret, own, svc);
        assert_eq!(
            text(&RowSource::Keychain, true),
            "Stored in macOS Keychain under “curl-runner studio · Payments API · staging · 0a1b2c3d”."
        );
        assert_eq!(
            text(&RowSource::Process, false),
            "Overridden by your shell environment. Edits are saved to .env.staging, but the shell value wins."
        );
        assert_eq!(
            text(&RowSource::File("/c/.env".into()), true),
            "In plain text in .env. Save moves it to the Keychain."
        );
        assert_eq!(
            text(&RowSource::NotSet, true),
            "Saved to the macOS Keychain under “curl-runner studio · Payments API · staging · 0a1b2c3d”."
        );
        assert_eq!(
            text(&RowSource::NotSet, false),
            "Saved to .env.staging in the collection folder."
        );
        assert_eq!(
            text(&RowSource::File(own.into()), false),
            "Stored in .env.staging in the collection folder."
        );
    }

    /// Opens the page, then the drawer on `var`.
    fn drawer_on(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        var: &str,
    ) -> Entity<CollectionSettings> {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_settings(window, cx);
                ws.settings_on(None, Some(var), window, cx);
            });
            window.render_frame(cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap())
    }

    fn save(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        view: &Entity<CollectionSettings>,
        how: Confirmed,
    ) -> bool {
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.save(how, window, cx));
            window.render_frame(cx);
            let asked = window.has_active_dialog(cx);
            window.close_dialog(cx);
            asked
        })
        .unwrap()
    }

    /// Marks `names` secret in the open collection's notes.
    fn mark(cx: &mut TestAppContext, ws: &Entity<Workspace>, names: &[&str]) {
        ws.update(cx, |ws, cx| {
            super::super::workspace::edit_notes(&ws.notes, cx, |meta| {
                for n in names {
                    meta.set_secret(n, true);
                }
            })
        });
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn secret_from_the_default_file_asks_about_the_copy(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        let env = dir.path().join(".env");
        std::fs::write(
            &env,
            "BASE_URL=https://api.example.com\nPLAIN_KEY=k1\nOTHER_KEY=k2\n",
        )
        .unwrap();
        mark(cx, &ws, &["PLAIN_KEY", "OTHER_KEY"]);
        // Remove: the Keychain gets it, `.env` loses it.
        let view = drawer_on(cx, window, &ws, "PLAIN_KEY");
        assert!(
            save(cx, window, &view, Confirmed::No),
            "asks about the copy"
        );
        assert_eq!(
            secrets.get(&service, "PLAIN_KEY"),
            None,
            "nothing before an answer"
        );
        let view = drawer_on(cx, window, &ws, "PLAIN_KEY");
        save(cx, window, &view, Confirmed::Yes);
        assert_eq!(secrets.get(&service, "PLAIN_KEY").as_deref(), Some("k1"));
        let text = std::fs::read_to_string(&env).unwrap();
        assert!(!text.contains("PLAIN_KEY"), "{text}");
        // Keep copy: the Keychain gets it, `.env` keeps it, the row warns.
        let view = drawer_on(cx, window, &ws, "OTHER_KEY");
        save(cx, window, &view, Confirmed::KeepCopy);
        assert_eq!(secrets.get(&service, "OTHER_KEY").as_deref(), Some("k2"));
        assert!(
            std::fs::read_to_string(&env)
                .unwrap()
                .contains("OTHER_KEY=k2")
        );
        let row = view.read_with(cx, |v, _| {
            v.rows()
                .iter()
                .find(|r| r.name == "OTHER_KEY")
                .cloned()
                .unwrap()
        });
        assert_eq!(row.source, RowSource::Keychain);
        assert_eq!(row.plain_copy, Some(env.clone()));
        assert!(shows(cx, window, "Masked · Plain copy remains in .env"));
    }

    #[gpui_kit::test]
    fn unedited_saves_write_no_value(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        mark(cx, &ws, &["SHELL_TOKEN"]);
        // A shell-sourced secret: Save leaves the Keychain alone.
        let view = drawer_on(cx, window, &ws, "SHELL_TOKEN");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_process_env(|k| (k == "SHELL_TOKEN").then(|| "from-shell".into()), cx);
                v.open_variable(Some("SHELL_TOKEN"), window, cx);
            });
        })
        .unwrap();
        let source = view.read_with(cx, |v, _| v.drawer.as_ref().unwrap().source.clone());
        assert_eq!(source, RowSource::Process);
        save(cx, window, &view, Confirmed::No);
        assert_eq!(secrets.get(&service, "SHELL_TOKEN"), None);
        // A Not set variable, description only: notes change, nothing else.
        let view = drawer_on(cx, window, &ws, "ACCOUNT_ID");
        cx.update_window(window, |_, window, cx| {
            let d = view.read(cx).drawer.as_ref().unwrap().description.clone();
            d.update(cx, |s, cx| {
                s.set_value("The account to test with.", window, cx)
            });
        })
        .unwrap();
        save(cx, window, &view, Confirmed::No);
        assert_eq!(secrets.get(&service, "ACCOUNT_ID"), None);
        let staging = std::fs::read_to_string(dir.path().join(".env.staging")).unwrap();
        assert!(!staging.contains("ACCOUNT_ID"), "{staging}");
        let meta = CollectionMeta::load(state.path(), dir.path());
        assert_eq!(
            meta.vars["ACCOUNT_ID"].description,
            "The account to test with."
        );
    }

    #[gpui_kit::test]
    fn secret_marks_follow_other_environments(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        std::fs::write(dir.path().join(".env.production"), "").unwrap();
        secrets
            .set(
                &service_for(state.path(), dir.path(), "production"),
                "API_TOKEN",
                "prod",
            )
            .unwrap();
        // Unmarked on Staging, it stays secret: Production keeps an item.
        let view = drawer_on(cx, window, &ws, "API_TOKEN");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.toggle_secret(window, cx));
        })
        .unwrap();
        save(cx, window, &view, Confirmed::Yes);
        let meta = CollectionMeta::load(state.path(), dir.path());
        assert!(meta.secret_names.contains(&"API_TOKEN".to_string()));
        // Deleted everywhere, the mark goes.
        let staging = dir.path().join(".env.staging");
        for env in ["staging", "production"] {
            cx.update_window(window, |_, window, cx| {
                view.update(cx, |v, cx| {
                    v.select_environment(env, cx);
                    v.delete("API_TOKEN", window, cx);
                });
            })
            .unwrap();
            let meta = CollectionMeta::load(state.path(), dir.path());
            let marked = meta.secret_names.contains(&"API_TOKEN".to_string());
            assert_eq!(marked, env == "staging", "after {env}");
        }
        assert!(
            !std::fs::read_to_string(staging)
                .unwrap()
                .contains("API_TOKEN")
        );
    }

    #[gpui_kit::test]
    fn moving_to_the_keychain_removes_every_plain_copy(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        let (env, staging) = (dir.path().join(".env"), dir.path().join(".env.staging"));
        std::fs::write(&env, "BASE_URL=https://api.example.com\nAPP_KEY=old\n").unwrap();
        std::fs::write(
            &staging,
            "BASE_URL=https://staging.api.example.com\nAPP_KEY=new\n",
        )
        .unwrap();
        mark(cx, &ws, &["APP_KEY"]);
        // Its source is `.env.staging`, but `.env` holds a copy too.
        let view = drawer_on(cx, window, &ws, "APP_KEY");
        assert!(
            save(cx, window, &view, Confirmed::No),
            "asks about both copies"
        );
        assert_eq!(secrets.get(&service, "APP_KEY"), None);
        let view = drawer_on(cx, window, &ws, "APP_KEY");
        save(cx, window, &view, Confirmed::Yes);
        assert_eq!(secrets.get(&service, "APP_KEY").as_deref(), Some("new"));
        for f in [&env, &staging] {
            let text = std::fs::read_to_string(f).unwrap();
            assert!(
                !text.contains("APP_KEY") && text.contains("BASE_URL"),
                "{text}"
            );
        }
    }

    #[gpui_kit::test]
    fn an_edited_shell_secret_asks_about_the_file_copy(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        let env = dir.path().join(".env");
        std::fs::write(
            &env,
            "BASE_URL=https://api.example.com\nSHELL_TOKEN=plain\n",
        )
        .unwrap();
        mark(cx, &ws, &["SHELL_TOKEN"]);
        let view = drawer_on(cx, window, &ws, "SHELL_TOKEN");
        let open_edited = |cx: &mut TestAppContext| {
            cx.update_window(window, |_, window, cx| {
                view.update(cx, |v, cx| {
                    v.set_process_env(|k| (k == "SHELL_TOKEN").then(|| "from-shell".into()), cx);
                    v.open_variable(Some("SHELL_TOKEN"), window, cx);
                    let d = v.drawer.as_mut().unwrap();
                    assert_eq!(d.source, RowSource::Process);
                    d.edited = true;
                    d.value.update(cx, |s, cx| s.set_value("typed", window, cx));
                });
            })
            .unwrap();
        };
        open_edited(cx);
        assert!(save(cx, window, &view, Confirmed::No), "asks about .env");
        assert_eq!(secrets.get(&service, "SHELL_TOKEN"), None);
        open_edited(cx);
        save(cx, window, &view, Confirmed::Yes);
        assert_eq!(
            secrets.get(&service, "SHELL_TOKEN").as_deref(),
            Some("typed")
        );
        assert!(
            !std::fs::read_to_string(&env)
                .unwrap()
                .contains("SHELL_TOKEN")
        );
    }

    #[cfg(unix)]
    #[gpui_kit::test]
    fn a_copy_that_cannot_be_removed_still_marks_the_secret(cx: &mut TestAppContext) {
        use std::os::unix::fs::PermissionsExt as _;
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let service = service_for(state.path(), dir.path(), "staging");
        std::fs::write(dir.path().join(".env"), "BASE_URL=x\nNEW_KEY=plain\n").unwrap();
        let view = drawer_on(cx, window, &ws, "NEW_KEY");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.toggle_secret(window, cx));
        })
        .unwrap();
        // The folder is read-only: `.env` can't be rewritten.
        let perms = |m| std::fs::Permissions::from_mode(m);
        std::fs::set_permissions(dir.path(), perms(0o555)).unwrap();
        save(cx, window, &view, Confirmed::Yes);
        std::fs::set_permissions(dir.path(), perms(0o755)).unwrap();
        assert_eq!(secrets.get(&service, "NEW_KEY").as_deref(), Some("plain"));
        let meta = CollectionMeta::load(state.path(), dir.path());
        assert!(meta.secret_names.contains(&"NEW_KEY".to_string()));
    }

    fn open_environments(cx: &mut TestAppContext, window: AnyWindowHandle) {
        cx.update_window(window, |_, window, cx| {
            window.dispatch_action(super::super::app::OpenEnvironments.boxed_clone(), cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn found(cx: &mut TestAppContext, window: AnyWindowHandle, id: &'static str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.try_find(id).is_some()
        })
        .unwrap()
    }

    #[gpui_kit::test]
    fn shared_by_rows_are_locked_and_only_for_received_copies(cx: &mut TestAppContext) {
        use crate::model::backend::SharedVar;
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, _) = open(cx, state.path());
        open_environments(cx, window);
        assert!(!shows(cx, window, "Shared by Acme"), "owned: no locks");
        assert!(!found(cx, window, "legend-shared"));
        cx.update_window(window, |_, window, cx| window.press("escape", cx))
            .unwrap();
        // A received copy, as adding a shared link leaves it.
        let var = |name: &str, secret: bool, default: Option<&str>| SharedVar {
            name: name.into(),
            description: String::new(),
            required: false,
            secret,
            default: default.map(String::from),
            locked: default.is_some(),
        };
        ws.update(cx, |ws, cx| {
            super::super::workspace::edit_notes(&ws.notes, cx, |meta| {
                meta.read_only_from = Some("Acme".into());
                meta.received_vars = vec![
                    var("CURRENCY", false, Some("eur")),
                    var("BASE_URL", false, Some("https://api.example.com/shipping")),
                    var("API_TOKEN", true, None),
                ];
            })
        });
        let saved = CollectionMeta::load(state.path(), dir.path());
        assert_eq!(saved.read_only_from.as_deref(), Some("Acme"), "saved");
        open_environments(cx, window);
        let rows = ws.read_with(cx, |ws, cx| {
            ws.settings_view.as_ref().unwrap().read(cx).rows().to_vec()
        });
        let currency = rows
            .iter()
            .position(|r| r.name == "CURRENCY")
            .expect("listed");
        assert_eq!(rows[currency].value.as_deref(), Some("eur"));
        let base_url = rows.iter().position(|r| r.name == "BASE_URL").unwrap();
        let label = |cx: &mut TestAppContext, id: &'static str, ix: usize| {
            cx.update_window(window, |_, window, cx| {
                window.render_frame(cx);
                window.find((id, ix)).label().map(String::from)
            })
            .unwrap()
        };
        assert_eq!(
            label(cx, "env-source", currency).as_deref(),
            Some("Shared by Acme")
        );
        assert_eq!(label(cx, "env-value", currency).as_deref(), Some("eur"));
        // Set locally too: the owner's value is locked over it, as a run
        // reads it.
        assert_eq!(
            label(cx, "env-source", base_url).as_deref(),
            Some("Shared by Acme")
        );
        assert_eq!(
            label(cx, "env-value", base_url).as_deref(),
            Some("https://api.example.com/shipping"),
            "the owner's, over the local file's"
        );
        assert!(
            shows(cx, window, "My keychain"),
            "a secret stays the customer's own"
        );
        assert!(found(cx, window, "legend-shared"));
        assert!(shows(
            cx,
            window,
            "Set by the collection owner, read only for others"
        ));
        // Locked: a click opens no drawer.
        cx.update_window(window, |_, window, cx| {
            window.click(("env-row", currency), cx)
        })
        .unwrap();
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        assert!(!view.read_with(cx, |v, _| v.drawer_open_on("CURRENCY")));
        assert!(view.read_with(cx, |v, _| v.status().summary.ends_with("1 missing")));
    }

    #[gpui_kit::test]
    fn suggested_rows_show_the_local_value_else_the_owners(cx: &mut TestAppContext) {
        use crate::model::backend::SharedVar;
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws, _) = open(cx, state.path());
        // Suggested (unlocked): BASE_URL is set locally, REGION is not.
        let var = |name: &str, default: &str| SharedVar {
            name: name.into(),
            description: String::new(),
            required: false,
            secret: false,
            default: Some(default.into()),
            locked: false,
        };
        ws.update(cx, |ws, cx| {
            super::super::workspace::edit_notes(&ws.notes, cx, |meta| {
                meta.read_only_from = Some("Acme".into());
                meta.received_vars = vec![
                    var("BASE_URL", "https://api.example.com/shipping"),
                    var("REGION", "eu"),
                ];
            })
        });
        open_environments(cx, window);
        let rows = ws.read_with(cx, |ws, cx| {
            ws.settings_view.as_ref().unwrap().read(cx).rows().to_vec()
        });
        let at = |name: &str| rows.iter().position(|r| r.name == name).unwrap();
        let label = |cx: &mut TestAppContext, id: &'static str, ix: usize| {
            cx.update_window(window, |_, window, cx| {
                window.render_frame(cx);
                window.find((id, ix)).label().map(String::from)
            })
            .unwrap()
        };
        let (base_url, region) = (at("BASE_URL"), at("REGION"));
        assert_eq!(
            label(cx, "env-source", base_url).as_deref(),
            Some(".env.staging"),
            "the local value wins"
        );
        assert_eq!(
            label(cx, "env-source", region).as_deref(),
            Some("Suggested by Acme")
        );
        assert_eq!(label(cx, "env-value", region).as_deref(), Some("eu"));
        assert!(!shows(cx, window, "Shared by Acme"), "nothing locked");
        // Not locked: a click opens the drawer to set their own.
        cx.update_window(window, |_, window, cx| {
            window.click(("env-row", region), cx)
        })
        .unwrap();
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        assert!(view.read_with(cx, |v, _| v.drawer_open_on("REGION")));
    }

    #[gpui_kit::test]
    fn a_drawer_save_keeps_the_owners_lock(cx: &mut TestAppContext) {
        use crate::model::backend::{SignInMethod, share_payload};
        let state = tempfile::tempdir().unwrap();
        let backend = super::super::workspace::test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (dir, window, ws, _) = open_with(cx, state.path(), backend.clone());
        open_environments(cx, window);
        cx.update_window(window, |_, window, cx| window.click("settings-sharing", cx))
            .unwrap();
        cx.run_until_parked();
        for id in ["share-default-BASE_URL", "share-lock-BASE_URL"] {
            cx.update_window(window, |_, window, cx| {
                window.render_frame(cx);
                window.click(id, cx)
            })
            .unwrap();
            cx.run_until_parked();
        }
        let locked = |cx: &mut TestAppContext| {
            ws.read_with(cx, |ws, cx| ws.meta(cx).vars["BASE_URL"].share_locked)
        };
        assert!(locked(cx));
        describe(cx, window, &ws, "BASE_URL", "Where Acme's API lives");
        let meta = ws.read_with(cx, |ws, cx| ws.meta(cx).clone());
        assert_eq!(meta.vars["BASE_URL"].description, "Where Acme's API lives");
        assert!(locked(cx), "the drawer keeps the lock");
        assert!(CollectionMeta::load(state.path(), dir.path()).vars["BASE_URL"].share_locked);
        // What the next variables call sends.
        let payload = share_payload(dir.path(), &meta);
        let base = payload.iter().find(|v| v.name == "BASE_URL").unwrap();
        assert!(base.locked && base.default.is_some(), "{base:?}");
    }

    #[gpui_kit::test]
    fn sharing_page_shares_like_the_dialog(cx: &mut TestAppContext) {
        use crate::model::backend::SignInMethod;
        let state = tempfile::tempdir().unwrap();
        let backend = super::super::workspace::test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (_dir, window, ws, _) = open_with(cx, state.path(), backend.clone());
        open_environments(cx, window);
        cx.update_window(window, |_, window, cx| window.click("settings-sharing", cx))
            .unwrap();
        cx.run_until_parked();
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        let share = view
            .read_with(cx, |v, _| v.share_view().cloned())
            .expect("the page shows the controls");
        assert!(found(cx, window, "sharing-page"));
        assert!(found(cx, window, "share-link"));
        assert!(
            !found(cx, window, "share-close"),
            "page form: no dialog chrome"
        );
        assert!(!found(cx, window, "share-default-API_TOKEN"));
        cx.update_window(window, |_, window, cx| {
            window.click("share-default-BASE_URL", cx)
        })
        .unwrap();
        cx.run_until_parked();
        assert!(share.read_with(cx, |s, cx| s.shares_default("BASE_URL", cx)));
        // The page's notes, the workspace's and the saved ones agree.
        assert!(view.read_with(cx, |v, cx| v.meta(cx).vars["BASE_URL"].share_default));
        assert!(ws.read_with(cx, |ws, cx| ws.meta(cx).vars["BASE_URL"].share_default));
        assert_eq!(ws.read_with(cx, |ws, _| ws.sync.people), Some(1));
        let saved = CollectionMeta::load(state.path(), _dir.path());
        assert!(saved.vars["BASE_URL"].share_default, "saved");
        assert!(
            saved.secret_names.contains(&"API_TOKEN".to_string()),
            "the rest kept"
        );
    }

    #[gpui_kit::test]
    fn sharing_page_signed_out_asks_to_sign_in(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (_dir, window, ws, _) = open(cx, state.path());
        open_environments(cx, window);
        cx.update_window(window, |_, window, cx| window.click("settings-sharing", cx))
            .unwrap();
        cx.run_until_parked();
        assert!(shows(cx, window, "Sign in to share this collection."));
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        assert!(view.read_with(cx, |v, _| v.share_view().is_none()));
    }

    #[gpui_kit::test]
    fn signing_out_drops_the_old_sessions_share(cx: &mut TestAppContext) {
        use crate::model::backend::SignInMethod;
        let state = tempfile::tempdir().unwrap();
        let backend = super::super::workspace::test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (_dir, window, ws, _) = open_with(cx, state.path(), backend.clone());
        open_environments(cx, window);
        cx.update_window(window, |_, window, cx| window.click("settings-sharing", cx))
            .unwrap();
        cx.run_until_parked();
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        assert_eq!(view.read_with(cx, |v, cx| v.sharing_count(cx)), Some(1));
        // Signed out from Settings: the workspace starts over.
        futures::executor::block_on(backend.sign_out()).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.start_sync(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |v, cx| v.sharing_count(cx)), None);
        assert!(view.read_with(cx, |v, _| v.share_view().is_none()));
        assert!(shows(cx, window, "Sign in to share this collection."));
    }

    /// How many times the workspace wrote the notes.
    fn saves(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> usize {
        ws.read_with(cx, |ws, _| ws.notes_saves)
    }

    /// The shown page's drawer on `name` (the page is not reopened), its
    /// description set to `text`, saved.
    fn describe(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        name: &str,
        text: &str,
    ) {
        if !ws.read_with(cx, |ws, _| ws.settings_shown()) {
            open_environments(cx, window);
        }
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_page(super::Page::Environments, window, cx);
                v.open_variable(Some(name), window, cx);
                let d = v.drawer.as_ref().unwrap();
                d.description
                    .update(cx, |s, cx| s.set_value(text, window, cx));
            })
        })
        .unwrap();
        assert!(!save(cx, window, &view, Confirmed::No), "nothing to ask");
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn an_import_on_the_settings_page_keeps_its_secret_marks(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, _) = open(cx, state.path());
        open_environments(cx, window);
        let before = ws.read_with(cx, |ws, cx| ws.meta(cx).secret_names.clone());
        // ⌘I with the settings page shown: a curl with a token.
        let curl = concat!(
            "curl -X POST https://api.example.com/v1/refunds -H 'X-Api-Key: rk_test",
            "_51HxAbCdEfGhIjKlMnOpQrSt'"
        );
        let saved = saves(cx, &ws);
        let dialog = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.import(window, cx).unwrap())
            })
            .unwrap();
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(curl, None, window, cx);
                d.save_secrets = false;
            });
            window.render_frame(cx);
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| d.import(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        let marked = ws.read_with(cx, |ws, cx| ws.meta(cx).secret_names.clone());
        let extracted: Vec<String> = marked
            .iter()
            .filter(|n| !before.contains(n))
            .cloned()
            .collect();
        assert_eq!(extracted.len(), 1, "the token's name is marked: {marked:?}");
        assert_eq!(saves(cx, &ws), saved + 1, "one write for the import");
        let token = extracted[0].clone();
        assert!(
            ws.read_with(cx, |ws, _| ws.settings_shown()),
            "still on the page"
        );
        // The page shows it at once (it reads the same notes).
        assert!(ws.read_with(cx, |ws, cx| {
            let view = ws.settings_view.as_ref().unwrap().read(cx);
            is_secret(&token, &view.meta(cx).secret_names)
        }));
        // Switching the page's environment, then a drawer save: the mark
        // stays, in memory and on disk.
        let view = ws.read_with(cx, |ws, _| ws.settings_view.clone().unwrap());
        let saved = saves(cx, &ws);
        view.update(cx, |v, cx| v.select_environment("Default", cx));
        cx.run_until_parked();
        assert_eq!(
            saves(cx, &ws),
            saved,
            "an environment switch writes nothing"
        );
        assert!(
            ws.read_with(cx, |ws, cx| ws.meta(cx).secret_names.contains(&token)),
            "kept through the switch"
        );
        describe(cx, window, &ws, "BASE_URL", "Where the API lives");
        assert_eq!(saves(cx, &ws), saved + 1, "one write for the drawer");
        let meta = ws.read_with(cx, |ws, cx| ws.meta(cx).clone());
        assert!(meta.secret_names.contains(&token), "{meta:?}");
        assert!(meta.secret_names.contains(&"API_TOKEN".to_string()));
        assert_eq!(meta.vars["BASE_URL"].description, "Where the API lives");
        assert_eq!(
            CollectionMeta::load(state.path(), dir.path()),
            meta,
            "saved"
        );
    }

    #[gpui_kit::test]
    fn marked_secret_while_its_tick_is_in_flight_shares_nothing(cx: &mut TestAppContext) {
        use crate::model::backend::SignInMethod;
        let state = tempfile::tempdir().unwrap();
        let backend = super::super::workspace::test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (dir, window, ws, _) = open_with(cx, state.path(), backend.clone());
        open_environments(cx, window);
        cx.update_window(window, |_, window, cx| window.click("settings-sharing", cx))
            .unwrap();
        cx.run_until_parked();
        let sync = ws.read_with(cx, |ws, _| ws.share_sync.clone().unwrap());
        let dev = backend.dev().unwrap();
        // Tick BASE_URL; its call is held in flight.
        dev.hold_updates(true);
        cx.update_window(window, |_, window, cx| {
            window.click("share-default-BASE_URL", cx)
        })
        .unwrap();
        cx.run_until_parked();
        assert!(sync.read_with(cx, |s, cx| s.shares_default("BASE_URL", cx)));
        // Meanwhile the drawer marks it secret (its plain copies kept).
        let saved = saves(cx, &ws);
        let view = drawer_on(cx, window, &ws, "BASE_URL");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.toggle_secret(window, cx))
        })
        .unwrap();
        save(cx, window, &view, Confirmed::KeepCopy);
        cx.run_until_parked();
        assert!(
            !sync.read_with(cx, |s, cx| s.shares_default("BASE_URL", cx)),
            "the tick went with the mark"
        );
        dev.hold_updates(false);
        cx.run_until_parked();
        // Nowhere: the notes, the saved notes, the share, the server.
        let meta = ws.read_with(cx, |ws, cx| ws.meta(cx).clone());
        assert!(is_secret("BASE_URL", &meta.secret_names));
        assert!(
            meta.vars.get("BASE_URL").is_none_or(|v| !v.share_default),
            "{meta:?}"
        );
        assert_eq!(
            CollectionMeta::load(state.path(), dir.path()),
            meta,
            "saved"
        );
        assert_eq!(saves(cx, &ws), saved + 1, "the drawer's write only");
        sync.read_with(cx, |s, cx| {
            assert!(!s.shares_default("BASE_URL", cx));
            assert!(!s.offers_share_default("BASE_URL", cx));
        });
        let key = ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        let server = futures::executor::block_on(backend.share_settings(&key)).unwrap();
        let base_url = server
            .variables
            .iter()
            .find(|v| v.name == "BASE_URL")
            .unwrap();
        assert!(
            base_url.secret && base_url.default.is_none(),
            "{base_url:?}"
        );
    }

    #[gpui_kit::test]
    fn each_change_writes_the_notes_once(cx: &mut TestAppContext) {
        use crate::model::backend::SignInMethod;
        let state = tempfile::tempdir().unwrap();
        let backend = super::super::workspace::test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (dir, window, ws, _) = open_with(cx, state.path(), backend.clone());
        let on_disk = || CollectionMeta::load(state.path(), dir.path());
        let start = saves(cx, &ws);
        describe(cx, window, &ws, "BASE_URL", "Where the API lives");
        assert_eq!(saves(cx, &ws), start + 1);
        assert_eq!(
            on_disk().vars["BASE_URL"].description,
            "Where the API lives"
        );
        // The same again changes nothing: nothing is written.
        describe(cx, window, &ws, "BASE_URL", "Where the API lives");
        assert_eq!(saves(cx, &ws), start + 1);
        // A tick on the Sharing page: one write once it is accepted.
        cx.update_window(window, |_, window, cx| window.click("settings-sharing", cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(
            saves(cx, &ws),
            start + 1,
            "opening the share writes nothing"
        );
        cx.update_window(window, |_, window, cx| {
            window.click("share-default-BASE_URL", cx)
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(saves(cx, &ws), start + 2);
        let saved = on_disk();
        assert!(saved.vars["BASE_URL"].share_default);
        assert_eq!(saved.vars["BASE_URL"].description, "Where the API lives");
        assert_eq!(ws.read_with(cx, |ws, cx| ws.meta(cx).clone()), saved);
    }
}
