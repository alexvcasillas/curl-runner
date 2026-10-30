//! A shared collection made local: the snapshot written as a read-only
//! copy under `{base}/shared/`, its notes, and the customer's own values
//! (secrets to the Keychain, the rest to its `.env`). All or nothing: files
//! are staged in a hidden sibling and moved in with one rename, and a step
//! that fails after it takes everything back out.

use crate::model::backend::{Backend, ReceivedShare, SharedCollection, SharedVar, SyncFile};
use crate::model::collection_meta::{CollectionMeta, VarMeta};
use crate::model::environments::{DEFAULT_ENVIRONMENT, is_secret, write_env_var};
use crate::model::secrets::{SecretStore, service_name};
use crate::model::sync_base::{self, SyncBase};
use std::path::{Component, Path, PathBuf};

/// A path from the server is written only inside the collection, as a
/// visible YAML file: plain components, none hidden.
pub fn writable(rel: &Path) -> bool {
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
pub fn resolves_inside(dir: &Path, real_root: &Path) -> bool {
    let mut probe = Some(dir);
    while let Some(d) = probe {
        if let Ok(real) = d.canonicalize() {
            return real.starts_with(real_root);
        }
        probe = d.parent();
    }
    false
}

/// `Shipping API` → `shipping-api`: lowercase ASCII letters and digits,
/// anything else one `-`.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "shared".into() } else { out }
}

/// Where a share of `org`'s `name` goes: `{shared_dir}/{org}-{name}`, else
/// the first free `-2`, `-3`… (another share took the name).
pub fn destination(shared_dir: &Path, org: &str, name: &str) -> PathBuf {
    let stem = format!("{}-{}", slug(org), slug(name));
    (1..)
        .map(|n| match n {
            1 => shared_dir.join(&stem),
            n => shared_dir.join(format!("{stem}-{n}")),
        })
        .find(|p| !p.exists())
        .expect("a free name")
}

/// The local copy of share `id` this studio already has, if its folder is
/// still there.
pub fn already_added(backend: &dyn Backend, id: &str) -> Option<PathBuf> {
    backend
        .received()
        .into_iter()
        .find(|r| r.id == id && r.root.is_dir())
        .map(|r| r.root)
}

/// `Add to my studio`, start to end, off the UI thread: fetch the snapshot,
/// [`materialize`] it, record the [`ReceivedShare`]; a refused record rolls
/// the copy back. The root of the new copy, or the error to show (never a
/// value).
pub async fn add_shared(
    backend: &dyn Backend,
    secrets: &dyn SecretStore,
    base: &Path,
    shared: &SharedCollection,
    values: &[(String, String)],
) -> Result<PathBuf, String> {
    let org = &shared.owner_org;
    let snapshot = backend
        .fetch_shared(&shared.id)
        .await
        .map_err(|e| e.link_text(org))?;
    let done = materialize(base, shared, &snapshot.files, values, secrets)?;
    let record = ReceivedShare {
        id: shared.id.clone(),
        collection_id: done.id.clone(),
        owner_org: org.clone(),
        name: shared.name.clone(),
        root: done.root.clone(),
        added: chrono::Utc::now(),
    };
    if let Err(e) = backend.add_received(record).await {
        rollback(base, &done, secrets);
        let shared_dir = base.join("shared");
        let _ = std::fs::remove_dir(&shared_dir);
        return Err(format!(
            "{} The folder was removed again.",
            e.link_text(org)
        ));
    }
    Ok(done.root)
}

/// What [`materialize`] wrote: the share record's fields and what a
/// rollback takes out.
#[derive(Debug)]
pub struct Materialized {
    pub root: PathBuf,
    /// The copy's [`CollectionMeta::id`].
    pub id: String,
    service: String,
    /// Names written to `service`.
    secrets: Vec<String>,
}

/// Writes the snapshot `files` of `shared` as a new collection under
/// `{base}/shared/`, then its notes (read only, the variables as shared),
/// the secret `values` to the Keychain and the others to its `.env`.
///
/// Before the rename nothing is left behind on failure; after it,
/// everything written is taken out again. Errors never carry a value.
pub fn materialize(
    base: &Path,
    shared: &SharedCollection,
    files: &[SyncFile],
    values: &[(String, String)],
    secrets: &dyn SecretStore,
) -> Result<Materialized, String> {
    let name = &shared.name;
    if let Some(bad) = files.iter().find(|f| !writable(&f.rel)) {
        return Err(format!(
            "Couldn't add {name}: it has a file the studio won't write ({}). Nothing was added.",
            bad.rel.display()
        ));
    }
    let nothing = |e: std::io::Error| format!("Couldn't add {name}: {e}. Nothing was added.");
    let shared_dir = base.join("shared");
    // Made here, it goes again if nothing ends up in it.
    let made = !shared_dir.exists();
    let unmake = || {
        if made {
            let _ = std::fs::remove_dir(&shared_dir);
        }
    };
    if let Err(e) = std::fs::create_dir_all(&shared_dir) {
        unmake();
        return Err(nothing(e));
    }
    let root = destination(&shared_dir, &shared.owner_org, name);
    let stem = root
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = shared_dir.join(format!(".{stem}.{}.{seq}.tmp", std::process::id()));
    let staged = stage(&tmp, files).and_then(|()| std::fs::rename(&tmp, &root));
    if let Err(e) = staged {
        let _ = std::fs::remove_dir_all(&tmp);
        unmake();
        return Err(nothing(e));
    }
    let mut done = Materialized {
        root,
        id: String::new(),
        service: String::new(),
        secrets: Vec::new(),
    };
    match settle(base, shared, files, values, secrets, &stem, &mut done) {
        Ok(()) => Ok(done),
        Err(why) => {
            rollback(base, &done, secrets);
            unmake();
            Err(format!(
                "Couldn't add {name}: {why}. The folder was removed again."
            ))
        }
    }
}

/// The files, in the fresh folder `tmp`, each checked to stay inside it.
fn stage(tmp: &Path, files: &[SyncFile]) -> std::io::Result<()> {
    std::fs::create_dir(tmp)?;
    let real = tmp.canonicalize()?;
    for f in files {
        let path = tmp.join(&f.rel);
        let dir = path.parent().unwrap_or(tmp);
        std::fs::create_dir_all(dir)?;
        if !resolves_inside(dir, &real) {
            return Err(std::io::Error::other(format!(
                "{} leads outside the collection",
                f.rel.display()
            )));
        }
        std::fs::write(&path, &f.text)?;
    }
    Ok(())
}

/// After the rename: notes, Keychain, `.env`, the sync base. Fills `done`
/// as it goes, so a rollback knows what to take out.
fn settle(
    base: &Path,
    shared: &SharedCollection,
    files: &[SyncFile],
    values: &[(String, String)],
    secrets: &dyn SecretStore,
    folder_name: &str,
    done: &mut Materialized,
) -> Result<(), String> {
    let mut meta = CollectionMeta::load_or_init(Some(base), &done.root);
    done.id = meta.id.clone();
    meta.read_only_from = Some(shared.owner_org.clone());
    meta.received_vars = shared.variables.clone();
    let secret = |v: &SharedVar| v.secret || is_secret(&v.name, &[]);
    for v in &shared.variables {
        if secret(v) {
            meta.set_secret(&v.name, true);
        }
        meta.vars.insert(
            v.name.clone(),
            VarMeta {
                description: v.description.clone(),
                required_for_shared: v.required,
                share_default: false,
            },
        );
    }
    meta.save(base, &done.root)
        .map_err(|e| format!("its notes weren't saved ({e})"))?;
    // The workspace names a collection after its folder.
    done.service = service_name(folder_name, DEFAULT_ENVIRONMENT, &meta.id);
    for (name, value) in values.iter().filter(|(_, v)| !v.is_empty()) {
        let Some(var) = shared.variables.iter().find(|v| &v.name == name) else {
            continue;
        };
        if secret(var) {
            done.secrets.push(name.clone());
            secrets
                .set(&done.service, name, value)
                .map_err(|e| format!("{name} wasn't saved to your keychain ({e})"))?;
        } else {
            write_env_var(&done.root, DEFAULT_ENVIRONMENT, name, value)
                .map_err(|e| format!("{name} wasn't saved to .env ({e})"))?;
        }
    }
    // Synced as of now: the first sync finds nothing waiting.
    let sync = SyncBase {
        files: files
            .iter()
            .map(|f| (f.rel.clone(), SyncFile::hash_of(&f.text)))
            .collect(),
        ..SyncBase::default()
    };
    sync_base::save(Some(base), &meta.id, &sync)
        .map_err(|e| format!("its sync state wasn't saved ({e})"))
}

/// Takes out what [`materialize`] wrote. The notes go first: their file is
/// named after the folder's real path, which can't be worked out once the
/// folder is gone (a symlinked base).
pub fn rollback(base: &Path, done: &Materialized, secrets: &dyn SecretStore) {
    for name in &done.secrets {
        let _ = secrets.delete(&done.service, name);
    }
    let _ = CollectionMeta::remove(base, &done.root);
    if !done.id.is_empty() {
        let _ = sync_base::remove(Some(base), &done.id);
    }
    let _ = std::fs::remove_dir_all(&done.root);
}

#[cfg(test)]
mod tests {
    use super::{
        CollectionMeta, SharedCollection, SyncFile, destination, materialize, rollback,
        service_name, slug, sync_base,
    };
    use crate::model::backend::SharedVar;
    use crate::model::secrets::MemorySecrets;
    use crate::model::secrets::SecretStore as _;
    use chrono::Utc;

    fn shipping() -> SharedCollection {
        SharedCollection {
            id: "acme/shipping-api".into(),
            owner_org: "Acme".into(),
            name: "Shipping API".into(),
            summary: "rates, labels, tracking".into(),
            updated: Utc::now() - chrono::Duration::days(3),
            request_count: 12,
            variables: variables(),
        }
    }

    fn variables() -> Vec<SharedVar> {
        let var = |name: &str, required, secret, default: Option<&str>| SharedVar {
            name: name.into(),
            description: format!("About {name}."),
            required,
            secret,
            default: default.map(String::from),
        };
        vec![
            var(
                "BASE_URL",
                true,
                false,
                Some("https://api.example.com/shipping"),
            ),
            var("API_TOKEN", true, true, None),
            var("ACCOUNT_ID", false, false, None),
        ]
    }

    fn snapshot() -> Vec<SyncFile> {
        ["rates/get-rates.yaml", "labels/create.yaml", "tracking/get.yaml"]
            .into_iter()
            .map(|rel| {
                SyncFile::new(
                    rel,
                    format!("request:\n  url: ${{BASE_URL}}/{rel}\n  headers:\n    Authorization: Bearer ${{API_TOKEN}}\n"),
                )
            })
            .collect()
    }

    #[test]
    fn slugs_and_destinations() {
        assert_eq!(slug("Shipping API"), "shipping-api");
        assert_eq!(slug("  Acme, Inc. "), "acme-inc");
        assert_eq!(slug("ÄÖ"), "shared");
        let d = tempfile::tempdir().unwrap();
        let first = destination(d.path(), "Acme", "Shipping API");
        assert_eq!(first, d.path().join("acme-shipping-api"));
        std::fs::create_dir(&first).unwrap();
        assert_eq!(
            destination(d.path(), "Acme", "Shipping API"),
            d.path().join("acme-shipping-api-2")
        );
    }

    fn values() -> Vec<(String, String)> {
        vec![
            ("BASE_URL".into(), "https://api.example.com/shipping".into()),
            ("API_TOKEN".into(), "sk-customer-1".into()),
            ("ACCOUNT_ID".into(), String::new()),
        ]
    }

    #[test]
    fn materialize_writes_files_notes_and_values_and_rolls_back() {
        let base = tempfile::tempdir().unwrap();
        let secrets = MemorySecrets::default();
        let files = snapshot();
        let done = materialize(base.path(), &shipping(), &files, &values(), &secrets).unwrap();
        assert_eq!(done.root, base.path().join("shared/acme-shipping-api"));
        for f in &files {
            assert_eq!(
                std::fs::read_to_string(done.root.join(&f.rel)).unwrap(),
                f.text
            );
        }
        let meta = CollectionMeta::load(base.path(), &done.root);
        assert_eq!(meta.id, done.id);
        assert_eq!(meta.read_only_from.as_deref(), Some("Acme"));
        assert_eq!(meta.received_vars, variables());
        assert_eq!(meta.secret_names, ["API_TOKEN"]);
        let service = service_name("acme-shipping-api", "Default", &done.id);
        assert_eq!(
            secrets.get(&service, "API_TOKEN").as_deref(),
            Some("sk-customer-1")
        );
        let env = std::fs::read_to_string(done.root.join(".env")).unwrap();
        assert!(
            env.contains("BASE_URL=https://api.example.com/shipping"),
            "{env}"
        );
        assert!(!env.contains("ACCOUNT_ID"), "empty is skipped: {env}");
        assert!(!env.contains("sk-customer-1"));
        assert_eq!(
            sync_base::load(Some(base.path()), &done.id).files.len(),
            files.len()
        );
        // Nothing hidden is left next to it.
        let names: Vec<_> = std::fs::read_dir(base.path().join("shared"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["acme-shipping-api"]);

        rollback(base.path(), &done, &secrets);
        assert!(!done.root.exists());
        assert_eq!(secrets.get(&service, "API_TOKEN"), None);
        assert_eq!(
            CollectionMeta::load(base.path(), &done.root),
            CollectionMeta::default()
        );
        assert!(
            sync_base::load(Some(base.path()), &done.id)
                .files
                .is_empty()
        );
    }

    #[test]
    fn unsafe_snapshot_paths_write_nothing() {
        for bad in [
            "../escape.yaml",
            "/abs.yaml",
            ".hidden/a.yaml",
            "rates/.env",
            "notes.txt",
        ] {
            let base = tempfile::tempdir().unwrap();
            let secrets = MemorySecrets::default();
            let mut files = snapshot();
            files.push(SyncFile::new(bad, "a: 1\n"));
            let err =
                materialize(base.path(), &shipping(), &files, &values(), &secrets).unwrap_err();
            assert!(err.ends_with("Nothing was added."), "{err}");
            assert!(!base.path().join("shared").exists(), "{bad}");
            assert!(!base.path().join("meta").exists(), "{bad}");
        }
    }

    /// The entries of `dir`, none when it is missing.
    fn entries(dir: &std::path::Path) -> Vec<std::ffi::OsString> {
        std::fs::read_dir(dir)
            .map(|d| d.flatten().map(|e| e.file_name()).collect())
            .unwrap_or_default()
    }

    #[cfg(unix)]
    #[test]
    fn rollback_leaves_no_notes_or_sync_state_under_a_symlinked_base() {
        let real = tempfile::tempdir().unwrap();
        let links = tempfile::tempdir().unwrap();
        let base = links.path().join("state");
        std::os::unix::fs::symlink(real.path(), &base).unwrap();
        let secrets = MemorySecrets::default();
        let done = materialize(&base, &shipping(), &snapshot(), &values(), &secrets).unwrap();
        assert_eq!(entries(&real.path().join("meta")).len(), 1);
        assert_eq!(entries(&real.path().join("sync")).len(), 1);
        rollback(&base, &done, &secrets);
        assert!(entries(&real.path().join("meta")).is_empty(), "notes left");
        assert!(
            entries(&real.path().join("sync")).is_empty(),
            "sync state left"
        );
        assert!(entries(&real.path().join("shared")).is_empty());
        assert!(secrets.names(&done_service(&done)).is_empty());
    }

    fn done_service(done: &super::Materialized) -> String {
        service_name("acme-shipping-api", "Default", &done.id)
    }

    #[test]
    fn failed_staging_leaves_no_shared_folder() {
        let base = tempfile::tempdir().unwrap();
        let secrets = MemorySecrets::default();
        // A file and a folder by the same name: the second can't be made.
        let files = vec![
            SyncFile::new("rates.yaml", "a: 1\n"),
            SyncFile::new("rates.yaml/get.yaml", "a: 1\n"),
        ];
        let err = materialize(base.path(), &shipping(), &files, &values(), &secrets).unwrap_err();
        assert!(err.ends_with("Nothing was added."), "{err}");
        assert!(
            entries(base.path()).is_empty(),
            "{:?}",
            entries(base.path())
        );
    }
}
