//! What the studio knows about a collection beyond its files: its id, which
//! names are secret and each variable's description. Kept on this device
//! under `{base}/meta/{root hash}.json`, never in the collection.

use crate::model::backend::SharedVar;
use crate::model::environments::is_secret;
use crate::model::history::root_hash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// One variable's notes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VarMeta {
    /// Shown to the people the collection is shared with.
    pub description: String,
    /// They are asked for it before their first run.
    pub required_for_shared: bool,
    /// Share the Default environment's (`.env`) value with the collection
    /// (non-secret names only; a secret never shares one).
    pub share_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CollectionMeta {
    /// Stable id (8 hex chars) in the collection's Keychain services, so two
    /// collections with the same name never share secrets; empty until
    /// [`CollectionMeta::load_or_init`] picks it.
    pub id: String,
    /// Names marked secret beyond the `SECRET_` prefix, sorted.
    pub secret_names: Vec<String>,
    pub vars: BTreeMap<String, VarMeta>,
    /// The environment the collection was last set up for, if any.
    pub environment: Option<String>,
    /// A received copy: the org that shared it. Read only.
    pub read_only_from: Option<String>,
    /// A received copy's variables as the owner shared them: their
    /// non-secret defaults are `Shared by {org}` and locked.
    pub received_vars: Vec<SharedVar>,
}

impl CollectionMeta {
    /// The collection at `root`'s notes; missing or unreadable is empty.
    pub fn load(base: &Path, root: &Path) -> CollectionMeta {
        std::fs::read(meta_path(base, root))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// [`CollectionMeta::load`], with an id: a new one is picked and saved
    /// on first use. With no state dir (`base` is `None`) the id is derived
    /// from the root, so it is still stable on this device.
    pub fn load_or_init(base: Option<&Path>, root: &Path) -> CollectionMeta {
        let Some(base) = base else {
            return CollectionMeta {
                id: root_hash(root)[..8].to_string(),
                ..Default::default()
            };
        };
        let mut meta = CollectionMeta::load(base, root);
        if meta.id.is_empty() {
            meta.id = new_id(root);
            // Unsaved, it still holds for this session; the next launch
            // picks again (and finds this session's Keychain items gone).
            let _ = meta.save(base, root);
        }
        meta
    }

    /// Writes atomically: a temp file, then a rename.
    pub fn save(&self, base: &Path, root: &Path) -> io::Result<()> {
        let path = meta_path(base, root);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        crate::model::write_atomic(&path, &json)
    }

    /// Forgets the notes of the collection at `root` (a received copy whose
    /// add was rolled back). A missing file is not an error.
    pub fn remove(base: &Path, root: &Path) -> io::Result<()> {
        match std::fs::remove_file(meta_path(base, root)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// The names whose Default value is shared, secrets left out.
    pub fn share_defaults(&self) -> Vec<String> {
        self.vars
            .iter()
            .filter(|(name, v)| v.share_default && !is_secret(name, &self.secret_names))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// A received copy's locked default for `name`: `(org, value)`.
    pub fn shared_by(&self, name: &str) -> Option<(&str, &str)> {
        let org = self.read_only_from.as_deref()?;
        self.received_vars
            .iter()
            .find(|v| v.name == name && !v.secret && !is_secret(name, &self.secret_names))
            .and_then(|v| v.default.as_deref())
            .map(|value| (org, value))
    }

    /// Marks or unmarks `name` as secret (a `SECRET_` name always is). A
    /// secret never shares its default: marking drops `share_default`.
    pub fn set_secret(&mut self, name: &str, secret: bool) {
        self.secret_names.retain(|n| n != name);
        if secret && !name.starts_with("SECRET_") {
            self.secret_names.push(name.to_string());
            self.secret_names.sort();
        }
        if secret && let Some(v) = self.vars.get_mut(name) {
            v.share_default = false;
        }
    }
}

/// 8 hex chars from the clock, the process and `root` (FNV-1a): random
/// enough to tell this device's collections apart.
fn new_id(root: &Path) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let seed = format!(
        "{nanos}:{}:{}:{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed),
        root.display()
    );
    let h = seed.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{:08x}", (h ^ (h >> 32)) as u32)
}

fn meta_path(base: &Path, root: &Path) -> PathBuf {
    base.join("meta").join(format!("{}.json", root_hash(root)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_round_trips_per_collection() {
        let base = tempfile::tempdir().unwrap();
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        assert_eq!(
            CollectionMeta::load(base.path(), a.path()),
            CollectionMeta::default()
        );
        let mut meta = CollectionMeta::default();
        meta.set_secret("API_TOKEN", true);
        meta.set_secret("SECRET_KEY", true);
        meta.vars.insert(
            "API_TOKEN".into(),
            VarMeta {
                description: "Your API key.".into(),
                required_for_shared: true,
                share_default: false,
            },
        );
        meta.save(base.path(), a.path()).unwrap();
        let loaded = CollectionMeta::load(base.path(), a.path());
        assert_eq!(loaded, meta);
        assert_eq!(loaded.secret_names, ["API_TOKEN"], "SECRET_ needs no mark");
        assert_eq!(
            CollectionMeta::load(base.path(), b.path()),
            CollectionMeta::default()
        );
        let file = base
            .path()
            .join("meta")
            .join(format!("{}.json", root_hash(a.path())));
        assert!(file.is_file());
        CollectionMeta::remove(base.path(), a.path()).unwrap();
        assert!(!file.exists());
        CollectionMeta::remove(base.path(), a.path()).unwrap();
        let mut m = loaded;
        m.set_secret("API_TOKEN", false);
        assert!(m.secret_names.is_empty());
    }

    #[test]
    fn id_is_picked_once_and_differs_per_collection() {
        let base = tempfile::tempdir().unwrap();
        let parent = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        // Same basename, two folders.
        let (a, b) = (parent.0.path().join("api"), parent.1.path().join("api"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let first = CollectionMeta::load_or_init(Some(base.path()), &a);
        assert_eq!(first.id.len(), 8);
        assert!(first.id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(CollectionMeta::load(base.path(), &a).id, first.id, "saved");
        assert_eq!(
            CollectionMeta::load_or_init(Some(base.path()), &a).id,
            first.id,
            "stable"
        );
        assert_ne!(
            CollectionMeta::load_or_init(Some(base.path()), &b).id,
            first.id
        );
        // No state dir: still stable per root.
        assert_eq!(
            CollectionMeta::load_or_init(None, &a).id,
            CollectionMeta::load_or_init(None, &a).id
        );
        assert_ne!(
            CollectionMeta::load_or_init(None, &a).id,
            CollectionMeta::load_or_init(None, &b).id
        );
    }

    #[test]
    fn sharing_fields_default_and_round_trip() {
        // Notes saved before sharing existed still load.
        let old: CollectionMeta =
            serde_json::from_str(r#"{"id":"0a1b2c3d","vars":{"BASE_URL":{"description":"x"}}}"#)
                .unwrap();
        assert!(!old.vars["BASE_URL"].share_default);
        assert_eq!(old.read_only_from, None);
        assert!(old.received_vars.is_empty());
        let mut meta = CollectionMeta::default();
        meta.set_secret("API_TOKEN", true);
        for name in ["BASE_URL", "API_TOKEN", "SECRET_KEY", "ACCOUNT_ID"] {
            meta.vars.insert(
                name.into(),
                VarMeta {
                    share_default: name != "ACCOUNT_ID",
                    ..VarMeta::default()
                },
            );
        }
        assert_eq!(meta.share_defaults(), ["BASE_URL"], "never a secret");
        meta.read_only_from = Some("Acme".into());
        meta.received_vars = vec![
            SharedVar {
                name: "CURRENCY".into(),
                description: String::new(),
                required: false,
                secret: false,
                default: Some("eur".into()),
            },
            SharedVar {
                name: "API_TOKEN".into(),
                description: String::new(),
                required: true,
                secret: true,
                default: Some("leak".into()),
            },
        ];
        assert_eq!(meta.shared_by("CURRENCY"), Some(("Acme", "eur")));
        assert_eq!(meta.shared_by("API_TOKEN"), None, "a secret is never shown");
        assert_eq!(meta.shared_by("BASE_URL"), None);
        meta.received_vars[0].name = "SECRET_CURRENCY".into();
        assert_eq!(meta.shared_by("SECRET_CURRENCY"), None, "secret here");
        meta.received_vars[0].name = "CURRENCY".into();
        let base = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        meta.save(base.path(), root.path()).unwrap();
        assert_eq!(CollectionMeta::load(base.path(), root.path()), meta);
        meta.read_only_from = None;
        assert_eq!(meta.shared_by("CURRENCY"), None, "only received copies");
    }

    #[test]
    fn corrupt_meta_is_empty() {
        let base = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let path = meta_path(base.path(), root.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{nope").unwrap();
        assert_eq!(
            CollectionMeta::load(base.path(), root.path()),
            CollectionMeta::default()
        );
    }

    #[test]
    fn marking_secret_drops_share_default() {
        let mut meta = CollectionMeta::default();
        meta.vars.insert(
            "ACCOUNT_ID".into(),
            VarMeta {
                description: "Yours".into(),
                share_default: true,
                ..VarMeta::default()
            },
        );
        meta.set_secret("ACCOUNT_ID", true);
        assert!(!meta.vars["ACCOUNT_ID"].share_default);
        assert_eq!(meta.vars["ACCOUNT_ID"].description, "Yours");
        meta.set_secret("ACCOUNT_ID", false);
        assert!(!meta.vars["ACCOUNT_ID"].share_default, "not re-ticked");
    }
}
