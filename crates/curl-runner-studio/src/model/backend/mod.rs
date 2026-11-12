//! The studio's only door to "the server": an async, object-safe
//! [`Backend`]. A release build has `NoServer`; with the `server`
//! feature (and in tests) there is `MockBackend`, in process, with fixture
//! data.
//!
//! Nothing that crosses this seam carries a variable value: share payloads
//! carry names, descriptions, the required flag and (only when the owner
//! ticks "Share default") a non-secret `.env` default; sync payloads carry
//! YAML request files. Build them with [`shared_vars`] and [`sync_files`].

// The mock and its fixtures (Acme, fake accounts and links) are never in a
// release build: only with the `server` feature, and for the tests.
#[cfg(any(test, feature = "server"))]
pub mod fixtures;
#[cfg(any(test, feature = "server"))]
pub mod http;
#[cfg(any(test, feature = "server"))]
mod mock;
#[cfg(any(test, not(feature = "server")))]
mod no_server;
#[cfg(any(test, feature = "server"))]
pub mod pending;

// Only the macOS app constructs the mock; off macOS the UI isn't built.
/// The server the mock's links name.
#[cfg(all(test, target_os = "macos"))]
pub use mock::MOCK_SERVER;
#[cfg(any(test, feature = "server"))]
#[cfg_attr(not(target_os = "macos"), allow(unused_imports))]
pub use mock::{MOCK_LATENCY, MockBackend};
// Only the macOS app constructs it; off macOS the UI isn't built.
/// Who a shared link opens for: `Public` needs no sign-in, `Restricted`
/// a session on the link's server.
pub use curl_runner_cloud_proto::wire::Access as LinkAccess;
/// How a server lets people sign in: what `/v1/info` reports.
pub use curl_runner_cloud_proto::wire::SignInMethod;
#[cfg(not(feature = "server"))]
#[cfg_attr(not(target_os = "macos"), allow(unused_imports))]
pub use no_server::NoServer;
#[cfg(any(test, feature = "server"))]
pub use pending::PendingSignIns;

use crate::model::collection_meta::CollectionMeta;
use crate::model::environments::is_secret;
use chrono::{DateTime, NaiveDate, Utc};
use curl_runner_cloud_client::link::{self, Link, LinkError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// What every [`Backend`] call returns: a boxed future, so the trait stays
/// object-safe (`Arc<dyn Backend>`).
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'a>>;

// Only a server (the mock, for now) makes some of these.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    Offline,
    /// A sign-in that failed.
    Unauthorized,
    /// A call that needs an account, made while signed out.
    SignedOut,
    NotFound,
    Expired,
    /// The input was refused; the text says why.
    Invalid(String),
    /// A build without the server (`NoServer`).
    #[cfg_attr(feature = "server", allow(dead_code))]
    Unavailable,
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
            BackendError::Invalid(why) => f.write_str(why),
            BackendError::Unavailable => {
                f.write_str("This needs the curl-runner server, which is coming soon.")
            }
        }
    }
}

impl std::error::Error for BackendError {}

/// The cloud server's address.
pub const CLOUD_URL: &str = "https://api.curl-runner.com";

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
    /// The server an API URL names: [`Server::Cloud`] for [`CLOUD_URL`].
    pub fn for_url(url: &str) -> Server {
        let url = url.trim().trim_end_matches('/');
        if url == CLOUD_URL {
            Server::Cloud
        } else {
            Server::SelfHosted { url: url.into() }
        }
    }

    pub fn url(&self) -> &str {
        match self {
            Server::Cloud => CLOUD_URL,
            Server::SelfHosted { url } => url,
        }
    }
}

/// `GitHub`, `Google`, `email`: as in `signed in with GitHub`.
pub fn method_label(method: SignInMethod) -> &'static str {
    match method {
        SignInMethod::Github => "GitHub",
        SignInMethod::Google => "Google",
        SignInMethod::Email => "email",
    }
}

/// `methods` in the order GitHub, Google, email, joined as a sentence:
/// `GitHub, Google or email`. `None` when there are none.
pub fn methods_text(methods: &[SignInMethod]) -> Option<String> {
    let labels: Vec<&str> = [
        SignInMethod::Github,
        SignInMethod::Google,
        SignInMethod::Email,
    ]
    .into_iter()
    .filter(|m| methods.contains(m))
    .map(method_label)
    .collect();
    match labels.as_slice() {
        [] => None,
        [one] => Some((*one).to_string()),
        [rest @ .., last] => Some(format!("{} or {last}", rest.join(", "))),
    }
}

/// Who is signed in on one server. Teams arrive with plan 3b.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub name: String,
    pub email: String,
    /// How they signed in, when the server says (the API doesn't).
    pub provider: Option<SignInMethod>,
}

/// What `Test connection` shows:
/// `server 2.1.1 · sign-in via GitHub or email · 42 ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub version: String,
    pub sign_in: Vec<SignInMethod>,
    pub latency_ms: u64,
}

/// What a sign-in callback gets when this studio didn't start a sign-in
/// with its server in the last 10 minutes, or the server isn't https.
pub const FOREIGN_CALLBACK: &str = "That sign-in link didn't come from this studio.";

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

/// What the server knows of one collection's sync. Anything about this
/// client's unsent work (changes waiting, offline, conflicts) is the
/// client's own state, never the server's.
// Only a server (the mock, for now) makes some of these.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// Signed out, or the server never got the collection.
    Off,
    Synced {
        at: DateTime<Utc>,
        /// Synced collections and their files, for this account.
        collections: usize,
        requests: usize,
    },
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
    /// The default is locked: the people it is shared with can't change it
    /// (`Shared by Acme`); else it is only suggested. Only with a default;
    /// never for a secret.
    #[serde(default)]
    pub locked: bool,
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

/// A share link: the API to ask and the token to ask with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShareLink {
    pub server: String,
    pub token: String,
}

impl ShareLink {
    /// `curl-runner://open[/team/collection]?server=&share=` or
    /// `https://{host}/s/{token}`; surrounding whitespace is ignored.
    ///
    /// A landing URL asks its host where the API is, so this blocks on the
    /// network for that form: call it off the UI thread.
    pub fn parse(text: &str) -> Result<ShareLink, LinkError> {
        match link::parse(text.trim())? {
            Link::Api { server, token } => Ok(ShareLink { server, token }),
            Link::Landing { origin, token } => Ok(ShareLink {
                server: link::api_for_landing(&origin, None)?,
                token,
            }),
        }
    }

    /// The link's server host, as the sign-in prompt names it:
    /// `api.curl-runner.com`.
    pub fn host(&self) -> String {
        url::Url::parse(&self.server)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| self.server.clone())
    }

    /// The deep link that opens it again.
    pub fn url(&self) -> String {
        let mut u = url::Url::parse("curl-runner://open").expect("a valid URL");
        u.query_pairs_mut()
            .append_pair("server", &self.server)
            .append_pair("share", &self.token);
        u.into()
    }
}

/// The dashboard's answer to `Continue in browser`:
/// `curl-runner://auth/callback?code=crh_…&server={api}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCallback {
    pub server: String,
    pub code: String,
}

impl AuthCallback {
    /// `None` unless the URL is an auth callback with both a `crh_` code and
    /// a server.
    pub fn parse(url: &str) -> Option<AuthCallback> {
        let u = url::Url::parse(url.trim()).ok()?;
        if u.scheme() != "curl-runner" || u.host_str() != Some("auth") || u.path() != "/callback" {
            return None;
        }
        let get = |k: &str| {
            u.query_pairs()
                .find(|(name, _)| name == k)
                .map(|(_, v)| v.into_owned())
        };
        let code = get("code").filter(|c| c.starts_with("crh_") && link::valid_token(c))?;
        let server = get("server")
            .map(|s| s.trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())?;
        Some(AuthCallback { server, code })
    }
}

/// What a link opens, before it is added (board OpenShared).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCollection {
    /// For [`Backend::fetch_shared`] and the update checks.
    pub link: ShareLink,
    /// The team's name: `Acme`.
    pub owner_org: String,
    pub team_slug: String,
    pub name: String,
    /// The folders line: `rates, labels, tracking`.
    pub summary: String,
    pub updated: DateTime<Utc>,
    pub request_count: usize,
    /// The published version it opens.
    pub version: u32,
    pub access: LinkAccess,
    /// The snapshot's inputs, one for one.
    pub variables: Vec<SharedVar>,
}

/// A shared collection's current version, files and all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedSnapshot {
    pub files: Vec<SyncFile>,
    /// The variables as the owner shares them now.
    pub variables: Vec<SharedVar>,
    pub version: u32,
    pub published_at: DateTime<Utc>,
}

/// A shared collection this studio added, kept on this Mac (the server
/// records nothing): nothing about where the copy lives (that mapping is
/// [`crate::model::received`]). Keyed by `link.token`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedShare {
    pub link: ShareLink,
    /// The team that shares it: `Acme`.
    pub team: String,
    pub name: String,
    /// The version the copy holds.
    pub version: u32,
    pub added: NaiveDate,
    /// [`SyncFile::hash_of`] each file as received.
    pub files: BTreeMap<PathBuf, u64>,
}

/// The mock's developer toggles (App menu › Developer, debug builds with
/// the `server` feature).
#[cfg_attr(not(feature = "server"), allow(dead_code))]
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
    /// The server refuses the current session once: signed out, shown as
    /// expired.
    fn expire_session(&self);
    /// Back to the first-launch fixture state.
    fn reset(&self);
    /// The toggles, for the menu's checkmarks.
    fn toggles(&self) -> DevToggles;
    /// While `held`, `update_share` makes its change but its answer waits
    /// until released, and `add_received` waits before it makes its change
    /// (tests: a call that stays in flight).
    #[cfg(test)]
    fn hold_updates(&self, held: bool);
    /// `Debug` text of every argument the backend received, oldest first
    /// (tests).
    #[cfg(test)]
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
    /// The server this person's own team publishes to (Settings ›
    /// Accounts & sync). Received collections carry their own.
    fn server(&self) -> Server;
    /// Sessions are per server: switching signs nothing out.
    fn set_server(&self, server: Server) -> BoxFut<'_, ()>;
    fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo>;
    /// Starts a sign-in with `server`: records it as pending for 10
    /// minutes and returns the page to open, `{app_url}/sign-in?client=studio`.
    fn sign_in_url(&self, server: &Server) -> BoxFut<'_, String>;
    /// Redeems the dashboard's callback. Refused with
    /// `Invalid(`[`FOREIGN_CALLBACK`]`)`, sending nothing, unless a sign-in
    /// with `callback.server` is pending ([`PendingSignIns`]).
    fn complete_sign_in(&self, callback: &AuthCallback) -> BoxFut<'_, Account>;
    /// Ends the session on `server` only.
    fn sign_out(&self, server: &str) -> BoxFut<'_, ()>;
    /// The account on [`Backend::server`].
    fn account(&self) -> Option<Account>;
    /// The account on `server` (an API URL).
    // Received collections, each on its own server, ask (Task 7 of 3a).
    #[cfg_attr(not(test), allow(dead_code))]
    fn account_on(&self, server: &str) -> Option<Account>;
    /// Who was signed in to [`Backend::server`] until it refused the
    /// session (401): shown as expired until they sign in again or out.
    fn lapsed(&self) -> Option<Account> {
        None
    }
    /// Every account signed in, by server API URL (no trailing `/`),
    /// sorted by server.
    fn accounts(&self) -> Vec<(String, Account)> {
        Vec::new()
    }
    /// Whether this backend syncs team collections at all ([`Backend::sync`]
    /// works). The real server's answers `false` until its team side ships
    /// (plan 3b): the studio then keeps collections local, without a sync
    /// call or a toast.
    fn team_sync(&self) -> bool {
        true
    }
    fn sync_status(&self, collection: &CollectionKey) -> SyncStatus;
    /// Reconciles `local` (each with its `base`) and `deleted` with the
    /// server's copy, file by file:
    /// - server copy is the base (or missing): the local change is pushed,
    ///   a deletion applied;
    /// - local is the base: the server's newer copy is `pulled`;
    /// - server, base and local all differ: a [`Conflict`], the server copy
    ///   kept (a deletion of a file the server changed is one too);
    /// - server and local already agree (whatever the base): nothing at
    ///   all, not a push, a pull nor a conflict.
    ///
    /// Only for collections this account owns: a received copy takes
    /// updates instead ([`Backend::latest_version`]).
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
    /// What `link` opens, sending the session for `link.server` when there
    /// is one. A restricted collection without one is `SignedOut`.
    fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection>;
    /// The current version's files and variables.
    fn fetch_shared(&self, link: &ShareLink) -> BoxFut<'_, SharedSnapshot>;
    /// The current version number, without the files.
    // The update checks call it (Task 7 of 3a).
    #[cfg_attr(not(test), allow(dead_code))]
    fn latest_version(&self, link: &ShareLink) -> BoxFut<'_, u32>;
    /// The collections this studio added.
    fn received(&self) -> Vec<ReceivedShare>;
    /// Records `share`, replacing one with the same token.
    fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()>;
    /// Replaces the record with `share`'s token (an update taken).
    // An update taken calls it (Task 7 of 3a).
    #[cfg_attr(not(test), allow(dead_code))]
    fn update_received(&self, share: ReceivedShare) -> BoxFut<'_, ()>;
    /// The developer toggles, when the backend has them (the mock).
    fn dev(&self) -> Option<&dyn DevControls>;
}

/// The variables to share for the collection at `root`: every name its
/// requests use plus every name `meta` describes, sorted, required first.
/// A default is the Default environment's `.env` value, only for a
/// non-secret name in `share_default`. Reads no Keychain, by design.
pub fn shared_vars(root: &Path, meta: &CollectionMeta, share_default: &[String]) -> Vec<SharedVar> {
    shared_vars_with(root, meta, |name| share_default.iter().any(|n| n == name))
}

/// What the Share dialog lists: [`shared_vars`] as if every non-secret
/// name shared its default, so a variable with a `.env` value can offer
/// `Share default` and show what it would send. Never a payload.
pub fn share_candidates(root: &Path, meta: &CollectionMeta) -> Vec<SharedVar> {
    shared_vars_with(root, meta, |_| true)
}

/// The variables to share, with what [`CollectionMeta::share_defaults`]
/// ticks: the one Share and sync payload builder.
pub fn share_payload(root: &Path, meta: &CollectionMeta) -> Vec<SharedVar> {
    shared_vars(root, meta, &meta.share_defaults())
}

fn shared_vars_with(
    root: &Path,
    meta: &CollectionMeta,
    share_default: impl Fn(&str) -> bool,
) -> Vec<SharedVar> {
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
            let default = (!secret && share_default(&name))
                .then(|| dotenv.get(&name).cloned())
                .flatten();
            SharedVar {
                description: notes.description,
                required: notes.required_for_shared,
                secret,
                locked: default.is_some() && notes.share_locked,
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
#[cfg(test)]
pub fn sync_files(root: &Path) -> Vec<SyncFile> {
    scan_sync_files(root).files
}

/// The backend the UI tests run on: the mock, in memory, with no latency.
/// (Views name only [`Backend`]; see `ui_never_imports_mock_backend`.)
#[cfg(test)]
pub fn test_backend() -> std::sync::Arc<dyn Backend> {
    std::sync::Arc::new(MockBackend::open(None, std::time::Duration::ZERO))
}

/// Test doubles around the mock.
#[cfg(test)]
pub mod testing {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Signs `b` in on its server as the studio does: `sign_in_url`, then
    /// the dashboard's callback. The mock signs in Jamie (GitHub).
    pub fn sign_in(b: &dyn Backend) -> Account {
        sign_in_with(b, fixtures::MOCK_CODE)
    }

    /// [`sign_in`], as the mock's email account (Maria).
    pub fn sign_in_by_email(b: &dyn Backend) -> Account {
        sign_in_with(b, fixtures::EMAIL_CODE)
    }

    fn sign_in_with(b: &dyn Backend, code: &str) -> Account {
        use futures::executor::block_on;
        let server = b.server();
        block_on(b.sign_in_url(&server)).unwrap();
        let callback = AuthCallback {
            server: server.url().trim_end_matches('/').to_string(),
            code: code.into(),
        };
        block_on(b.complete_sign_in(&callback)).unwrap()
    }

    /// The files the server holds for `key`, read through its share link
    /// (made if it has none).
    // The sync tests, which need the server feature.
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub fn server_files(b: &dyn Backend, key: &CollectionKey) -> Vec<SyncFile> {
        use futures::executor::block_on;
        let url = block_on(b.update_share(key, ShareUpdate::default()))
            .unwrap()
            .link
            .unwrap();
        let link = ShareLink::parse(&url).unwrap();
        block_on(b.fetch_shared(&link)).unwrap().files
    }

    /// Signs `b` out of its server.
    pub fn sign_out(b: &dyn Backend) {
        futures::executor::block_on(b.sign_out(b.server().url())).unwrap();
    }

    /// The backend, recording every file a sync sends (text included).
    pub struct Spy {
        pub inner: Arc<dyn Backend>,
        pub sent: Mutex<Vec<SyncFile>>,
        /// `share_settings` fails as offline while set.
        pub fail_share_settings: std::sync::atomic::AtomicBool,
        /// When set, `sign_in_url` (still started on the inner backend)
        /// answers with this page, as a real server does.
        pub browser_url: Mutex<Option<String>>,
        /// When set, every share has this snapshot as its latest version
        /// (asked of the inner backend first, so offline still fails).
        pub newer: Mutex<Option<SharedSnapshot>>,
        /// How many times `latest_version` was asked.
        pub checks: std::sync::atomic::AtomicUsize,
        /// While set, the spy is not the mock to the studio: no
        /// Developer controls, as a real server's backend.
        pub hide_dev: std::sync::atomic::AtomicBool,
    }

    impl Spy {
        pub fn new(inner: Arc<dyn Backend>) -> Spy {
            Spy {
                inner,
                sent: Mutex::default(),
                fail_share_settings: Default::default(),
                browser_url: Mutex::default(),
                newer: Mutex::default(),
                checks: Default::default(),
                hide_dev: Default::default(),
            }
        }
    }

    impl Backend for Spy {
        fn server(&self) -> Server {
            self.inner.server()
        }
        fn set_server(&self, server: Server) -> BoxFut<'_, ()> {
            self.inner.set_server(server)
        }
        fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo> {
            self.inner.test_connection(server)
        }
        fn sign_in_url(&self, server: &Server) -> BoxFut<'_, String> {
            let started = self.inner.sign_in_url(server);
            Box::pin(async move {
                let url = started.await?;
                Ok(self.browser_url.lock().unwrap().clone().unwrap_or(url))
            })
        }
        fn complete_sign_in(&self, callback: &AuthCallback) -> BoxFut<'_, Account> {
            self.inner.complete_sign_in(callback)
        }
        fn sign_out(&self, server: &str) -> BoxFut<'_, ()> {
            self.inner.sign_out(server)
        }
        fn account(&self) -> Option<Account> {
            self.inner.account()
        }
        fn account_on(&self, server: &str) -> Option<Account> {
            self.inner.account_on(server)
        }
        fn lapsed(&self) -> Option<Account> {
            self.inner.lapsed()
        }
        fn accounts(&self) -> Vec<(String, Account)> {
            self.inner.accounts()
        }
        fn team_sync(&self) -> bool {
            self.inner.team_sync()
        }
        fn sync_status(&self, collection: &CollectionKey) -> SyncStatus {
            self.inner.sync_status(collection)
        }
        fn sync(
            &self,
            collection: &CollectionKey,
            local: &[SyncFile],
            deleted: &[SyncDeletion],
        ) -> BoxFut<'_, SyncReport> {
            self.sent.lock().unwrap().extend(local.iter().cloned());
            self.inner.sync(collection, local, deleted)
        }
        fn share_settings(&self, collection: &CollectionKey) -> BoxFut<'_, ShareSettings> {
            if self
                .fail_share_settings
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return Box::pin(async { Err(BackendError::Offline) });
            }
            self.inner.share_settings(collection)
        }
        fn update_share(
            &self,
            collection: &CollectionKey,
            update: ShareUpdate,
        ) -> BoxFut<'_, ShareSettings> {
            self.inner.update_share(collection, update)
        }
        fn invite(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, Person> {
            self.inner.invite(collection, email)
        }
        fn remove_person(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, ()> {
            self.inner.remove_person(collection, email)
        }
        fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection> {
            self.inner.resolve_link(link)
        }
        fn fetch_shared(&self, link: &ShareLink) -> BoxFut<'_, SharedSnapshot> {
            let asked = self.inner.fetch_shared(link);
            Box::pin(async move {
                let snapshot = asked.await?;
                Ok(self.newer.lock().unwrap().clone().unwrap_or(snapshot))
            })
        }
        fn latest_version(&self, link: &ShareLink) -> BoxFut<'_, u32> {
            self.checks
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let asked = self.inner.latest_version(link);
            Box::pin(async move {
                let version = asked.await?;
                let newer = self.newer.lock().unwrap().as_ref().map(|s| s.version);
                Ok(newer.unwrap_or(version))
            })
        }
        fn received(&self) -> Vec<ReceivedShare> {
            self.inner.received()
        }
        fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
            self.inner.add_received(share)
        }
        fn update_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
            self.inner.update_received(share)
        }
        fn dev(&self) -> Option<&dyn DevControls> {
            if self.hide_dev.load(std::sync::atomic::Ordering::SeqCst) {
                return None;
            }
            self.inner.dev()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const API: &str = "https://api.x.test";

    #[test]
    fn share_link_from_deep_link() {
        let want = ShareLink {
            server: API.into(),
            token: "crsl_Ab-9_".into(),
        };
        for text in [
            "curl-runner://open?server=https%3A%2F%2Fapi.x.test&share=crsl_Ab-9_",
            "  curl-runner://open/acme/pay?share=crsl_Ab-9_&server=https%3A%2F%2Fapi.x.test\n",
        ] {
            assert_eq!(ShareLink::parse(text).unwrap(), want, "{text}");
        }
    }

    #[test]
    fn share_link_from_landing_url_asks_the_well_known() {
        let server = testkit::TestServer::with_handler(|req| {
            if req.url == "/.well-known/curl-runner.json" {
                testkit::Response::from_data(br#"{"api":"https://api.x.test"}"#.to_vec())
            } else {
                testkit::Response::from_data(Vec::new()).with_status_code(404)
            }
        });
        let link = ShareLink::parse(&format!("{}/s/crsl_Ab-9_", server.base_url)).unwrap();
        assert_eq!(link.server, API);
        assert_eq!(link.token, "crsl_Ab-9_");
        let none = testkit::TestServer::with_handler(|_| {
            testkit::Response::from_data(Vec::new()).with_status_code(404)
        });
        assert_eq!(
            ShareLink::parse(&format!("{}/s/crsl_Ab-9_", none.base_url)),
            Err(LinkError::NoApi)
        );
    }

    #[test]
    fn share_link_rejects_bad_tokens() {
        let open =
            |t: &str| ShareLink::parse(&format!("curl-runner://open?server={API}&share={t}"));
        let max = "a_-".repeat(85) + "z";
        assert_eq!(max.len(), 256);
        assert_eq!(open(&max).unwrap().token, max);
        assert_eq!(open(&(max + "z")), Err(LinkError::Malformed));
        for bad in ["", "7K/x", "7K x", "a.b"] {
            assert_eq!(open(bad), Err(LinkError::Malformed), "{bad:?}");
        }
        for bad in [
            "",
            "hello",
            "curl-runner://open?server=https%3A%2F%2Fapi.x.test",
            "curl-runner://open?share=crsl_abc",
            "curl-runner://auth/callback?share=crsl_abc&server=https%3A%2F%2Fapi.x.test",
        ] {
            assert_eq!(ShareLink::parse(bad), Err(LinkError::Malformed), "{bad:?}");
        }
        assert_eq!(
            ShareLink::parse("curl-runner://open?server=http%3A%2F%2Fevil.test&share=crsl_a"),
            Err(LinkError::InsecureServer("http://evil.test".into()))
        );
    }

    #[test]
    fn auth_callback_parse() {
        assert_eq!(
            AuthCallback::parse(
                "curl-runner://auth/callback?code=crh_abc123&server=https%3A%2F%2Fapi.x.test"
            ),
            Some(AuthCallback {
                server: API.into(),
                code: "crh_abc123".into()
            })
        );
        for bad in [
            "curl-runner://auth/callback?code=crh_abc123",
            "curl-runner://auth/callback?server=https%3A%2F%2Fapi.x.test",
            "curl-runner://auth/callback?code=abc123&server=https%3A%2F%2Fapi.x.test",
            "curl-runner://open?code=crh_abc123&server=https%3A%2F%2Fapi.x.test",
            "https://x.test/auth/callback?code=crh_abc123&server=https%3A%2F%2Fapi.x.test",
        ] {
            assert_eq!(AuthCallback::parse(bad), None, "{bad:?}");
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
    fn payload_shares_only_ticked_non_secret_defaults() {
        use crate::model::collection_meta::VarMeta;
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(
            root.join("a.yaml"),
            "request:\n  url: ${BASE_URL}/a?acct=${ACCOUNT_ID}\n  headers:\n    Authorization: ${API_TOKEN}\n",
        )
        .unwrap();
        std::fs::write(
            root.join(".env"),
            "BASE_URL=https://api.example.com\nACCOUNT_ID=acct-1\nAPI_TOKEN=sk-plain\n",
        )
        .unwrap();
        let mut meta = CollectionMeta::default();
        meta.set_secret("API_TOKEN", true);
        for name in ["BASE_URL", "API_TOKEN"] {
            meta.vars.insert(
                name.into(),
                VarMeta {
                    share_default: true,
                    ..VarMeta::default()
                },
            );
        }
        let by_name = |vars: Vec<SharedVar>| {
            vars.into_iter()
                .map(|v| (v.name, v.default))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let payload = by_name(share_payload(root, &meta));
        assert_eq!(
            payload["BASE_URL"].as_deref(),
            Some("https://api.example.com")
        );
        assert_eq!(payload["ACCOUNT_ID"], None, "not ticked");
        assert_eq!(payload["API_TOKEN"], None, "a secret, ticked or not");
        let offered = by_name(share_candidates(root, &meta));
        assert_eq!(offered["ACCOUNT_ID"].as_deref(), Some("acct-1"));
        assert_eq!(offered["API_TOKEN"], None);
    }

    #[test]
    fn server_and_sign_in_copy() {
        assert_eq!(Server::Cloud.url(), "https://api.curl-runner.com");
        let own = Server::SelfHosted {
            url: "https://curl-runner.example.com".into(),
        };
        assert_eq!(own.url(), "https://curl-runner.example.com");
        assert_eq!(method_label(SignInMethod::Github), "GitHub");
        assert_eq!(
            methods_text(&[SignInMethod::Email, SignInMethod::Github]).as_deref(),
            Some("GitHub or email")
        );
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
    }
}
