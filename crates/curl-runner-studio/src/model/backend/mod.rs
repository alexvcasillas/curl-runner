//! The studio's only door to "the server": an async, object-safe
//! [`Backend`]. v1 ships [`MockBackend`], in process, with fixture data.
//!
//! Nothing that crosses this seam carries a variable value: share payloads
//! carry names, descriptions, the required flag and (only when the owner
//! ticks "Share default") a non-secret `.env` default; sync payloads carry
//! YAML request files. Build them with [`shared_vars`] and [`sync_files`].

pub mod fixtures;
mod mock;

pub use mock::{MOCK_LATENCY, MockBackend};

use crate::model::collection_meta::CollectionMeta;
use crate::model::environments::is_secret;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// What every [`Backend`] call returns: a boxed future, so the trait stays
/// object-safe (`Arc<dyn Backend>`).
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    Offline,
    /// A sign-in that failed.
    Unauthorized,
    /// A call that needs an account, made while signed out.
    SignedOut,
    NotFound,
    Expired,
    Conflict(Box<Conflict>),
    /// The input was refused; the text says why.
    Invalid(String),
}

impl BackendError {
    /// The OpenShared board's error copy for a link `org` shared.
    pub fn link_text(&self, org: &str) -> String {
        match self {
            BackendError::Expired => format!("This link has expired. Ask {org} for a new one."),
            BackendError::Offline => "You're offline. Connect to open shared links.".into(),
            other => other.to_string(),
        }
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::Offline => f.write_str("You're offline."),
            BackendError::Unauthorized => f.write_str("Sign-in failed. Try again."),
            BackendError::SignedOut => f.write_str("Sign in to sync and share."),
            BackendError::NotFound => f.write_str("This link doesn't work."),
            BackendError::Expired => f.write_str("This link has expired."),
            BackendError::Conflict(c) => {
                write!(f, "{} changed on both sides", file_name(&c.rel))
            }
            BackendError::Invalid(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for BackendError {}

fn file_name(rel: &Path) -> String {
    rel.file_name()
        .map_or_else(|| rel.display().to_string(), |n| n.to_string_lossy().into())
}

/// Where collections live.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Server {
    #[default]
    Cloud,
    SelfHosted {
        url: String,
    },
}

impl Server {
    pub fn url(&self) -> &str {
        match self {
            Server::Cloud => fixtures::CLOUD_URL,
            Server::SelfHosted { url } => url,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignInMethod {
    Sso,
    GitHub,
    Email,
}

impl SignInMethod {
    /// `SSO`, `GitHub`, `email`: as in `sign-in via SSO`.
    pub fn label(self) -> &'static str {
        match self {
            SignInMethod::Sso => "SSO",
            SignInMethod::GitHub => "GitHub",
            SignInMethod::Email => "email",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub name: String,
    pub email: String,
    pub org: Option<String>,
    pub role: String,
    pub provider: SignInMethod,
}

/// What `Test connection` shows: `server 2.1.1 · sign-in via SSO · 42 ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub version: String,
    pub sign_in: SignInMethod,
    pub latency_ms: u64,
}

/// A collection as the server knows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollectionKey {
    /// The collection's [`CollectionMeta::id`].
    pub id: String,
    pub name: String,
}

/// One YAML file of a collection, as synced. Never a `.env` file, never a
/// secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncFile {
    /// Relative to the collection root.
    pub rel: PathBuf,
    /// [`SyncFile::hash_of`] the text.
    pub hash: u64,
    pub text: String,
    /// The hash this client last synced for the file (`None`: never). The
    /// server reports a conflict when its copy is neither this nor `hash`.
    #[serde(default)]
    pub base: Option<u64>,
}

impl SyncFile {
    pub fn new(rel: impl Into<PathBuf>, text: impl Into<String>) -> SyncFile {
        let text = text.into();
        SyncFile {
            rel: rel.into(),
            hash: SyncFile::hash_of(&text),
            text,
            base: None,
        }
    }

    /// With the hash this client last synced.
    pub fn with_base(mut self, base: Option<u64>) -> SyncFile {
        self.base = base;
        self
    }

    /// FNV-1a 64 of `text`: stable across releases and machines.
    pub fn hash_of(text: &str) -> u64 {
        crate::model::history::fnv1a(text.as_bytes())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Local files the server took.
    pub pushed: usize,
    /// Server files newer than the local copy, to write locally.
    pub pulled: Vec<SyncFile>,
    /// Files changed on both sides; nothing was written for them.
    pub conflicts: Vec<Conflict>,
    /// Files the server no longer has that the client synced before
    /// (received copies): remove them locally.
    pub deleted: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub rel: PathBuf,
    /// Empty when `local_deleted`.
    pub local: String,
    /// Empty when `server_deleted`.
    pub server: String,
    pub server_author: String,
    pub server_at: DateTime<Utc>,
    /// This side deleted the file; the server changed it.
    #[serde(default)]
    pub local_deleted: bool,
    /// The server deleted the file; this side changed it.
    #[serde(default)]
    pub server_deleted: bool,
}

/// A file this client synced before and no longer has, with the hash it
/// last synced: the server deletes it only if its copy is still that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncDeletion {
    pub rel: PathBuf,
    pub base: u64,
}

/// The sidebar footer's state for one collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// Signed out, or the collection was never synced: `Local only`.
    Off,
    Synced {
        at: DateTime<Utc>,
        /// Synced collections and their files, for this account.
        collections: usize,
        requests: usize,
    },
    /// Changes the server does not have yet.
    Pending(usize),
    /// Offline, with changes waiting.
    Offline(usize),
    /// The last sync left this many conflicts.
    Conflict(usize),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Access {
    /// `Anyone with the link · Can view and run`.
    #[default]
    AnyoneWithLink,
    InvitedOnly,
}

/// The Share board's `Expires` choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expiry {
    Days(u32),
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Owner,
    Viewer,
    Editor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub name: String,
    pub email: String,
    pub role: Role,
    /// Invited and has not opened it yet: `Invited · not opened`.
    pub invited: bool,
}

/// A variable the people a collection is shared with fill in themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedVar {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub secret: bool,
    /// The owner's non-secret default; always `None` when `secret`.
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareSettings {
    pub link: Option<String>,
    pub access: Access,
    pub expires: Option<NaiveDate>,
    pub people: Vec<Person>,
    pub variables: Vec<SharedVar>,
}

/// A change from the Share dialog; `None` leaves a field as it is. Any
/// update gives the collection a link if it has none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareUpdate {
    pub access: Option<Access>,
    pub expires: Option<Expiry>,
    pub variables: Option<Vec<SharedVar>>,
    /// `Reset link`: a new token; the old link stops working.
    pub reset_link: bool,
}

/// A share link, either form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShareLink {
    /// Empty for the web form, which does not carry it.
    pub owner_slug: String,
    pub collection_slug: String,
    pub token: String,
}

impl ShareLink {
    /// `curl-runner://open/{owner}/{collection}?share={token}` or
    /// `https://studio.example.com/s/{collection}-{token}`; surrounding
    /// whitespace is ignored.
    pub fn parse(text: &str) -> Option<ShareLink> {
        let text = text.trim();
        let slug = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        };
        let token = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric());
        if let Some(rest) = text.strip_prefix("curl-runner://open/") {
            let (path, query) = rest.split_once('?')?;
            let (owner, collection) = path.trim_end_matches('/').split_once('/')?;
            let t = query.split('&').find_map(|kv| kv.strip_prefix("share="))?;
            return (slug(owner) && slug(collection) && token(t)).then(|| ShareLink {
                owner_slug: owner.into(),
                collection_slug: collection.into(),
                token: t.into(),
            });
        }
        let rest = text
            .strip_prefix(fixtures::SHARE_HOST)?
            .strip_prefix("/s/")?;
        let rest = rest.trim_end_matches('/');
        let (collection, t) = rest.rsplit_once('-')?;
        (slug(collection) && token(t)).then(|| ShareLink {
            owner_slug: String::new(),
            collection_slug: collection.into(),
            token: t.into(),
        })
    }

    /// The web form, as the Share dialog shows it.
    pub fn web(collection_slug: &str, token: &str) -> String {
        format!("{}/s/{collection_slug}-{token}", fixtures::SHARE_HOST)
    }
}

/// What a link opens, before it is added (board OpenShared).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCollection {
    /// For [`Backend::fetch_shared`].
    pub id: String,
    pub owner_org: String,
    pub name: String,
    /// The folders line: `rates, labels, tracking`.
    pub summary: String,
    pub updated: DateTime<Utc>,
    pub request_count: usize,
    pub variables: Vec<SharedVar>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedSnapshot {
    pub files: Vec<SyncFile>,
}

/// A shared collection added to this studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedShare {
    /// [`SharedCollection::id`].
    pub id: String,
    /// The local copy's [`CollectionMeta::id`]: its syncs pull.
    pub collection_id: String,
    pub owner_org: String,
    pub name: String,
    pub root: PathBuf,
    pub added: DateTime<Utc>,
}

/// The mock's developer toggles (App menu › Developer, debug builds).
pub trait DevControls: Send + Sync {
    /// Every call fails with [`BackendError::Offline`] while on.
    fn set_offline(&self, on: bool);
    fn offline(&self) -> bool;
    /// The next sync returns one conflict, then the toggle clears.
    fn conflict_next_sync(&self, on: bool);
    /// Every link resolves to [`BackendError::Expired`] while on.
    fn expire_links(&self, on: bool);
    /// The next sign-in fails, then the toggle clears.
    fn fail_next_sign_in(&self, on: bool);
    /// Back to the first-launch fixture state.
    fn reset(&self);
    /// The toggles, for the menu's checkmarks.
    fn toggles(&self) -> DevToggles;
    /// `Debug` text of every argument the backend received, oldest first
    /// (debug builds and tests; empty in release).
    fn recorded(&self) -> Vec<String>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DevToggles {
    pub offline: bool,
    pub conflict_next_sync: bool,
    pub expire_links: bool,
    pub fail_next_sign_in: bool,
}

/// The server, as the studio sees it. Calls that need the network return a
/// [`BoxFut`]; the rest read what the backend already knows.
pub trait Backend: Send + Sync {
    fn server(&self) -> Server;
    /// Switching servers signs out.
    fn set_server(&self, server: Server) -> BoxFut<'_, ()>;
    fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo>;
    fn sign_in(&self, method: SignInMethod) -> BoxFut<'_, Account>;
    fn sign_out(&self) -> BoxFut<'_, ()>;
    fn account(&self) -> Option<Account>;
    fn sync_status(&self, collection: &CollectionKey) -> SyncStatus;
    /// Reconciles `local` (each with its `base`) and `deleted` with the
    /// server's copy, file by file:
    /// - server copy is the base (or missing): the local change is pushed,
    ///   a deletion applied;
    /// - local is the base: the server's newer copy is `pulled`;
    /// - server, base and local all differ: a [`Conflict`], the server copy
    ///   kept (a deletion of a file the server changed is one too).
    ///
    /// Shared (received) collections never push or delete: they pull, and
    /// report in `deleted` the files the owner removed.
    fn sync(
        &self,
        collection: &CollectionKey,
        local: &[SyncFile],
        deleted: &[SyncDeletion],
    ) -> BoxFut<'_, SyncReport>;
    fn share_settings(&self, collection: &CollectionKey) -> BoxFut<'_, ShareSettings>;
    fn update_share(
        &self,
        collection: &CollectionKey,
        update: ShareUpdate,
    ) -> BoxFut<'_, ShareSettings>;
    /// Gives the collection a link first if it has none.
    fn invite(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, Person>;
    fn remove_person(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, ()>;
    fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection>;
    fn fetch_shared(&self, id: &str) -> BoxFut<'_, SharedSnapshot>;
    fn received(&self) -> Vec<ReceivedShare>;
    fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()>;
    /// The developer toggles, when the backend has them (the mock).
    fn dev(&self) -> Option<&dyn DevControls>;
}

/// The variables to share for the collection at `root`: every name its
/// requests use plus every name `meta` describes, sorted, required first.
/// A default is the Default environment's `.env` value, only for a
/// non-secret name in `share_default`. Reads no Keychain, by design.
pub fn shared_vars(root: &Path, meta: &CollectionMeta, share_default: &[String]) -> Vec<SharedVar> {
    use crate::model::environments::UsageIndex;
    let mut names = UsageIndex::scan(root).referenced();
    names.extend(meta.vars.keys().cloned());
    names.sort();
    names.dedup();
    let dotenv = std::fs::read_to_string(root.join(".env"))
        .map(|t| curl_runner_core::env::parse_env_content(&t))
        .unwrap_or_default();
    let mut vars: Vec<SharedVar> = names
        .into_iter()
        .map(|name| {
            let notes = meta.vars.get(&name).cloned().unwrap_or_default();
            let secret = is_secret(&name, &meta.secret_names);
            let default = (!secret && share_default.contains(&name))
                .then(|| dotenv.get(&name).cloned())
                .flatten();
            SharedVar {
                description: notes.description,
                required: notes.required_for_shared,
                secret,
                default,
                name,
            }
        })
        .collect();
    vars.sort_by_key(|v| !v.required);
    vars
}

/// The largest file sync sends; bigger ones stay local (and are listed in
/// [`SyncScan::too_big`]).
pub const MAX_SYNC_BYTES: u64 = 1024 * 1024;

/// What [`scan_sync_files`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncScan {
    pub files: Vec<SyncFile>,
    /// Tracked files over [`MAX_SYNC_BYTES`], relative to the root.
    pub too_big: Vec<PathBuf>,
    /// Tracked files found but not read (permissions, not UTF-8, I/O, a
    /// link out): still there, never deletions.
    pub unreadable: Vec<PathBuf>,
    /// The root itself could be listed; if not, nothing else is trusted.
    pub root_ok: bool,
}

/// The collection at `root`'s YAML files, as synced: request files and
/// `curl-runner.yaml`, no hidden paths (so no `.env*`, nothing in `.git`),
/// nothing that resolves outside `root` (a symlink out) and nothing over
/// [`MAX_SYNC_BYTES`].
pub fn scan_sync_files(root: &Path) -> SyncScan {
    let mut scan = SyncScan {
        root_ok: std::fs::read_dir(root).is_ok(),
        ..SyncScan::default()
    };
    if !scan.root_ok {
        return scan;
    }
    let real_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let rels = curl_runner_core::discover::find_yaml_files(&[], true, root)
        .into_iter()
        .map(|f| PathBuf::from(f.strip_prefix("./").unwrap_or(&f)))
        .filter(|rel| {
            rel.components().all(|c| {
                matches!(c, std::path::Component::Normal(_))
                    && !c.as_os_str().to_string_lossy().starts_with('.')
            })
        });
    for rel in rels {
        let path = root.join(&rel);
        let inside = path
            .canonicalize()
            .is_ok_and(|real| real.starts_with(&real_root));
        let Ok(meta) = std::fs::metadata(&path) else {
            scan.unreadable.push(rel);
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        if !inside {
            scan.unreadable.push(rel);
            continue;
        }
        if meta.len() > MAX_SYNC_BYTES {
            scan.too_big.push(rel);
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => scan.files.push(SyncFile::new(rel, text)),
            Err(_) => scan.unreadable.push(rel),
        }
    }
    scan
}

/// [`scan_sync_files`]' files.
pub fn sync_files(root: &Path) -> Vec<SyncFile> {
    scan_sync_files(root).files
}

/// The backend the UI tests run on: the mock, in memory, with no latency.
/// (Views name only [`Backend`]; see `ui_never_imports_mock_backend`.)
#[cfg(test)]
pub fn test_backend() -> std::sync::Arc<dyn Backend> {
    std::sync::Arc::new(MockBackend::open(None, std::time::Duration::ZERO))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_both_link_forms() {
        let deep = ShareLink::parse("curl-runner://open/acme/shipping-api?share=7Kx2Qm").unwrap();
        assert_eq!(
            deep,
            ShareLink {
                owner_slug: "acme".into(),
                collection_slug: "shipping-api".into(),
                token: "7Kx2Qm".into(),
            }
        );
        let web = ShareLink::parse("  https://studio.example.com/s/shipping-api-7Kx2Qm\n").unwrap();
        assert_eq!(web.owner_slug, "");
        assert_eq!(web.collection_slug, "shipping-api");
        assert_eq!(web.token, "7Kx2Qm");
        assert_eq!(
            ShareLink::web("shipping-api", "7Kx2Qm"),
            "https://studio.example.com/s/shipping-api-7Kx2Qm"
        );
    }

    #[test]
    fn garbage_link_is_none() {
        for bad in [
            "",
            "hello",
            "https://example.com/s/shipping-api-7Kx2Qm",
            "https://studio.example.com/s/",
            "https://studio.example.com/s/shippingapi",
            "https://studio.example.com/s/shipping-api-",
            "curl-runner://open/acme?share=7Kx2Qm",
            "curl-runner://open/acme/shipping-api",
            "curl-runner://open/acme/shipping-api?share=",
            "curl-runner://open/acme/ship ping?share=7Kx2Qm",
            "curl-runner://open/acme/shipping-api?share=7K/x",
        ] {
            assert_eq!(ShareLink::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn sync_files_skip_env_and_hidden() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("rates")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("rates/get.yaml"), "request:\n  url: x\n").unwrap();
        std::fs::write(root.join(".git/x.yaml"), "a: 1\n").unwrap();
        std::fs::write(root.join(".env"), "BASE_URL=x\n").unwrap();
        std::fs::write(root.join(".env.yaml"), "a: 1\n").unwrap();
        let files = sync_files(root);
        let rels: Vec<_> = files.iter().map(|f| f.rel.clone()).collect();
        assert_eq!(rels, [PathBuf::from("rates/get.yaml")]);
        assert_eq!(files[0].hash, SyncFile::hash_of("request:\n  url: x\n"));
    }

    #[test]
    fn sync_files_skip_big_files_and_links_out() {
        let d = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("small.yaml"), "a: 1\n").unwrap();
        let big = format!("a: \"{}\"\n", "x".repeat(MAX_SYNC_BYTES as usize));
        std::fs::write(root.join("big.yaml"), big).unwrap();
        std::fs::write(outside.path().join("out.yaml"), "secret: 1\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path().join("out.yaml"), root.join("link.yaml"))
            .unwrap();
        let scan = scan_sync_files(root);
        let rels: Vec<_> = scan.files.iter().map(|f| f.rel.clone()).collect();
        assert_eq!(rels, [PathBuf::from("small.yaml")]);
        assert_eq!(scan.too_big, [PathBuf::from("big.yaml")]);
        if cfg!(unix) {
            assert_eq!(scan.unreadable, [PathBuf::from("link.yaml")], "a link out");
        }
        assert!(scan.root_ok);
    }

    #[test]
    fn unreadable_files_and_missing_roots_are_told_apart() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("ok.yaml"), "a: 1\n").unwrap();
        std::fs::write(root.join("latin1.yaml"), b"a: caf\xe9\n").unwrap();
        let scan = scan_sync_files(root);
        assert_eq!(scan.files.len(), 1);
        assert_eq!(scan.unreadable, [PathBuf::from("latin1.yaml")]);
        let gone = scan_sync_files(&root.join("missing"));
        assert!(!gone.root_ok && gone.files.is_empty());
    }

    #[test]
    fn server_and_sign_in_copy() {
        assert_eq!(Server::Cloud.url(), "https://cloud.curl-runner.com");
        let own = Server::SelfHosted {
            url: "https://curl-runner.example.com".into(),
        };
        assert_eq!(own.url(), "https://curl-runner.example.com");
        assert_eq!(SignInMethod::Sso.label(), "SSO");
        assert_eq!(MOCK_LATENCY, std::time::Duration::from_millis(120));
        let b: Box<dyn Backend> = Box::new(MockBackend::open(None, MOCK_LATENCY));
        assert_eq!(b.server(), Server::Cloud);
    }

    #[test]
    fn errors_read_like_the_boards() {
        assert_eq!(
            BackendError::Expired.link_text("Acme"),
            "This link has expired. Ask Acme for a new one."
        );
        assert_eq!(
            BackendError::NotFound.link_text("Acme"),
            "This link doesn't work."
        );
        assert_eq!(
            BackendError::Offline.link_text("Acme"),
            "You're offline. Connect to open shared links."
        );
        let c = Conflict {
            rel: "charges/create-charge.yaml".into(),
            local: String::new(),
            server: String::new(),
            server_author: "Sam".into(),
            server_at: Utc::now(),
            local_deleted: false,
            server_deleted: false,
        };
        assert_eq!(
            BackendError::Conflict(Box::new(c)).to_string(),
            "create-charge.yaml changed on both sides"
        );
    }
}
