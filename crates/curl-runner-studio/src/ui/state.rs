//! What the window has open, and the folder it reopens on the next launch.

use crate::model::collection::Collection;
use crate::model::document::Document;
use chrono::{DateTime, Local};
use std::io;
use std::path::{Path, PathBuf};

/// The open collection and its open files (tabs), held by the root view.
#[derive(Debug, Default)]
pub struct AppState {
    pub collection: Option<Collection>,
    pub open: Vec<Document>,
    pub active: Option<usize>,
}

impl AppState {
    pub fn active_document(&self) -> Option<&Document> {
        self.active.and_then(|i| self.open.get(i))
    }

    pub fn position(&self, rel: &Path) -> Option<usize> {
        self.open.iter().position(|d| d.rel_path == rel)
    }

    /// Opens `rel` from the collection (or finds its tab) and makes it
    /// active. Returns its tab index and whether it was newly opened.
    pub fn open_document(&mut self, rel: &Path) -> io::Result<(usize, bool)> {
        if let Some(i) = self.position(rel) {
            self.active = Some(i);
            return Ok((i, false));
        }
        let (root, read_only) = self
            .collection
            .as_ref()
            .map(|c| (c.root.clone(), c.read_only.is_some()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no collection is open"))?;
        let mut doc = Document::open(&root, rel)?;
        // A received copy's files are locked from the start.
        doc.set_read_only(read_only);
        self.open.push(doc);
        let i = self.open.len() - 1;
        self.active = Some(i);
        Ok((i, true))
    }

    /// Drops tab `idx`; the active tab moves to its neighbour when needed.
    pub fn close(&mut self, idx: usize) {
        if idx >= self.open.len() {
            return;
        }
        self.open.remove(idx);
        self.active = match self.active {
            _ if self.open.is_empty() => None,
            Some(a) if a > idx => Some(a - 1),
            Some(a) if a == idx => Some(idx.min(self.open.len() - 1)),
            other => other,
        };
    }

    /// Tabs at or below `rel` (a file, or a folder and everything in it),
    /// highest index first so they can be closed in order.
    pub fn tabs_under(&self, rel: &Path) -> Vec<usize> {
        let mut found: Vec<usize> = (0..self.open.len())
            .filter(|&i| self.open[i].rel_path.starts_with(rel))
            .collect();
        found.reverse();
        found
    }

    /// Follows a rename of `from` (file or folder) to `to` in the open tabs.
    pub fn renamed(&mut self, from: &Path, to: &Path) {
        for doc in &mut self.open {
            if let Ok(rest) = doc.rel_path.strip_prefix(from) {
                doc.rel_path = if rest.as_os_str().is_empty() {
                    to.to_path_buf()
                } else {
                    to.join(rest)
                };
            }
        }
    }
}

/// Where the studio keeps what it remembers between launches. The app uses
/// [`StateStore::app_support`]; tests use a temporary folder or
/// [`StateStore::none`], so they never touch the user's state.
#[derive(Debug, Clone, Default)]
pub struct StateStore {
    /// `None`: nothing is read or written.
    base: Option<PathBuf>,
}

impl StateStore {
    /// `~/Library/Application Support/curl-runner-studio`.
    pub fn app_support() -> Self {
        let base = std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support/curl-runner-studio"));
        StateStore { base }
    }

    /// State kept in `base`: tests, and the `CURL_RUNNER_STUDIO_STATE` dev
    /// hook.
    pub fn at(base: impl Into<PathBuf>) -> Self {
        StateStore {
            base: Some(base.into()),
        }
    }

    /// Remembers nothing.
    #[cfg(test)]
    pub fn none() -> Self {
        StateStore::default()
    }

    /// Where studio data lives (run history too); `None` keeps nothing.
    pub fn base(&self) -> Option<&Path> {
        self.base.as_deref()
    }

    /// The folder opened last, if it is still recorded.
    pub fn last_folder(&self) -> Option<PathBuf> {
        last_folder_in(self.base.as_deref()?)
    }

    /// Records `p` as the folder to reopen and moves it to the front of
    /// Recent. Failures are logged, not fatal.
    pub fn remember_folder(&self, p: &Path) {
        self.remember_folder_at(p, Local::now());
    }

    fn remember_folder_at(&self, p: &Path, at: DateTime<Local>) {
        let Some(base) = &self.base else { return };
        let path = p.to_string_lossy().into_owned();
        let result = update_state_in(base, |json| {
            let mut recent = recent_in(json);
            recent.retain(|r| r.path != p);
            recent.insert(
                0,
                RecentEntry {
                    path: p.to_path_buf(),
                    opened_at: at,
                },
            );
            recent.truncate(RECENT_MAX);
            json["recent"] = recent_json(&recent);
            json["lastFolder"] = path.into();
        });
        if let Err(e) = result {
            log::warn!("could not save the last folder: {e}");
        }
    }

    /// Folders opened before, most recent first.
    pub fn recent(&self) -> Vec<RecentEntry> {
        self.base
            .as_deref()
            .and_then(read_state_in)
            .map(|json| recent_in(&json))
            .unwrap_or_default()
    }

    /// Forgets every recent folder (the folder to reopen stays).
    pub fn clear_recent(&self) {
        self.edit_recent(|recent| recent.clear());
    }

    /// Forgets the recent folder `p`.
    pub fn remove_recent(&self, p: &Path) {
        self.edit_recent(|recent| recent.retain(|r| r.path != p));
    }

    fn edit_recent(&self, edit: impl FnOnce(&mut Vec<RecentEntry>)) {
        let Some(base) = &self.base else { return };
        let result = update_state_in(base, |json| {
            let mut recent = recent_in(json);
            edit(&mut recent);
            json["recent"] = recent_json(&recent);
        });
        if let Err(e) = result {
            log::warn!("could not save the recent folders: {e}");
        }
    }

    /// The environment last picked for the collection at `root`.
    pub fn environment(&self, root: &Path) -> Option<String> {
        let json = read_state_in(self.base.as_deref()?)?;
        json["environments"][root.to_string_lossy().as_ref()]
            .as_str()
            .map(String::from)
    }

    /// Records `name` as the environment of the collection at `root`.
    pub fn remember_environment(&self, root: &Path, name: &str) {
        let Some(base) = &self.base else { return };
        let key = root.to_string_lossy().into_owned();
        let result = update_state_in(base, |json| {
            if !json["environments"].is_object() {
                json["environments"] = serde_json::json!({});
            }
            json["environments"][key] = name.into();
        });
        if let Err(e) = result {
            log::warn!("could not save the environment: {e}");
        }
    }
}

/// How the studio was told to work, on the sign-in screen. Which server
/// (Cloud or a self-hosted URL) belongs to the backend; this only records
/// that the screen was answered, and whether the answer was "offline".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    /// With a server: signed in, or signed out later from Settings.
    Online,
    /// `Continue offline`: local files only, never asked again.
    Offline,
}

impl StateStore {
    /// The sign-in screen's answer; `None` on first launch (show it).
    pub fn connection(&self) -> Option<Connection> {
        let json = read_state_in(self.base.as_deref()?)?;
        match json["connection"].as_str()? {
            "online" => Some(Connection::Online),
            "offline" => Some(Connection::Offline),
            _ => None,
        }
    }

    /// Records the answer; `None` forgets it, so the next launch asks again.
    pub fn set_connection(&self, connection: Option<Connection>) {
        let Some(base) = &self.base else { return };
        let result = update_state_in(base, |json| match connection {
            Some(c) => {
                json["connection"] = match c {
                    Connection::Online => "online",
                    Connection::Offline => "offline",
                }
                .into();
            }
            None => {
                if let Some(map) = json.as_object_mut() {
                    map.remove("connection");
                }
            }
        });
        if let Err(e) = result {
            log::warn!("could not save the connection choice: {e}");
        }
    }
}

/// A folder opened before, for the Welcome screen's Recent list.
#[derive(Debug, Clone, PartialEq)]
pub struct RecentEntry {
    pub path: PathBuf,
    pub opened_at: DateTime<Local>,
}

/// How many folders Recent keeps.
pub const RECENT_MAX: usize = 10;

const STATE_FILE: &str = "state.json";

fn read_state_in(dir: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(dir.join(STATE_FILE)).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .filter(serde_json::Value::is_object)
}

/// The `recent` list of a state file; entries that don't read are skipped.
fn recent_in(json: &serde_json::Value) -> Vec<RecentEntry> {
    let Some(items) = json["recent"].as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let path = item["path"].as_str()?;
            let at = DateTime::parse_from_rfc3339(item["openedAt"].as_str()?).ok()?;
            Some(RecentEntry {
                path: PathBuf::from(path),
                opened_at: at.with_timezone(&Local),
            })
        })
        .take(RECENT_MAX)
        .collect()
}

fn recent_json(recent: &[RecentEntry]) -> serde_json::Value {
    recent
        .iter()
        .map(|r| {
            serde_json::json!({
                "path": r.path.to_string_lossy(),
                "openedAt": r.opened_at.to_rfc3339(),
            })
        })
        .collect()
}

fn last_folder_in(dir: &Path) -> Option<PathBuf> {
    read_state_in(dir)?["lastFolder"]
        .as_str()
        .map(PathBuf::from)
}

/// Rewrites the state file with `edit` applied, keeping the other keys; a
/// missing or corrupt file starts empty.
fn update_state_in(dir: &Path, edit: impl FnOnce(&mut serde_json::Value)) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut json = read_state_in(dir).unwrap_or_else(|| serde_json::json!({}));
    edit(&mut json);
    std::fs::write(dir.join(STATE_FILE), format!("{json:#}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_choice_round_trips_beside_the_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::at(dir.path());
        assert_eq!(store.connection(), None);
        store.remember_folder(Path::new("/tmp/Payments API"));
        store.set_connection(Some(Connection::Offline));
        assert_eq!(store.connection(), Some(Connection::Offline));
        store.set_connection(Some(Connection::Online));
        assert_eq!(store.connection(), Some(Connection::Online));
        store.set_connection(None);
        assert_eq!(store.connection(), None);
        assert_eq!(
            store.last_folder(),
            Some(PathBuf::from("/tmp/Payments API"))
        );
        assert_eq!(StateStore::none().connection(), None);
    }

    #[test]
    fn state_round_trips_last_folder() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("curl-runner-studio");
        let store = StateStore::at(&base);
        assert_eq!(store.last_folder(), None);
        store.remember_folder(Path::new("/tmp/Payments API"));
        assert_eq!(
            store.last_folder(),
            Some(PathBuf::from("/tmp/Payments API"))
        );
        // `none` neither reads nor writes.
        StateStore::none().remember_folder(Path::new("/x"));
        assert_eq!(StateStore::none().last_folder(), None);
        let raw = std::fs::read_to_string(base.join(STATE_FILE)).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["lastFolder"], "/tmp/Payments API");
    }

    #[test]
    fn environment_is_kept_per_collection_beside_the_last_folder() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::at(dir.path());
        let (a, b) = (Path::new("/tmp/Payments API"), Path::new("/tmp/Shipping"));
        assert_eq!(store.environment(a), None);
        store.remember_folder(a);
        store.remember_environment(a, "staging");
        store.remember_environment(b, "production");
        store.remember_environment(a, "local");
        assert_eq!(store.environment(a).as_deref(), Some("local"));
        assert_eq!(store.environment(b).as_deref(), Some("production"));
        assert_eq!(store.last_folder(), Some(a.to_path_buf()));
        StateStore::none().remember_environment(a, "x");
        assert_eq!(StateStore::none().environment(a), None);
    }

    #[test]
    fn corrupt_state_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(STATE_FILE), "{not json").unwrap();
        assert_eq!(last_folder_in(dir.path()), None);
    }

    fn state_with(files: &[&str]) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        for f in files {
            let p = dir.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "request:\n  url: https://api.example.com\n").unwrap();
        }
        let state = AppState {
            collection: Some(Collection::open(dir.path()).unwrap()),
            ..AppState::default()
        };
        (dir, state)
    }

    #[test]
    fn opening_an_open_file_reuses_its_tab() {
        let (_d, mut s) = state_with(&["a.yaml", "b.yaml"]);
        assert_eq!(s.open_document(Path::new("a.yaml")).unwrap(), (0, true));
        assert_eq!(s.open_document(Path::new("b.yaml")).unwrap(), (1, true));
        assert_eq!(s.open_document(Path::new("a.yaml")).unwrap(), (0, false));
        assert_eq!(s.active, Some(0));
        assert_eq!(s.open.len(), 2);
    }

    #[test]
    fn closing_tabs_moves_active() {
        let (_d, mut s) = state_with(&["a.yaml", "b.yaml", "c.yaml"]);
        for f in ["a.yaml", "b.yaml", "c.yaml"] {
            s.open_document(Path::new(f)).unwrap();
        }
        s.close(2); // active, last
        assert_eq!(s.active, Some(1));
        s.close(0); // left of active
        assert_eq!(s.active, Some(0));
        assert_eq!(s.active_document().unwrap().rel_path, Path::new("b.yaml"));
        s.close(0);
        assert_eq!(s.active, None);
    }

    #[test]
    fn rename_and_delete_follow_folders() {
        let (_d, mut s) = state_with(&["auth/login.yaml", "auth/token.yaml", "x.yaml"]);
        for f in ["auth/login.yaml", "x.yaml", "auth/token.yaml"] {
            s.open_document(Path::new(f)).unwrap();
        }
        s.renamed(Path::new("auth"), Path::new("identity"));
        let paths: Vec<_> = s.open.iter().map(|d| d.rel_path.clone()).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("identity/login.yaml"),
                PathBuf::from("x.yaml"),
                PathBuf::from("identity/token.yaml"),
            ]
        );
        assert_eq!(s.tabs_under(Path::new("identity")), [2, 0]);
        assert_eq!(s.tabs_under(Path::new("x.yaml")), [1]);
        // A sibling whose name only starts with the folder's isn't under it.
        assert!(s.tabs_under(Path::new("ident")).is_empty());
    }

    #[test]
    fn recent_is_mru_capped_at_10() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::at(dir.path());
        assert!(store.recent().is_empty());
        let t0 = Local::now();
        for i in 0..12 {
            let at = t0 + chrono::Duration::minutes(i);
            store.remember_folder_at(Path::new(&format!("/tmp/c{i}")), at);
        }
        // Reopening one moves it to the front, once.
        store.remember_folder_at(Path::new("/tmp/c5"), t0 + chrono::Duration::hours(1));
        let paths: Vec<String> = store
            .recent()
            .iter()
            .map(|r| r.path.display().to_string())
            .collect();
        assert_eq!(paths.len(), RECENT_MAX);
        assert_eq!(paths[0], "/tmp/c5");
        assert_eq!(paths[1], "/tmp/c11");
        assert_eq!(paths.iter().filter(|p| *p == "/tmp/c5").count(), 1);
        assert!(!paths.contains(&"/tmp/c0".to_string()));
        assert!(!paths.contains(&"/tmp/c1".to_string()));
        let first = &store.recent()[0];
        assert_eq!(
            first.opened_at.timestamp(),
            (t0 + chrono::Duration::hours(1)).timestamp()
        );
        // The last folder is still the one opened last.
        assert_eq!(store.last_folder(), Some(PathBuf::from("/tmp/c5")));

        store.remove_recent(Path::new("/tmp/c11"));
        assert_eq!(store.recent().len(), RECENT_MAX - 1);
        assert_eq!(store.recent()[1].path, Path::new("/tmp/c10"));
        store.clear_recent();
        assert!(store.recent().is_empty());
        // Clearing the list keeps the folder to reopen.
        assert_eq!(store.last_folder(), Some(PathBuf::from("/tmp/c5")));
        StateStore::none().remember_folder(Path::new("/x"));
        assert!(StateStore::none().recent().is_empty());
    }

    #[test]
    fn a_bad_recent_entry_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(STATE_FILE),
            r#"{"recent":[{"path":"/a","openedAt":"nope"},{"path":"/b","openedAt":"2026-10-28T10:00:00+00:00"},7]}"#,
        )
        .unwrap();
        let recent = StateStore::at(dir.path()).recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].path, Path::new("/b"));
    }
}
