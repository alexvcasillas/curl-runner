//! The client's side of sync: per collection, the hash of each file as this
//! studio last synced it, in `{base}/sync/{collection id}.json`. It tells
//! pending changes (hash differs from base), deletions (in base, gone
//! locally) and lets the server spot conflicts (its copy is not the base).
//! The last sync's conflicts are kept with it, so a reopen shows them and
//! the ones found here (`held`) are never overwritten by the next pull.
//!
//! A collection of one's own is synced per server and account: its base is
//! kept under [`scoped_id`], so signing in elsewhere starts from nothing
//! (no stale base to fake conflicts or deletions with) and coming back
//! finds the old one. A received copy's base is the owner's snapshot as
//! written here, whoever is signed in: it stays under the plain id.

use super::backend::Conflict;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncBase {
    /// Relative path → hash last synced.
    pub files: HashMap<PathBuf, u64>,
    /// What the last sync left changed on both sides.
    pub conflicts: Vec<Conflict>,
    /// Paths of `conflicts` found by this client (a file changed on disk
    /// while the server's copy came down): pulls skip them until resolved.
    pub held: Vec<PathBuf>,
}

/// The file's form: `/`-joined keys, sorted.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    files: BTreeMap<String, u64>,
    conflicts: Vec<Conflict>,
    held: Vec<String>,
}

fn path(base: &Path, id: &str) -> Option<PathBuf> {
    let safe = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    safe.then(|| base.join("sync").join(format!("{id}.json")))
}

/// `rates/get.yaml` on every OS.
fn key(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The id collection `id`'s base is kept under for one server and account:
/// `{id}-{16 hex digits}` (safe as a file name).
pub fn scoped_id(id: &str, server_url: &str, account_email: &str) -> String {
    let scope = super::history::fnv1a(format!("{server_url}\n{account_email}").as_bytes());
    format!("{id}-{scope:016x}")
}

/// The base of collection `id`; missing or unreadable is empty (never
/// synced). `base` `None` keeps nothing.
pub fn load(base: Option<&Path>, id: &str) -> SyncBase {
    let Some(file) = base.and_then(|b| path(b, id)) else {
        return SyncBase::default();
    };
    let stored: Stored = std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    SyncBase {
        files: stored
            .files
            .into_iter()
            .map(|(k, v)| (PathBuf::from(k), v))
            .collect(),
        conflicts: stored.conflicts,
        held: stored.held.into_iter().map(PathBuf::from).collect(),
    }
}

/// Writes atomically; `base` `None` is a no-op.
pub fn save(base: Option<&Path>, id: &str, sync: &SyncBase) -> std::io::Result<()> {
    let Some(file) = base.and_then(|b| path(b, id)) else {
        return Ok(());
    };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let stored = Stored {
        files: sync.files.iter().map(|(k, v)| (key(k), *v)).collect(),
        conflicts: sync.conflicts.clone(),
        held: sync.held.iter().map(|p| key(p)).collect(),
    };
    let json = serde_json::to_string_pretty(&stored).map_err(std::io::Error::other)?;
    super::write_atomic(&file, &json)
}

/// Forgets collection `id`'s base; missing is not an error.
pub fn remove(base: Option<&Path>, id: &str) -> std::io::Result<()> {
    match base.and_then(|b| path(b, id)).map(std::fs::remove_file) {
        Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_round_trips_and_ignores_bad_ids() {
        let d = tempfile::tempdir().unwrap();
        let conflict = Conflict {
            rel: "rates/get.yaml".into(),
            local: "a: 2\n".into(),
            server: "a: 3\n".into(),
            server_author: "Acme".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        let sync = SyncBase {
            files: HashMap::from([(PathBuf::from("rates/get.yaml"), 42u64)]),
            conflicts: vec![conflict],
            held: vec!["rates/get.yaml".into()],
        };
        save(Some(d.path()), "2230f875", &sync).unwrap();
        assert!(d.path().join("sync/2230f875.json").is_file());
        assert_eq!(load(Some(d.path()), "2230f875"), sync);
        assert_eq!(load(Some(d.path()), "other"), SyncBase::default());
        save(Some(d.path()), "../x", &sync).unwrap();
        assert!(!d.path().join("x.json").exists());
        assert_eq!(load(None, "2230f875"), SyncBase::default());
        std::fs::write(d.path().join("sync/bad.json"), "{not json").unwrap();
        assert_eq!(load(Some(d.path()), "bad"), SyncBase::default());
        remove(Some(d.path()), "2230f875").unwrap();
        assert_eq!(load(Some(d.path()), "2230f875"), SyncBase::default());
        remove(Some(d.path()), "2230f875").unwrap();
    }

    #[test]
    fn scoped_ids_differ_per_server_and_account_and_are_safe() {
        let cloud = scoped_id(
            "2230f875",
            "https://cloud.curl-runner.com",
            "jamie@acme.dev",
        );
        assert_eq!(
            cloud,
            scoped_id(
                "2230f875",
                "https://cloud.curl-runner.com",
                "jamie@acme.dev"
            )
        );
        assert_ne!(
            cloud,
            scoped_id(
                "2230f875",
                "https://cloud.curl-runner.com",
                "maria@example.com"
            )
        );
        assert_ne!(
            cloud,
            scoped_id(
                "2230f875",
                "https://curl-runner.example.com",
                "jamie@acme.dev"
            )
        );
        assert!(cloud.starts_with("2230f875-"));
        let d = tempfile::tempdir().unwrap();
        let sync = SyncBase {
            files: HashMap::from([(PathBuf::from("a.yaml"), 1u64)]),
            ..SyncBase::default()
        };
        save(Some(d.path()), &cloud, &sync).unwrap();
        assert_eq!(load(Some(d.path()), &cloud), sync);
    }
}
