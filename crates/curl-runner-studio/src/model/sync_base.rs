//! The client's side of sync: per collection, the hash of each file as this
//! studio last synced it, in `{base}/sync/{collection id}.json`. It tells
//! pending changes (hash differs from base), deletions (in base, gone
//! locally) and lets the server spot conflicts (its copy is not the base).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Relative path → hash last synced.
pub type SyncBase = HashMap<PathBuf, u64>;

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

/// The base of collection `id`; missing or unreadable is empty (never
/// synced). `base` `None` keeps nothing.
pub fn load(base: Option<&Path>, id: &str) -> SyncBase {
    let Some(file) = base.and_then(|b| path(b, id)) else {
        return SyncBase::new();
    };
    std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, u64>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (PathBuf::from(k), v))
        .collect()
}

/// Writes atomically; `base` `None` is a no-op.
pub fn save(base: Option<&Path>, id: &str, map: &SyncBase) -> std::io::Result<()> {
    let Some(file) = base.and_then(|b| path(b, id)) else {
        return Ok(());
    };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let sorted: BTreeMap<String, u64> = map.iter().map(|(k, v)| (key(k), *v)).collect();
    let json = serde_json::to_string_pretty(&sorted).map_err(std::io::Error::other)?;
    super::write_atomic(&file, &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_round_trips_and_ignores_bad_ids() {
        let d = tempfile::tempdir().unwrap();
        let map = SyncBase::from([(PathBuf::from("rates/get.yaml"), 42u64)]);
        save(Some(d.path()), "2230f875", &map).unwrap();
        assert!(d.path().join("sync/2230f875.json").is_file());
        assert_eq!(load(Some(d.path()), "2230f875"), map);
        assert!(load(Some(d.path()), "other").is_empty());
        save(Some(d.path()), "../x", &map).unwrap();
        assert!(!d.path().join("x.json").exists());
        assert!(load(None, "2230f875").is_empty());
        std::fs::write(d.path().join("sync/bad.json"), "{not json").unwrap();
        assert!(load(Some(d.path()), "bad").is_empty());
    }
}
