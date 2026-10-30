//! Sync: the open collection's tracked YAML files go to the server about
//! [`SYNC_DELAY`] after the last change (a save, or an edit on disk), or at
//! once with `Sync now`. One sync runs at a time; a request while one is in
//! flight queues a single follow-up.
//!
//! Sync is on only while signed in and not `Continue offline`. Pending
//! changes are the tracked files whose hash differs from the one the
//! server last took: the map is seeded on open by an initial sync (with
//! "Sync automatically" off, by the files as they are on open). Until it
//! is seeded (opened offline), the count is the server's own, from
//! [`Backend::sync_status`](crate::model::backend::Backend::sync_status).
//! Offline, the count keeps growing and a probe runs every [`OFFLINE_POLL`] (or on
//! the next save); once back, the waiting changes go.

use super::backend_call;
use super::color;
use super::workspace::Workspace;
use crate::model::backend::{
    BackendError, Conflict, SyncFile, SyncReport, SyncStatus, scan_sync_files,
};
use crate::model::history::relative_time;
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
    /// Each tracked file's hash as the server last took it.
    synced: HashMap<PathBuf, u64>,
    /// `synced` holds the server's hashes (a sync got through, or seeded).
    seeded: bool,
    /// Tracked files the server does not have yet.
    pub(super) pending: usize,
    /// A sync is in flight.
    running: bool,
    /// A sync was asked for while one was in flight.
    again: bool,
    /// The last call failed as offline.
    pub(super) offline: bool,
    /// When the last sync finished (or the server's, on open).
    at: Option<DateTime<Utc>>,
    /// Files the last syncs left changed on both sides; they are held back
    /// from syncs until resolved (Task 7).
    pub(super) conflicts: Vec<Conflict>,
    /// People with access, when the collection has a share link.
    pub(super) people: Option<usize>,
    /// `people` came from the server (a fetch got through); else the next
    /// successful sync fetches it.
    share_known: bool,
    debounce: Option<Task<()>>,
    poll: Option<Task<()>>,
    scan: Option<Task<()>>,
    task: Option<Task<()>>,
    share: Option<Task<()>>,
    tick: Option<Task<()>>,
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
            SyncLine::Conflicts(_) => Some(("danger", false)),
        }
    }

    /// The text's token: amber offline, as on the board.
    pub fn text_color(&self) -> &'static str {
        match self {
            SyncLine::Offline(_) => "warning",
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

/// Number of `files` the server does not have, by `synced` hash.
fn pending_of(
    files: &[SyncFile],
    synced: &HashMap<PathBuf, u64>,
    held: &HashSet<PathBuf>,
) -> usize {
    files
        .iter()
        .filter(|f| !held.contains(&f.rel) && synced.get(&f.rel) != Some(&f.hash))
        .count()
}

/// A pulled path is written only inside the collection, as a visible YAML
/// file: plain components, none hidden.
fn writable(rel: &Path) -> bool {
    let yaml = rel
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml"));
    yaml && rel.components().all(|c| match c {
        Component::Normal(n) => !n.to_string_lossy().starts_with('.'),
        _ => false,
    })
}

/// Writes the server's newer files under `root`; returns the paths written.
fn write_pulled(root: &Path, pulled: &[SyncFile]) -> Vec<PathBuf> {
    let mut written = Vec::new();
    for f in pulled {
        if !writable(&f.rel) {
            log::warn!("sync: refused to write {}", f.rel.display());
            continue;
        }
        let path = root.join(&f.rel);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match crate::model::write_atomic(&path, &f.text) {
            Ok(()) => written.push(f.rel.clone()),
            Err(e) => log::warn!("sync: couldn't write {}: {e}", f.rel.display()),
        }
    }
    written
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
    /// server changed): the server's last sync time, the share's people
    /// and, with "Sync automatically", an initial sync that seeds the
    /// hashes.
    pub(super) fn start_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync = SyncState::default();
        let key = self.collection_key().filter(|_| self.sync_on());
        if let Some(key) = key {
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
                self.refresh_pending(true, false, window, cx);
            }
        }
        self.sync_changed(cx);
    }

    /// A tracked file changed (saved here, or on disk): recount what is
    /// waiting and, with "Sync automatically", sync after a pause.
    pub(super) fn mark_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sync_on() {
            self.refresh_pending(false, self.settings.sync.automatic, window, cx);
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
        let Some(key) = self.collection_key().filter(|_| self.sync_on()) else {
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

    /// Opens the Share dialog (Task 4).
    pub(super) fn open_share(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Opens the conflict dialog (Task 7).
    pub(super) fn open_conflicts(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Counts the waiting changes in the background. `seed` takes the files
    /// as synced instead; `schedule` syncs [`SYNC_DELAY`] later.
    fn refresh_pending(
        &mut self,
        seed: bool,
        schedule: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.state.collection.as_ref().map(|c| c.root.clone()) else {
            return;
        };
        let scan = cx.background_spawn(async move { scan_sync_files(&root).files });
        self.sync.scan = Some(cx.spawn_in(window, async move |this, cx| {
            let files = scan.await;
            this.update_in(cx, |this, window, cx| {
                if seed {
                    this.sync.synced = files.iter().map(|f| (f.rel.clone(), f.hash)).collect();
                    this.sync.seeded = true;
                }
                let held = this.held_back();
                this.sync.pending = if this.sync.seeded {
                    pending_of(&files, &this.sync.synced, &held)
                } else {
                    this.server_pending()
                };
                if schedule && this.sync.pending > 0 {
                    this.schedule_sync(window, cx);
                }
                this.sync_changed(cx);
            })
            .ok();
        }));
    }

    /// What the server counts as waiting, before the hashes are seeded.
    fn server_pending(&self) -> usize {
        let status = self.collection_key().map(|k| self.backend.sync_status(&k));
        match status {
            Some(SyncStatus::Pending(n) | SyncStatus::Offline(n)) => n,
            _ => 0,
        }
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

    /// One sync: the tracked files (less conflicted ones) up, the server's
    /// newer files down. While one is in flight, queues one more.
    fn run_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.collection_key().filter(|_| self.sync_on()) else {
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
        let held = self.held_back();
        let backend = self.backend.clone();
        self.sync.task = Some(cx.spawn_in(window, async move |this, cx| {
            let scan_root = root.clone();
            let scan = cx
                .background_spawn(async move { scan_sync_files(&scan_root) })
                .await;
            for rel in &scan.too_big {
                log::info!("sync: {} stays local (over 1 MB)", rel.display());
            }
            let files: Vec<SyncFile> = scan
                .files
                .into_iter()
                .filter(|f| !held.contains(&f.rel))
                .collect();
            let sent: Vec<(PathBuf, u64)> = files.iter().map(|f| (f.rel.clone(), f.hash)).collect();
            let Ok(call) = this.update(cx, |_, cx| {
                backend_call(cx, &backend, move |b| b.sync(&key, &files))
            }) else {
                return;
            };
            let result = call.await;
            let written = match &result {
                Ok(report) if !report.pulled.is_empty() => {
                    let pulled = report.pulled.clone();
                    cx.background_spawn(async move { write_pulled(&root, &pulled) })
                        .await
                }
                _ => Vec::new(),
            };
            this.update_in(cx, |this, window, cx| {
                this.sync_done(sent, result, written, window, cx);
            })
            .ok();
        }));
        self.sync_changed(cx);
    }

    fn sync_done(
        &mut self,
        sent: Vec<(PathBuf, u64)>,
        result: Result<SyncReport, BackendError>,
        written: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync.running = false;
        match result {
            Ok(report) => {
                let conflicted: HashSet<&Path> =
                    report.conflicts.iter().map(|c| c.rel.as_path()).collect();
                for (rel, hash) in sent {
                    if !conflicted.contains(rel.as_path()) {
                        self.sync.synced.insert(rel, hash);
                    }
                }
                for f in report.pulled.iter().filter(|f| written.contains(&f.rel)) {
                    self.sync.synced.insert(f.rel.clone(), f.hash);
                }
                for c in report.conflicts {
                    self.sync.conflicts.retain(|old| old.rel != c.rel);
                    self.sync.conflicts.push(c);
                }
                self.sync.seeded = true;
                self.sync.at = Some(Utc::now());
                self.sync.offline = false;
                self.sync.poll = None;
                if !self.sync.share_known {
                    self.refresh_share(window, cx);
                }
                if !written.is_empty() {
                    self.disk_changed(&written, window, cx);
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
        self.refresh_pending(false, false, window, cx);
        if std::mem::take(&mut self.sync.again) {
            self.run_sync(window, cx);
        }
        self.sync_changed(cx);
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
    use super::{HashMap, HashSet, Path, PathBuf, SyncFile, SyncLine, Utc, pending_of, writable};

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

    #[test]
    fn pending_counts_changed_and_new_files_not_held_ones() {
        let files = [
            SyncFile::new("a.yaml", "a"),
            SyncFile::new("b.yaml", "b"),
            SyncFile::new("c.yaml", "c"),
        ];
        let synced = HashMap::from([
            (PathBuf::from("a.yaml"), SyncFile::hash_of("a")),
            (PathBuf::from("b.yaml"), SyncFile::hash_of("old")),
        ]);
        let held = HashSet::from([PathBuf::from("c.yaml")]);
        assert_eq!(pending_of(&files, &synced, &held), 1);
        assert_eq!(pending_of(&files, &synced, &HashSet::new()), 2);
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
                cx.new(|cx| {
                    Workspace::new(
                        Some(root),
                        StateStore::none(),
                        test_secrets(),
                        backend,
                        window,
                        cx,
                    )
                })
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
        ws.read_with(cx, |ws, _| ws.collection_key().unwrap())
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
        let backend = signed_in();
        let _first = open(cx, dir.path(), backend.clone());
        std::fs::write(
            dir.path().join("a.yaml"),
            "request:\n  url: https://x.example.com\n",
        )
        .unwrap();
        backend.dev().unwrap().set_offline(true);
        let (window, ws) = open(cx, dir.path(), backend.clone());
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
