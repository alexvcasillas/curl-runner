//! The open collection's share, as the studio keeps it: the server's
//! settings and the calls that change them. Owned by the workspace, so
//! calls queued from the Share dialog or the Sharing page still land (and
//! the notes are still saved) after the dialog closes; both are views onto
//! it.
//!
//! Calls run one at a time, each built from the latest local state when it
//! starts; only the newest call's settings show. `Share default` ticks are
//! optimistic: shown at once, rolled back one by one if their call fails.
//! The variables sent are [`share_payload`]'s; no Keychain value is read.
//!
//! The notes are the workspace's one copy (`Entity<CollectionMeta>`): the
//! share reads them when a call starts, writes accepted flags into them
//! (the workspace saves), and watches them for secret marks and deleted
//! variables.

use super::collection_settings::Sharing;
use super::workspace::{Workspace, edit_notes};
use crate::model::backend::{
    Access, Account, Backend, BackendError, CollectionKey, Expiry, ShareSettings, ShareUpdate,
    SharedVar, share_candidates, share_payload,
};
use crate::model::collection::TreeNode;
use crate::model::collection_meta::CollectionMeta;
use crate::model::environments::is_secret;
use chrono::{Local, NaiveDate};
use gpui_kit::component::WindowExt as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

/// What the share tells whoever shows it or keeps notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareSyncEvent {
    /// The server accepted a change (link, people, variables): re-read
    /// `Shared · N`. Accepted `share_default` flags are in the notes
    /// already.
    Changed,
    /// The address was invited: its field can be cleared.
    Invited(String),
}

/// The collection being shared.
pub struct ShareSource {
    pub backend: Arc<dyn Backend>,
    pub key: CollectionKey,
    pub root: PathBuf,
    /// The workspace's notes, shared.
    pub notes: Entity<CollectionMeta>,
    /// `18 requests in 4 folders`.
    pub requests: usize,
    pub folders: usize,
}

/// One change to the share.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    /// Opening: gives the share a link, sends the variables, lists them.
    Open,
    /// The variables as they are when the call starts; the tick that asked
    /// for it rolls back if it fails.
    Variables {
        tick: Option<String>,
    },
    Access(Access),
    Expiry(Expiry),
    Reset,
    /// Invite, then re-read the share.
    Invite(String),
    /// Remove, then re-read the share.
    Remove(String),
}

/// What a finished call hands back.
struct Done {
    op: Op,
    /// The notes the call was built from.
    sent: CollectionMeta,
    candidates: Option<Vec<SharedVar>>,
    result: Result<ShareSettings, BackendError>,
    /// An invite or a removal went through, whatever its re-read did.
    landed: bool,
}

/// What a failed call says at the top: offline and signed out in the
/// boards' words, and that nothing changed.
pub fn share_error(e: &BackendError) -> String {
    match e {
        BackendError::Offline => {
            "You're offline. Connect to change sharing; nothing changed.".into()
        }
        BackendError::SignedOut => "Sign in to share. Nothing changed.".into(),
        other => other.to_string(),
    }
}

/// The Expires choice a link's date is, if it is one of the menu's.
pub fn current_expiry(date: Option<NaiveDate>, today: NaiveDate) -> Option<Expiry> {
    match date {
        None => Some(Expiry::Never),
        Some(d) => match (d - today).num_days() {
            n @ (7 | 30) => Some(Expiry::Days(n as u32)),
            _ => None,
        },
    }
}

/// The requests and top-level folders of a collection's tree.
pub fn tree_counts(tree: &[TreeNode]) -> (usize, usize) {
    fn requests(nodes: &[TreeNode]) -> usize {
        nodes
            .iter()
            .map(|n| match n {
                TreeNode::Folder { children, .. } => requests(children),
                TreeNode::File { requests, .. } => requests.len().max(1),
            })
            .sum()
    }
    let folders = tree
        .iter()
        .filter(|n| matches!(n, TreeNode::Folder { .. }))
        .count();
    (requests(tree), folders)
}

pub struct ShareSync {
    backend: Arc<dyn Backend>,
    key: CollectionKey,
    root: PathBuf,
    pub(super) requests: usize,
    pub(super) folders: usize,
    /// The workspace's notes, with `share_default` as the server last
    /// accepted it.
    notes: Entity<CollectionMeta>,
    /// The notes' secret marks and variable names as last seen: what a
    /// change marked secret or deleted.
    seen_secrets: Vec<String>,
    seen_vars: BTreeSet<String>,
    /// Opened (the dialog or the page): the server has the variables, so a
    /// changed secret mark is sent at once.
    opened: bool,
    /// Ticks shown but not accepted yet, by name.
    ticks: BTreeMap<String, bool>,
    /// The server's view, once a call got through.
    pub(super) settings: Option<ShareSettings>,
    /// Every variable, with the default it would share where it has one
    /// (never a secret's).
    pub(super) vars: Vec<SharedVar>,
    /// The last call's failure.
    pub(super) error: Option<String>,
    /// An invite the server refused, under its field.
    pub(super) invite_error: Option<String>,
    /// Access and Expires as last asked for (queued or sent).
    access_intent: Option<Access>,
    expiry_intent: Option<Expiry>,
    queue: VecDeque<Op>,
    busy: bool,
    /// Bumped per call started: only the newest call's settings show.
    generation: u64,
    _notes: Subscription,
}

impl EventEmitter<ShareSyncEvent> for ShareSync {}

impl ShareSync {
    pub fn new(src: ShareSource, cx: &mut Context<Self>) -> ShareSync {
        let watch = cx.observe(&src.notes, |this: &mut ShareSync, _, cx| {
            this.notes_changed(cx)
        });
        let meta = src.notes.read(cx);
        let (seen_secrets, seen_vars) = (
            meta.secret_names.clone(),
            meta.vars.keys().cloned().collect(),
        );
        ShareSync {
            backend: src.backend,
            key: src.key,
            root: src.root,
            requests: src.requests,
            folders: src.folders,
            notes: src.notes,
            seen_secrets,
            seen_vars,
            opened: false,
            ticks: BTreeMap::new(),
            settings: None,
            vars: Vec::new(),
            error: None,
            invite_error: None,
            access_intent: None,
            expiry_intent: None,
            queue: VecDeque::new(),
            busy: false,
            generation: 0,
            _notes: watch,
        }
    }

    pub fn key(&self) -> &CollectionKey {
        &self.key
    }

    pub fn account(&self) -> Option<Account> {
        self.backend.account()
    }

    /// The notes changed (a drawer save, a delete, an import, an accepted
    /// flag): later calls send them as they are. A tick on a name now
    /// secret, or deleted from the notes, is dropped; a secret stops
    /// offering its default; a changed secret mark is sent at once, so the
    /// server stops sharing that name's default.
    fn notes_changed(&mut self, cx: &mut Context<Self>) {
        let meta = self.notes.read(cx);
        let names: BTreeSet<String> = meta.vars.keys().cloned().collect();
        let secrets = meta.secret_names.clone();
        let gone: BTreeSet<String> = self.seen_vars.difference(&names).cloned().collect();
        self.ticks
            .retain(|name, _| !is_secret(name, &secrets) && !gone.contains(name));
        self.mask_secrets(cx);
        let resend = self.opened && secrets != self.seen_secrets;
        self.seen_vars = names;
        self.seen_secrets = secrets;
        if resend {
            self.push(Op::Variables { tick: None }, cx);
        }
        cx.notify();
    }

    /// The listed variables as the notes mark them now: a secret has no
    /// default to offer.
    fn mask_secrets(&mut self, cx: &App) {
        let marked = &self.notes.read(cx).secret_names;
        for v in &mut self.vars {
            if is_secret(&v.name, marked) {
                v.secret = true;
                v.default = None;
                v.locked = false;
            }
        }
    }

    /// The notes with the ticks in flight: what the next call sends.
    fn effective(&self, cx: &App) -> CollectionMeta {
        let mut meta = self.notes.read(cx).clone();
        for (name, on) in &self.ticks {
            meta.vars.entry(name.clone()).or_default().share_default = *on;
        }
        meta
    }

    /// The switch's state: ticks show at once.
    pub fn shares_default(&self, name: &str, cx: &App) -> bool {
        self.ticks.get(name).copied().unwrap_or_else(|| {
            self.notes
                .read(cx)
                .vars
                .get(name)
                .is_some_and(|v| v.share_default)
        })
    }

    /// `name` can share its Default value: not a secret (as the notes mark
    /// it now), and `.env` has one.
    pub fn offers_share_default(&self, name: &str, cx: &App) -> bool {
        !is_secret(name, &self.notes.read(cx).secret_names)
            && self
                .vars
                .iter()
                .any(|v| v.name == name && !v.secret && v.default.is_some())
    }

    pub fn people(&self) -> Option<usize> {
        self.settings.as_ref().map(|s| s.people.len())
    }

    /// The Access menu's choice: the last asked for, else the server's.
    pub fn access(&self) -> Access {
        self.access_intent
            .or_else(|| self.settings.as_ref().map(|s| s.access))
            .unwrap_or_default()
    }

    /// The Expires menu's choice, when it is one of the menu's.
    pub fn expiry(&self) -> Option<Expiry> {
        self.expiry_intent.or_else(|| {
            let today = Local::now().date_naive();
            self.settings
                .as_ref()
                .and_then(|s| current_expiry(s.expires, today))
        })
    }

    /// The dialog or the page opened: a link if none, the variables sent.
    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.opened = true;
        self.push(Op::Open, cx);
    }

    fn push(&mut self, op: Op, cx: &mut Context<Self>) {
        self.queue.push_back(op);
        self.run_next(cx);
    }

    /// Starts the next queued call, unless one is in flight.
    fn run_next(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(op) = self.queue.pop_front() else {
            return;
        };
        self.busy = true;
        self.generation += 1;
        let generation = self.generation;
        let (backend, key, root) = (self.backend.clone(), self.key.clone(), self.root.clone());
        let sent = self.effective(cx);
        let job = cx.background_spawn(async move {
            // The list follows every variables call (a secret mark, a
            // deleted `.env` line).
            let candidates = matches!(op, Op::Open | Op::Variables { .. })
                .then(|| share_candidates(&root, &sent));
            let update = |u: ShareUpdate| backend.update_share(&key, u);
            let mut landed = false;
            let result = match &op {
                Op::Open | Op::Variables { .. } => {
                    update(ShareUpdate {
                        variables: Some(share_payload(&root, &sent)),
                        ..ShareUpdate::default()
                    })
                    .await
                }
                Op::Access(a) => {
                    update(ShareUpdate {
                        access: Some(*a),
                        ..ShareUpdate::default()
                    })
                    .await
                }
                Op::Expiry(e) => {
                    update(ShareUpdate {
                        expires: Some(*e),
                        ..ShareUpdate::default()
                    })
                    .await
                }
                Op::Reset => {
                    update(ShareUpdate {
                        reset_link: true,
                        ..ShareUpdate::default()
                    })
                    .await
                }
                Op::Invite(email) => match backend.invite(&key, email).await {
                    Ok(_) => {
                        landed = true;
                        backend.share_settings(&key).await
                    }
                    Err(e) => Err(e),
                },
                Op::Remove(email) => match backend.remove_person(&key, email).await {
                    Ok(()) => {
                        landed = true;
                        backend.share_settings(&key).await
                    }
                    Err(e) => Err(e),
                },
            };
            Done {
                op,
                sent,
                candidates,
                result,
                landed,
            }
        });
        cx.spawn(async move |this, cx| {
            let done = job.await;
            this.update(cx, |this, cx| this.finish(generation, done, cx))
                .ok();
        })
        .detach();
    }

    /// A call's answer: the newest settings show, an accepted variables
    /// call's flags go into the notes (saved by the workspace), a failure
    /// is reported and a failed tick (or menu choice) falls back to what
    /// the server has.
    fn finish(&mut self, generation: u64, done: Done, cx: &mut Context<Self>) {
        self.busy = false;
        if let Some(vars) = done.candidates {
            self.vars = vars;
            // Built from `sent`: a name marked secret since offers nothing.
            self.mask_secrets(cx);
        }
        match done.result {
            Ok(settings) => {
                if generation == self.generation {
                    self.settings = Some(settings);
                }
                self.error = None;
                if matches!(done.op, Op::Open | Op::Variables { .. }) {
                    self.accept(&done.sent, cx);
                }
                if let Op::Invite(email) = &done.op {
                    cx.emit(ShareSyncEvent::Invited(email.clone()));
                }
                cx.emit(ShareSyncEvent::Changed);
            }
            // The change went through; only the list is stale.
            Err(_) if done.landed => {
                self.error = Some(match &done.op {
                    Op::Invite(email) => {
                        cx.emit(ShareSyncEvent::Invited(email.clone()));
                        "Invited — couldn't refresh the list".into()
                    }
                    _ => "Removed — couldn't refresh the list".into(),
                });
                cx.emit(ShareSyncEvent::Changed);
            }
            Err(BackendError::Invalid(why)) if matches!(done.op, Op::Invite(_)) => {
                self.invite_error = Some(why);
            }
            Err(e) => {
                self.error = Some(share_error(&e));
                if let Op::Variables { tick: Some(name) } = &done.op {
                    self.ticks.remove(name);
                }
            }
        }
        // A menu choice settles once its call is back (the server's
        // settings show it, or show why not), unless a newer one waits.
        match &done.op {
            Op::Access(_) if !self.queue.iter().any(|o| matches!(o, Op::Access(_))) => {
                self.access_intent = None;
            }
            Op::Expiry(_) if !self.queue.iter().any(|o| matches!(o, Op::Expiry(_))) => {
                self.expiry_intent = None;
            }
            _ => {}
        }
        cx.notify();
        self.run_next(cx);
    }

    /// The server took `sent`'s ticks: they go into the notes, and ticks
    /// that say the same are settled. Judged against the notes as they are
    /// now, not as sent: a name marked secret since, or deleted from the
    /// notes since (and not ticked again after), never gets a flag.
    fn accept(&mut self, sent: &CollectionMeta, cx: &mut Context<Self>) {
        let now = self.notes.read(cx);
        let mut flags = Vec::new();
        for (name, v) in &sent.vars {
            let on = v.share_default && !is_secret(name, &sent.secret_names);
            // A tick still here was made after any deletion: those are
            // dropped as the notes change.
            let ticked = self.ticks.get(name).copied();
            if ticked == Some(on) {
                self.ticks.remove(name);
            }
            let deleted = !now.vars.contains_key(name) && ticked.is_none();
            if on && (is_secret(name, &now.secret_names) || deleted) {
                continue;
            }
            let was = now.vars.get(name).is_some_and(|n| n.share_default);
            if was != on {
                flags.push((name.clone(), on));
            }
        }
        edit_notes(&self.notes, cx, |meta| {
            for (name, on) in flags {
                let v = meta.vars.entry(name).or_default();
                v.share_default = on;
                // No shared default, nothing to lock: a later tick starts
                // unlocked.
                v.share_locked &= on;
            }
        });
    }

    pub fn set_access(&mut self, access: Access, cx: &mut Context<Self>) {
        if self.access() == access {
            return;
        }
        self.access_intent = Some(access);
        cx.notify();
        self.push(Op::Access(access), cx);
    }

    pub fn set_expiry(&mut self, expiry: Expiry, cx: &mut Context<Self>) {
        if self.expiry() == Some(expiry) {
            return;
        }
        self.expiry_intent = Some(expiry);
        cx.notify();
        self.push(Op::Expiry(expiry), cx);
    }

    /// `Reset link`: a new token; the old link stops working.
    pub fn reset_link(&mut self, cx: &mut Context<Self>) {
        self.push(Op::Reset, cx);
    }

    /// Ticks or unticks `Share default` for `name` (only where offered).
    pub fn toggle_share_default(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.offers_share_default(name, cx) {
            return;
        }
        let on = !self.shares_default(name, cx);
        self.ticks.insert(name.to_string(), on);
        cx.notify();
        self.push(
            Op::Variables {
                tick: Some(name.to_string()),
            },
            cx,
        );
    }

    /// Whether `name`'s shared default is locked for the people it is
    /// shared with (`Lock for them`).
    pub fn locks_default(&self, name: &str, cx: &App) -> bool {
        self.shares_default(name, cx)
            && self
                .notes
                .read(cx)
                .vars
                .get(name)
                .is_some_and(|v| v.share_locked)
    }

    /// `Lock for them`: only while the default is shared. The notes take
    /// it at once (the workspace saves them) and the variables go again.
    pub fn toggle_lock(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.offers_share_default(name, cx) || !self.shares_default(name, cx) {
            return;
        }
        let on = !self.locks_default(name, cx);
        edit_notes(&self.notes, cx, |meta| {
            meta.vars.entry(name.to_string()).or_default().share_locked = on;
        });
        self.push(Op::Variables { tick: None }, cx);
        cx.notify();
    }

    /// The variables go again as they are now (a description, a lock or a
    /// shared `.env` default changed), unless a call that sends them is
    /// already waiting.
    pub fn resend_variables(&mut self, cx: &mut Context<Self>) {
        let waiting = self
            .queue
            .iter()
            .any(|op| matches!(op, Op::Open | Op::Variables { .. }));
        if !waiting {
            self.push(Op::Variables { tick: None }, cx);
        }
    }

    pub fn invite(&mut self, email: String, cx: &mut Context<Self>) {
        self.invite_error = None;
        self.push(Op::Invite(email), cx);
    }

    pub fn set_invite_error(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        self.invite_error = error;
        cx.notify();
    }

    pub fn remove(&mut self, email: String, cx: &mut Context<Self>) {
        self.push(Op::Remove(email), cx);
    }
}

/// Quiet time after the last change before the share's variables go
/// again.
pub(super) const VARIABLES_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

impl Workspace {
    /// Something the share's variables are built from changed (the notes,
    /// a Default value, a synced file): when the open collection is this
    /// account's and has a share link, they go again [`VARIABLES_DELAY`]
    /// after the last such change, through the share's queue (made if
    /// needed), so the people it is shared with get them on their next
    /// sync or `Check for updates`.
    pub(super) fn variables_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shares_variables() {
            return;
        }
        self.variables_resend = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(VARIABLES_DELAY).await;
            this.update_in(cx, |this, window, cx| {
                this.variables_resend = None;
                if !this.shares_variables() {
                    return;
                }
                if let Some(sync) = this.share_sync(window, cx) {
                    sync.update(cx, |s, cx| s.resend_variables(cx));
                }
            })
            .ok();
        }));
    }

    /// The open collection is this account's, online, with a share link.
    fn shares_variables(&self) -> bool {
        !self.pull_only() && self.sync_on() && !self.sync.offline && self.sync.people.is_some()
    }

    /// What the share needs of the open collection.
    pub(super) fn share_source(&self, cx: &App) -> Option<ShareSource> {
        let c = self.state.collection.as_ref()?;
        let (requests, folders) = tree_counts(&c.tree);
        Some(ShareSource {
            backend: self.backend.clone(),
            key: self.collection_key(cx)?,
            root: c.root.clone(),
            notes: self.notes.clone(),
            requests,
            folders,
        })
    }

    /// The open collection's share, made on first use and kept while the
    /// collection stays open, on the workspace's notes.
    pub(super) fn share_sync(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<ShareSync>> {
        let src = self.share_source(cx)?;
        if let Some(sync) = &self.share_sync
            && sync.read(cx).key() == &src.key
        {
            return Some(sync.clone());
        }
        let sync = cx.new(|cx| ShareSync::new(src, cx));
        self._share_sync_events = Some(cx.subscribe_in(
            &sync,
            window,
            |this, sync, event: &ShareSyncEvent, window, cx| {
                // A share of a collection no longer open says nothing here.
                if this.collection_key(cx).as_ref() != Some(sync.read(cx).key()) {
                    return;
                }
                if let ShareSyncEvent::Changed = event {
                    this.refresh_share(window, cx);
                }
            },
        ));
        self.share_sync = Some(sync.clone());
        Some(sync)
    }

    /// Forgets the share (another collection, another account): its calls'
    /// answers go nowhere, an open Share dialog closes and the settings
    /// page moves to a fresh one.
    pub(super) fn drop_share(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.share_view().is_some() {
            window.close_dialog(cx);
        }
        self.share_view = None;
        self._share_events = None;
        self.share_sync = None;
        self._share_sync_events = None;
        if self.settings_view.is_some() {
            let sharing = self.share_sync(window, cx).map(|sync| Sharing {
                backend: self.backend.clone(),
                sync,
                people: None,
            });
            if let Some(view) = &self.settings_view {
                view.update(cx, |v, cx| v.replace_sharing(sharing, window, cx));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendError, Expiry, current_expiry, share_error};
    use chrono::NaiveDate;

    use super::{ShareSource, ShareSync};
    use crate::model::backend::CollectionKey;
    use crate::model::collection_meta::{CollectionMeta, VarMeta};
    use gpui_kit::{AppContext as _, Entity, TestAppContext};

    /// A share on `meta`, never opened (nothing is sent).
    fn sync_on(
        cx: &mut TestAppContext,
        meta: CollectionMeta,
    ) -> (Entity<CollectionMeta>, Entity<ShareSync>) {
        let notes = cx.new(|_| meta);
        let src = ShareSource {
            backend: crate::model::backend::test_backend(),
            key: CollectionKey {
                id: "0a1b2c3d".into(),
                name: "Payments API".into(),
            },
            root: std::path::PathBuf::from("/nowhere"),
            notes: notes.clone(),
            requests: 0,
            folders: 0,
        };
        let sync = cx.new(|cx| ShareSync::new(src, cx));
        (notes, sync)
    }

    fn flagged(name: &str) -> CollectionMeta {
        let mut meta = CollectionMeta::default();
        meta.vars.insert(
            name.into(),
            VarMeta {
                share_default: true,
                ..VarMeta::default()
            },
        );
        meta
    }

    #[gpui_kit::test]
    fn new_notes_replace_the_old_and_deleted_vars_stay_gone(cx: &mut TestAppContext) {
        let (notes, sync) = sync_on(cx, flagged("OLD_URL"));
        assert!(sync.read_with(cx, |s, cx| s.shares_default("OLD_URL", cx)));
        notes.update(cx, |m, cx| {
            *m = CollectionMeta::default();
            cx.notify();
        });
        assert!(
            !sync.read_with(cx, |s, cx| s.shares_default("OLD_URL", cx)),
            "deleted, not re-created"
        );
        assert!(sync.read_with(cx, |s, cx| s.effective(cx).vars.is_empty()));
    }

    #[gpui_kit::test]
    fn accepted_flags_are_judged_against_the_notes_now(cx: &mut TestAppContext) {
        // Sent with X ticked; X marked secret before the answer.
        let (notes, sync) = sync_on(cx, CollectionMeta::default());
        let sent = flagged("X");
        notes.update(cx, |m, cx| {
            m.set_secret("X", true);
            cx.notify();
        });
        sync.update(cx, |s, cx| s.accept(&sent, cx));
        assert!(
            notes.read_with(cx, |m, _| !m.vars.contains_key("X")),
            "secret"
        );

        // Sent with X's notes; X deleted from them before the answer.
        let (notes, sync) = sync_on(cx, flagged("X"));
        notes.update(cx, |m, cx| {
            m.vars.clear();
            cx.notify();
        });
        sync.update(cx, |s, cx| s.accept(&sent, cx));
        assert!(notes.read_with(cx, |m, _| m.vars.is_empty()), "deleted");

        // Ticked again after the deletion: that one lands.
        sync.update(cx, |s, _| {
            s.ticks.insert("X".into(), true);
        });
        sync.update(cx, |s, cx| s.accept(&sent, cx));
        assert!(notes.read_with(cx, |m, _| m.vars["X"].share_default));
        assert!(sync.read_with(cx, |s, _| s.ticks.is_empty()), "settled");

        // A tick made before the deletion goes with it.
        let (notes, sync) = sync_on(cx, flagged("Y"));
        sync.update(cx, |s, _| {
            s.ticks.insert("Y".into(), false);
        });
        notes.update(cx, |m, cx| {
            m.vars.clear();
            cx.notify();
        });
        assert!(sync.read_with(cx, |s, _| s.ticks.is_empty()));
    }

    #[test]
    fn expiry_and_errors_read_like_the_board() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let in_days = |n| today.checked_add_days(chrono::Days::new(n));
        assert_eq!(current_expiry(in_days(30), today), Some(Expiry::Days(30)));
        assert_eq!(current_expiry(in_days(7), today), Some(Expiry::Days(7)));
        assert_eq!(current_expiry(None, today), Some(Expiry::Never));
        assert_eq!(
            current_expiry(in_days(12), today),
            None,
            "none of the menu's"
        );
        assert_eq!(
            share_error(&BackendError::Offline),
            "You're offline. Connect to change sharing; nothing changed."
        );
    }
}
