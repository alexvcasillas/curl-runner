//! A received (shared-with-me) collection, board ReadOnly: every write
//! refused with `Read only · shared by {org}`, `Duplicate to my workspace`
//! for an editable copy, and updates from the owner: checked when the copy
//! opens, every [`UPDATE_EVERY`] and on `Check for updates`, offered in the
//! banner ([`super::update_banner`]) and applied whole on **Update**.
//!
//! The lock itself sits in the model: [`Collection`] refuses to create,
//! rename or delete files and each [`Document`] refuses edits, saves and
//! fixes. The workspace checks first, so the user gets the toast instead of
//! an error.
//!
//! [`Collection`]: crate::model::collection::Collection
//! [`Document`]: crate::model::document::Document

use super::backend_call;
use super::update_banner::{UPDATE_EVERY, UpdateOffer};
use super::workspace::{Workspace, edit_notes, notify_error};
use crate::model::backend::{
    BackendError, LinkAccess, ReceivedShare, SharedCollection, SharedVar, SyncFile,
};
use crate::model::collection::read_only_text;
use crate::model::environments::{DEFAULT_ENVIRONMENT, resolve_vars};
use crate::model::history::relative_time;
use crate::model::received::ReceivedCopy;
use crate::model::secrets::service_name;
use crate::model::shared_copy::{
    STATE_FOLDER_TEXT, UpdatePlan, apply_update, duplicate, holds_studio_state, plan_update,
    received_from, secret_in_either,
};
use crate::model::sync_base::SyncBase;
use curl_runner_core::interpolate::Store;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// What a duplicate says once it opens.
pub const DUPLICATED: &str = "Duplicated — enter your own values in Environments";

/// `3 days ago`, lowercase: `updated yesterday`.
pub fn updated_ago(
    at: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let text = relative_time(
        at.with_timezone(&chrono::Local),
        now.with_timezone(&chrono::Local),
    );
    let mut c = text.chars();
    c.next()
        .map_or_else(String::new, |f| f.to_lowercase().chain(c).collect())
}

/// `Check for updates` while a check of the open copy is in flight.
pub const CHECKING: &str = "Checking…";

/// What a manual check says when the copy holds the latest version.
pub const UP_TO_DATE: &str = "Up to date";

/// What a check found.
enum Checked {
    UpToDate,
    /// Newer, and the banner already offers that version.
    Offered,
    Newer(UpdateOffer),
}

enum CheckFailed {
    Backend(BackendError),
    /// The new version can't be written here (a path the studio refuses).
    Plan(String),
}

impl Workspace {
    /// The org a received copy came from, while one is open: it is read
    /// only.
    pub(super) fn read_only_org(&self) -> Option<&str> {
        self.state.collection.as_ref()?.read_only.as_deref()
    }

    /// For a received copy, shows `Read only · shared by {org}` and returns
    /// `true`: the caller does nothing.
    pub(super) fn refuse_read_only(&mut self, window: &mut Window, cx: &mut App) -> bool {
        let Some(org) = self.read_only_org().map(str::to_string) else {
            return false;
        };
        self.toast_read_only(&org, window, cx);
        true
    }

    /// For a folder picked to write into (an import target, a new
    /// collection) that is a received copy, shows `Read only · shared by
    /// {org}` (or, for the studio's own folder, [`STATE_FOLDER_TEXT`]) and
    /// returns `true`: nothing is written.
    pub(super) fn refuse_received(
        &mut self,
        dir: &Path,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        match self.refusal_of(dir) {
            Some(text) => {
                self.toast(text, window, cx);
                true
            }
            None => false,
        }
    }

    /// Why a folder picked by hand can't take files: a received copy (or a
    /// folder in one), or the studio's own data folder or one holding it.
    fn refusal_of(&self, dir: &Path) -> Option<String> {
        let base = self.store.base();
        // Without the server there are no received copies to protect.
        let received = received_from(base, dir).filter(|_| crate::features::SERVER);
        if let Some(org) = received {
            Some(read_only_text(&org))
        } else if holds_studio_state(base, dir) {
            Some(STATE_FOLDER_TEXT.to_string())
        } else {
            None
        }
    }

    /// [`refuse_read_only`] or [`refuse_received`] for writing into
    /// `dir`: the refusal's text, toasted, or `None` to go ahead.
    ///
    /// [`refuse_read_only`]: Workspace::refuse_read_only
    /// [`refuse_received`]: Workspace::refuse_received
    pub(super) fn refuse_write_into(
        &mut self,
        dir: &Path,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let text = self
            .read_only_org()
            .map(read_only_text)
            .or_else(|| self.refusal_of(dir))?;
        self.toast(text.clone(), window, cx);
        Some(text)
    }

    fn toast_read_only(&mut self, org: &str, window: &mut Window, cx: &mut App) {
        self.toast(read_only_text(org), window, cx);
    }

    fn toast(&mut self, text: String, window: &mut Window, cx: &mut App) {
        #[cfg(test)]
        self.toasts.push(text.clone());
        window.push_notification(Notification::warning(text), cx);
    }

    /// The open collection as the boards name it: a received copy by its
    /// shared name (`Shipping API`), not its folder's.
    pub(super) fn collection_title(&self) -> Option<String> {
        let c = self.state.collection.as_ref()?;
        Some(match &self.received {
            Some(r) if c.read_only.is_some() => r.share.name.clone(),
            _ => c.name.clone(),
        })
    }

    /// Where a received copy's exports start: the user's Documents (else
    /// home), never the copy's own folder. `None`: the collection root.
    pub(super) fn export_dir(&self) -> Option<PathBuf> {
        self.read_only_org()?;
        let home = std::env::home_dir()?;
        let documents = home.join("Documents");
        Some(if documents.is_dir() { documents } else { home })
    }

    /// `updated 3 days ago`: when the owner last changed the open copy.
    pub(super) fn received_updated(&self, cx: &App) -> Option<String> {
        let at = self.meta(cx).received_updated?;
        Some(updated_ago(at, chrono::Utc::now()))
    }

    /// `Check for updates` (the sidebar's foot, the note): a manual check
    /// of the open copy, which says `Up to date` or why it couldn't ask.
    pub(super) fn check_for_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only_org().is_none() {
            return;
        }
        let Some(token) = self.received.as_ref().map(|r| r.share.link.token.clone()) else {
            // A copy with no share record (removed on the server, or a
            // folder under shared/ added by hand) has nowhere to ask.
            let text = "Couldn't check for updates: this copy has no share to ask.".to_string();
            #[cfg(test)]
            self.toasts.push(text.clone());
            window.push_notification(Notification::warning(text), cx);
            return;
        };
        self.check_received(&token, true, window, cx);
    }

    /// Every [`UPDATE_EVERY`] while the window is open, a quiet check of
    /// every received copy, each offer kept by its token (the banner shows
    /// it once that copy is open). A tick that fails says nothing; the
    /// next one asks again.
    pub(super) fn start_update_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !crate::features::SERVER {
            return;
        }
        let timer = cx.background_executor().clone();
        self._update_timer = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                timer.timer(UPDATE_EVERY).await;
                let ticked =
                    this.update_in(cx, |this, window, cx| this.check_all_received(window, cx));
                if ticked.is_err() {
                    break;
                }
            }
        }));
    }

    /// Every received copy, checked quietly (the 15-minute tick).
    pub(super) fn check_all_received(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_received_all();
        let tokens: Vec<String> = self
            .received_all
            .iter()
            .map(|r| r.share.link.token.clone())
            .collect();
        for token in tokens {
            self.check_received(&token, false, window, cx);
        }
    }

    /// A check of the open copy is in flight (its `Check for updates`
    /// reads `Checking…`).
    pub(super) fn checking_open(&self) -> bool {
        self.received
            .as_ref()
            .is_some_and(|r| self.update_checks.contains_key(&r.share.link.token))
    }

    /// The open copy, checked quietly as it opens.
    pub(super) fn check_open_received(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(token) = self.received.as_ref().map(|r| r.share.link.token.clone()) {
            self.check_received(&token, false, window, cx);
        }
    }

    /// Asks the copy's server for its latest version; only a newer one is
    /// fetched and planned against the copy, and offered in the banner.
    /// Nothing is written. An automatic check (`manual` false) says nothing
    /// on any error: the next one asks again. A manual one says `Up to
    /// date` or why it couldn't ask. One check per copy at a time: a manual
    /// ask during an automatic check of the same copy makes that check's
    /// answer speak.
    pub(super) fn check_received(
        &mut self,
        token: &str,
        manual: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(copy) = self
            .received_all
            .iter()
            .find(|r| r.share.link.token == token)
            .cloned()
        else {
            return;
        };
        if self.update_checks.contains_key(token) {
            if manual {
                self.manual_checks.insert(token.to_string());
            }
            return;
        }
        let offered = self.update_offers.get(token).map(|o| o.version);
        let backend = self.backend.clone();
        let (share, root) = (copy.share, copy.root);
        let (link, held) = (share.link.clone(), share.version);
        let job = cx.background_spawn(async move {
            let latest = backend
                .latest_version(&link)
                .await
                .map_err(CheckFailed::Backend)?;
            if latest <= held {
                return Ok(Checked::UpToDate);
            }
            if offered == Some(latest) {
                return Ok(Checked::Offered);
            }
            let snapshot = backend
                .fetch_shared(&link)
                .await
                .map_err(CheckFailed::Backend)?;
            let plan = plan_update(&root, snapshot.files).map_err(CheckFailed::Plan)?;
            Ok(Checked::Newer(UpdateOffer::new(
                snapshot.version,
                snapshot.published_at,
                plan,
                snapshot.variables,
            )))
        });
        let key = token.to_string();
        let token = key.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let checked = job.await;
            this.update_in(cx, |this, window, cx| {
                this.update_checks.remove(&token);
                let manual = this.manual_checks.remove(&token) || manual;
                this.update_checked(&token, &share.team, checked, manual, window, cx);
                cx.notify();
            })
            .ok();
        });
        self.update_checks.insert(key, task);
        cx.notify();
    }

    fn update_checked(
        &mut self,
        token: &str,
        org: &str,
        checked: Result<Checked, CheckFailed>,
        manual: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match checked {
            Ok(Checked::UpToDate) => {
                self.update_offers.remove(token);
                if manual {
                    #[cfg(test)]
                    self.toasts.push(UP_TO_DATE.into());
                    window.push_notification(Notification::info(UP_TO_DATE), cx);
                }
            }
            Ok(Checked::Offered) => {}
            Ok(Checked::Newer(offer)) => {
                self.update_offers.insert(token.to_string(), offer);
            }
            // Quiet, whatever went wrong: the next tick asks again.
            Err(_) if !manual => {}
            Err(e) => {
                let why = match e {
                    CheckFailed::Backend(e @ BackendError::Offline) => e.to_string(),
                    CheckFailed::Backend(e) => e.link_text(org),
                    CheckFailed::Plan(why) => why,
                };
                let text = format!("Couldn't check for updates: {why}");
                #[cfg(test)]
                self.toasts.push(text.clone());
                notify_error(text, window, cx);
            }
        }
    }

    /// **Update**: the open copy takes the offered version, all or nothing
    /// (a hidden folder swapped in: on failure the copy is as it was and
    /// the offer stays). Saved secrets and `.env` values stay; the record
    /// takes the new version and file hashes; inputs the version newly
    /// requires are asked for in the Credentials step. Only the copy the
    /// offer is for, and only while it is open.
    pub(super) fn apply_offer(&mut self, token: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(copy) = self
            .received
            .clone()
            .filter(|r| r.share.link.token == token)
        else {
            return;
        };
        let Some(offer) = self.update_offers.get(token).cloned() else {
            return;
        };
        let org = copy.share.team.clone();
        // Planned again: the copy may have changed since the check.
        let plan = match plan_update(&copy.root, offer.plan.files.clone()) {
            Ok(plan) => plan,
            Err(e) => return notify_error(e, window, cx),
        };
        if let Err(e) = apply_update(&copy.root, &plan) {
            notify_error(e, window, cx);
            // Nothing changed; the tree shows what is there all the same.
            self.disk_changed(&touched(&plan), window, cx);
            return;
        }
        self.update_offers.remove(token);
        // The folder is a new one now: a watcher on the old one (inotify
        // follows the inode) would miss every change from here on.
        self.watch(&copy.root, window, cx);
        let hashes = || {
            plan.files
                .iter()
                .map(|f| (f.rel.clone(), SyncFile::hash_of(&f.text)))
        };
        let share = ReceivedShare {
            version: offer.version,
            files: hashes().collect(),
            ..copy.share.clone()
        };
        // The copy holds it now, whatever the record's write does.
        for r in self.received_all.iter_mut().chain(self.received.as_mut()) {
            if r.share.link.token == token {
                r.share = share.clone();
            }
        }
        edit_notes(&self.notes, cx, |meta| {
            meta.received_updated = Some(offer.published_at);
        });
        // Synced as of now: nothing shows as waiting. The base goes through
        // the sync's saver, so its writes stay in order.
        let base = SyncBase {
            files: hashes().collect(),
            ..SyncBase::default()
        };
        self.synced_as(base, window, cx);
        self.disk_changed(&touched(&plan), window, cx);
        let asked = self.take_shared_vars(offer.variables.clone(), cx);
        let text = format!("Updated to version {} from {org}", offer.version);
        #[cfg(test)]
        self.toasts.push(text.clone());
        window.push_notification(Notification::success(text), cx);
        let call = backend_call(cx, &self.backend, move |b| b.update_received(share));
        cx.spawn_in(window, async move |this, cx| {
            let recorded = call.await;
            this.update_in(cx, |this, window, cx| match recorded {
                Ok(()) => this.refresh_received_all(),
                Err(e) => notify_error(
                    format!("Updated, but the new version wasn't recorded: {e}"),
                    window,
                    cx,
                ),
            })
            .ok();
        })
        .detach();
        if !asked.is_empty() {
            self.ask_new_inputs(&copy, &offer, asked, window, cx);
        }
        cx.notify();
    }

    /// The new required inputs, in the Add flow's Credentials step: saved
    /// into the copy as an Add saves them.
    fn ask_new_inputs(
        &mut self,
        copy: &ReceivedCopy,
        offer: &UpdateOffer,
        vars: Vec<SharedVar>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(c) = &self.state.collection else {
            return;
        };
        let service = service_name(&c.name, DEFAULT_ENVIRONMENT, &self.meta(cx).id);
        let shared = SharedCollection {
            link: copy.share.link.clone(),
            owner_org: copy.share.team.clone(),
            team_slug: String::new(),
            name: copy.share.name.clone(),
            summary: String::new(),
            updated: offer.published_at,
            request_count: offer.plan.files.len(),
            version: offer.version,
            access: LinkAccess::Public,
            variables: vars,
        };
        let root = copy.root.clone();
        let view = self.open_shared(None, window, cx);
        view.update(cx, |v, cx| v.ask_inputs(shared, root, service, window, cx));
    }

    /// A received copy takes its owner's variables as shared now (on
    /// Update): through the notes (the workspace's one writer, which saves
    /// them and redoes the status bar's missing count), with new names'
    /// descriptions and secret marks. A variable secret before, or in the
    /// new version, stays secret. The newly required variables with no
    /// value here yet: the caller asks for them.
    pub(super) fn take_shared_vars(
        &mut self,
        vars: Vec<SharedVar>,
        cx: &mut Context<Self>,
    ) -> Vec<SharedVar> {
        let before = self.meta(cx).received_vars.clone();
        let marked: Vec<SharedVar> = self
            .meta(cx)
            .secret_names
            .iter()
            .map(|name| SharedVar {
                name: name.clone(),
                description: String::new(),
                required: false,
                secret: true,
                default: None,
                locked: false,
            })
            .collect();
        let vars = secret_in_either(secret_in_either(vars, &before), &marked);
        let newly_required: Vec<SharedVar> = vars
            .iter()
            .filter(|v| v.required && !before.iter().any(|b| b.name == v.name && b.required))
            .filter(|v| !(v.locked && v.default.is_some() && !v.secret))
            .cloned()
            .collect();
        if before != vars {
            edit_notes(&self.notes, cx, |meta| {
                for v in &vars {
                    if v.secret {
                        meta.set_secret(&v.name, true);
                    }
                    let notes = meta.vars.entry(v.name.clone()).or_default();
                    notes.description = v.description.clone();
                    notes.required_for_shared = v.required;
                }
                meta.received_vars = vars;
            });
        }
        let Some(c) = &self.state.collection else {
            return Vec::new();
        };
        let ctx = self.env_ctx(c, cx);
        let names: Vec<String> = newly_required.iter().map(|v| v.name.clone()).collect();
        let unset: Vec<String> =
            resolve_vars(&names, &Store::default(), &*self.secret_cache, &ctx, &[])
                .into_iter()
                .filter(|r| r.value.is_none())
                .map(|r| r.name)
                .collect();
        newly_required
            .into_iter()
            .filter(|v| unset.contains(&v.name))
            .collect()
    }

    /// `Duplicate to my workspace`: a folder picker, then [`duplicate_to`].
    ///
    /// [`duplicate_to`]: Workspace::duplicate_to
    pub(super) fn duplicate_to_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only_org().is_none() {
            return;
        }
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Duplicate here".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(dest) = paths.into_iter().next() else {
                return;
            };
            this.update_in(cx, |this, window, cx| this.duplicate_to(&dest, window, cx))
                .ok();
        })
        .detach();
    }

    /// Copies the open received copy's YAML into `dest` (no `.env`, no
    /// Keychain value; secret names and descriptions come along), then
    /// opens `dest` as an ordinary, editable collection.
    pub(super) fn duplicate_to(
        &mut self,
        dest: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(c) = self
            .state
            .collection
            .as_ref()
            .filter(|c| c.read_only.is_some())
        else {
            return;
        };
        let from = c.root.clone();
        let meta = self.meta(cx).clone();
        if let Err(e) = duplicate(self.store.base(), &from, &meta, dest) {
            return notify_error(e, window, cx);
        }
        let dest: PathBuf = dest.to_path_buf();
        if self.open_collection(&dest, window, cx) {
            self.remember_folder(&dest);
            #[cfg(test)]
            self.toasts.push(DUPLICATED.into());
            window.push_notification(Notification::success(DUPLICATED), cx);
        }
    }

    /// Opens another collection from the sidebar (Shared with me, My
    /// collections), once unsaved changes are settled.
    pub(super) fn switch_to(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_secrets();
        self.settle_unsaved(
            Rc::new(move |this, window, cx| {
                if this.open_collection(&root, window, cx) {
                    this.remember_folder(&root);
                }
            }),
            window,
            cx,
        );
    }
}

/// The paths an update wrote or removed.
fn touched(plan: &UpdatePlan) -> Vec<PathBuf> {
    plan.added
        .iter()
        .chain(&plan.changed)
        .chain(&plan.removed)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::updated_ago;

    #[test]
    fn copy_reads_like_the_board() {
        let now = chrono::Utc::now();
        assert_eq!(
            updated_ago(now - chrono::Duration::days(3), now),
            "3 days ago"
        );
    }
}

/// Server behaviour: only in builds with the `server` feature.
#[cfg(all(test, feature = "server"))]
mod gpui_tests {
    use super::super::app::{Import, NewRequest, QuickFix, RunActive, SaveFile};
    use super::super::open_shared::Step;
    use super::super::palette::PaletteAction;
    use super::super::state::StateStore;
    use super::super::update_banner::UPDATE_EVERY;
    use super::super::workspace::edit_notes;
    use super::super::workspace::{Workspace, test_backend, wait_for_run};
    use super::{CHECKING, DUPLICATED, STATE_FOLDER_TEXT, UP_TO_DATE};
    use crate::model::backend::{
        Backend, SharedSnapshot, SharedVar, SyncFile, fixtures, sync_files,
    };
    use crate::model::backend::{CollectionKey, ShareUpdate};
    use crate::model::collection_meta::CollectionMeta;
    use crate::model::secrets::{MemorySecrets, SecretStore, service_name};
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    const LINK: &str =
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm";
    const TOKEN: &str = "sk-live-customer-4242";
    const RATES: &str = "rates/get-rates.yaml";

    use crate::model::backend::testing::Spy;

    struct Studio {
        state: tempfile::TempDir,
        /// The copy's folder under the state dir.
        folder: &'static str,
        secrets: Arc<MemorySecrets>,
        spy: Arc<Spy>,
        window: AnyWindowHandle,
        ws: Entity<Workspace>,
    }

    impl Studio {
        fn root(&self) -> PathBuf {
            self.state.path().join(self.folder)
        }

        fn read(&self, rel: &str) -> String {
            std::fs::read_to_string(self.root().join(rel)).unwrap()
        }
    }

    /// A studio with Acme's Shipping API added from its link (the Add
    /// flow, `BASE_URL` as given) and open: a received, read-only copy.
    fn received(cx: &mut TestAppContext, signed_in: bool, base_url: Option<&str>) -> Studio {
        let inner = test_backend();
        if signed_in {
            crate::model::backend::testing::sign_in(&*inner);
        }
        let mut fill = vec![("API_TOKEN", TOKEN)];
        if let Some(url) = base_url {
            fill.insert(0, ("BASE_URL", url));
        }
        added(cx, inner, LINK, &fill, "shared/acme-shipping-api")
    }

    /// A studio with the share `link` added (its fields filled from
    /// `fill`) into `folder` and open.
    fn added(
        cx: &mut TestAppContext,
        inner: Arc<dyn Backend>,
        link: &str,
        fill: &[(&str, &str)],
        folder: &'static str,
    ) -> Studio {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let secrets = Arc::new(MemorySecrets::default());
        let spy = Arc::new(Spy::new(inner));
        let (s, b) = (
            secrets.clone() as Arc<dyn SecretStore>,
            spy.clone() as Arc<dyn Backend>,
        );
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
                cx.new(|cx| Workspace::new(None, store, s, b, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        add_link(cx, window, &ws, link, fill);
        let s = Studio {
            state,
            folder,
            secrets,
            spy,
            window,
            ws,
        };
        s.ws.read_with(cx, |ws, _| {
            let c = ws.state.collection.as_ref().expect("the copy is open");
            assert_eq!(c.root, s.root());
            assert_eq!(c.read_only.as_deref(), Some("Acme"));
        });
        s
    }

    /// The Add flow for `link`, its fields filled from `fill`.
    fn add_link(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        link: &str,
        fill: &[(&str, &str)],
    ) {
        let view = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.open_shared(None, window, cx))
            })
            .unwrap();
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.paste(link, window, cx));
            window.render_frame(cx);
            window.click("open-shared-open", cx);
        })
        .unwrap();
        cx.run_until_parked();
        click(cx, window, "open-shared-continue");
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                for (name, value) in fill {
                    v.fill(name, value, window, cx);
                }
            });
        })
        .unwrap();
        click(cx, window, "open-shared-add");
    }

    fn click(cx: &mut TestAppContext, window: AnyWindowHandle, id: &str) {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click(id.to_string(), cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn open_file(cx: &mut TestAppContext, s: &Studio, rel: &str) {
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_file(Path::new(rel), window, cx));
            window.render_frame(cx);
        })
        .unwrap();
    }

    fn dispatch(cx: &mut TestAppContext, s: &Studio, action: &dyn gpui_kit::Action) {
        let action = action.boxed_clone();
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.dispatch_action(action, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn toasts(cx: &mut TestAppContext, s: &Studio) -> Vec<String> {
        s.ws.read_with(cx, |ws, _| ws.toasts.clone())
    }

    fn dialog_open(cx: &mut TestAppContext, s: &Studio) -> bool {
        cx.update_window(s.window, |_, window, cx| window.has_active_dialog(cx))
            .unwrap()
    }

    /// Every YAML file of the copy, as on disk.
    fn snapshot(s: &Studio) -> Vec<SyncFile> {
        sync_files(&s.root())
    }

    #[gpui_kit::test]
    fn read_only_blocks_every_write_path(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        // A file with a quick fix, as the owner could have shared it.
        let typo = "request:\n  url: https://api.example.com\n  timout: 5000\n";
        std::fs::write(s.root().join("rates/typo.yaml"), typo).unwrap();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.disk_changed(&["rates/typo.yaml".into()], window, cx)
            });
        })
        .unwrap();
        let before = snapshot(&s);
        let refused = "Read only · shared by Acme";
        let mut expected = 0;
        let mut refused_once = |cx: &mut TestAppContext, what: &str| {
            expected += 1;
            let told = toasts(cx, &s);
            assert_eq!(told.len(), expected, "{what}: {told:?}");
            assert_eq!(told.last().map(String::as_str), Some(refused), "{what}");
            assert!(!dialog_open(cx, &s), "{what} opened a dialog");
        };

        // Typing into the editor.
        open_file(cx, &s, RATES);
        let rates = s.read(RATES);
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.input("# mine\n", cx);
        })
        .unwrap();
        cx.run_until_parked();
        s.ws.read_with(cx, |ws, cx| {
            let doc = &ws.state.open[0];
            assert_eq!(doc.text, rates);
            assert!(!doc.is_dirty());
            assert_eq!(ws.tabs[0].editor.read(cx).value().as_ref(), rates);
        });

        // ⌘S.
        dispatch(cx, &s, &SaveFile);
        refused_once(cx, "⌘S");

        // Rename and delete, as the tree's menu would ask.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.prompt_rename(RATES.into(), window, cx));
        })
        .unwrap();
        refused_once(cx, "rename");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.prompt_delete("rates".into(), window, cx));
        })
        .unwrap();
        refused_once(cx, "delete");

        // Quick fix (⌘.), and the editor offers no code action.
        open_file(cx, &s, "rates/typo.yaml");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let ix = ws.state.active.unwrap();
                ws.tabs[ix].editor.update(cx, |e, cx| {
                    e.set_cursor_position(
                        gpui_kit::component::input::Position::new(2, 4),
                        window,
                        cx,
                    )
                });
                let editor = ws.tabs[ix].editor.clone();
                let at = typo.find("timout").unwrap() + 2;
                assert!(ws.code_actions(&editor, at..at, cx).is_empty());
            });
        })
        .unwrap();
        dispatch(cx, &s, &QuickFix);
        refused_once(cx, "quick fix");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let ix = ws.state.active.unwrap();
                let fix = ws.state.open[ix].diagnostics[0].fix.clone().unwrap();
                let path = ws.state.open[ix].diagnostics[0].path.clone();
                ws.apply_named_fix(ix, &path, &fix, window, cx);
            });
        })
        .unwrap();
        refused_once(cx, "the popover's fix");

        // New request: ⌘N and the palette (which doesn't list it).
        dispatch(cx, &s, &NewRequest);
        refused_once(cx, "⌘N");
        let palette = cx
            .update_window(s.window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.open_palette(window, cx))
            })
            .unwrap();
        palette.read_with(cx, |p, _| {
            assert!(!p.lists(PaletteAction::NewRequest));
            assert!(!p.lists(PaletteAction::Import));
            assert!(p.lists(PaletteAction::Environments));
        });
        cx.update_window(s.window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        cx.run_until_parked();

        // Import (⌘I).
        dispatch(cx, &s, &Import);
        refused_once(cx, "import");

        // An edit on disk reloads (the owner's copy wins); Keep mine is
        // refused and changes nothing.
        let theirs = rates.replace("Get rates", "Get quotes");
        std::fs::write(s.root().join(RATES), &theirs).unwrap();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.disk_changed(&[RATES.into()], window, cx);
                let ix = ws.state.position(Path::new(RATES)).unwrap();
                ws.activate_tab(ix, window, cx);
                assert!(!ws.state.open[ix].disk_conflict);
                assert_eq!(ws.state.open[ix].text, theirs);
                ws.resolve_conflict(true, window, cx);
            });
        })
        .unwrap();
        refused_once(cx, "keep mine");

        // The model refuses even when asked directly.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let ix = ws.state.active.unwrap();
                ws.state.open[ix].set_text("mine\n".into());
                assert!(!ws.save(ix, window, cx));
                let c = ws.state.collection.as_mut().unwrap();
                assert!(c.create_file(Path::new("x.yaml"), "x").is_err());
                assert!(c.delete(Path::new("rates")).is_err());
            });
        })
        .unwrap();
        assert_eq!(s.read(RATES), theirs, "nothing was saved");
        let after = snapshot(&s);
        let changed: Vec<_> = after
            .iter()
            .filter(|f| !before.contains(f))
            .map(|f| f.rel.clone())
            .collect();
        assert_eq!(changed, [PathBuf::from(RATES)], "only the disk edit");
        assert_eq!(after.len(), before.len());
        // Typing said nothing (the editor took no change); the direct
        // save above was the ninth refusal.
        assert_eq!(toasts(cx, &s).len(), 9);
    }

    #[gpui_kit::test]
    fn read_only_view_reads_like_the_board(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        let label = |cx: &mut TestAppContext, id: &str| {
            cx.update_window(s.window, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find(id.to_string())
                    .and_then(|e| e.label().map(str::to_string))
            })
            .unwrap()
        };
        assert_eq!(
            label(cx, "read-only-note").as_deref(),
            Some("Read only · shared by Acme · updated 3 days ago")
        );
        assert_eq!(
            label(cx, "title-read-only").as_deref(),
            Some("Read only · Acme")
        );
        assert_eq!(label(cx, "sidebar-title").as_deref(), Some("Shipping API"));
        assert_eq!(
            label(cx, "sidebar-updated").as_deref(),
            Some("Updated 3 days ago")
        );
        assert_eq!(
            label(cx, "status-language").as_deref(),
            Some("YAML · read only")
        );
        assert!(label(cx, "sidebar-new-request").is_none());
        // Back in a collection of the user's own: Shared with me lists it.
        let mine = tempfile::tempdir().unwrap();
        std::fs::write(mine.path().join("a.yaml"), "request:\n  url: https://a\n").unwrap();
        let ws = s.ws.clone();
        let path = mine.path().to_path_buf();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(&path, window, cx));
                ws.remember_folder(&path);
            });
        })
        .unwrap();
        assert_eq!(
            label(cx, "shared-with-me").as_deref(),
            Some("Shared with me")
        );
        assert_eq!(
            label(cx, "other-collection-0").as_deref(),
            Some("Shipping API")
        );
        assert!(label(cx, "read-only-note").is_none());
        click(cx, s.window, "other-collection-0");
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, s.root());
        });
        // And from the copy, My collections lists the user's own.
        assert_eq!(
            label(cx, "my-collections").as_deref(),
            Some("My collections")
        );
        let name = mine
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(label(cx, "other-collection-0"), Some(name));
    }

    #[gpui_kit::test]
    fn run_works_in_read_only(cx: &mut TestAppContext) {
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = testkit::TestServer::with_handler(move |req| {
            let auth = req
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            log.lock().unwrap().push(auth);
            testkit::Response::from_data(br#"{"rates":[{"carrier":"ground"}]}"#.to_vec())
                .with_header("Content-Type", "application/json")
        });
        let s = received(cx, false, Some(&srv.base_url));
        open_file(cx, &s, RATES);
        let rates = s.read(RATES);
        dispatch(cx, &s, &RunActive);
        wait_for_run(cx, &s.ws);
        // The customer's own token, from their keychain.
        assert_eq!(*seen.lock().unwrap(), [format!("Bearer {TOKEN}")]);
        s.ws.read_with(cx, |ws, _| {
            let run = &ws.runs.last[Path::new(RATES)];
            // The fixture passes core's checks (status, body.rates.length).
            assert!(run.results[&0].success, "{:?}", run.results[&0].error);
            assert_eq!(run.keychain_names, ["API_TOKEN"]);
        });
        // The board's summary line, as drawn.
        let summary = cx
            .update_window(s.window, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find("inspector-summary")
                    .and_then(|e| e.label().map(str::to_string))
            })
            .unwrap();
        assert_eq!(
            summary.as_deref(),
            Some("2 of 2 checks passed · ran with your API_TOKEN")
        );
        // The run saved nothing and asked nothing.
        assert_eq!(s.read(RATES), rates);
        assert!(toasts(cx, &s).is_empty(), "{:?}", toasts(cx, &s));
        // Copy as CLI: the locked CURRENCY leads (BASE_URL is set in the
        // copy's .env, so the CLI reads it there); never the token.
        let entries = crate::model::history::History::open(s.state.path(), &s.root()).list();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_history(window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(entries.len(), 1);
        let view = s.ws.read_with(cx, |ws, _| ws.history_view.clone()).unwrap();
        view.update(cx, |v, cx| {
            v.select(&entries[0].id, cx);
            v.copy_cli(cx);
        });
        let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
        assert!(copied.contains("CURRENCY=usd curl-runner "), "{copied}");
        assert!(!copied.contains("BASE_URL="), "{copied}");
        assert!(!copied.contains(TOKEN), "{copied}");
    }

    /// The owner publishes `version`, `ago`, with `files` and `vars`.
    fn publish(
        s: &Studio,
        version: u32,
        ago: chrono::Duration,
        files: Vec<SyncFile>,
        vars: Vec<SharedVar>,
    ) {
        *s.spy.newer.lock().unwrap() = Some(SharedSnapshot {
            files,
            variables: vars,
            version,
            published_at: chrono::Utc::now() - ago,
        });
    }

    /// The fixture with three requests changed: one edited, one added,
    /// one removed.
    fn three_changed() -> Vec<SyncFile> {
        let mut files: Vec<SyncFile> = fixtures::shipping_snapshot()
            .into_iter()
            .filter(|f| f.rel != Path::new("rates/validate-address.yaml"))
            .map(|f| {
                if f.rel == Path::new(RATES) {
                    SyncFile::new(f.rel, "request:\n  url: https://v14/rates\n")
                } else {
                    f
                }
            })
            .collect();
        files.push(SyncFile::new(
            "labels/new.yaml",
            "request:\n  url: https://v14/labels\n",
        ));
        files
    }

    /// A quiet check (`manual` false, as on open and each tick) or
    /// `Check for updates`.
    fn check(cx: &mut TestAppContext, s: &Studio, manual: bool) {
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                if manual {
                    ws.check_for_updates(window, cx)
                } else {
                    ws.check_open_received(window, cx)
                }
            });
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn label(cx: &mut TestAppContext, s: &Studio, id: &str) -> Option<String> {
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window
                .try_find(id.to_string())
                .and_then(|e| e.label().map(str::to_string))
        })
        .unwrap()
    }

    fn fetches(s: &Studio) -> usize {
        s.spy
            .dev()
            .unwrap()
            .recorded()
            .iter()
            .filter(|c| c.starts_with("fetch_shared("))
            .count()
    }

    #[gpui_kit::test]
    fn newer_version_shows_banner_with_counts(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        let before = snapshot(&s);
        publish(
            &s,
            14,
            chrono::Duration::hours(2),
            three_changed(),
            fixtures::shipping_variables(),
        );
        check(cx, &s, false);
        assert_eq!(
            label(cx, &s, "update-banner-text").as_deref(),
            Some("Version 14 available · published 2 hours ago · 3 requests changed")
        );
        assert_eq!(
            label(cx, &s, "update-banner-update").as_deref(),
            Some("Update")
        );
        // It takes the read-only note's place.
        assert!(label(cx, &s, "read-only-note").is_none());
        // Offered, not written; nothing asked.
        assert_eq!(snapshot(&s), before);
        assert!(!dialog_open(cx, &s));
        assert!(toasts(cx, &s).is_empty(), "{:?}", toasts(cx, &s));
        // The same version again: offered already, not fetched again.
        let fetched = fetches(&s);
        check(cx, &s, false);
        assert_eq!(fetches(&s), fetched);
        // What changed: the plan's files, still nothing written.
        click(cx, s.window, "update-banner-what-changed");
        assert!(dialog_open(cx, &s));
        let rows: Vec<_> = (0..4)
            .filter_map(|i| label(cx, &s, &format!("what-changed-row-{i}")))
            .collect();
        assert_eq!(
            rows,
            [
                "Added labels/new.yaml",
                "Changed rates/get-rates.yaml",
                "Removed rates/validate-address.yaml"
            ]
        );
        click(cx, s.window, "what-changed-close");
        assert!(!dialog_open(cx, &s));
        assert_eq!(snapshot(&s), before);
    }

    #[gpui_kit::test]
    fn same_version_shows_nothing(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        let fetched = fetches(&s);
        check(cx, &s, false);
        assert!(label(cx, &s, "update-banner").is_none());
        assert!(label(cx, &s, "read-only-note").is_some());
        assert!(toasts(cx, &s).is_empty(), "{:?}", toasts(cx, &s));
        assert_eq!(fetches(&s), fetched, "no snapshot for the same version");
        // Asked by hand: it says so.
        check(cx, &s, true);
        assert_eq!(toasts(cx, &s), [UP_TO_DATE]);
        assert!(label(cx, &s, "update-banner").is_none());
        assert_eq!(fetches(&s), fetched);
    }

    #[gpui_kit::test]
    fn offline_auto_check_is_silent_manual_check_says_offline(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        publish(
            &s,
            14,
            chrono::Duration::hours(2),
            three_changed(),
            fixtures::shipping_variables(),
        );
        let dev = s.spy.dev().unwrap();
        dev.set_offline(true);
        check(cx, &s, false);
        assert!(label(cx, &s, "update-banner").is_none(), "no banner");
        assert!(toasts(cx, &s).is_empty(), "no toast: {:?}", toasts(cx, &s));
        // The next tick asks again, and finds it.
        dev.set_offline(false);
        cx.executor().advance_clock(UPDATE_EVERY);
        cx.run_until_parked();
        assert!(label(cx, &s, "update-banner").is_some());
        assert!(toasts(cx, &s).is_empty());
        // By hand, offline says so.
        dev.set_offline(true);
        check(cx, &s, true);
        assert_eq!(
            toasts(cx, &s),
            ["Couldn't check for updates: You're offline."]
        );
    }

    #[gpui_kit::test]
    fn update_applies_and_records_version(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let owners = snapshot(&s);
        let env = std::fs::read_to_string(s.root().join(".env")).unwrap();
        // Out of date: two files changed, one gone, one extra.
        std::fs::write(s.root().join(RATES), "request:\n  url: https://old\n").unwrap();
        std::fs::write(
            s.root().join("rates/list-carriers.yaml"),
            "request:\n  url: https://old\n",
        )
        .unwrap();
        std::fs::remove_file(s.root().join("rates/validate-address.yaml")).unwrap();
        std::fs::write(
            s.root().join("rates/old.yaml"),
            "request:\n  url: https://x\n",
        )
        .unwrap();
        open_file(cx, &s, RATES);
        let ago = chrono::Duration::hours(5);
        publish(&s, 4, ago, owners.clone(), fixtures::shipping_variables());
        let published = s.spy.newer.lock().unwrap().as_ref().unwrap().published_at;
        check(cx, &s, true);
        assert_eq!(
            label(cx, &s, "update-banner-text").as_deref(),
            Some("Version 4 available · published 5 hours ago · 4 requests changed")
        );
        assert!(
            s.root().join("rates/old.yaml").exists(),
            "nothing written yet"
        );
        let watched = s.ws.read_with(cx, |ws, _| ws.watched.len());
        click(cx, s.window, "update-banner-update");
        assert!(!dialog_open(cx, &s), "no input asked: all are set");
        // The watcher follows the swapped-in folder.
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.watched.len(), watched + 1);
            assert_eq!(ws.watched.last(), Some(&s.root()));
        });
        assert_eq!(snapshot(&s), owners, "the owner's files, exactly");
        assert!(!s.root().join("rates/old.yaml").exists());
        // Values carry over: the .env and the Keychain token.
        assert_eq!(std::fs::read_to_string(s.root().join(".env")).unwrap(), env);
        let id = s.ws.read_with(cx, |ws, cx| ws.meta(cx).id.clone());
        let service = service_name("acme-shipping-api", "Default", &id);
        assert_eq!(s.secrets.get(&service, "API_TOKEN").as_deref(), Some(TOKEN));
        // Nothing hidden left beside the copy.
        let beside: Vec<_> = std::fs::read_dir(s.root().parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(beside, ["acme-shipping-api"]);
        // Recorded: the version and each file's hash as received.
        let record = s.spy.received().into_iter().next().unwrap();
        assert_eq!(record.version, 4);
        let hashes: std::collections::BTreeMap<_, _> =
            owners.iter().map(|f| (f.rel.clone(), f.hash)).collect();
        assert_eq!(record.files, hashes);
        assert!(label(cx, &s, "update-banner").is_none(), "the banner went");
        assert_eq!(
            toasts(cx, &s).last().map(String::as_str),
            Some("Updated to version 4 from Acme")
        );
        s.ws.read_with(cx, |ws, cx| {
            // The open tab took the new text; the tree has the old file gone.
            let ix = ws.state.position(Path::new(RATES)).unwrap();
            let theirs = owners.iter().find(|f| f.rel == Path::new(RATES)).unwrap();
            assert_eq!(ws.state.open[ix].text, theirs.text);
            assert!(ws.method_of(Path::new("rates/old.yaml")).is_none());
            assert_eq!(ws.meta(cx).received_updated, Some(published));
            assert_eq!(ws.received.as_ref().unwrap().share.version, 4);
        });
        let base = crate::model::sync_base::load(Some(s.state.path()), &id);
        assert_eq!(base.files.len(), owners.len());
        // Again: up to date.
        check(cx, &s, true);
        assert_eq!(toasts(cx, &s).last().map(String::as_str), Some(UP_TO_DATE));
    }

    #[gpui_kit::test]
    fn update_with_new_required_input_asks_for_it(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        let mut vars = fixtures::shipping_variables();
        let var = |name: &str, secret| SharedVar {
            name: name.into(),
            description: format!("About {name}."),
            required: true,
            secret,
            default: None,
            locked: false,
        };
        vars.push(var("API_REGION", false));
        vars.push(var("SIGNING_KEY", true));
        publish(&s, 4, chrono::Duration::hours(1), three_changed(), vars);
        check(cx, &s, false);
        click(cx, s.window, "update-banner-update");
        // Only what's new and unset: API_TOKEN carried over.
        assert!(dialog_open(cx, &s));
        let view = s.ws.read_with(cx, |ws, _| ws.open_shared_view()).unwrap();
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Credentials);
        assert_eq!(
            view.read_with(cx, |v, _| v.fields()),
            [
                ("API_REGION".to_string(), false),
                ("SIGNING_KEY".to_string(), true)
            ]
        );
        cx.update_window(s.window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.fill("API_REGION", "eu-west", window, cx);
                v.fill("SIGNING_KEY", "sig-secret-9", window, cx);
            });
        })
        .unwrap();
        click(cx, s.window, "open-shared-save");
        assert!(!dialog_open(cx, &s));
        let env = std::fs::read_to_string(s.root().join(".env")).unwrap();
        assert!(env.contains("API_REGION=eu-west"), "{env}");
        assert!(!env.contains("sig-secret-9"), "{env}");
        let id = s.ws.read_with(cx, |ws, cx| ws.meta(cx).id.clone());
        let service = service_name("acme-shipping-api", "Default", &id);
        assert_eq!(
            s.secrets.get(&service, "SIGNING_KEY").as_deref(),
            Some("sig-secret-9")
        );
        assert_eq!(s.secrets.get(&service, "API_TOKEN").as_deref(), Some(TOKEN));
        // The toasts never carry a value.
        assert!(toasts(cx, &s).iter().all(|t| !t.contains("sig-secret-9")));
    }

    /// A variable secret before stays secret when the new version says
    /// otherwise: its field is masked and its value goes to the Keychain.
    #[gpui_kit::test]
    fn an_input_secret_before_stays_secret(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let mut vars = fixtures::shipping_variables();
        for v in &mut vars {
            if v.name == "API_TOKEN" {
                v.secret = false;
            }
        }
        // Newly required and unset: ACCOUNT_ID, now required and no longer
        // marked secret, though the user's notes mark it so.
        s.ws.update(cx, |ws, cx| {
            edit_notes(&ws.notes, cx, |m| m.set_secret("ACCOUNT_ID", true));
        });
        for v in &mut vars {
            if v.name == "ACCOUNT_ID" {
                v.required = true;
            }
        }
        publish(&s, 4, chrono::Duration::hours(1), three_changed(), vars);
        check(cx, &s, false);
        click(cx, s.window, "update-banner-update");
        let view = s.ws.read_with(cx, |ws, _| ws.open_shared_view()).unwrap();
        assert_eq!(
            view.read_with(cx, |v, _| v.fields()),
            [("ACCOUNT_ID".to_string(), true)]
        );
        cx.update_window(s.window, |_, window, cx| {
            view.update(cx, |v, cx| v.fill("ACCOUNT_ID", "acct-7", window, cx));
        })
        .unwrap();
        click(cx, s.window, "open-shared-save");
        let env = std::fs::read_to_string(s.root().join(".env")).unwrap();
        assert!(!env.contains("acct-7"), "{env}");
        s.ws.read_with(cx, |ws, cx| {
            let meta = ws.meta(cx);
            assert!(meta.secret_names.iter().any(|n| n == "API_TOKEN"));
            let api = meta.received_vars.iter().find(|v| v.name == "API_TOKEN");
            assert!(api.unwrap().secret, "still secret");
        });
    }

    #[gpui_kit::test]
    fn checks_every_15_minutes(cx: &mut TestAppContext) {
        use std::sync::atomic::Ordering::SeqCst;
        let s = received(cx, false, None);
        // Asked once as the copy opened.
        let opened = s.spy.checks.load(SeqCst);
        assert!(opened >= 1);
        cx.executor()
            .advance_clock(UPDATE_EVERY - std::time::Duration::from_secs(1));
        cx.run_until_parked();
        let first = s.spy.checks.load(SeqCst);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        let second = s.spy.checks.load(SeqCst);
        assert_eq!(second, first + 1, "a tick at 15 minutes");
        cx.executor().advance_clock(UPDATE_EVERY);
        cx.run_until_parked();
        assert_eq!(s.spy.checks.load(SeqCst), second + 1, "and every 15 after");
        // A newer version turns up on a tick, with no click.
        publish(
            &s,
            14,
            chrono::Duration::hours(2),
            three_changed(),
            fixtures::shipping_variables(),
        );
        cx.executor().advance_clock(UPDATE_EVERY);
        cx.run_until_parked();
        assert!(label(cx, &s, "update-banner").is_some());
    }

    /// Any error on an automatic check is quiet (an expired link, a lost
    /// session); asked by hand, it says why.
    #[gpui_kit::test]
    fn automatic_checks_are_quiet_on_every_error(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        let dev = s.spy.dev().unwrap();
        dev.expire_links(true);
        check(cx, &s, false);
        cx.executor().advance_clock(UPDATE_EVERY);
        cx.run_until_parked();
        assert!(toasts(cx, &s).is_empty(), "{:?}", toasts(cx, &s));
        assert!(label(cx, &s, "update-banner").is_none());
        check(cx, &s, true);
        assert_eq!(
            toasts(cx, &s),
            ["Couldn't check for updates: This link has expired. Ask Acme for a new one."]
        );
    }

    /// Two received copies: each tick asks about both, and an offer for
    /// the one not open shows once it opens.
    #[gpui_kit::test]
    fn a_tick_checks_every_received_copy(cx: &mut TestAppContext) {
        use futures::executor::block_on;
        let inner = test_backend();
        crate::model::backend::testing::sign_in(&*inner);
        let owner = CollectionKey {
            id: "0a0b0c0d".into(),
            name: "Payments API".into(),
        };
        let v1 = "request:\n  url: https://pay/rates\n";
        block_on(inner.sync(&owner, &[SyncFile::new("rates.yaml", v1)], &[])).unwrap();
        let payments = block_on(inner.update_share(&owner, ShareUpdate::default()))
            .unwrap()
            .link
            .unwrap();
        let s = added(
            cx,
            inner.clone(),
            &payments,
            &[],
            "shared/acme-payments-api",
        );
        // An Add with no field to fill leaves its dialog up in tests (the
        // focus check in `add_finished`): closed by hand here.
        if dialog_open(cx, &s) {
            cx.update_window(s.window, |_, window, cx| window.close_dialog(cx))
                .unwrap();
            cx.run_until_parked();
        }
        add_link(cx, s.window, &s.ws, LINK, &[("API_TOKEN", TOKEN)]);
        let shipping = s.state.path().join("shared/acme-shipping-api");
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.received_all.len(), 2);
            assert_eq!(ws.state.collection.as_ref().unwrap().root, shipping);
        });
        let dev = s.spy.dev().unwrap();
        let asked = |from: usize| -> Vec<String> {
            dev.recorded()[from..]
                .iter()
                .filter(|c| c.starts_with("latest_version("))
                .cloned()
                .collect()
        };
        let from = dev.recorded().len();
        // Payments publishes a new version while Shipping is open.
        let v2 = SyncFile::new("rates.yaml", "request:\n  url: https://pay/v2\n")
            .with_base(Some(SyncFile::hash_of(v1)));
        block_on(inner.sync(&owner, &[v2], &[])).unwrap();
        cx.executor().advance_clock(UPDATE_EVERY);
        cx.run_until_parked();
        let ticked = asked(from);
        assert_eq!(ticked.len(), 2, "{ticked:?}");
        assert!(
            ticked.iter().any(|c| c.contains("crsl_7Kx2Qm")),
            "{ticked:?}"
        );
        assert!(
            label(cx, &s, "update-banner").is_none(),
            "Shipping is current"
        );
        assert_eq!(s.ws.read_with(cx, |ws, _| ws.update_offers.len()), 1);
        // Payments opens: its offer shows.
        let root = s.root();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| assert!(ws.open_collection(&root, window, cx)));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(
            label(cx, &s, "update-banner-text")
                .is_some_and(|t| t.starts_with("Version 2 available")),
            "{:?}",
            label(cx, &s, "update-banner-text")
        );
    }

    /// `Check for updates` during an automatic check of the same copy
    /// reads `Checking…`, and that check's answer speaks.
    #[gpui_kit::test]
    fn a_manual_check_during_an_automatic_one_answers(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let ws = s.ws.clone();
        let shown = cx
            .update_window(s.window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.check_open_received(window, cx));
                window.render_frame(cx);
                let shown = window
                    .try_find("sidebar-check-updates")
                    .and_then(|e| e.label().map(str::to_string));
                ws.update(cx, |ws, cx| ws.check_for_updates(window, cx));
                shown
            })
            .unwrap();
        assert_eq!(shown.as_deref(), Some(CHECKING));
        cx.run_until_parked();
        assert_eq!(toasts(cx, &s), [UP_TO_DATE]);
        assert_eq!(
            label(cx, &s, "sidebar-check-updates").as_deref(),
            Some("Check for updates")
        );
    }

    #[gpui_kit::test]
    fn an_offer_lands_only_in_its_own_copy(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        open_file(cx, &s, RATES);
        publish(
            &s,
            14,
            chrono::Duration::hours(2),
            three_changed(),
            fixtures::shipping_variables(),
        );
        check(cx, &s, false);
        let before = snapshot(&s);
        let ws = s.ws.clone();
        // The offer moved to another share's token: not this copy's.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let offer = ws.update_offers.drain().next().unwrap().1;
                ws.update_offers.insert("crsl_other".into(), offer);
                ws.apply_offer("crsl_other", window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(snapshot(&s), before, "nothing written here");
    }

    #[gpui_kit::test]
    fn duplicate_copies_yaml_not_env_and_is_editable(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let dest = tempfile::tempdir().unwrap();
        let from = s.ws.read_with(cx, |ws, cx| ws.meta(cx).clone());
        let ws = s.ws.clone();
        // Not into the shared copy itself.
        let inside = s.root().join("rates");
        let before = snapshot(&s);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.duplicate_to(&inside, window, cx));
        })
        .unwrap();
        assert!(s.ws.read_with(cx, |ws, _| ws.read_only_org().is_some()));
        assert_eq!(snapshot(&s), before, "nothing copied into itself");
        let path = dest.path().to_path_buf();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.duplicate_to(&path, window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        let copied: Vec<_> = sync_files(dest.path()).into_iter().map(|f| f.rel).collect();
        let theirs: Vec<_> = snapshot(&s).into_iter().map(|f| f.rel).collect();
        assert_eq!(copied, theirs);
        assert!(!dest.path().join(".env").exists(), "no .env");
        assert_eq!(toasts(cx, &s).last().map(String::as_str), Some(DUPLICATED));
        let notes = CollectionMeta::load(s.state.path(), dest.path());
        assert_eq!(notes.read_only_from, None);
        assert!(notes.received_vars.is_empty());
        assert_ne!(notes.id, from.id, "a new id");
        assert_eq!(notes.secret_names, from.secret_names);
        assert_eq!(
            notes.vars["API_TOKEN"].description,
            from.vars["API_TOKEN"].description
        );
        // No Keychain value under the copy's service.
        let service = service_name(
            &dest.path().file_name().unwrap().to_string_lossy(),
            "Default",
            &notes.id,
        );
        assert_eq!(s.secrets.get(&service, "API_TOKEN"), None);
        // Open, and editable: a save writes.
        open_file(cx, &s, RATES);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                let c = ws.state.collection.as_ref().unwrap();
                assert_eq!(c.root, dest.path());
                assert_eq!(c.read_only, None);
                let ix = ws.state.active.unwrap();
                assert!(!ws.state.open[ix].is_read_only());
                ws.state.open[ix].set_text("request:\n  url: https://mine\n".into());
                assert!(ws.save(ix, window, cx));
            });
        })
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.path().join(RATES)).unwrap(),
            "request:\n  url: https://mine\n"
        );
        assert_ne!(s.read(RATES), "request:\n  url: https://mine\n");
    }

    #[gpui_kit::test]
    fn a_received_copy_resolves_only_by_taking_theirs(cx: &mut TestAppContext) {
        let s = received(cx, true, None);
        let owners = s.read(RATES);
        let mine = "request:\n  url: https://mine\n";
        std::fs::write(s.root().join(RATES), mine).unwrap();
        let conflict = crate::model::backend::Conflict {
            rel: RATES.into(),
            local: mine.into(),
            server: owners.clone(),
            server_author: "Acme".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.sync.conflicts = vec![conflict];
                ws.open_conflicts(window, cx);
            });
            window.render_frame(cx);
            assert!(window.try_find("conflict-take-theirs").is_some());
            assert!(
                window.try_find("conflict-keep-mine").is_none(),
                "the owner wins"
            );
            assert!(window.try_find("conflict-keep-both").is_none());
        })
        .unwrap();
        // Nor by hand.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.resolve_shown(crate::model::conflict::Choice::KeepMine, false, window, cx)
            })
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(s.read(RATES), mine);
        assert_eq!(s.ws.read_with(cx, |ws, _| ws.sync.conflicts.len()), 1);
        s.spy.sent.lock().unwrap().clear();
        click(cx, s.window, "conflict-take-theirs");
        assert_eq!(s.read(RATES), owners);
        assert!(s.ws.read_with(cx, |ws, _| ws.sync.conflicts.is_empty()));
        let sent = s.spy.sent.lock().unwrap().clone();
        assert!(
            sent.iter().all(|f| f.text.is_empty()),
            "no text left the studio"
        );
    }

    #[gpui_kit::test]
    fn a_received_copy_never_syncs(cx: &mut TestAppContext) {
        use crate::ui::sync::SyncLine;
        let s = received(cx, true, None);
        // Changed on disk behind the studio's back, then `Sync now`.
        let mine = "request:\n  url: https://mine\n";
        std::fs::write(s.root().join(RATES), mine).unwrap();
        s.spy.sent.lock().unwrap().clear();
        let dev = s.spy.dev().unwrap();
        let calls = dev.recorded().len();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.mark_changed(window, cx);
                ws.sync_now(window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(s.spy.sent.lock().unwrap().is_empty(), "nothing sent");
        let since = &dev.recorded()[calls..];
        assert!(
            !since.iter().any(|c| c.starts_with("sync")),
            "no sync at all: {since:?}"
        );
        // It takes updates instead: the file stays as edited.
        assert_eq!(s.read(RATES), mine);
        s.ws.read_with(cx, |ws, _| {
            assert!(ws.received.is_some());
            assert_eq!(ws.sync_line(), SyncLine::LocalOnly);
        });
    }

    #[gpui_kit::test]
    fn a_copy_the_server_forgot_never_pushes_and_only_takes_theirs(cx: &mut TestAppContext) {
        use crate::model::conflict::{Choice, ResolveError, resolve_on_disk};
        use crate::ui::sync::SyncLine;
        let s = received(cx, true, None);
        let owners = s.read(RATES);
        let mine = "request:\n  url: https://mine\n";
        // Reset mock: the server forgets the share (and the sign-in); the
        // folder stays. Signed in again, the copy is opened as before.
        let dev = s.spy.dev().unwrap();
        dev.reset();
        crate::model::backend::testing::sign_in(&*s.spy);
        std::fs::write(s.root().join(RATES), mine).unwrap();
        std::fs::remove_file(s.root().join("rates/validate-address.yaml")).unwrap();
        s.spy.sent.lock().unwrap().clear();
        let calls = dev.recorded().len();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.start_sync(window, cx);
                ws.mark_changed(window, cx);
                ws.sync_now(window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        s.ws.read_with(cx, |ws, _| {
            assert!(ws.received.is_none() && ws.received_all.is_empty());
            assert!(ws.pull_only(), "the copy's own lock decides");
            assert_eq!(ws.sync_line(), SyncLine::NotLinked);
        });
        assert_eq!(
            SyncLine::NotLinked.text(chrono::Utc::now()),
            "Not linked to a share"
        );
        assert!(s.spy.sent.lock().unwrap().is_empty(), "no text, no call");
        let since = &dev.recorded()[calls..];
        assert!(
            !since.iter().any(|c| c.starts_with("sync")),
            "nothing synced, nothing deleted: {since:?}"
        );
        // A conflict kept from before: only Take theirs, by click or hand.
        let conflict = crate::model::backend::Conflict {
            rel: RATES.into(),
            local: mine.into(),
            server: owners.clone(),
            server_author: "Acme".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        let c = conflict.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.sync.conflicts = vec![c];
                ws.open_conflicts(window, cx);
            });
            window.render_frame(cx);
            assert!(window.try_find("conflict-take-theirs").is_some());
            assert!(window.try_find("conflict-keep-mine").is_none());
            assert!(window.try_find("conflict-keep-both").is_none());
            ws.update(cx, |ws, cx| {
                ws.resolve_shown(Choice::KeepBoth, false, window, cx)
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!s.root().join("rates/get-rates.server.yaml").exists());
        assert_eq!(s.ws.read_with(cx, |ws, _| ws.sync.conflicts.len()), 1);
        // The model refuses on its own: the folder is a received copy.
        assert_eq!(
            resolve_on_disk(
                &s.root(),
                &conflict,
                Choice::KeepBoth,
                false,
                Some(s.state.path())
            ),
            Err(ResolveError::Refused)
        );
        assert!(!s.root().join("rates/get-rates.server.yaml").exists());
        click(cx, s.window, "conflict-take-theirs");
        assert_eq!(s.read(RATES), owners);
        assert!(s.spy.sent.lock().unwrap().is_empty());
    }

    #[gpui_kit::test]
    fn shared_variables_follow_the_owner_on_update(cx: &mut TestAppContext) {
        use crate::model::backend::SharedVar;
        use futures::executor::block_on;
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = testkit::TestServer::with_handler(move |req| {
            let currency = req
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("x-currency"))
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            log.lock().unwrap().push(currency);
            testkit::Response::from_data(b"{}".to_vec())
                .with_header("Content-Type", "application/json")
        });
        // The owner (the mock's one account) shares Payments API with
        // CURRENCY locked to eur.
        let inner = test_backend();
        crate::model::backend::testing::sign_in(&*inner);
        let owner = CollectionKey {
            id: "0a0b0c0d".into(),
            name: "Payments API".into(),
        };
        let v1 = "request:\n  name: Rates\n  url: ${BASE_URL}/rates\n  headers:\n    X-Currency: ${CURRENCY}\n";
        block_on(inner.sync(&owner, &[SyncFile::new("rates.yaml", v1)], &[])).unwrap();
        let vars = |currency: &str, region: bool| {
            let var = |name: &str, default: Option<&str>, locked: bool| SharedVar {
                name: name.into(),
                description: String::new(),
                required: true,
                secret: false,
                default: default.map(String::from),
                locked,
            };
            let mut out = vec![
                var("BASE_URL", Some(&srv.base_url), false),
                var("CURRENCY", Some(currency), true),
            ];
            if region {
                out.push(var("API_REGION", None, false));
            }
            out
        };
        let share = |vars: Vec<SharedVar>| {
            let update = ShareUpdate {
                variables: Some(vars),
                ..ShareUpdate::default()
            };
            block_on(inner.update_share(&owner, update)).unwrap()
        };
        let link = share(vars("eur", false)).link.unwrap();
        let s = added(cx, inner.clone(), &link, &[], "shared/acme-payments-api");
        let run = |cx: &mut TestAppContext| {
            open_file(cx, &s, "rates.yaml");
            dispatch(cx, &s, &RunActive);
            wait_for_run(cx, &s.ws);
            seen.lock().unwrap().pop().unwrap()
        };
        assert_eq!(run(cx), "eur");
        // The owner locks a new value and adds a required variable its
        // request uses, and publishes: the version goes up.
        share(vars("gbp", true));
        let v2 = format!("{v1}    X-Region: ${{API_REGION}}\n");
        let v2 = SyncFile::new("rates.yaml", v2).with_base(Some(SyncFile::hash_of(v1)));
        assert_eq!(block_on(inner.sync(&owner, &[v2], &[])).unwrap().pushed, 1);
        // Check for updates offers it; nothing changes before Update.
        check(cx, &s, true);
        assert_eq!(
            label(cx, &s, "update-banner-text").as_deref(),
            Some("Version 2 available · published just now · 1 request changed")
        );
        assert_eq!(run(cx), "eur");
        click(cx, s.window, "update-banner-update");
        // The new input is asked for in the Credentials step.
        let view = s.ws.read_with(cx, |ws, _| ws.open_shared_view()).unwrap();
        assert_eq!(
            view.read_with(cx, |v, _| v.fields()),
            [("API_REGION".to_string(), false)]
        );
        s.ws.read_with(cx, |ws, cx| {
            assert_eq!(ws.meta(cx).received_vars, vars("gbp", true));
            assert_eq!(ws.missing, ["API_REGION"], "the status bar says so");
        });
        cx.update_window(s.window, |_, window, cx| {
            view.update(cx, |v, cx| v.fill("API_REGION", "eu", window, cx));
        })
        .unwrap();
        click(cx, s.window, "open-shared-save");
        s.ws.read_with(cx, |ws, _| {
            assert!(ws.missing.is_empty(), "{:?}", ws.missing)
        });
        assert_eq!(run(cx), "gbp");
    }

    /// A second window on the same studio state with nothing open.
    fn nothing_open(cx: &mut TestAppContext, s: &Studio) -> (AnyWindowHandle, Entity<Workspace>) {
        let store = StateStore::at(s.state.path());
        let (secrets, backend) = (
            s.secrets.clone() as Arc<dyn SecretStore>,
            s.spy.clone() as Arc<dyn Backend>,
        );
        let out = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Workspace::new(None, store, secrets, backend, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        out
    }

    #[gpui_kit::test]
    fn a_received_copy_picked_by_hand_takes_nothing(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let before = snapshot(&s);
        let (window, ws) = nothing_open(cx, &s);
        let refused = "Read only · shared by Acme";
        let told = |cx: &mut TestAppContext| ws.read_with(cx, |ws, _| ws.toasts.clone());
        // Import: the copy can't be picked as the target…
        let dialog = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.import(window, cx).unwrap())
            })
            .unwrap();
        let root = s.root();
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill("curl https://api.example.com/v1/x", None, window, cx);
                d.set_target(Some(root.join("rates")), window, cx);
                assert_eq!(d.picked_target, None);
            });
        })
        .unwrap();
        assert_eq!(told(cx), [refused]);
        // …and should it get there anyway, nothing is written.
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.picked_target = Some(root.clone());
                d.import(window, cx);
                assert_eq!(d.error.as_deref(), Some(refused));
            });
        })
        .unwrap();
        assert_eq!(told(cx), [refused, refused]);
        cx.update_window(window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        // New request or collection with no collection open.
        let text = "request:\n  url: https://api.example.com\n".to_string();
        for dir in [s.root(), s.root().join("new-folder")] {
            let text = text.clone();
            cx.update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| {
                    ws.create_collection(dir, "first.yaml".into(), text, window, cx)
                });
            })
            .unwrap();
            cx.run_until_parked();
        }
        assert_eq!(told(cx).len(), 4);
        assert!(told(cx).iter().all(|t| t == refused));
        ws.read_with(cx, |ws, _| assert!(ws.state.collection.is_none()));
        // The model refuses on its own too.
        let base = Some(s.state.path());
        let mut c = crate::model::collection::Collection::open_in(&s.root(), base).unwrap();
        assert!(c.create_file(Path::new("x.yaml"), "x").is_err());
        assert_eq!(snapshot(&s), before, "nothing was written");
        assert!(!s.root().join("new-folder").exists());
        // An ordinary folder still takes a new collection.
        let mine = tempfile::tempdir().unwrap();
        let dir = mine.path().to_path_buf();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.create_collection(dir, "first.yaml".into(), text, window, cx)
            });
        })
        .unwrap();
        assert!(mine.path().join("first.yaml").exists());
    }

    #[gpui_kit::test]
    fn the_studio_folder_is_never_a_target(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let (window, ws) = nothing_open(cx, &s);
        let base = s.state.path().to_path_buf();
        let told = |cx: &mut TestAppContext| ws.read_with(cx, |ws, _| ws.toasts.clone());
        let dialog = cx
            .update_window(window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.import(window, cx).unwrap())
            })
            .unwrap();
        let mine = tempfile::tempdir().unwrap();
        let (ok, state) = (mine.path().to_path_buf(), base.clone());
        cx.update_window(window, |_, window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill("curl https://api.example.com/v1/x", None, window, cx);
                d.set_target(Some(ok.clone()), window, cx);
                assert_eq!(d.picked_target.as_deref(), Some(ok.as_path()));
                // Refused: the earlier pick goes too.
                d.set_target(Some(state.clone()), window, cx);
                assert_eq!(d.picked_target, None);
                d.picked_target = Some(state);
                d.import(window, cx);
                assert_eq!(d.error.as_deref(), Some(STATE_FOLDER_TEXT));
            });
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        let text = "request:\n  url: https://api.example.com\n".to_string();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.create_collection(base.clone(), "first.yaml".into(), text, window, cx)
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(told(cx), [STATE_FOLDER_TEXT; 3]);
        assert!(!base.join("first.yaml").exists());
        ws.read_with(cx, |ws, _| assert!(ws.state.collection.is_none()));
    }

    #[gpui_kit::test]
    fn duplicate_never_lands_in_shared_folders(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let ws = s.ws.clone();
        let other = s.state.path().join("shared/someone-else");
        std::fs::create_dir_all(&other).unwrap();
        // A folder holding the copy, the shared folder itself, one beside it.
        for dest in [
            s.state.path().to_path_buf(),
            s.state.path().join("shared"),
            other.clone(),
        ] {
            cx.update_window(s.window, |_, window, cx| {
                ws.update(cx, |ws, cx| ws.duplicate_to(&dest, window, cx));
            })
            .unwrap();
            s.ws.read_with(cx, |ws, _| {
                assert_eq!(
                    ws.state.collection.as_ref().unwrap().root,
                    s.root(),
                    "{dest:?}"
                );
            });
        }
        assert!(std::fs::read_dir(&other).unwrap().next().is_none());
        assert!(!s.state.path().join("rates").exists());
        assert!(!s.state.path().join("shared/rates").exists());
        assert!(!toasts(cx, &s).iter().any(|t| t == DUPLICATED));
    }

    #[gpui_kit::test]
    fn check_for_updates_without_a_share_says_so(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        // A folder under shared/ with no share record, opened by hand.
        let orphan = s.state.path().join("shared/orphan");
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join("a.yaml"), "request:\n  url: https://a\n").unwrap();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(&orphan, window, cx));
                assert_eq!(ws.read_only_org(), Some("its owner"));
                ws.check_for_updates(window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!dialog_open(cx, &s));
        assert_eq!(
            toasts(cx, &s).last().map(String::as_str),
            Some("Couldn't check for updates: this copy has no share to ask.")
        );
    }

    #[gpui_kit::test]
    fn exports_from_a_copy_start_outside_it(cx: &mut TestAppContext) {
        let s = received(cx, false, None);
        let dir = s.ws.read_with(cx, |ws, _| ws.export_dir()).unwrap();
        assert!(!dir.starts_with(s.state.path()), "{dir:?}");
        let mine = tempfile::tempdir().unwrap();
        let path = mine.path().to_path_buf();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(&path, window, cx));
                assert_eq!(
                    ws.export_dir(),
                    None,
                    "an own collection exports beside itself"
                );
            });
        })
        .unwrap();
    }
}
