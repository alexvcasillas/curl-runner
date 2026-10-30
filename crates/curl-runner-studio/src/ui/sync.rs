//! Sync: the open collection's tracked YAML files go to the server about
//! [`SYNC_DELAY`] after the last change (a save, or an edit on disk), or at
//! once with `Sync now`. One sync runs at a time; a request while one is in
//! flight queues a single follow-up.
//!
//! Sync is on only while signed in and not `Continue offline`. The client
//! keeps, per collection, the hash of each file as it last synced it (the
//! base, [`sync_base`]) with the last sync's conflicts. Pending changes are
//! the files whose hash is not their base, plus files in the base that are
//! really gone (deletions; an unreadable file is not gone). Each file goes
//! with its base, so the server pulls, pushes or reports a conflict file by
//! file. A folder that can't be read sends nothing. Offline, the count
//! keeps growing and a probe runs every [`OFFLINE_POLL`] (or on the next
//! save); once back, the waiting changes go.

use super::backend_call;
use super::color;
use super::workspace::Workspace;
use crate::model::backend::{
    BackendError, Conflict, SyncDeletion, SyncFile, SyncReport, SyncScan, SyncStatus,
    scan_sync_files,
};
use crate::model::history::relative_time;
use crate::model::sync_base::{self, SyncBase};
use crate::ui::state::Connection;
use chrono::{DateTime, Local, Utc};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

/// Quiet time after the last change before a sync.
pub(super) const SYNC_DELAY: Duration = Duration::from_secs(2);
/// How often an offline studio checks whether the server is back.
pub(super) const OFFLINE_POLL: Duration = Duration::from_secs(10);
/// How often `Synced 2 min ago` is redrawn.
const RELATIVE_TICK: Duration = Duration::from_secs(30);

/// The workspace's sync state for the open collection.
#[derive(Default)]
pub(super) struct SyncState {
    /// Each file's hash as this client last synced it, and the last
    /// sync's conflicts (persisted).
    base: SyncBase,
    /// Changed, new or deleted files the server does not have yet.
    pub(super) pending: usize,
    /// A sync is in flight.
    running: bool,
    /// A sync was asked for while one was in flight.
    again: bool,
    /// The last call failed as offline.
    pub(super) offline: bool,
    /// The collection folder could not be read: nothing is sent.
    pub(super) unreadable: bool,
    /// When the last sync finished (or the server's, on open).
    at: Option<DateTime<Utc>>,
    /// Files the last sync found changed on both sides (Task 7 resolves).
    pub(super) conflicts: Vec<Conflict>,
    /// People with access, when the collection has a share link.
    pub(super) people: Option<usize>,
    /// `people` came from the server (a fetch got through); else the next
    /// successful sync fetches it.
    share_known: bool,
    /// Writes the base, one at a time; outlives a collection switch.
    saver: BaseSaver,
    debounce: Option<Task<()>>,
    poll: Option<Task<()>>,
    scan: Option<Task<()>>,
    task: Option<Task<()>>,
    share: Option<Task<()>>,
    tick: Option<Task<()>>,
}

/// Base saves in order: at most one write in flight, and per collection
/// only the latest waiting one is kept.
#[derive(Default)]
struct BaseSaver {
    queue: Vec<(String, Option<PathBuf>, SyncBase)>,
    busy: bool,
    task: Option<Task<()>>,
}

/// What the sidebar footer (and the status bar) say about sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncLine {
    /// Signed out, or `Continue offline`: no sync UI.
    LocalOnly,
    /// Signed in, never synced and nothing sent yet.
    NotSynced,
    Syncing,
    Synced(DateTime<Utc>),
    /// Changes waiting for the next sync.
    Waiting(usize),
    Offline(usize),
    Conflicts(usize),
    /// The collection folder can't be read: nothing is synced.
    CantRead,
}

fn changes(n: usize) -> String {
    if n == 1 {
        "1 change".into()
    } else {
        format!("{n} changes")
    }
}

impl SyncLine {
    /// `Synced 2 min ago`, `Offline · 3 changes waiting`, `1 conflict`.
    pub fn text(&self, now: DateTime<Utc>) -> String {
        match self {
            SyncLine::LocalOnly => "Local only".into(),
            SyncLine::NotSynced => "Not synced yet".into(),
            SyncLine::Syncing => "Syncing…".into(),
            SyncLine::Synced(at) => format!(
                "Synced {}",
                relative_time(at.with_timezone(&Local), now.with_timezone(&Local))
            ),
            SyncLine::Waiting(n) => format!("{} waiting", changes(*n)),
            SyncLine::Offline(0) => "Offline".into(),
            SyncLine::Offline(n) => format!("Offline · {} waiting", changes(*n)),
            SyncLine::Conflicts(1) => "1 conflict".into(),
            SyncLine::Conflicts(n) => format!("{n} conflicts"),
            SyncLine::CantRead => "Can't read the collection folder".into(),
        }
    }

    /// The dot before the text: its token and whether it is a ring (the
    /// SyncConflict board's offline dot). `None`: no dot (`Local only`).
    pub fn dot(&self) -> Option<(&'static str, bool)> {
        match self {
            SyncLine::LocalOnly => None,
            SyncLine::NotSynced | SyncLine::Syncing => Some(("text-tertiary", false)),
            SyncLine::Synced(_) => Some(("success", false)),
            SyncLine::Waiting(_) => Some(("warning", false)),
            SyncLine::Offline(_) => Some(("warning", true)),
            SyncLine::Conflicts(_) | SyncLine::CantRead => Some(("danger", false)),
        }
    }

    /// The text's token: amber offline, as on the board.
    pub fn text_color(&self) -> &'static str {
        match self {
            SyncLine::Offline(_) => "warning",
            SyncLine::CantRead => "danger",
            _ => "text-secondary",
        }
    }

    /// Whether the status bar repeats it: only states that ask for
    /// attention (the boards' status bars show nothing when synced).
    pub fn in_status_bar(&self) -> bool {
        matches!(
            self,
            SyncLine::Syncing
                | SyncLine::Waiting(_)
                | SyncLine::Offline(_)
                | SyncLine::Conflicts(_)
                | SyncLine::CantRead
        )
    }
}

/// The 6px dot of a [`SyncLine`].
pub(super) fn sync_dot(token: &'static str, ring: bool) -> Div {
    let dot = div().flex_none().size(px(6.)).rounded_full();
    if ring {
        dot.border_1().border_color(color(token))
    } else {
        dot.bg(color(token))
    }
}

/// The scan can't be trusted: the root can't be listed, or it lists
/// nothing although files were synced before (an unmounted or emptied
/// folder). Nothing is sent then, deletions least of all.
fn untrusted(scan: &SyncScan, base: &SyncBase) -> bool {
    let nothing = scan.files.is_empty() && scan.too_big.is_empty() && scan.unreadable.is_empty();
    !scan.root_ok || (nothing && !base.files.is_empty())
}

/// Changes waiting: files whose hash is not their base (less `held`
/// conflicts), plus, with `deletions`, base paths no longer on disk.
fn pending_of(scan: &SyncScan, base: &SyncBase, held: &HashSet<PathBuf>, deletions: bool) -> usize {
    let changed = scan
        .files
        .iter()
        .filter(|f| !held.contains(&f.rel) && base.files.get(&f.rel) != Some(&f.hash))
        .count();
    let deleted = if deletions {
        deleted_of(scan, base).len()
    } else {
        0
    };
    changed + deleted
}

/// The base paths the scan did not see at all (too-big and unreadable files
/// are still there), with their base hash.
fn deleted_of(scan: &SyncScan, base: &SyncBase) -> Vec<SyncDeletion> {
    let here: HashSet<&Path> = scan
        .files
        .iter()
        .map(|f| f.rel.as_path())
        .chain(scan.too_big.iter().map(PathBuf::as_path))
        .chain(scan.unreadable.iter().map(PathBuf::as_path))
        .collect();
    let mut gone: Vec<SyncDeletion> = base
        .files
        .iter()
        .filter(|(rel, _)| !here.contains(rel.as_path()))
        .map(|(rel, hash)| SyncDeletion {
            rel: rel.clone(),
            base: *hash,
        })
        .collect();
    gone.sort_by(|a, b| a.rel.cmp(&b.rel));
    gone
}

/// Only paths the OS says are not there: a folder that can't be read
/// (permissions, I/O) is not a deletion.
fn really_gone(root: &Path, deleted: Vec<SyncDeletion>) -> Vec<SyncDeletion> {
    deleted
        .into_iter()
        .filter(|d| {
            root.join(&d.rel)
                .symlink_metadata()
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        })
        .collect()
}

/// A pulled path is written only inside the collection, as a visible YAML
/// file: plain components, none hidden.
pub(super) fn writable(rel: &Path) -> bool {
    let yaml = rel
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml"));
    yaml && rel.components().all(|c| match c {
        Component::Normal(n) => !n.to_string_lossy().starts_with('.'),
        _ => false,
    })
}

/// `dir` resolves under `real_root` (no symlinked folder leading out). A
/// missing `dir` is judged by its deepest existing ancestor.
pub(super) fn resolves_inside(dir: &Path, real_root: &Path) -> bool {
    let mut probe = Some(dir);
    while let Some(d) = probe {
        if let Ok(real) = d.canonicalize() {
            return real.starts_with(real_root);
        }
        probe = d.parent();
    }
    false
}

/// What [`apply_pulled`] did on disk.
#[derive(Debug, Default, PartialEq)]
struct Applied {
    /// Paths written, with the hash now on disk.
    written: Vec<(PathBuf, u64)>,
    /// Paths removed (or already gone) for the server's deletions.
    removed: Vec<PathBuf>,
    /// Files changed on disk while the sync was in flight, or held from an
    /// earlier one: left alone.
    conflicts: Vec<Conflict>,
    /// Files that could not be read: left alone, their base kept.
    errors: Vec<PathBuf>,
}

/// The disk text of `path`: `Ok(None)` when it is not there; any other
/// failure is an error, never "gone".
fn read_disk(path: &Path) -> Result<Option<String>, ()> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(()),
    }
}

/// Applies the server's newer files and deletions under `root`. A file is
/// written only if the disk still holds what was `sent` (or it is absent and
/// was not sent), and removed only if the disk still holds its base;
/// anything else, and any path still `held` from an earlier conflict, is a
/// conflict by `author`, so an edit saved meanwhile is never clobbered.
fn apply_pulled(
    root: &Path,
    report: &SyncReport,
    sent: &HashMap<PathBuf, u64>,
    base: &SyncBase,
    author: &str,
) -> Applied {
    let mut out = Applied::default();
    let Ok(real_root) = root.canonicalize() else {
        return out;
    };
    let conflict = |rel: &Path, local: Option<String>, server: Option<&str>| Conflict {
        rel: rel.to_path_buf(),
        local_deleted: local.is_none(),
        local: local.unwrap_or_default(),
        server_deleted: server.is_none(),
        server: server.unwrap_or_default().to_string(),
        server_author: author.to_string(),
        server_at: Utc::now(),
    };
    let inside = |rel: &Path| {
        let path = root.join(rel);
        let ok = writable(rel)
            && path
                .parent()
                .is_some_and(|d| resolves_inside(d, &real_root));
        if !ok {
            log::warn!("sync: refused to touch {}", rel.display());
        }
        ok.then_some(path)
    };
    for f in &report.pulled {
        let Some(path) = inside(&f.rel) else { continue };
        let Ok(disk) = read_disk(&path) else {
            out.errors.push(f.rel.clone());
            continue;
        };
        let unchanged = match (&disk, sent.get(&f.rel)) {
            (Some(d), Some(h)) => SyncFile::hash_of(d) == *h,
            (None, None) => true,
            _ => false,
        };
        if !unchanged || base.held.contains(&f.rel) {
            out.conflicts.push(conflict(&f.rel, disk, Some(&f.text)));
            continue;
        }
        let Some(dir) = path.parent() else { continue };
        let _ = std::fs::create_dir_all(dir);
        // Again once it exists: nothing may have swapped in a link.
        if !dir.canonicalize().is_ok_and(|d| d.starts_with(&real_root)) {
            log::warn!("sync: refused to write {}", f.rel.display());
            continue;
        }
        match crate::model::write_atomic(&path, &f.text) {
            Ok(()) => out.written.push((f.rel.clone(), f.hash)),
            Err(e) => {
                log::warn!("sync: couldn't write {}: {e}", f.rel.display());
                out.errors.push(f.rel.clone());
            }
        }
    }
    for rel in &report.deleted {
        let Some(path) = inside(rel) else { continue };
        match read_disk(&path) {
            Err(()) => out.errors.push(rel.clone()),
            Ok(None) => out.removed.push(rel.clone()),
            Ok(Some(d))
                if !base.held.contains(rel)
                    && base.files.get(rel) == Some(&SyncFile::hash_of(&d)) =>
            {
                match std::fs::remove_file(&path) {
                    Ok(()) => out.removed.push(rel.clone()),
                    Err(e) => {
                        log::warn!("sync: couldn't remove {}: {e}", rel.display());
                        out.errors.push(rel.clone());
                    }
                }
            }
            Ok(Some(d)) => out.conflicts.push(conflict(rel, Some(d), None)),
        }
    }
    out
}

impl Workspace {
    /// Whether the open collection syncs: signed in, not `Continue
    /// offline`.
    pub(super) fn sync_on(&self) -> bool {
        self.state.collection.is_some()
            && self.connection() != Some(Connection::Offline)
            && self.backend.account().is_some()
    }

    /// The footer's state.
    pub(super) fn sync_line(&self) -> SyncLine {
        let s = &self.sync;
        if !self.sync_on() {
            SyncLine::LocalOnly
        } else if s.unreadable {
            SyncLine::CantRead
        } else if !s.conflicts.is_empty() {
            SyncLine::Conflicts(s.conflicts.len())
        } else if s.offline {
            SyncLine::Offline(s.pending)
        } else if s.running {
            SyncLine::Syncing
        } else if s.pending > 0 {
            SyncLine::Waiting(s.pending)
        } else {
            s.at.map_or(SyncLine::NotSynced, SyncLine::Synced)
        }
    }

    /// Starts over for the open collection (opened, signed in or out, the
    /// server changed): the persisted base and conflicts, the server's last
    /// sync time, the share's people, the waiting count and, with "Sync
    /// automatically", a sync (which also rebuilds conflicts).
    pub(super) fn start_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The share belongs to the collection and account it was made for.
        self.drop_share(window, cx);
        let saver = std::mem::take(&mut self.sync.saver);
        self.sync = SyncState {
            saver,
            ..SyncState::default()
        };
        let key = self.collection_key(cx).filter(|_| self.sync_on());
        if let Some(key) = key {
            self.sync.base = sync_base::load(self.store.base(), &key.id);
            self.sync.conflicts = self.sync.base.conflicts.clone();
            if let SyncStatus::Synced { at, .. } = self.backend.sync_status(&key) {
                self.sync.at = Some(at);
            }
            self.refresh_share(window, cx);
            self.sync.tick = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(RELATIVE_TICK).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }));
            if self.settings.sync.automatic {
                self.run_sync(window, cx);
            } else {
                self.refresh_pending(false, window, cx);
            }
        }
        self.sync_changed(cx);
    }

    /// A tracked file changed (saved here, or on disk): recount what is
    /// waiting and, with "Sync automatically", sync after a pause.
    pub(super) fn mark_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sync_on() {
            self.refresh_pending(self.settings.sync.automatic, window, cx);
        }
    }

    /// `Sync now` (Settings, the dev menu's Offline): at once.
    pub(super) fn sync_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync.debounce = None;
        self.run_sync(window, cx);
    }

    /// Re-reads the share's people (the title bar's `Shared · N`): on open
    /// and after the Share dialog changes it.
    pub(super) fn refresh_share(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.collection_key(cx).filter(|_| self.sync_on()) else {
            return;
        };
        let call = backend_call(cx, &self.backend, move |b| b.share_settings(&key));
        self.sync.share = Some(cx.spawn_in(window, async move |this, cx| {
            // Offline or signed out keeps what is shown.
            if let Ok(settings) = call.await {
                this.update(cx, |this, cx| {
                    this.sync.people = settings.link.map(|_| settings.people.len());
                    this.sync.share_known = true;
                    cx.notify();
                })
                .ok();
            }
        }));
    }

    /// Why Share is disabled, as its tooltip: signed out, sign in first;
    /// signed in but offline (chosen, or the server can't be reached), go
    /// online first.
    pub(super) fn share_blocked(&self, cx: &App) -> Option<&'static str> {
        if self.backend.account().is_none() {
            Some("Sign in to share")
        } else if self.meta(cx).read_only_from.is_some() {
            Some("Shared with you: only the owner shares it")
        } else if !self.sync_on() || self.sync.offline {
            Some("Go online to share")
        } else {
            None
        }
    }

    /// Opens the conflict dialog (Task 7).
    pub(super) fn open_conflicts(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// The open collection is a received copy: it pulls, never pushes or
    /// deletes. Its owner, for conflicts on pull.
    fn received_from(&self, cx: &App) -> Option<String> {
        let id = &self.meta(cx).id;
        self.backend
            .received()
            .into_iter()
            .find(|r| &r.collection_id == id)
            .map(|r| r.owner_org)
    }

    /// Counts the waiting changes in the background; `schedule` syncs
    /// [`SYNC_DELAY`] later when there are any.
    fn refresh_pending(&mut self, schedule: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.state.collection.as_ref().map(|c| c.root.clone()) else {
            return;
        };
        let deletions = self.received_from(cx).is_none();
        let scan = cx.background_spawn(async move { scan_sync_files(&root) });
        self.sync.scan = Some(cx.spawn_in(window, async move |this, cx| {
            let scan = scan.await;
            this.update_in(cx, |this, window, cx| {
                this.sync.unreadable = untrusted(&scan, &this.sync.base);
                if !this.sync.unreadable {
                    let held = this.held_back();
                    this.sync.pending = pending_of(&scan, &this.sync.base, &held, deletions);
                    if schedule && this.sync.pending > 0 {
                        this.schedule_sync(window, cx);
                    }
                }
                this.sync_changed(cx);
            })
            .ok();
        }));
    }

    /// Paths with an unresolved conflict.
    fn held_back(&self) -> HashSet<PathBuf> {
        self.sync.conflicts.iter().map(|c| c.rel.clone()).collect()
    }

    /// A sync [`SYNC_DELAY`] from now, replacing one already waiting.
    fn schedule_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync.debounce = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(SYNC_DELAY).await;
            this.update_in(cx, |this, window, cx| this.run_sync(window, cx))
                .ok();
        }));
    }

    /// One sync: every tracked file with its base up, real deletions too,
    /// the server's newer files and deletions down. While one is in flight,
    /// queues one more. A folder that can't be read sends nothing.
    fn run_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.collection_key(cx).filter(|_| self.sync_on()) else {
            return;
        };
        if self.sync.running {
            self.sync.again = true;
            return;
        }
        let root = self.state.collection.as_ref().map(|c| c.root.clone());
        let Some(root) = root else { return };
        self.sync.running = true;
        self.sync.debounce = None;
        let received = self.received_from(cx);
        let author = received
            .clone()
            .or_else(|| self.backend.account().map(|a| a.name))
            .unwrap_or_default();
        let base = self.sync.base.clone();
        let backend = self.backend.clone();
        self.sync.task = Some(cx.spawn_in(window, async move |this, cx| {
            let (scan_root, scan_base) = (root.clone(), base.clone());
            let scanned = cx
                .background_spawn(async move {
                    let scan = scan_sync_files(&scan_root);
                    if untrusted(&scan, &scan_base) {
                        return None;
                    }
                    let deleted = if received.is_some() {
                        // Read-only copies never delete (Task 6).
                        Vec::new()
                    } else {
                        really_gone(&scan_root, deleted_of(&scan, &scan_base))
                    };
                    Some((scan, deleted))
                })
                .await;
            let Some((scan, deleted)) = scanned else {
                this.update(cx, |this, cx| {
                    this.sync.running = false;
                    this.sync.again = false;
                    this.sync.unreadable = true;
                    this.sync_changed(cx);
                })
                .ok();
                return;
            };
            for rel in &scan.too_big {
                log::info!("sync: {} stays local (over 1 MB)", rel.display());
            }
            let files: Vec<SyncFile> = scan
                .files
                .into_iter()
                .map(|f| {
                    let b = base.files.get(&f.rel).copied();
                    f.with_base(b)
                })
                .collect();
            let sent: HashMap<PathBuf, u64> =
                files.iter().map(|f| (f.rel.clone(), f.hash)).collect();
            let gone = deleted.clone();
            let Ok(call) = this.update(cx, |_, cx| {
                backend_call(cx, &backend, move |b| b.sync(&key, &files, &gone))
            }) else {
                return;
            };
            let result = call.await;
            let (result, applied) = match result {
                Ok(report) if !report.pulled.is_empty() || !report.deleted.is_empty() => {
                    let s = sent.clone();
                    cx.background_spawn(async move {
                        let applied = apply_pulled(&root, &report, &s, &base, &author);
                        (Ok(report), applied)
                    })
                    .await
                }
                other => (other, Applied::default()),
            };
            this.update_in(cx, |this, window, cx| {
                this.sync_done(sent, deleted, result, applied, window, cx);
            })
            .ok();
        }));
        self.sync_changed(cx);
    }

    fn sync_done(
        &mut self,
        sent: HashMap<PathBuf, u64>,
        deleted: Vec<SyncDeletion>,
        result: Result<SyncReport, BackendError>,
        applied: Applied,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync.running = false;
        self.sync.unreadable = false;
        match result {
            Ok(report) => {
                // The server's conflicts first; ours for other paths.
                let mut conflicts = report.conflicts;
                let held: Vec<PathBuf> = applied.conflicts.iter().map(|c| c.rel.clone()).collect();
                for c in applied.conflicts {
                    if !conflicts.iter().any(|s| s.rel == c.rel) {
                        conflicts.push(c);
                    }
                }
                let keep: HashSet<PathBuf> = conflicts
                    .iter()
                    .map(|c| c.rel.clone())
                    .chain(applied.errors.iter().cloned())
                    .collect();
                let base = &mut self.sync.base;
                for (rel, hash) in sent {
                    if !keep.contains(&rel) {
                        base.files.insert(rel, hash);
                    }
                }
                for d in deleted.iter().filter(|d| !keep.contains(&d.rel)) {
                    base.files.remove(&d.rel);
                }
                for rel in applied.removed.iter().filter(|r| !keep.contains(*r)) {
                    base.files.remove(rel);
                }
                for (rel, hash) in &applied.written {
                    base.files.insert(rel.clone(), *hash);
                }
                base.conflicts = conflicts.clone();
                base.held = held;
                self.sync.conflicts = conflicts;
                self.save_base(cx);
                for rel in &applied.errors {
                    super::workspace::notify_error(
                        format!("Couldn't read {}; it was left as it is.", rel.display()),
                        window,
                        cx,
                    );
                }
                self.sync.at = Some(Utc::now());
                self.sync.offline = false;
                self.sync.poll = None;
                if !self.sync.share_known {
                    self.refresh_share(window, cx);
                }
                let touched: Vec<PathBuf> = applied
                    .written
                    .into_iter()
                    .map(|(rel, _)| rel)
                    .chain(applied.removed)
                    .collect();
                if !touched.is_empty() {
                    self.disk_changed(&touched, window, cx);
                }
            }
            Err(BackendError::Offline) => {
                self.sync.offline = true;
                self.poll_connection(window, cx);
            }
            // Signed out meanwhile: the footer reads `Local only`.
            Err(BackendError::SignedOut) => {}
            Err(e) => super::workspace::notify_error(format!("Couldn't sync: {e}"), window, cx),
        }
        self.refresh_pending(false, window, cx);
        if std::mem::take(&mut self.sync.again) {
            self.run_sync(window, cx);
        }
        self.sync_changed(cx);
    }

    /// Queues the base for writing: one write at a time, the latest per
    /// collection wins.
    fn save_base(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.collection_key(cx) else {
            return;
        };
        let dir = self.store.base().map(Path::to_path_buf);
        let saver = &mut self.sync.saver;
        saver.queue.retain(|(id, _, _)| *id != key.id);
        saver.queue.push((key.id, dir, self.sync.base.clone()));
        if saver.busy {
            return;
        }
        saver.busy = true;
        saver.task = Some(cx.spawn(async move |this, cx| {
            loop {
                let next = this
                    .update(cx, |this, _| {
                        let saver = &mut this.sync.saver;
                        if saver.queue.is_empty() {
                            saver.busy = false;
                            None
                        } else {
                            Some(saver.queue.remove(0))
                        }
                    })
                    .ok()
                    .flatten();
                let Some((id, dir, base)) = next else { break };
                let saved = cx
                    .background_spawn(async move { sync_base::save(dir.as_deref(), &id, &base) })
                    .await;
                if let Err(e) = saved {
                    log::warn!("sync: couldn't save the sync base: {e}");
                }
            }
        }));
    }

    /// Offline: a sync every [`OFFLINE_POLL`] until one gets through.
    fn poll_connection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sync.poll.is_some() {
            return;
        }
        self.sync.poll = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(OFFLINE_POLL).await;
                let back = this.update_in(cx, |this, window, cx| {
                    if !this.sync.offline {
                        return true;
                    }
                    this.run_sync(window, cx);
                    false
                });
                if back.unwrap_or(true) {
                    break;
                }
            }
        }));
    }

    /// The status bar follows the footer.
    fn sync_changed(&mut self, cx: &mut Context<Self>) {
        let line = Some(self.sync_line()).filter(SyncLine::in_status_bar);
        self.status.update(cx, |s, cx| s.set_sync(line, cx));
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HashMap, HashSet, Path, PathBuf, SyncBase, SyncDeletion, SyncFile, SyncLine, SyncReport,
        SyncScan, Utc, apply_pulled, deleted_of, pending_of, really_gone, untrusted, writable,
    };

    #[test]
    fn lines_read_like_the_boards() {
        let now = Utc::now();
        let at = now - chrono::Duration::minutes(2);
        assert_eq!(SyncLine::Synced(at).text(now), "Synced 2 min ago");
        assert_eq!(
            SyncLine::Offline(3).text(now),
            "Offline · 3 changes waiting"
        );
        assert_eq!(SyncLine::Waiting(1).text(now), "1 change waiting");
        assert_eq!(SyncLine::Conflicts(1).text(now), "1 conflict");
        assert_eq!(SyncLine::LocalOnly.text(now), "Local only");
        assert_eq!(SyncLine::LocalOnly.dot(), None);
        assert_eq!(SyncLine::Offline(3).dot(), Some(("warning", true)));
        assert!(!SyncLine::Synced(at).in_status_bar());
    }

    #[test]
    fn pulled_paths_stay_inside_as_visible_yaml() {
        assert!(writable(Path::new("rates/get.yaml")));
        for bad in [
            "../x.yaml",
            "/etc/x.yaml",
            ".env",
            "a/.git/x.yaml",
            "a/b.txt",
        ] {
            assert!(!writable(Path::new(bad)), "{bad}");
        }
    }

    fn base_of(files: &[(&str, u64)]) -> SyncBase {
        SyncBase {
            files: files.iter().map(|(r, h)| (PathBuf::from(r), *h)).collect(),
            ..SyncBase::default()
        }
    }

    #[test]
    fn pending_counts_changed_and_new_files_not_held_ones() {
        let scan = SyncScan {
            files: vec![
                SyncFile::new("a.yaml", "a"),
                SyncFile::new("b.yaml", "b"),
                SyncFile::new("c.yaml", "c"),
            ],
            too_big: vec!["huge.yaml".into()],
            unreadable: vec!["locked.yaml".into()],
            root_ok: true,
        };
        let h = SyncFile::hash_of;
        let synced = base_of(&[("a.yaml", h("a")), ("b.yaml", h("old"))]);
        let held = HashSet::from([PathBuf::from("c.yaml")]);
        assert_eq!(pending_of(&scan, &synced, &held, true), 1);
        assert_eq!(pending_of(&scan, &synced, &HashSet::new(), true), 2);
        // Deletions: in the base, not seen at all (too big or unreadable
        // files are still there).
        let base = base_of(&[
            ("a.yaml", h("a")),
            ("b.yaml", h("old")),
            ("gone.yaml", 1),
            ("huge.yaml", 2),
            ("locked.yaml", 3),
        ]);
        let gone = deleted_of(&scan, &base);
        assert_eq!(
            gone,
            [SyncDeletion {
                rel: "gone.yaml".into(),
                base: 1
            }]
        );
        assert_eq!(pending_of(&scan, &base, &held, true), 2);
        assert_eq!(pending_of(&scan, &base, &held, false), 1, "read only");
    }

    #[test]
    fn an_unreadable_or_emptied_folder_is_not_trusted() {
        let base = base_of(&[("a.yaml", 1)]);
        let ok = SyncScan {
            root_ok: true,
            ..SyncScan::default()
        };
        assert!(untrusted(&ok, &base), "nothing seen, something synced");
        assert!(!untrusted(&ok, &SyncBase::default()), "a new, empty folder");
        assert!(
            untrusted(&SyncScan::default(), &SyncBase::default()),
            "no root"
        );
        let locked = SyncScan {
            unreadable: vec!["a.yaml".into()],
            root_ok: true,
            ..SyncScan::default()
        };
        assert!(!untrusted(&locked, &base));
    }

    #[test]
    fn only_paths_the_os_says_are_missing_are_gone() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("here.yaml"), "a: 1\n").unwrap();
        let del = |rel: &str| SyncDeletion {
            rel: rel.into(),
            base: 1,
        };
        let gone = really_gone(d.path(), vec![del("here.yaml"), del("gone.yaml")]);
        assert_eq!(gone, [del("gone.yaml")]);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_files_are_never_overwritten_or_removed() {
        use std::os::unix::fs::PermissionsExt as _;
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir(root.join("locked")).unwrap();
        std::fs::write(root.join("locked/a.yaml"), "a: 1\n").unwrap();
        std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o000))
            .unwrap();
        let base = base_of(&[("locked/a.yaml", SyncFile::hash_of("a: 1\n"))]);
        let out = apply_pulled(
            root,
            &report(
                &[SyncFile::new("locked/a.yaml", "a: 2\n")],
                &["locked/a.yaml"],
            ),
            &HashMap::new(),
            &base,
            "Acme",
        );
        std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        assert!(out.written.is_empty() && out.removed.is_empty());
        assert_eq!(out.errors.len(), 2);
        assert_eq!(
            std::fs::read_to_string(root.join("locked/a.yaml")).unwrap(),
            "a: 1\n"
        );
    }

    #[test]
    fn held_conflicts_are_not_pulled_over() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.yaml"), "a: mine\n").unwrap();
        let mut base = base_of(&[("a.yaml", SyncFile::hash_of("a: 1\n"))]);
        base.held = vec!["a.yaml".into()];
        let sent = HashMap::from([(PathBuf::from("a.yaml"), SyncFile::hash_of("a: mine\n"))]);
        let out = apply_pulled(
            d.path(),
            &report(&[SyncFile::new("a.yaml", "a: theirs\n")], &[]),
            &sent,
            &base,
            "Acme",
        );
        assert!(out.written.is_empty());
        assert_eq!(out.conflicts.len(), 1);
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            "a: mine\n"
        );
    }

    fn report(pulled: &[SyncFile], deleted: &[&str]) -> SyncReport {
        SyncReport {
            pulled: pulled.to_vec(),
            deleted: deleted.iter().map(PathBuf::from).collect(),
            ..SyncReport::default()
        }
    }

    #[test]
    fn save_during_sync_is_not_clobbered() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("a.yaml"), "a: saved meanwhile\n").unwrap();
        std::fs::write(root.join("b.yaml"), "b: 1\n").unwrap();
        let sent = HashMap::from([
            (PathBuf::from("a.yaml"), SyncFile::hash_of("a: 1\n")),
            (PathBuf::from("b.yaml"), SyncFile::hash_of("b: 1\n")),
        ]);
        let pulled = [
            SyncFile::new("a.yaml", "a: theirs\n"),
            SyncFile::new("b.yaml", "b: theirs\n"),
            SyncFile::new("new/c.yaml", "c: theirs\n"),
        ];
        let out = apply_pulled(
            root,
            &report(&pulled, &[]),
            &sent,
            &SyncBase::default(),
            "Acme",
        );
        let read = |rel: &str| std::fs::read_to_string(root.join(rel)).unwrap();
        assert_eq!(read("a.yaml"), "a: saved meanwhile\n", "kept");
        assert_eq!(read("b.yaml"), "b: theirs\n");
        assert_eq!(read("new/c.yaml"), "c: theirs\n");
        assert_eq!(out.conflicts.len(), 1);
        let c = &out.conflicts[0];
        assert_eq!(c.rel, PathBuf::from("a.yaml"));
        assert_eq!(
            (c.local.as_str(), c.server.as_str()),
            ("a: saved meanwhile\n", "a: theirs\n")
        );
        assert_eq!(c.server_author, "Acme");
        let written: Vec<_> = out.written.iter().map(|(r, _)| r.clone()).collect();
        assert_eq!(
            written,
            [PathBuf::from("b.yaml"), PathBuf::from("new/c.yaml")]
        );
        // Created on disk meanwhile (not sent): a conflict too.
        std::fs::write(root.join("d.yaml"), "d: mine\n").unwrap();
        let out = apply_pulled(
            root,
            &report(&[SyncFile::new("d.yaml", "d: theirs\n")], &[]),
            &sent,
            &SyncBase::default(),
            "Acme",
        );
        assert_eq!(out.conflicts.len(), 1);
        assert_eq!(read("d.yaml"), "d: mine\n");
    }

    #[test]
    fn pulled_deletions_remove_only_unchanged_files() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("same.yaml"), "s: 1\n").unwrap();
        std::fs::write(root.join("edited.yaml"), "e: 2\n").unwrap();
        let base = base_of(&[
            ("same.yaml", SyncFile::hash_of("s: 1\n")),
            ("edited.yaml", SyncFile::hash_of("e: 1\n")),
            ("gone.yaml", 7),
        ]);
        let out = apply_pulled(
            root,
            &report(&[], &["same.yaml", "edited.yaml", "gone.yaml"]),
            &HashMap::new(),
            &base,
            "Acme",
        );
        assert!(!root.join("same.yaml").exists());
        assert!(root.join("edited.yaml").exists(), "edited: kept");
        assert_eq!(
            out.removed,
            [PathBuf::from("same.yaml"), PathBuf::from("gone.yaml")]
        );
        assert_eq!(out.conflicts.len(), 1);
        assert_eq!(out.conflicts[0].rel, PathBuf::from("edited.yaml"));
        assert_eq!(out.conflicts[0].server, "");
    }

    #[cfg(unix)]
    #[test]
    fn pulled_files_never_land_through_a_symlinked_folder() {
        let d = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join("linked")).unwrap();
        let pulled = [
            SyncFile::new("linked/x.yaml", "x: 1\n"),
            SyncFile::new("linked/deep/y.yaml", "y: 1\n"),
        ];
        let out = apply_pulled(
            d.path(),
            &report(&pulled, &[]),
            &HashMap::new(),
            &SyncBase::default(),
            "Acme",
        );
        assert!(out.written.is_empty());
        assert!(!outside.path().join("x.yaml").exists());
        assert!(
            !outside.path().join("deep").exists(),
            "no folder made outside"
        );
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_backend, test_secrets};
    use super::{OFFLINE_POLL, SYNC_DELAY, SyncLine};
    use crate::model::backend::{Backend, CollectionKey, ShareUpdate, SignInMethod, SyncStatus};
    use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;
    use std::sync::Arc;

    const REQUEST: &str = "request:\n  url: https://api.example.com/a\n";

    /// A collection of `files` (relative path, text).
    fn collection(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    fn signed_in() -> Arc<dyn Backend> {
        let backend = test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        backend
    }

    fn open(
        cx: &mut TestAppContext,
        root: &Path,
        backend: Arc<dyn Backend>,
    ) -> (AnyWindowHandle, Entity<Workspace>) {
        open_in(cx, root, backend, StateStore::none())
    }

    /// [`open`] with a store that keeps state (the sync base) on disk.
    fn open_in(
        cx: &mut TestAppContext,
        root: &Path,
        backend: Arc<dyn Backend>,
        store: StateStore,
    ) -> (AnyWindowHandle, Entity<Workspace>) {
        let root = root.to_path_buf();
        let out = cx.update(|cx| {
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
                cx.new(|cx| Workspace::new(Some(root), store, test_secrets(), backend, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        out
    }

    /// Opens `rel`, replaces its text and saves it.
    fn save(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        rel: &str,
        text: &str,
    ) {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new(rel), window, cx);
                let ix = ws.state.active.unwrap();
                ws.state.open[ix].set_text(text.into());
                assert!(ws.save(ix, window, cx));
            });
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn line(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> SyncLine {
        ws.read_with(cx, |ws, _| ws.sync_line())
    }

    fn shows(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        scope: &'static str,
        label: &str,
    ) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within(scope).try_find_by_label(label).is_some()
        })
        .unwrap()
    }

    /// Element `id` is there, labelled `label`.
    fn labelled(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        id: &'static str,
        label: &str,
    ) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.try_find(id).is_some() && window.try_find_by_label(label).is_some()
        })
        .unwrap()
    }

    fn key(ws: &Entity<Workspace>, cx: &mut TestAppContext) -> CollectionKey {
        ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap())
    }

    /// The sync calls the backend got, as recorded.
    fn syncs(backend: &Arc<dyn Backend>) -> Vec<String> {
        let dev = backend.dev().unwrap();
        dev.recorded()
            .into_iter()
            .filter(|c| c.starts_with("sync("))
            .collect()
    }

    #[gpui_kit::test]
    fn save_marks_pending_then_syncs(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST), ("rates/b.yaml", REQUEST)]);
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        // Opening seeds the hashes with a first sync.
        assert_eq!(syncs(&backend).len(), 1);
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
        assert!(shows(cx, window, "sidebar-sync", "Synced just now"));
        save(
            cx,
            window,
            &ws,
            "a.yaml",
            "request:\n  url: https://api.example.com/b\n",
        );
        assert_eq!(line(cx, &ws), SyncLine::Waiting(1));
        assert!(shows(cx, window, "sidebar-sync", "1 change waiting"));
        assert!(shows(cx, window, "status-bar", "1 change waiting"));
        // Not before the pause is over.
        cx.executor().advance_clock(SYNC_DELAY / 2);
        cx.run_until_parked();
        assert_eq!(syncs(&backend).len(), 1);
        cx.executor().advance_clock(SYNC_DELAY);
        cx.run_until_parked();
        let calls = syncs(&backend);
        assert_eq!(calls.len(), 2, "{calls:?}");
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
        assert!(!shows(cx, window, "status-bar", "Synced just now"));
        assert!(matches!(
            backend.sync_status(&key(&ws, cx)),
            SyncStatus::Synced { requests: 2, .. }
        ));
        // `Sync now` goes at once; a second one while it runs is queued,
        // never concurrent.
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.sync_now(window, cx);
                ws.sync_now(window, cx);
                assert_eq!(ws.sync_line(), SyncLine::Syncing);
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(syncs(&backend).len(), 4);
    }

    #[gpui_kit::test]
    fn offline_counts_changes_and_flushes_when_back(cx: &mut TestAppContext) {
        let dir = collection(&[
            ("a.yaml", REQUEST),
            ("b.yaml", REQUEST),
            ("c.yaml", REQUEST),
        ]);
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let dev = backend.dev().unwrap();
        dev.set_offline(true);
        for rel in ["a.yaml", "b.yaml", "c.yaml"] {
            save(
                cx,
                window,
                &ws,
                rel,
                &format!("request:\n  url: https://api.example.com/{rel}\n"),
            );
        }
        cx.executor().advance_clock(SYNC_DELAY);
        cx.run_until_parked();
        assert_eq!(line(cx, &ws), SyncLine::Offline(3));
        assert!(shows(
            cx,
            window,
            "sidebar-sync",
            "Offline · 3 changes waiting"
        ));
        assert!(labelled(cx, window, "title-offline", "Offline"));
        // Local work is intact.
        let a = std::fs::read_to_string(dir.path().join("a.yaml")).unwrap();
        assert!(a.contains("/a.yaml"));
        // Still offline at the next probe: still waiting.
        cx.executor().advance_clock(OFFLINE_POLL);
        cx.run_until_parked();
        assert_eq!(line(cx, &ws), SyncLine::Offline(3));
        dev.set_offline(false);
        cx.executor().advance_clock(OFFLINE_POLL);
        cx.run_until_parked();
        assert!(
            matches!(line(cx, &ws), SyncLine::Synced(_)),
            "{:?}",
            line(cx, &ws)
        );
        assert!(!shows(
            cx,
            window,
            "sidebar-sync",
            "Offline · 3 changes waiting"
        ));
        let last = syncs(&backend).pop().unwrap();
        for rel in ["a.yaml", "b.yaml", "c.yaml"] {
            assert!(last.contains(rel), "{last}");
        }
        // The server has the edits: nothing waits any more.
        assert!(matches!(
            backend.sync_status(&key(&ws, cx)),
            SyncStatus::Synced { .. }
        ));
    }

    #[gpui_kit::test]
    fn opened_offline_counts_only_what_the_server_lacks(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST), ("b.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let _first = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        std::fs::write(
            dir.path().join("a.yaml"),
            "request:\n  url: https://x.example.com\n",
        )
        .unwrap();
        backend.dev().unwrap().set_offline(true);
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        assert_eq!(line(cx, &ws), SyncLine::Offline(1));
        assert!(shows(
            cx,
            window,
            "sidebar-sync",
            "Offline · 1 change waiting"
        ));
        // The share's people could not be read offline: once back, they are.
        let key = key(&ws, cx);
        backend.dev().unwrap().set_offline(false);
        futures::executor::block_on(backend.update_share(&key, ShareUpdate::default())).unwrap();
        cx.executor().advance_clock(OFFLINE_POLL);
        cx.run_until_parked();
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
        assert!(labelled(cx, window, "title-shared", "Shared · 1"));
    }

    #[gpui_kit::test]
    fn edit_while_closed_is_pending_on_open(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST), ("b.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let _first = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        assert_eq!(syncs(&backend).len(), 1);
        // Closed: an edit, and "Sync automatically" off so nothing syncs.
        std::fs::write(
            dir.path().join("b.yaml"),
            "request:\n  url: https://x.example.com\n",
        )
        .unwrap();
        std::fs::write(
            state.path().join("settings.json"),
            r#"{"sync":{"automatic":false}}"#,
        )
        .unwrap();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        assert_eq!(line(cx, &ws), SyncLine::Waiting(1));
        assert!(shows(cx, window, "sidebar-sync", "1 change waiting"));
        assert_eq!(syncs(&backend).len(), 1, "nothing synced on open");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
    }

    #[gpui_kit::test]
    fn unresolved_conflict_survives_reopen(cx: &mut TestAppContext) {
        let dir = collection(&[(
            "a.yaml",
            "request:\n  url: https://a.example.com\n  timeout: 30\n",
        )]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        backend.dev().unwrap().conflict_next_sync(true);
        let mine = "request:\n  url: https://a.example.com\n  timeout: 40\n";
        save(cx, window, &ws, "a.yaml", mine);
        cx.executor().advance_clock(SYNC_DELAY);
        cx.run_until_parked();
        assert_eq!(line(cx, &ws), SyncLine::Conflicts(1));
        assert!(shows(cx, window, "sidebar-sync", "Resolve"));
        let key = key(&ws, cx);
        let server = || {
            futures::executor::block_on(backend.update_share(&key, ShareUpdate::default()))
                .unwrap();
            let files = futures::executor::block_on(backend.fetch_shared(&key.id))
                .unwrap()
                .files;
            files[0].text.clone()
        };
        let theirs = server();
        assert_ne!(theirs, mine);
        // Reopened (as after a restart): the sync on open finds it again.
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        assert_eq!(line(cx, &ws), SyncLine::Conflicts(1));
        assert!(shows(cx, window, "sidebar-sync", "1 conflict"));
        ws.read_with(cx, |ws, _| {
            let c = &ws.sync.conflicts[0];
            assert_eq!(
                (c.local.as_str(), c.server.as_str()),
                (mine, theirs.as_str())
            );
        });
        assert_eq!(server(), theirs, "the server copy is not overwritten");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.yaml")).unwrap(),
            mine
        );
    }

    #[gpui_kit::test]
    fn deleting_a_file_deletes_it_on_the_server(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST), ("b.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        std::fs::remove_file(dir.path().join("b.yaml")).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.disk_changed(&["b.yaml".into()], window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(line(cx, &ws), SyncLine::Waiting(1));
        cx.executor().advance_clock(SYNC_DELAY);
        cx.run_until_parked();
        let last = syncs(&backend).pop().unwrap();
        assert!(last.contains("SyncDeletion { rel: \"b.yaml\""), "{last}");
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
        let key = key(&ws, cx);
        futures::executor::block_on(backend.update_share(&key, ShareUpdate::default())).unwrap();
        let files = futures::executor::block_on(backend.fetch_shared(&key.id))
            .unwrap()
            .files;
        let rels: Vec<_> = files.iter().map(|f| f.rel.clone()).collect();
        assert_eq!(rels, [std::path::PathBuf::from("a.yaml")]);
        // Not sent again.
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert!(syncs(&backend).pop().unwrap().ends_with("[])"));
    }

    #[cfg(unix)]
    #[gpui_kit::test]
    fn unreadable_file_is_not_deleted_on_the_server(cx: &mut TestAppContext) {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = collection(&[("a.yaml", REQUEST), ("b.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        let b = dir.path().join("b.yaml");
        std::fs::set_permissions(&b, std::fs::Permissions::from_mode(0o000)).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        std::fs::set_permissions(&b, std::fs::Permissions::from_mode(0o644)).unwrap();
        let last = syncs(&backend).pop().unwrap();
        assert!(last.ends_with(", [])"), "no deletion: {last}");
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
        let key = key(&ws, cx);
        futures::executor::block_on(backend.update_share(&key, ShareUpdate::default())).unwrap();
        let files = futures::executor::block_on(backend.fetch_shared(&key.id))
            .unwrap()
            .files;
        assert_eq!(files.len(), 2, "b.yaml stays on the server");
    }

    #[gpui_kit::test]
    fn removed_folder_sends_nothing(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST), ("b.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        let before = syncs(&backend).len();
        // Emptied, then gone: neither sends a thing.
        for rel in ["a.yaml", "b.yaml"] {
            std::fs::remove_file(dir.path().join(rel)).unwrap();
        }
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            syncs(&backend).len(),
            before,
            "an emptied folder is suspicious"
        );
        assert_eq!(line(cx, &ws), SyncLine::CantRead);
        std::fs::remove_dir_all(dir.path()).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(syncs(&backend).len(), before);
        assert!(shows(
            cx,
            window,
            "sidebar-sync",
            "Can't read the collection folder"
        ));
        assert!(shows(
            cx,
            window,
            "status-bar",
            "Can't read the collection folder"
        ));
        let key = key(&ws, cx);
        futures::executor::block_on(backend.update_share(&key, ShareUpdate::default())).unwrap();
        let files = futures::executor::block_on(backend.fetch_shared(&key.id))
            .unwrap()
            .files;
        assert_eq!(files.len(), 2, "the server copy is intact");
    }

    #[gpui_kit::test]
    fn newer_server_copy_is_pulled_when_unchanged_here(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        // Another device pushes on top of what this one synced.
        let key = key(&ws, cx);
        let theirs = "request:\n  url: https://api.example.com/theirs\n";
        let other = crate::model::backend::SyncFile::new("a.yaml", theirs)
            .with_base(Some(crate::model::backend::SyncFile::hash_of(REQUEST)));
        futures::executor::block_on(backend.sync(&key, &[other], &[])).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.yaml")).unwrap(),
            theirs
        );
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
        // The base follows: nothing waits, nothing conflicts next time.
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert!(matches!(line(cx, &ws), SyncLine::Synced(_)));
    }

    #[gpui_kit::test]
    fn conflicts_found_here_come_back_on_reopen(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        let key = key(&ws, cx);
        let conflict = crate::model::backend::Conflict {
            rel: "a.yaml".into(),
            local: "a: mine\n".into(),
            server: "a: theirs\n".into(),
            server_author: "Acme".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        cx.update_window(window, |_, _, cx| {
            ws.update(cx, |ws, cx| {
                ws.sync.base.conflicts = vec![conflict.clone()];
                ws.sync.base.held = vec!["a.yaml".into()];
                ws.save_base(cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
        let stored = crate::model::sync_base::load(Some(state.path()), &key.id);
        assert_eq!(stored.held, [std::path::PathBuf::from("a.yaml")]);
        std::fs::write(
            state.path().join("settings.json"),
            r#"{"sync":{"automatic":false}}"#,
        )
        .unwrap();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        assert_eq!(line(cx, &ws), SyncLine::Conflicts(1));
        assert!(shows(cx, window, "sidebar-sync", "Resolve"));
    }

    #[gpui_kit::test]
    fn quick_base_saves_end_with_the_newest(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST)]);
        let state = tempfile::tempdir().unwrap();
        let backend = signed_in();
        let (window, ws) = open_in(
            cx,
            dir.path(),
            backend.clone(),
            StateStore::at(state.path()),
        );
        let key = key(&ws, cx);
        cx.update_window(window, |_, _, cx| {
            ws.update(cx, |ws, cx| {
                for n in 1..=5u64 {
                    ws.sync.base.files.insert("a.yaml".into(), n);
                    ws.save_base(cx);
                }
                assert!(
                    ws.sync.saver.queue.len() <= 1,
                    "latest wins while one is in flight"
                );
            })
        })
        .unwrap();
        cx.run_until_parked();
        let stored = crate::model::sync_base::load(Some(state.path()), &key.id);
        assert_eq!(stored.files[std::path::Path::new("a.yaml")], 5);
        ws.read_with(cx, |ws, _| assert!(!ws.sync.saver.busy));
    }

    #[gpui_kit::test]
    fn share_is_disabled_signed_out_or_offline(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST)]);
        let backend = test_backend();
        let (_, ws) = open(cx, dir.path(), backend.clone());
        assert_eq!(
            ws.read_with(cx, |ws, cx| ws.share_blocked(cx)),
            Some("Sign in to share")
        );
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        assert_eq!(ws.read_with(cx, |ws, cx| ws.share_blocked(cx)), None);
        backend.dev().unwrap().set_offline(true);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.sync_now(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(
            ws.read_with(cx, |ws, cx| ws.share_blocked(cx)),
            Some("Go online to share")
        );
    }

    #[gpui_kit::test]
    fn env_files_never_in_sync_payload(cx: &mut TestAppContext) {
        let big = format!("a: \"{}\"\n", "x".repeat(1024 * 1024));
        let dir = collection(&[
            ("a.yaml", REQUEST),
            ("rates/b.yml", REQUEST),
            (".env", "API_TOKEN=sk_live_dotenv\n"),
            (".env.staging", "API_TOKEN=sk_live_staging\n"),
            (".env.yaml", "token: sk_live_yaml\n"),
            (".git/config.yaml", "token: sk_live_git\n"),
            (".hidden/c.yaml", "token: sk_live_hidden\n"),
            ("huge.yaml", &big),
        ]);
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        save(
            cx,
            window,
            &ws,
            "a.yaml",
            "request:\n  url: https://api.example.com/z\n",
        );
        cx.executor().advance_clock(SYNC_DELAY);
        cx.run_until_parked();
        let calls = syncs(&backend);
        assert_eq!(calls.len(), 2);
        for call in &calls {
            assert!(
                call.contains("\"a.yaml\"") && call.contains("rates/b.yml"),
                "{call}"
            );
            for bad in [".env", ".git", ".hidden", "huge.yaml", "sk_live"] {
                assert!(!call.contains(bad), "{bad} in {call}");
            }
        }
        for call in backend.dev().unwrap().recorded() {
            assert!(!call.contains("sk_live"), "{call}");
        }
    }

    #[gpui_kit::test]
    fn signed_out_hides_sync(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST)]);
        let backend = test_backend();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        assert_eq!(line(cx, &ws), SyncLine::LocalOnly);
        assert!(shows(cx, window, "sidebar-sync", "Local only"));
        assert!(
            shows(cx, window, "sidebar-sync", "Share"),
            "Share stays, disabled"
        );
        save(
            cx,
            window,
            &ws,
            "a.yaml",
            "request:\n  url: https://api.example.com/b\n",
        );
        cx.executor().advance_clock(SYNC_DELAY * 2);
        cx.run_until_parked();
        assert!(syncs(&backend).is_empty(), "no sync signed out");
        assert_eq!(line(cx, &ws), SyncLine::LocalOnly);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("title-shared").is_none());
            assert!(window.try_find("status-sync").is_none());
        })
        .unwrap();
        // Clicking the disabled Share does nothing (no dialog, no call).
        cx.update_window(window, |_, window, cx| {
            window.within("sidebar-sync").click("sidebar-share", cx);
        })
        .unwrap();
        assert!(
            !backend
                .dev()
                .unwrap()
                .recorded()
                .iter()
                .any(|c| c.starts_with("share"))
        );
    }

    #[gpui_kit::test]
    fn shared_pill_counts_people_with_access(cx: &mut TestAppContext) {
        let dir = collection(&[("a.yaml", REQUEST)]);
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("title-shared").is_none(), "no link yet");
        })
        .unwrap();
        let key = key(&ws, cx);
        futures::executor::block_on(backend.invite(&key, "maria@acme.dev")).unwrap();
        futures::executor::block_on(backend.invite(&key, "sam@acme.dev")).unwrap();
        futures::executor::block_on(backend.update_share(&key, ShareUpdate::default())).unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.refresh_share(window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(labelled(cx, window, "title-shared", "Shared · 3"));
    }
}
