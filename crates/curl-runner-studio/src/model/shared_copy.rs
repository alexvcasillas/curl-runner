//! A shared collection made local: the snapshot written as the customer's
//! own copy under `{base}/shared/`, its notes, and the customer's own values
//! (secrets to the Keychain, the rest to its `.env`). All or nothing: files
//! are staged in a hidden sibling and moved in with one rename, and a step
//! that fails after it takes everything back out.

use crate::model::backend::{Backend, ReceivedShare, SharedCollection, SharedVar, SyncFile};
use crate::model::collection_meta::{CollectionMeta, VarMeta};
use crate::model::environments::{DEFAULT_ENVIRONMENT, is_secret, write_env_var};
use crate::model::received;
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

/// Whether `dir` is (or is inside) a received copy: its notes name the
/// org that shared it, or it sits under `{base}/shared/` (canonicalized,
/// so a symlink or `..` can't hide it). The org, or `its owner` when only
/// the location says so. Every write into a folder the user picked asks
/// this first.
pub fn received_from(base: Option<&Path>, dir: &Path) -> Option<String> {
    let base = base?;
    // The copy's own notes, or those of a copy it sits in.
    if let Some(org) = dir
        .ancestors()
        .find_map(|d| CollectionMeta::load(base, d).read_only_from)
    {
        return Some(org);
    }
    let shared = base.join("shared").canonicalize().ok()?;
    // A folder not made yet is judged by its deepest existing ancestor.
    let real = dir.ancestors().find_map(|d| d.canonicalize().ok())?;
    real.starts_with(&shared).then(|| "its owner".to_string())
}

/// What a write into the studio's own folder is refused with.
pub const STATE_FOLDER_TEXT: &str = "That folder holds the studio's data. Pick another one.";

/// `dir` is the studio's data folder `{base}`, sits inside it, or holds it
/// (any ancestor of `{base}/shared`), canonicalized: as a collection it
/// would take in the studio's own files. A folder not made yet is judged
/// by its deepest existing ancestor for "inside"; it holds nothing.
pub fn holds_studio_state(base: Option<&Path>, dir: &Path) -> bool {
    let Some(base) = base else { return false };
    let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let inside = dir
        .ancestors()
        .find_map(|d| d.canonicalize().ok())
        .is_some_and(|real| real.starts_with(&base));
    let holds = dir
        .canonicalize()
        .is_ok_and(|real| base.join("shared").starts_with(&real));
    inside || holds
}

/// [`received_from`] as a yes or no.
pub fn is_received(base: Option<&Path>, dir: &Path) -> bool {
    received_from(base, dir).is_some()
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

/// The local copy of the share with `token` this studio already has: it
/// is recorded, and this Mac has its folder.
pub fn already_added(backend: &dyn Backend, base: Option<&Path>, token: &str) -> Option<PathBuf> {
    received::join(backend.received(), &received::load(base))
        .into_iter()
        .find(|r| r.share.link.token == token)
        .map(|r| r.root)
}

/// `Add to my studio`, start to end, off the UI thread: fetch the snapshot,
/// [`materialize`] it, note the copy here ([`received::record`]: token →
/// folder), then record the [`ReceivedShare`] (its link, team, name, the
/// version and the hash of each file as received: never the folder). A
/// refused record rolls the copy back. The root of the new copy, or the
/// error to show (never a value).
pub async fn add_shared(
    backend: &dyn Backend,
    secrets: &dyn SecretStore,
    base: &Path,
    shared: &SharedCollection,
    values: &[(String, String)],
) -> Result<PathBuf, String> {
    let org = &shared.owner_org;
    let snapshot = backend
        .fetch_shared(&shared.link)
        .await
        .map_err(|e| e.link_text(org))?;
    // The copy is of the version fetched, whatever the review showed. A
    // variable masked in the form stays secret: never written to `.env`.
    let shared = SharedCollection {
        variables: secret_in_either(snapshot.variables, &shared.variables),
        updated: snapshot.published_at,
        version: snapshot.version,
        ..shared.clone()
    };
    let done = materialize(base, &shared, &snapshot.files, values, secrets)?;
    let token = &shared.link.token;
    let local = received::LocalCopy {
        share_id: token.clone(),
        root: done.root.clone(),
        collection_id: done.id.clone(),
    };
    if let Err(e) = received::record(base, local) {
        rollback(base, &done, secrets);
        let _ = std::fs::remove_dir(base.join("shared"));
        return Err(format!(
            "Couldn't add {}: {e}. The folder was removed again.",
            shared.name
        ));
    }
    let record = received_record(&shared, &snapshot.files);
    if let Err(e) = backend.add_received(record).await {
        let _ = received::forget(base, token);
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

/// `vars` with each one secret in either list marked secret: a value typed
/// into a masked field (the form's list) must never land in `.env`, even if
/// the list fetched again (`vars`) says otherwise.
pub fn secret_in_either(mut vars: Vec<SharedVar>, seen: &[SharedVar]) -> Vec<SharedVar> {
    for v in &mut vars {
        v.secret |= seen.iter().any(|s| s.name == v.name && s.secret);
    }
    vars
}

/// The record of `shared` as added today with `files`: each file's
/// [`SyncFile::hash_of`] as received, so local edits show later.
fn received_record(shared: &SharedCollection, files: &[SyncFile]) -> ReceivedShare {
    ReceivedShare {
        link: shared.link.clone(),
        team: shared.owner_org.clone(),
        name: shared.name.clone(),
        version: shared.version,
        added: chrono::Local::now().date_naive(),
        files: files
            .iter()
            .map(|f| (f.rel.clone(), SyncFile::hash_of(&f.text)))
            .collect(),
    }
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
/// `{base}/shared/`, then its notes (the sharing org, the variables as shared),
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
    write_files(tmp, files)
}

/// The files into the folder `tmp`, each checked to stay inside it.
fn write_files(tmp: &Path, files: &[SyncFile]) -> std::io::Result<()> {
    let real = tmp.canonicalize()?;
    for f in files {
        let path = tmp.join(&f.rel);
        let dir = path.parent().unwrap_or(tmp);
        // Checked before a folder is made and again once it exists, so a
        // symlinked folder carried over never leads a write outside.
        let outside =
            || std::io::Error::other(format!("{} leads outside the collection", f.rel.display()));
        if !resolves_inside(dir, &real) {
            return Err(outside());
        }
        std::fs::create_dir_all(dir)?;
        if !resolves_inside(dir, &real) {
            return Err(outside());
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
    meta.received_updated = Some(shared.updated);
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
                share_locked: false,
            },
        );
    }
    meta.save(base, &done.root)
        .map_err(|e| format!("its notes weren't saved ({e})"))?;
    // The workspace names a collection after its folder.
    done.service = service_name(folder_name, DEFAULT_ENVIRONMENT, &meta.id);
    write_values(
        &done.root,
        &done.service,
        &shared.variables,
        values,
        secrets,
        &mut done.secrets,
    )?;
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

/// The customer's `values` for `vars`: a secret one to the Keychain under
/// `service` (its name pushed to `saved`), any other to `root`'s `.env`.
/// Empty values, names not in `vars` and locked defaults are skipped. The
/// error never carries a value.
fn write_values(
    root: &Path,
    service: &str,
    vars: &[SharedVar],
    values: &[(String, String)],
    secrets: &dyn SecretStore,
    saved: &mut Vec<String>,
) -> Result<(), String> {
    let secret = |v: &SharedVar| v.secret || is_secret(&v.name, &[]);
    for (name, value) in values.iter().filter(|(_, v)| !v.is_empty()) {
        let Some(var) = vars.iter().find(|v| &v.name == name) else {
            continue;
        };
        // A locked default is the owner's: nothing local stands in for it.
        if var.locked && !secret(var) {
            continue;
        }
        if secret(var) {
            saved.push(name.clone());
            secrets
                .set(service, name, value)
                .map_err(|e| format!("{name} wasn't saved to your keychain ({e})"))?;
        } else {
            write_env_var(root, DEFAULT_ENVIRONMENT, name, value)
                .map_err(|e| format!("{name} wasn't saved to .env ({e})"))?;
        }
    }
    Ok(())
}

/// After an update, the values for the inputs it newly requires, into the
/// copy at `root` (Keychain `service`, else its `.env`), as [`materialize`]
/// writes them. `vars` are the inputs as the form showed them: a field it
/// masked is secret here too.
pub fn save_inputs(
    root: &Path,
    service: &str,
    vars: &[SharedVar],
    values: &[(String, String)],
    secrets: &dyn SecretStore,
) -> Result<(), String> {
    write_values(root, service, vars, values, secrets, &mut Vec::new())
        .map_err(|why| format!("Couldn't save: {why}."))
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

/// What `Check for updates` would change in a received copy: the owner's
/// snapshot against the YAML on disk.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UpdatePlan {
    pub added: Vec<PathBuf>,
    pub changed: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    /// The snapshot: what the copy holds once updated.
    pub files: Vec<SyncFile>,
}

impl UpdatePlan {
    /// Files the update touches.
    pub fn count(&self) -> usize {
        self.added.len() + self.changed.len() + self.removed.len()
    }
}

/// Compares the owner's `files` with the copy at `root`. A snapshot with a
/// path the studio won't write is refused whole.
pub fn plan_update(root: &Path, files: Vec<SyncFile>) -> Result<UpdatePlan, String> {
    if let Some(bad) = files.iter().find(|f| !writable(&f.rel)) {
        return Err(format!(
            "The update has a file the studio won't write ({}). Nothing was changed.",
            bad.rel.display()
        ));
    }
    let scan = crate::model::backend::scan_sync_files(root);
    if !scan.root_ok {
        return Err("Couldn't read the collection folder. Nothing was changed.".into());
    }
    let local: std::collections::HashMap<&Path, &str> = scan
        .files
        .iter()
        .map(|f| (f.rel.as_path(), f.hash.as_str()))
        .collect();
    let mut plan = UpdatePlan::default();
    for f in &files {
        match local.get(f.rel.as_path()) {
            None if scan.too_big.contains(&f.rel) || scan.unreadable.contains(&f.rel) => {
                plan.changed.push(f.rel.clone())
            }
            None => plan.added.push(f.rel.clone()),
            Some(hash) if *hash != f.hash => plan.changed.push(f.rel.clone()),
            Some(_) => {}
        }
    }
    let theirs: std::collections::HashSet<&Path> = files.iter().map(|f| f.rel.as_path()).collect();
    plan.removed = scan
        .files
        .iter()
        .map(|f| f.rel.clone())
        .filter(|rel| !theirs.contains(rel.as_path()) && writable(rel))
        .collect();
    plan.files = files;
    Ok(plan)
}

/// Replaces the copy at `root` with `plan`'s snapshot, all or nothing: a
/// hidden sibling takes everything of the copy's own (`.env`, any file
/// that isn't a replaced YAML) plus the snapshot, each path checked to stay
/// inside, then moves into place by rename. Until then `root` is untouched;
/// a failed swap moves the old folder back. Never the Keychain. The caller
/// makes the snapshot the copy's sync base. The error never carries a value.
pub fn apply_update(root: &Path, plan: &UpdatePlan) -> Result<(), String> {
    let nothing = |e: &dyn std::fmt::Display| format!("Couldn't update: {e}. Nothing was changed.");
    if let Some(bad) = plan.files.iter().find(|f| !writable(&f.rel)) {
        return Err(format!(
            "Couldn't update {}: it leads outside the collection. Nothing was changed.",
            bad.rel.display()
        ));
    }
    let (Some(parent), Some(stem)) = (root.parent(), root.file_name()) else {
        return Err(nothing(&"the collection has no folder"));
    };
    let stem = stem.to_string_lossy();
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let pid = std::process::id();
    let tmp = parent.join(format!(".{stem}.update.{pid}.{seq}.tmp"));
    let old = parent.join(format!(".{stem}.old.{pid}.{seq}.tmp"));
    // Replaced by the snapshot, or gone with it: not carried over.
    let replaced: std::collections::HashSet<&Path> = plan
        .files
        .iter()
        .map(|f| f.rel.as_path())
        .chain(plan.removed.iter().map(PathBuf::as_path))
        .collect();
    let staged = std::fs::create_dir(&tmp)
        .and_then(|()| carry_over(root, &tmp, Path::new(""), &replaced))
        .and_then(|()| write_files(&tmp, &plan.files));
    if let Err(e) = staged {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(nothing(&e));
    }
    swap_in(&tmp, root, &old, |from, to| std::fs::rename(from, to))
}

/// Moves `root` aside to `old` and the staged `tmp` into its place with
/// `rename`, then drops `old`. A failed second move puts `old` back; should
/// that fail too, the error says where the copy is now.
fn swap_in(
    tmp: &Path,
    root: &Path,
    old: &Path,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), String> {
    let nothing = |e: &std::io::Error| format!("Couldn't update: {e}. Nothing was changed.");
    if let Err(e) = rename(root, old) {
        let _ = std::fs::remove_dir_all(tmp);
        return Err(nothing(&e));
    }
    if let Err(e) = rename(tmp, root) {
        let _ = std::fs::remove_dir_all(tmp);
        // Back as it was.
        return match rename(old, root) {
            Ok(()) => Err(nothing(&e)),
            Err(_) => Err(format!(
                "Couldn't update: {e}. Your copy couldn't be moved back; it is in {}.",
                old.display()
            )),
        };
    }
    let _ = std::fs::remove_dir_all(old);
    Ok(())
}

/// Copies `from/rel` into `to/rel`, leaving out the `skip` paths (relative
/// to the copy). A symlink stays a link, never followed.
fn carry_over(
    from: &Path,
    to: &Path,
    rel: &Path,
    skip: &std::collections::HashSet<&Path>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(from.join(rel))? {
        let entry = entry?;
        let rel = rel.join(entry.file_name());
        if skip.contains(rel.as_path()) {
            continue;
        }
        let kind = entry.file_type()?;
        let (src, dst) = (from.join(&rel), to.join(&rel));
        if kind.is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(std::fs::read_link(&src)?, &dst)?;
        } else if kind.is_dir() {
            std::fs::create_dir(&dst)?;
            carry_over(from, to, &rel, skip)?;
        } else if kind.is_file() {
            std::fs::copy(&src, &dst)?;
        }
        // Anything else (a socket, a pipe) holds no data of the copy's.
    }
    Ok(())
}

/// `Duplicate to my workspace`: the copy's YAML files into `dest` (hidden
/// entries skipped, nothing overwritten: a taken name gets `-2`), then
/// `dest`'s own notes, editable, with the copy's secret names and
/// variable descriptions. Never its `.env` or a Keychain value: the user
/// enters their own. How many files were copied.
pub fn duplicate(
    base: Option<&Path>,
    from: &Path,
    meta: &CollectionMeta,
    dest: &Path,
) -> Result<usize, String> {
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let (real_from, real_dest) = (real(from), real(dest));
    if real_dest.starts_with(&real_from) || real_from.starts_with(&real_dest) {
        return Err("Pick a folder outside the shared collection.".into());
    }
    if is_received(base, dest) {
        return Err("That folder is a shared collection. Pick another one.".into());
    }
    if holds_studio_state(base, dest) {
        return Err(STATE_FOLDER_TEXT.into());
    }
    let mut notes = CollectionMeta::load_or_init(base, dest);
    let plan = crate::model::import::plan_yaml_copy(from, Some(dest), Path::new(""))
        .map_err(|e| format!("Couldn't read {}: {e}", from.display()))?;
    let (copied, failed) = crate::model::import::copy_planned(dest, &plan);
    for name in &meta.secret_names {
        notes.set_secret(name, true);
    }
    for (name, v) in &meta.vars {
        notes.vars.entry(name.clone()).or_insert_with(|| VarMeta {
            description: v.description.clone(),
            required_for_shared: v.required_for_shared,
            share_default: false,
            share_locked: false,
        });
    }
    if let Some(base) = base {
        notes
            .save(base, dest)
            .map_err(|e| format!("Copied {copied} files, but their notes weren't saved ({e})"))?;
    }
    match failed {
        Some(e) => Err(format!("Copied {copied} of {} files: {e}", plan.len())),
        None => Ok(copied),
    }
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
            link: crate::model::backend::ShareLink {
                server: "https://api.example.test".into(),
                token: "crsl_ship".into(),
            },
            owner_org: "Acme".into(),
            team_slug: "acme".into(),
            name: "Shipping API".into(),
            summary: "rates, labels, tracking".into(),
            updated: Utc::now() - chrono::Duration::days(3),
            request_count: 12,
            version: 1,
            access: crate::model::backend::LinkAccess::Public,
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
            locked: false,
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
    fn add_shared_records_version_and_file_hashes() {
        use crate::model::backend::{ShareLink, fixtures, test_backend};
        let d = tempfile::tempdir().unwrap();
        let backend = test_backend();
        let secrets = MemorySecrets::default();
        let link = ShareLink {
            server: "https://api.mock.curl-runner.test".into(),
            token: fixtures::SHIPPING_TOKEN.into(),
        };
        let shared = futures::executor::block_on(backend.resolve_link(&link)).unwrap();
        let values = [("API_TOKEN".to_string(), "sk-live-4242".to_string())];
        let root = futures::executor::block_on(super::add_shared(
            &*backend,
            &secrets,
            d.path(),
            &shared,
            &values,
        ))
        .unwrap();
        let received = backend.received();
        assert_eq!(received.len(), 1);
        let r = &received[0];
        assert_eq!(r.link, link);
        assert_eq!((r.team.as_str(), r.name.as_str()), ("Acme", "Shipping API"));
        assert_eq!(r.version, fixtures::SHIPPING_VERSION);
        assert_eq!(r.added, chrono::Local::now().date_naive());
        let want: std::collections::BTreeMap<_, _> = fixtures::shipping_snapshot()
            .into_iter()
            .map(|f| (f.rel, f.hash))
            .collect();
        assert_eq!(r.files, want);
        assert!(!want.is_empty());
        // Hashed as received: the same FNV hash as the file on disk.
        for (rel, hash) in &r.files {
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            assert_eq!(SyncFile::hash_of(&text), *hash);
        }
        // The secret went to the Keychain only: no record or file has it.
        let json = serde_json::to_string(&received).unwrap();
        assert!(!json.contains("sk-live-4242"));
        assert!(super::already_added(&*backend, Some(d.path()), &link.token).is_some());
    }

    /// The form masked ACCOUNT_ID (secret when reviewed); the snapshot
    /// fetched at Add says it isn't. A masked value never lands in `.env`.
    #[test]
    fn a_value_masked_at_review_stays_out_of_env() {
        use crate::model::backend::{ShareLink, fixtures, test_backend};
        let d = tempfile::tempdir().unwrap();
        let backend = test_backend();
        let secrets = MemorySecrets::default();
        let link = ShareLink {
            server: "https://api.mock.curl-runner.test".into(),
            token: fixtures::SHIPPING_TOKEN.into(),
        };
        let mut shared = futures::executor::block_on(backend.resolve_link(&link)).unwrap();
        for v in &mut shared.variables {
            if v.name == "ACCOUNT_ID" {
                v.secret = true;
            }
        }
        let values = [
            ("API_TOKEN".to_string(), "sk-live-4242".to_string()),
            ("ACCOUNT_ID".to_string(), "acct-masked-77".to_string()),
        ];
        let root = futures::executor::block_on(super::add_shared(
            &*backend,
            &secrets,
            d.path(),
            &shared,
            &values,
        ))
        .unwrap();
        let env = std::fs::read_to_string(root.join(".env")).unwrap_or_default();
        assert!(!env.contains("acct-masked-77"), "{env}");
        let meta = CollectionMeta::load(d.path(), &root);
        assert!(meta.secret_names.iter().any(|n| n == "ACCOUNT_ID"));
        let service = service_name("acme-shipping-api", "Default", &meta.id);
        assert_eq!(
            secrets.get(&service, "ACCOUNT_ID").as_deref(),
            Some("acct-masked-77")
        );
    }

    #[test]
    fn the_studio_folder_and_its_ancestors_hold_state() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().join("a/studio");
        std::fs::create_dir_all(base.join("shared/acme-x")).unwrap();
        std::fs::create_dir_all(d.path().join("a/other")).unwrap();
        let holds = |dir: &std::path::Path| super::holds_studio_state(Some(&base), dir);
        assert!(holds(&base));
        assert!(holds(&d.path().join("a")));
        assert!(holds(d.path()));
        assert!(holds(&base.join("shared")));
        assert!(holds(&d.path().join("a/other/..")), "canonicalized");
        assert!(holds(&base.join("shared/acme-x")), "inside");
        assert!(holds(&base.join("history")), "inside, not made yet");
        assert!(!holds(&d.path().join("a/other")));
        assert!(!holds(&d.path().join("a/new")), "not made yet");
        assert!(!super::holds_studio_state(None, d.path()));
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

    /// An update that fails partway leaves the copy exactly as it was,
    /// with nothing hidden left beside it; one that lands keeps the
    /// copy's own files (`.env`, notes of the user's) and swaps the YAML.
    #[test]
    fn an_update_swaps_the_whole_folder_or_changes_nothing() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("acme-shipping-api");
        std::fs::create_dir_all(root.join("rates")).unwrap();
        std::fs::write(root.join("a.yaml"), "old a\n").unwrap();
        std::fs::write(root.join("rates/b.yaml"), "old b\n").unwrap();
        std::fs::write(root.join("gone.yaml"), "gone\n").unwrap();
        std::fs::write(root.join(".env"), "API=1\n").unwrap();
        std::fs::write(root.join("notes.txt"), "mine\n").unwrap();
        let before = super::super::backend::sync_files(&root);
        // `a.yaml` both a file and a folder: the second write fails.
        let bad = super::plan_update(
            &root,
            vec![
                SyncFile::new("a.yaml", "new a\n"),
                SyncFile::new("a.yaml/c.yaml", "c\n"),
            ],
        )
        .unwrap();
        assert!(super::apply_update(&root, &bad).is_err());
        assert_eq!(super::super::backend::sync_files(&root), before);
        assert_eq!(
            std::fs::read_to_string(root.join(".env")).unwrap(),
            "API=1\n"
        );
        assert_eq!(entries(parent.path()), ["acme-shipping-api"]);

        let plan = super::plan_update(
            &root,
            vec![
                SyncFile::new("a.yaml", "new a\n"),
                SyncFile::new("rates/b.yaml", "old b\n"),
                SyncFile::new("labels/new.yaml", "new\n"),
            ],
        )
        .unwrap();
        super::apply_update(&root, &plan).unwrap();
        let read = |rel: &str| std::fs::read_to_string(root.join(rel)).unwrap();
        assert_eq!(read("a.yaml"), "new a\n");
        assert_eq!(read("rates/b.yaml"), "old b\n");
        assert_eq!(read("labels/new.yaml"), "new\n");
        assert!(!root.join("gone.yaml").exists());
        assert_eq!(read(".env"), "API=1\n");
        assert_eq!(read("notes.txt"), "mine\n");
        assert_eq!(entries(parent.path()), ["acme-shipping-api"]);
    }

    /// The swap's last step fails and so does putting the copy back: the
    /// error says where the copy is, never "Nothing was changed".
    #[test]
    fn a_swap_that_cant_move_back_says_where_the_copy_is() {
        let parent = tempfile::tempdir().unwrap();
        let (root, tmp, old) = (
            parent.path().join("copy"),
            parent.path().join(".copy.update.tmp"),
            parent.path().join(".copy.old.tmp"),
        );
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("a.yaml"), "a\n").unwrap();
        std::fs::create_dir(&tmp).unwrap();
        let calls = std::cell::Cell::new(0);
        let err = super::swap_in(&tmp, &root, &old, |from, to| {
            calls.set(calls.get() + 1);
            match calls.get() {
                1 => std::fs::rename(from, to),
                _ => Err(std::io::Error::other("disk full")),
            }
        })
        .unwrap_err();
        assert!(!err.contains("Nothing was changed"), "{err}");
        assert!(err.contains(&old.display().to_string()), "{err}");
        assert!(old.join("a.yaml").exists(), "the copy is where it says");
        // One failure only: back in place, nothing changed.
        std::fs::rename(&old, &root).unwrap();
        std::fs::create_dir(&tmp).unwrap();
        let calls = std::cell::Cell::new(0);
        let err = super::swap_in(&tmp, &root, &old, |from, to| {
            calls.set(calls.get() + 1);
            match calls.get() {
                2 => Err(std::io::Error::other("disk full")),
                _ => std::fs::rename(from, to),
            }
        })
        .unwrap_err();
        assert!(err.ends_with("Nothing was changed."), "{err}");
        assert!(root.join("a.yaml").exists());
    }

    #[test]
    fn an_update_with_an_unsafe_path_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.yaml"), "request:\n  url: a\n").unwrap();
        for bad in ["../x.yaml", ".env", "a/.git/x.yaml", "notes.txt"] {
            let files = vec![SyncFile::new("a.yaml", "x"), SyncFile::new(bad, "x")];
            assert!(super::plan_update(dir.path(), files).is_err(), "{bad}");
        }
        let plan = super::plan_update(dir.path(), vec![SyncFile::new("b.yaml", "y")]).unwrap();
        assert_eq!(plan.added, [std::path::PathBuf::from("b.yaml")]);
        assert_eq!(plan.removed, [std::path::PathBuf::from("a.yaml")]);
        assert_eq!(plan.count(), 2);
        std::fs::write(dir.path().join(".env"), "API=1\n").unwrap();
        super::apply_update(dir.path(), &plan).unwrap();
        assert!(!dir.path().join("a.yaml").exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("b.yaml")).unwrap(),
            "y"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".env")).unwrap(),
            "API=1\n"
        );
    }
}
