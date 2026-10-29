//! What the window has open, and the folder it reopens on the next launch.

use crate::model::collection::Collection;
use crate::model::document::Document;
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
        let root = self
            .collection
            .as_ref()
            .map(|c| c.root.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no collection is open"))?;
        self.open.push(Document::open(&root, rel)?);
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

/// The folder opened last, if it is still recorded.
pub fn last_folder() -> Option<PathBuf> {
    last_folder_in(&state_dir()?)
}

/// Records `p` as the folder to reopen. Failures are logged, not fatal.
pub fn remember_folder(p: &Path) {
    let Some(dir) = state_dir() else { return };
    if let Err(e) = remember_folder_in(&dir, p) {
        log::warn!("could not save the last folder: {e}");
    }
}

/// `~/Library/Application Support/curl-runner-studio`.
fn state_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/curl-runner-studio"))
}

const STATE_FILE: &str = "state.json";

fn last_folder_in(dir: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dir.join(STATE_FILE)).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json["lastFolder"].as_str().map(PathBuf::from)
}

fn remember_folder_in(dir: &Path, p: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let json = serde_json::json!({ "lastFolder": p.to_string_lossy() });
    std::fs::write(dir.join(STATE_FILE), format!("{json:#}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_last_folder() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("curl-runner-studio");
        assert_eq!(last_folder_in(&base), None);
        remember_folder_in(&base, Path::new("/tmp/Payments API")).unwrap();
        assert_eq!(
            last_folder_in(&base),
            Some(PathBuf::from("/tmp/Payments API"))
        );
        let raw = std::fs::read_to_string(base.join(STATE_FILE)).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["lastFolder"], "/tmp/Payments API");
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
}
