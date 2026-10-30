//! The collection settings page (board: Environments): its own left nav,
//! the General page (name and folder, read only) and the Environments page:
//! one tab per environment, its variables with where each value comes from,
//! and the drawer that edits one. Secret values live in the Keychain only:
//! the table never shows them, and the drawer reads one when it opens.
//!
//! It takes the sidebar's, the main area's and the inspector's place. The
//! workspace re-reads what changed when the page emits [`SettingsEvent`].

use super::app::{CloseSettings, mono_font, ui_font};
use super::color;
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::run_view::pill;
use super::workspace::notify_error;
use crate::model::collection_meta::{CollectionMeta, VarMeta};
use crate::model::environments::{
    CapturedVar, DEFAULT_ENVIRONMENT, EnvRow, RowSource, UsageIndex, create_environment, env_file,
    environment_rows, file_readers, import_dotenv, is_secret, list_environments, move_to_file,
    move_to_keychain, name_in_env_files, remove_env_var, remove_env_var_at, write_env_var,
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

pub const NAV_WIDTH: f32 = 240.;
pub const DRAWER_WIDTH: f32 = 380.;
const ROW_HEIGHT: f32 = 40.;
/// The table's fixed columns.
const NAME_COL: f32 = 180.;
const SOURCE_COL: f32 = 150.;
const SCOPE_COL: f32 = 110.;
/// What a masked value reads as: never its length.
const MASK: &str = "••••••••••••";
/// The board's callout, verbatim.
const CALLOUT_HEAD: &str = "How customers bring their own values.";
const CALLOUT_BODY: &str = " When you share this collection, people get the variable names, descriptions and non-secret defaults. Secrets are never shared or synced: each person enters their own, and it stays in their device keychain. Values marked ";
const CALLOUT_SHARED: &str = "Shared by Acme";
const CALLOUT_TAIL: &str = " are locked for them.";
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    General,
    Environments,
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
    /// Into the Keychain, and out of this environment's own file.
    Keychain,
    /// Into the Keychain, out of a file this tab does not own (`.env` on
    /// Staging, a `.local` override): asks what to do with that copy.
    KeychainFrom(PathBuf),
    /// Into this environment's file.
    File,
    /// Out of the Keychain into this environment's file: asks first.
    KeychainToFile,
}

/// The write a Save makes. A value is written only when typed into, or
/// when it moves between the file and the Keychain: a never-edited value
/// is never written (an empty Keychain item would count as set, and win
/// over the shell).
pub fn plan_value(secret: bool, source: &RowSource, edited: bool, own_file: &Path) -> ValueWrite {
    match (secret, source) {
        (true, RowSource::File(p)) if p == own_file => ValueWrite::Keychain,
        (true, RowSource::File(p)) => ValueWrite::KeychainFrom(p.clone()),
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
    /// Where the collection's notes live (`None` in tests that keep nothing).
    base: Option<PathBuf>,
    secrets: Arc<CachedSecrets>,
    environments: Vec<String>,
    environment: String,
    meta: CollectionMeta,
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
        base: Option<PathBuf>,
        secrets: Arc<CachedSecrets>,
        environment: String,
        captured: Vec<CapturedVar>,
        cx: &mut Context<Self>,
    ) -> Self {
        let meta = base
            .as_deref()
            .map(|b| CollectionMeta::load(b, &root))
            .unwrap_or_default();
        // Reading every request file is the slow part: off the main thread.
        let scan_root = root.clone();
        let scan = cx.background_spawn(async move { UsageIndex::scan(&scan_root) });
        let scan = cx.spawn(async move |this, cx| {
            let usage = scan.await;
            this.update(cx, |this, cx| {
                this.usage = Arc::new(usage);
                this.reload_rows();
                cx.emit(SettingsEvent::Status);
                cx.notify();
            })
            .ok();
        });
        let mut this = CollectionSettings {
            environments: list_environments(&root),
            root,
            collection,
            base,
            secrets,
            environment,
            meta,
            keychain: Vec::new(),
            keychain_locked: false,
            usage: Arc::default(),
            captured,
            process_env: std::rc::Rc::new(|k: &str| std::env::var(k).ok()),
            rows: Vec::new(),
            page: Page::Environments,
            drawer: None,
            focus: cx.focus_handle(),
            _scan: scan,
        };
        if !this.environments.contains(&this.environment) {
            this.environment = DEFAULT_ENVIRONMENT.to_string();
        }
        this.reload();
        this
    }

    fn service(&self) -> String {
        service_name(&self.collection, &self.environment)
    }

    /// Re-reads the Keychain's item names and the rows.
    fn reload(&mut self) {
        match self.secrets.try_names(&self.service()) {
            Ok(names) => self.keychain = names,
            Err(e) => {
                log::warn!("Couldn't list Keychain items: {e}");
                self.keychain.clear();
                self.keychain_locked = true;
            }
        }
        self.reload_rows();
    }

    fn reload_rows(&mut self) {
        self.rows = environment_rows(
            &self.root,
            &self.environment,
            &self.meta.secret_names,
            &self.keychain,
            &self.usage.referenced(),
            &self.captured,
            &*self.process_env,
        );
    }

    #[cfg(test)]
    pub fn set_process_env(&mut self, env: impl Fn(&str) -> Option<String> + 'static) {
        self.process_env = std::rc::Rc::new(env);
        self.reload_rows();
    }

    #[cfg(test)]
    pub fn rows(&self) -> &[EnvRow] {
        &self.rows
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

    /// The collection's notes as the page last saved them.
    pub fn meta(&self) -> &CollectionMeta {
        &self.meta
    }

    pub fn select_environment(&mut self, name: &str, cx: &mut Context<Self>) {
        if self.environment == name {
            return;
        }
        self.environment = name.to_string();
        self.drawer = None;
        self.reload();
        cx.emit(SettingsEvent::Changed);
        cx.notify();
    }

    fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        self.drawer = None;
        cx.notify();
    }

    /// Esc: the drawer first, then the page.
    fn escape(&mut self, cx: &mut Context<Self>) {
        if self.drawer.take().is_some() {
            cx.notify();
        } else {
            cx.emit(SettingsEvent::Close);
        }
    }

    // The drawer.

    /// Opens the drawer on `name` (its row), or on a new variable. A
    /// Keychain value is read here, once, through the cached store.
    pub fn open_variable(
        &mut self,
        name: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let row = name
            .and_then(|n| self.rows.iter().find(|r| r.name == n))
            .cloned();
        let (value, read_error) = match &row {
            Some(r) => self.read_value(r),
            None => (String::new(), None),
        };
        let secret = match &row {
            Some(r) => r.secret,
            None => name.is_some_and(|n| is_secret(n, &self.meta.secret_names)),
        };
        let notes = name
            .and_then(|n| self.meta.vars.get(n))
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
    fn read_value(&mut self, row: &EnvRow) -> (String, Option<String>) {
        match &row.source {
            RowSource::Keychain => match self.secrets.try_get(&self.service(), &row.name) {
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
            RowSource::Process => (row.value.clone().unwrap_or_default(), None),
            RowSource::Captured { .. } | RowSource::NotSet => (String::new(), None),
        }
    }

    fn drawer_name(&self, cx: &App) -> Option<String> {
        let d = self.drawer.as_ref()?;
        Some(match &d.name {
            Some(n) => n.clone(),
            None => d.name_input.read(cx).value().trim().to_string(),
        })
    }

    fn toggle_reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = &mut self.drawer else { return };
        d.revealed = !d.revealed;
        let masked = d.secret && !d.revealed;
        d.value.update(cx, |s, cx| s.set_masked(masked, window, cx));
        cx.notify();
    }

    fn toggle_secret(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn toggle_required(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = &mut self.drawer {
            d.required = !d.required;
            cx.notify();
        }
    }

    /// The file environment writes go to, as the confirmation names it.
    fn file_name(&self) -> String {
        env_file(&self.root, &self.environment)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| ".env".into())
    }

    fn own_file(&self) -> PathBuf {
        env_file(&self.root, &self.environment).unwrap_or_else(|_| self.root.join(".env"))
    }

    /// The other environments with a Keychain item for `name` (names only).
    fn keychain_elsewhere(&self, name: &str) -> Vec<String> {
        self.environments
            .iter()
            .filter(|e| **e != self.environment)
            .filter(|e| {
                let service = service_name(&self.collection, e);
                // Unreadable counts as holding it: the mark stays.
                self.secrets
                    .try_names(&service)
                    .map_or(true, |n| n.iter().any(|n| n == name))
            })
            .map(|e| env_label(e))
            .collect()
    }

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
        let plan = plan_value(secret, &d.source, d.edited, &own_file);
        let unmarking = !secret && self.meta.secret_names.contains(&name);
        let elsewhere = if unmarking {
            self.keychain_elsewhere(&name)
        } else {
            Vec::new()
        };
        if confirmed == Confirmed::No {
            match &plan {
                ValueWrite::KeychainFrom(p) => {
                    self.confirm_copy(p.clone(), window, cx);
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
        let (root, env, service) = (self.root.clone(), self.environment.clone(), self.service());
        let secrets: &dyn SecretStore = &*self.secrets;
        let written = match &plan {
            ValueWrite::Nothing => Ok(()),
            ValueWrite::Keychain => move_to_keychain(&root, &env, &name, &value, secrets, &service),
            ValueWrite::KeychainFrom(p) => {
                move_to_keychain(&root, &env, &name, &value, secrets, &service).and_then(|()| {
                    match confirmed {
                        Confirmed::Yes => remove_env_var_at(&root, p, &name),
                        _ => Ok(()),
                    }
                })
            }
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
        // Unmarked here, a name other environments keep in the Keychain
        // stays secret collection-wide.
        if secret || elsewhere.is_empty() {
            self.meta.set_secret(&name, secret);
        }
        let notes = VarMeta {
            description,
            required_for_shared: required,
        };
        if notes == VarMeta::default() {
            self.meta.vars.remove(&name);
        } else {
            self.meta.vars.insert(name.clone(), notes);
        }
        self.save_meta(window, cx);
        self.drawer = None;
        self.reload();
        cx.emit(SettingsEvent::Changed);
        window.push_notification(Notification::success(format!("Saved {name}")), cx);
        self.focus.focus(window, cx);
        cx.notify();
    }

    fn save_meta(&self, window: &mut Window, cx: &mut App) {
        if let Some(base) = &self.base
            && let Err(e) = self.meta.save(base, &self.root)
        {
            notify_error(format!("Couldn't save the variable notes: {e}"), window, cx);
        }
    }

    /// Before a Keychain value goes into a file, or a name other
    /// environments keep secret is unmarked.
    fn confirm_plain_text(
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

    /// Before a value moves into the Keychain from a file this tab does
    /// not own: Remove the copy there, Keep it, or Cancel.
    fn confirm_copy(&self, file: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let file_name = file
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let readers = match file_readers(&file_name).as_deref() {
            Some("") | None => "every environment reads".to_string(),
            Some(env) => format!("the {} environment reads", env_label(env)),
        };
        // A `.local` override is the person's own: never written.
        let removable = !file_name.ends_with(".local");
        let description = if removable {
            format!("A copy remains in {file_name}, which {readers}. Remove it there too?")
        } else {
            format!(
                "A copy remains in {file_name}, which {readers}. It is your local override, so the studio leaves it."
            )
        };
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

    fn confirm_delete(&self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn delete(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let service = self.service();
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
                    .try_names(&service_name(&self.collection, e))
                    .map_or(true, |n| n.iter().any(|n| n == name))
        });
        if !left && self.meta.secret_names.iter().any(|n| n == name) {
            self.meta.set_secret(name, false);
            self.save_meta(window, cx);
        }
        self.drawer = None;
        self.reload();
        cx.emit(SettingsEvent::Changed);
        cx.notify();
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
        let service = self.service();
        let marked = self.meta.secret_names.clone();
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
        self.reload();
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
            })
            .children(drawer)
    }
}

/// An element the smoke tests can find.
type Observed = gpui_kit::base::ObservedElement<Stateful<Div>>;

/// The board's `.btn` shape at 28px, without colors.
fn button_base(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Observed {
    let label = label.into();
    div()
        .id(id)
        .role(Role::Button)
        .test_support()
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(10.))
        .border_1()
        .rounded(px(6.))
        .text_size(px(12.))
        .whitespace_nowrap()
        .cursor_pointer()
}

/// The board's `.btn.sm`: a 28px outlined button.
fn small_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Observed {
    button_base(id, label)
        .border_color(color("border-200"))
        .bg(color("bg-100"))
        .text_color(color("text-primary"))
        .hover(|s| s.bg(color("surface-100")))
}

/// The board's `.btn.sm.pri`.
fn primary_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Observed {
    button_base(id, label)
        .border_color(color("primary"))
        .bg(color("primary"))
        .text_color(color("on-primary"))
        .font_weight(FontWeight::MEDIUM)
        .hover(|s| s.bg(color("primary-hover")))
}

/// A tinted pill: `tone` text, a 30% border and an 8% fill.
fn tinted_pill(tone: &str) -> Div {
    pill(("text-secondary", "", "border-200"))
        .font_weight(FontWeight::NORMAL)
        .text_color(color(tone))
        .border_color(color(tone).opacity(0.3))
        .bg(color(tone).opacity(0.08))
}

fn neutral_pill() -> Div {
    pill(("text-secondary", "", "border-200")).font_weight(FontWeight::NORMAL)
}

/// The secret key glyph in `size`.
fn key_icon(size: f32) -> impl IntoElement {
    glyph(icons::KEY).size(px(size))
}

/// The board's 28×16 switch.
fn switch(on: bool, enabled: bool) -> Div {
    let knob = div()
        .absolute()
        .top(px(2.))
        .size(px(12.))
        .rounded_full()
        .map(|k| if on { k.right(px(2.)) } else { k.left(px(2.)) })
        .bg(color(if on { "bg-100" } else { "text-secondary" }));
    div()
        .relative()
        .flex_none()
        .w(px(28.))
        .h(px(16.))
        .mt(px(2.))
        .rounded_full()
        .bg(color(if on { "primary" } else { "surface-300" }))
        .when(!enabled, |d| d.opacity(0.5))
        .child(knob)
}

fn label_text(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color("text-secondary"))
        .child(text.into())
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
        let soon = || {
            div()
                .flex_none()
                .px(px(6.))
                .rounded_full()
                .border_1()
                .border_color(color("border-200"))
                .text_size(px(11.))
                .text_color(color("text-disabled"))
                .child("Soon")
        };
        let item = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .role(Role::Button)
                .test_support()
                .aria_label(label)
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.))
                .h(px(28.))
                .px(px(8.))
                .rounded(px(6.))
                .text_size(px(13.))
                .whitespace_nowrap()
                .child(div().flex_1().child(label))
        };
        let page_item =
            |id: &'static str, label: &'static str, page: Page, cx: &mut Context<Self>| {
                let on = self.page == page;
                item(id, label)
                    .cursor_pointer()
                    .map(|d| {
                        if on {
                            d.bg(color("surface-300")).text_color(color("text-primary"))
                        } else {
                            d.text_color(color("text-secondary"))
                                .hover(|s| s.bg(color("surface-100")))
                        }
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.set_page(page, cx)))
            };
        let disabled = |id: &'static str, label: &'static str| {
            item(id, label)
                .text_color(color("text-disabled"))
                .child(soon())
        };
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
                div()
                    .id("settings-back")
                    .role(Role::Button)
                    .test_support()
                    .aria_label("Back to workspace")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .pt(px(4.))
                    .pb(px(12.))
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .cursor_pointer()
                    .hover(|s| s.text_color(color("text-primary")))
                    .child(glyph(icons::CHEVRON_LEFT).size(px(12.)))
                    .child("Back to workspace")
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
            .child(disabled("settings-sharing", "Sharing"))
            .child(section("Workspace", false, false))
            .child(disabled("settings-members", "Members"))
            .child(disabled("settings-sync", "Sync & hosting"))
            .child(disabled("settings-billing", "Billing"))
            .child(section("App", true, false))
            .child(disabled("settings-appearance", "Appearance"))
            .child(disabled("settings-editor", "Editor"))
            .child(disabled("settings-shortcuts", "Keyboard shortcuts"))
            .child(disabled("settings-updates", "Updates"))
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
                    .child(render_legend()),
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
            .child(
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
            .child(
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
        let clickable = !matches!(r.source, RowSource::Captured { .. });
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

    fn render_drawer(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let d = self.drawer.as_ref()?;
        let name = self.drawer_name(cx).unwrap_or_default();
        let secret = d.secret || name.starts_with("SECRET_");
        let prefix_secret = name.starts_with("SECRET_");
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
        let where_stored = where_stored(&d.source, secret, &self.own_file(), &self.service());
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
                    .child(Textarea::new(&d.description)),
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
            .when(d.name.is_some(), |f| {
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
                        !prefix_secret,
                        "Secret",
                        "Masked in the UI, logs and exported reports. Never leaves this device.",
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_secret(window, cx))),
                )
                .child(
                    toggle(
                        "required-toggle",
                        d.required,
                        true,
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

fn render_legend() -> impl IntoElement {
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
        .child("Resolution order: captured → my keychain → environment → collection default")
}

#[cfg(test)]
mod tests {
    use super::{
        CollectionMeta, CollectionSettings, Confirmed, EnvRow, Path, RowSource, SecretStore,
        ValueWrite, captured_text, plan_value, service_name, source_label, summary_text,
        used_by_text, where_stored,
    };
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
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        secrets
            .set(&service_name(&name, "staging"), "API_TOKEN", "tok_live_1")
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
                cx.new(|cx| Workspace::new(Some(root), store, shared, window, cx))
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
    fn drawer_saves_a_secret_to_the_keychain_only(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let service = service_name(&name, "staging");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_settings(window, cx);
                ws.dev_settings(None, Some("ACCOUNT_ID"), window, cx);
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
        ws.read_with(cx, |ws, _| {
            assert!(ws.meta.secret_names.contains(&"ACCOUNT_ID".to_string()));
        });
    }

    #[gpui_kit::test]
    fn keychain_to_file_asks_first(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let service = service_name(&name, "staging");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_settings(window, cx);
                ws.dev_settings(None, Some("API_TOKEN"), window, cx);
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
        let own = Path::new("/c/.env.staging");
        let default = RowSource::File("/c/.env".into());
        let staging = RowSource::File(own.into());
        // Never-edited: nothing written, whatever the source.
        for source in [RowSource::Process, RowSource::NotSet, RowSource::Keychain] {
            assert_eq!(
                plan_value(true, &source, false, own),
                ValueWrite::Nothing,
                "{source:?}"
            );
        }
        assert_eq!(
            plan_value(false, &RowSource::NotSet, false, own),
            ValueWrite::Nothing
        );
        assert_eq!(plan_value(false, &staging, false, own), ValueWrite::Nothing);
        assert_eq!(
            plan_value(false, &RowSource::Process, false, own),
            ValueWrite::Nothing
        );
        // Edited or moving.
        assert_eq!(
            plan_value(true, &RowSource::Process, true, own),
            ValueWrite::Keychain
        );
        assert_eq!(plan_value(true, &staging, false, own), ValueWrite::Keychain);
        assert_eq!(
            plan_value(true, &default, false, own),
            ValueWrite::KeychainFrom("/c/.env".into())
        );
        assert_eq!(
            plan_value(false, &RowSource::Keychain, false, own),
            ValueWrite::KeychainToFile
        );
        assert_eq!(
            plan_value(false, &RowSource::NotSet, true, own),
            ValueWrite::File
        );
    }

    #[test]
    fn where_stored_follows_the_source() {
        let own = Path::new("/c/.env.staging");
        let svc = "curl-runner studio · Payments API · staging";
        let text = |s: &RowSource, secret| where_stored(s, secret, own, svc);
        assert_eq!(
            text(&RowSource::Keychain, true),
            "Stored in macOS Keychain under “curl-runner studio · Payments API · staging”."
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
            "Saved to the macOS Keychain under “curl-runner studio · Payments API · staging”."
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
                ws.dev_settings(None, Some(var), window, cx);
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

    fn mark(state: &Path, root: &Path, names: &[&str]) {
        let mut meta = CollectionMeta::load(state, root);
        for n in names {
            meta.set_secret(n, true);
        }
        meta.save(state, root).unwrap();
    }

    #[gpui_kit::test]
    fn secret_from_the_default_file_asks_about_the_copy(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (dir, window, ws, secrets) = open(cx, state.path());
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let service = service_name(&name, "staging");
        let env = dir.path().join(".env");
        std::fs::write(
            &env,
            "BASE_URL=https://api.example.com\nPLAIN_KEY=k1\nOTHER_KEY=k2\n",
        )
        .unwrap();
        mark(state.path(), dir.path(), &["PLAIN_KEY", "OTHER_KEY"]);
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
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let service = service_name(&name, "staging");
        mark(state.path(), dir.path(), &["SHELL_TOKEN"]);
        // A shell-sourced secret: Save leaves the Keychain alone.
        let view = drawer_on(cx, window, &ws, "SHELL_TOKEN");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_process_env(|k| (k == "SHELL_TOKEN").then(|| "from-shell".into()));
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
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        std::fs::write(dir.path().join(".env.production"), "").unwrap();
        secrets
            .set(&service_name(&name, "production"), "API_TOKEN", "prod")
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
}
