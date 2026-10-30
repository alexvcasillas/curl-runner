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

use super::collection_settings::Sharing;
use super::workspace::Workspace;
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
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

/// What the share tells whoever shows it or keeps notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareSyncEvent {
    /// The server accepted a change (link, people, variables): re-read
    /// `Shared · N` and merge these `share_default` flags (the ones that
    /// changed) into the notes, which their owner saves.
    Changed(Vec<(String, bool)>),
    /// The address was invited: its field can be cleared.
    Invited(String),
}

/// The collection being shared.
pub struct ShareSource {
    pub backend: Arc<dyn Backend>,
    pub key: CollectionKey,
    pub root: PathBuf,
    pub meta: CollectionMeta,
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
    /// The notes, with `share_default` as the server last accepted it.
    notes: CollectionMeta,
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
}

impl EventEmitter<ShareSyncEvent> for ShareSync {}

impl ShareSync {
    pub fn new(src: ShareSource) -> ShareSync {
        ShareSync {
            backend: src.backend,
            key: src.key,
            root: src.root,
            requests: src.requests,
            folders: src.folders,
            notes: src.meta,
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
        }
    }

    pub fn key(&self) -> &CollectionKey {
        &self.key
    }

    pub fn account(&self) -> Option<Account> {
        self.backend.account()
    }

    /// The owner's notes changed elsewhere (a description, a secret mark):
    /// later calls send them. Accepted ticks stay as accepted.
    /// The owner's notes: the workspace keeps (and saves) them, accepted
    /// flags merged in as each `Changed` arrives, so they replace these
    /// whole. Ticks in flight stay on top.
    pub fn set_notes(&mut self, meta: CollectionMeta) {
        self.notes = meta;
    }

    /// The notes with the ticks in flight: what the next call sends.
    fn effective(&self) -> CollectionMeta {
        let mut meta = self.notes.clone();
        for (name, on) in &self.ticks {
            meta.vars.entry(name.clone()).or_default().share_default = *on;
        }
        meta
    }

    /// The switch's state: ticks show at once.
    pub fn shares_default(&self, name: &str) -> bool {
        self.ticks
            .get(name)
            .copied()
            .unwrap_or_else(|| self.notes.vars.get(name).is_some_and(|v| v.share_default))
    }

    /// `name` can share its Default value: not a secret, and `.env` has one.
    pub fn offers_share_default(&self, name: &str) -> bool {
        self.vars
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
        let sent = self.effective();
        let job = cx.background_spawn(async move {
            let candidates = (op == Op::Open).then(|| share_candidates(&root, &sent));
            let update = |u: ShareUpdate| backend.update_share(&key, u);
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
                    Ok(_) => backend.share_settings(&key).await,
                    Err(e) => Err(e),
                },
                Op::Remove(email) => match backend.remove_person(&key, email).await {
                    Ok(()) => backend.share_settings(&key).await,
                    Err(e) => Err(e),
                },
            };
            Done {
                op,
                sent,
                candidates,
                result,
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
    /// call becomes the notes (saved), a failure is reported and a failed
    /// tick (or menu choice) falls back to what the server has.
    fn finish(&mut self, generation: u64, done: Done, cx: &mut Context<Self>) {
        self.busy = false;
        if let Some(vars) = done.candidates {
            self.vars = vars;
        }
        match done.result {
            Ok(settings) => {
                if generation == self.generation {
                    self.settings = Some(settings);
                }
                self.error = None;
                let accepted = if matches!(done.op, Op::Open | Op::Variables { .. }) {
                    self.accept(&done.sent)
                } else {
                    Vec::new()
                };
                if let Op::Invite(email) = &done.op {
                    cx.emit(ShareSyncEvent::Invited(email.clone()));
                }
                cx.emit(ShareSyncEvent::Changed(accepted));
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

    /// The server took `sent`'s ticks: they are the notes now, and ticks
    /// that say the same are settled. Returns the flags that changed (never
    /// a secret's), for the notes' owner.
    fn accept(&mut self, sent: &CollectionMeta) -> Vec<(String, bool)> {
        let mut changed = Vec::new();
        for (name, v) in &sent.vars {
            let on = v.share_default && !is_secret(name, &sent.secret_names);
            if self.ticks.get(name) == Some(&on) {
                self.ticks.remove(name);
            }
            let was = self.notes.vars.get(name).is_some_and(|n| n.share_default);
            if was != on {
                self.notes
                    .vars
                    .entry(name.clone())
                    .or_default()
                    .share_default = on;
                changed.push((name.clone(), on));
            }
        }
        changed
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
        if !self.offers_share_default(name) {
            return;
        }
        let on = !self.shares_default(name);
        self.ticks.insert(name.to_string(), on);
        cx.notify();
        self.push(
            Op::Variables {
                tick: Some(name.to_string()),
            },
            cx,
        );
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

impl Workspace {
    /// What the share needs of the open collection.
    pub(super) fn share_source(&self) -> Option<ShareSource> {
        let c = self.state.collection.as_ref()?;
        let (requests, folders) = tree_counts(&c.tree);
        Some(ShareSource {
            backend: self.backend.clone(),
            key: self.collection_key()?,
            root: c.root.clone(),
            meta: self.meta.clone(),
            requests,
            folders,
        })
    }

    /// The open collection's share, made on first use and kept while the
    /// collection stays open, on the notes as they are now.
    pub(super) fn share_sync(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<ShareSync>> {
        let src = self.share_source()?;
        if let Some(sync) = &self.share_sync
            && sync.read(cx).key() == &src.key
        {
            let meta = self.meta.clone();
            sync.update(cx, |s, _| s.set_notes(meta));
            return Some(sync.clone());
        }
        let sync = cx.new(|_| ShareSync::new(src));
        self._share_sync_events = Some(cx.subscribe_in(
            &sync,
            window,
            |this, sync, event: &ShareSyncEvent, window, cx| {
                // A share of a collection no longer open says nothing here.
                if this.collection_key().as_ref() != Some(sync.read(cx).key()) {
                    return;
                }
                if let ShareSyncEvent::Changed(flags) = event {
                    this.merge_share_defaults(flags, window, cx);
                    this.refresh_share(window, cx);
                }
            },
        ));
        self.share_sync = Some(sync.clone());
        Some(sync)
    }

    /// Accepted `Share default` flags join the notes, which are saved from
    /// here (the one owner), and the open settings page's copy.
    fn merge_share_defaults(
        &mut self,
        flags: &[(String, bool)],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if flags.is_empty() {
            return;
        }
        for (name, on) in flags {
            self.meta
                .vars
                .entry(name.clone())
                .or_default()
                .share_default = *on;
        }
        self.save_meta(window, cx);
        if let Some(view) = &self.settings_view {
            view.update(cx, |v, cx| v.merge_share_defaults(flags, cx));
        }
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

    #[test]
    fn new_notes_replace_the_old_and_deleted_vars_stay_gone() {
        use super::{ShareSource, ShareSync};
        use crate::model::backend::CollectionKey;
        use crate::model::collection_meta::{CollectionMeta, VarMeta};
        let mut meta = CollectionMeta::default();
        meta.vars.insert(
            "OLD_URL".into(),
            VarMeta {
                share_default: true,
                ..VarMeta::default()
            },
        );
        let mut sync = ShareSync::new(ShareSource {
            backend: crate::model::backend::test_backend(),
            key: CollectionKey {
                id: "0a1b2c3d".into(),
                name: "Payments API".into(),
            },
            root: std::path::PathBuf::from("/nowhere"),
            meta,
            requests: 0,
            folders: 0,
        });
        assert!(sync.shares_default("OLD_URL"));
        sync.set_notes(CollectionMeta::default());
        assert!(!sync.shares_default("OLD_URL"), "deleted, not re-created");
        assert!(sync.effective().vars.is_empty());
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
