//! [`HttpBackend`]: the studio's Backend over the curl-runner Cloud API:
//! server info, browser sign-in, accounts, and the shared collections
//! this studio opens and adds. Calls run the blocking client inside a
//! boxed future, polled on GPUI's background executor.
//!
//! The team side (`sync`, sharing, people) answers `Unavailable` until
//! plan 3b.

use super::{
    Account, AuthCallback, Backend, BackendError, BoxFut, CollectionKey, ConnectionInfo,
    DevControls, PendingSignIns, Person, ReceivedShare, Server, ShareLink, ShareSettings,
    ShareUpdate, SharedCollection, SharedSnapshot, SharedVar, SyncDeletion, SyncFile, SyncReport,
    SyncStatus, TeamRef,
};
use super::{
    BundleFile, CollectionSource, PublishRequest, PublishResponse, RemoteCollection, ShareChange,
    ShareEntry, TeamShare,
};
use crate::model::sessions::{AccountCache, CachedAccount, SessionStore};
use curl_runner_cloud_client::{Client, CloudError, User, check_server, describe, escape_control};
use curl_runner_cloud_proto::{ErrorCode, Input, Snapshot};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

/// curl-runner Cloud's dashboard.
const CLOUD_APP_URL: &str = "https://cloud.curl-runner.com";
/// Where the server choice is kept, under the state folder.
const SERVER_FILE: &str = "server.json";
/// The collections this studio added, under the state folder.
const RECEIVED_FILE: &str = "received-shares.json";

pub struct HttpBackend {
    state: Option<PathBuf>,
    sessions: Arc<dyn SessionStore>,
    server: Mutex<Server>,
    /// Name and email of each signed-in server (no tokens), mirrored in
    /// `{state}/accounts.json`.
    accounts: Mutex<AccountCache>,
    /// Accounts whose session a server refused (401), by server, until
    /// they sign in again or sign out. Memory only: a relaunch reads as
    /// signed out.
    lapsed: Mutex<BTreeMap<String, Account>>,
    pending: PendingSignIns,
    /// Mirrored in `{state}/received-shares.json`.
    received: Mutex<Vec<ReceivedShare>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A server URL as the sessions and the account cache key it.
fn key(server: &str) -> String {
    server.trim().trim_end_matches('/').to_string()
}

fn account_of(c: &CachedAccount) -> Account {
    Account {
        name: c.name.clone().unwrap_or_else(|| c.email.clone()),
        email: c.email.clone(),
        provider: None,
        teams: c.teams.clone(),
    }
}

impl HttpBackend {
    /// `state` keeps the server choice and the account cache (`None`:
    /// memory only, for tests). Tokens live in `sessions`.
    pub fn open(state: Option<&Path>, sessions: Arc<dyn SessionStore>) -> HttpBackend {
        let server = state
            .and_then(|s| std::fs::read(s.join(SERVER_FILE)).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        HttpBackend {
            state: state.map(Path::to_path_buf),
            sessions,
            server: Mutex::new(server),
            accounts: Mutex::new(AccountCache::load(state)),
            lapsed: Mutex::default(),
            pending: PendingSignIns::default(),
            received: Mutex::new(
                state
                    .and_then(|s| std::fs::read(s.join(RECEIVED_FILE)).ok())
                    .and_then(|b| serde_json::from_slice(&b).ok())
                    .unwrap_or_default(),
            ),
        }
    }

    /// `call` on a client for `server`, with its session when there is one
    /// (none: anonymous, no `Authorization` at all). Every error goes
    /// through [`HttpBackend::map_error`], so a 401 forgets that server's
    /// session. A 403 says why in the server's words (not on the list).
    ///
    /// A session the server refuses (stale, expired) is forgotten and the
    /// call made once more without one: a public collection needs no
    /// sign-in at any point. Only a second refusal is `SignedOut`.
    fn authed<T>(
        &self,
        server: &str,
        call: impl Fn(&Client) -> Result<T, CloudError>,
    ) -> Result<T, BackendError> {
        let server = key(server);
        let token = self
            .sessions
            .get(&server)
            .map_err(|_| keychain_unreadable())?;
        let sent = token.is_some();
        let map = |e| match e {
            CloudError::Api(b, _) if b.error == ErrorCode::Forbidden => {
                BackendError::Invalid(escape_control(&b.message))
            }
            e => self.map_error(&server, e),
        };
        match call(&Client::new(&server, token)).map_err(map) {
            Err(BackendError::SignedOut) if sent => call(&Client::new(&server, None)).map_err(map),
            other => other,
        }
    }

    /// Changes the received records with `f`, then writes them (atomic).
    fn edit_received(&self, f: impl FnOnce(&mut Vec<ReceivedShare>)) -> Result<(), BackendError> {
        let mut received = lock(&self.received);
        let mut next = received.clone();
        f(&mut next);
        if let Some(state) = &self.state {
            let json = serde_json::to_string_pretty(&next)
                .map_err(|e| BackendError::Invalid(format!("Couldn't save it: {e}")))?;
            std::fs::create_dir_all(state)
                .and_then(|()| crate::model::write_atomic(&state.join(RECEIVED_FILE), &json))
                .map_err(|e| BackendError::Invalid(format!("Couldn't save it: {e}")))?;
        }
        *received = next;
        Ok(())
    }

    fn remember(&self, server: &str, user: User, teams: Vec<TeamRef>) -> Account {
        let cached = CachedAccount {
            name: user.name,
            email: user.email,
            teams,
        };
        let account = account_of(&cached);
        lock(&self.lapsed).remove(server);
        let mut accounts = lock(&self.accounts);
        accounts.0.insert(server.to_string(), cached);
        // A cache that can't be written costs only the offline display.
        let _ = accounts.save(self.state.as_deref());
        account
    }

    /// Forgets `server`'s session and account. Local only.
    fn forget(&self, server: &str) -> Result<(), BackendError> {
        self.sessions.delete(server).map_err(|_| {
            BackendError::Invalid("Couldn't remove your sign-in from the Keychain.".into())
        })?;
        lock(&self.accounts).0.remove(server);
        Ok(())
    }

    /// A failed call to `server` as the studio shows it. A 401 also
    /// forgets that server's session (and no other). Every authenticated
    /// call must route its errors through here: that keeps a 401 signing
    /// out on every call, not only the first one written.
    fn map_error(&self, server: &str, err: CloudError) -> BackendError {
        let mapped = Self::describe_error(server, err);
        if mapped == BackendError::SignedOut {
            let server = key(server);
            // Who it was stays on show as an expired session.
            if let Some(a) = self.account_on(&server) {
                lock(&self.lapsed).insert(server.clone(), a);
            }
            // Signed out either way; a Keychain that won't delete is
            // retried by the next sign-out.
            let _ = self.forget(&server);
        }
        mapped
    }

    /// [`HttpBackend::map_error`] without the side effect: for calls that
    /// carry no session (a failed redeem must never sign anyone out).
    fn describe_error(server: &str, err: CloudError) -> BackendError {
        let (body, _) = match err {
            CloudError::Unreachable(_) => return BackendError::Offline,
            CloudError::Api(ref b, status) => (b.clone(), status),
        };
        match body.error {
            ErrorCode::Unauthorized => BackendError::SignedOut,
            ErrorCode::Forbidden => BackendError::Unauthorized,
            ErrorCode::NotFound | ErrorCode::LinkDisabled => BackendError::NotFound,
            ErrorCode::CodeExpired => BackendError::Expired,
            ErrorCode::CodeInvalid => BackendError::Invalid(describe(&err, server)),
            ErrorCode::Unavailable | ErrorCode::RateLimited => BackendError::Unavailable,
            _ => BackendError::Invalid(curl_runner_cloud_client::escape_control(&body.message)),
        }
    }

    /// A call for one of the account's teams, with its session (none:
    /// signed out). Refusals come back as [`HttpBackend::publish_error`].
    fn team_call<T>(
        &self,
        server: &str,
        team: &str,
        call: impl FnOnce(&Client) -> Result<T, CloudError>,
    ) -> Result<T, BackendError> {
        let token = match self.sessions.get(server) {
            Ok(Some(t)) => t,
            Ok(None) => return Err(BackendError::SignedOut),
            Err(_) => return Err(keychain_unreadable()),
        };
        call(&Client::new(server, Some(token))).map_err(|e| self.publish_error(server, team, e))
    }

    /// A refused publish in the server's words: the plan limit and a
    /// studio-synced collection by kind, a pause, rate limit or suspension
    /// as said, a missing team as a team the account can't publish to.
    fn publish_error(&self, server: &str, team: &str, err: CloudError) -> BackendError {
        if let CloudError::Api(b, _) = &err {
            let msg = escape_control(&b.message);
            match b.error {
                ErrorCode::PlanLimit => return BackendError::PlanLimit(msg),
                ErrorCode::WrongSource => return BackendError::WrongSource(msg),
                ErrorCode::RateLimited
                | ErrorCode::Unavailable
                | ErrorCode::TeamSuspended
                | ErrorCode::InvalidRequest => {
                    return BackendError::Invalid(msg);
                }
                ErrorCode::NotFound => {
                    return BackendError::Invalid(format!(
                        "You can't publish to {team}. Check that you're still in the team."
                    ));
                }
                _ => {}
            }
        }
        self.map_error(server, err)
    }

    fn save_server(&self, server: &Server) {
        let Some(state) = &self.state else { return };
        let Ok(json) = serde_json::to_string_pretty(server) else {
            return;
        };
        let _ = std::fs::create_dir_all(state)
            .and_then(|()| crate::model::write_atomic(&state.join(SERVER_FILE), &json));
    }
}

fn keychain_unreadable() -> BackendError {
    BackendError::Invalid("Couldn't read your sign-in from the Keychain.".into())
}

/// An owner's input as the studio's variable, one for one.
fn variable_of(input: Input) -> SharedVar {
    SharedVar {
        name: input.name,
        description: input.description.unwrap_or_default(),
        required: input.required,
        secret: input.secret,
        default: input.default,
        locked: input.locked,
    }
}

fn snapshot_of(snap: Snapshot) -> SharedSnapshot {
    SharedSnapshot {
        files: snap
            .files
            .into_iter()
            .map(|f| SyncFile::new(f.path, f.text))
            .collect(),
        variables: snap.inputs.into_iter().map(variable_of).collect(),
        version: snap.info.version,
        published_at: snap.info.updated_at,
    }
}

/// The top folders of `files`, in order of first use, at most three:
/// `rates, labels, tracking`.
fn folders_line(files: &[SyncFile]) -> String {
    let mut tops: Vec<String> = Vec::new();
    for f in files {
        let mut parts = f.rel.components();
        let (Some(top), Some(_)) = (parts.next(), parts.next()) else {
            continue;
        };
        let top = top.as_os_str().to_string_lossy().into_owned();
        if !tops.contains(&top) {
            tops.push(top);
        }
        if tops.len() == 3 {
            break;
        }
    }
    tops.join(", ")
}

fn ready<'a, T: Send + 'a>(r: Result<T, BackendError>) -> BoxFut<'a, T> {
    Box::pin(async move { r })
}

fn unavailable<'a, T: Send + 'a>() -> BoxFut<'a, T> {
    Box::pin(async { Err(BackendError::Unavailable) })
}

impl Backend for HttpBackend {
    fn server(&self) -> Server {
        lock(&self.server).clone()
    }

    /// Sessions are per server: nothing is signed out.
    fn set_server(&self, server: Server) -> BoxFut<'_, ()> {
        let server = match server {
            Server::SelfHosted { url } => Server::SelfHosted { url: key(&url) },
            cloud => cloud,
        };
        self.save_server(&server);
        *lock(&self.server) = server;
        ready(Ok(()))
    }

    fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo> {
        let url = key(server.url());
        if let Err(why) = check_server(&url) {
            return ready(Err(BackendError::Invalid(why)));
        }
        Box::pin(async move {
            let started = Instant::now();
            // No session sent: a 401 here signs no one out.
            let info = Client::new(&url, None)
                .info()
                .map_err(|e| Self::describe_error(&url, e))?;
            Ok(ConnectionInfo {
                version: info.version,
                sign_in: info.sign_in,
                latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            })
        })
    }

    fn sign_in_url(&self, server: &Server) -> BoxFut<'_, String> {
        let url = key(server.url());
        // A plain-http server fails here, not as a refused callback later.
        if let Err(why) = check_server(&url) {
            return ready(Err(BackendError::Invalid(why)));
        }
        Box::pin(async move {
            let info = Client::new(&url, None)
                .info()
                .map_err(|e| Self::describe_error(&url, e))?;
            let app = info
                .app_url
                .map(|a| key(&a))
                .filter(|a| !a.is_empty())
                .ok_or_else(|| BackendError::Invalid("This server has no sign-in page.".into()))?;
            // The server names the page the browser opens: only a web page
            // (no other scheme, no `curl-runner://` callback of its own).
            check_server(&app).map_err(BackendError::Invalid)?;
            self.pending.start(&url);
            Ok(format!("{app}/sign-in?client=studio"))
        })
    }

    fn complete_sign_in(&self, callback: &AuthCallback) -> BoxFut<'_, Account> {
        // Before anything is sent: only a sign-in this studio started.
        if let Err(e) = self.pending.redeem(callback) {
            return ready(Err(e));
        }
        let (server, code) = (key(&callback.server), callback.code.clone());
        Box::pin(async move {
            let verified = Client::new(&server, None)
                .handoff_redeem(&code)
                .map_err(|e| match Self::describe_error(&server, e) {
                    // Not a session going bad: the sign-in failed.
                    BackendError::SignedOut => BackendError::Unauthorized,
                    other => other,
                })?;
            self.sessions.set(&server, &verified.session).map_err(|_| {
                BackendError::Invalid("Couldn't save your sign-in to the Keychain.".into())
            })?;
            // The redeem names no teams: ask. Signed in either way; the
            // teams come with the next refresh if this fails.
            let teams = Client::new(&server, Some(verified.session))
                .me()
                .map(|me| me.teams.into_iter().map(TeamRef::from).collect())
                .unwrap_or_default();
            Ok(self.remember(&server, verified.user, teams))
        })
    }

    /// Local first: the session is forgotten even if the server can't be
    /// reached to hear about it.
    fn sign_out(&self, server: &str) -> BoxFut<'_, ()> {
        let server = key(server);
        Box::pin(async move {
            if let Ok(Some(token)) = self.sessions.get(&server) {
                // Best effort: an unreachable server keeps its copy until
                // it expires.
                let _ = Client::new(&server, Some(token)).sign_out();
            }
            lock(&self.lapsed).remove(&server);
            self.forget(&server)
        })
    }

    fn account(&self) -> Option<Account> {
        self.account_on(self.server().url())
    }

    fn account_on(&self, server: &str) -> Option<Account> {
        lock(&self.accounts).0.get(&key(server)).map(account_of)
    }

    fn refresh_account(&self, server: &str) -> BoxFut<'_, Account> {
        let server = key(server);
        Box::pin(async move {
            let token = match self.sessions.get(&server) {
                Ok(Some(t)) => t,
                Ok(None) => return Err(BackendError::SignedOut),
                Err(_) => return Err(keychain_unreadable()),
            };
            let me = Client::new(&server, Some(token))
                .me()
                .map_err(|e| self.map_error(&server, e))?;
            let teams = me.teams.into_iter().map(TeamRef::from).collect();
            Ok(self.remember(&server, me.user, teams))
        })
    }

    fn lapsed(&self) -> Option<Account> {
        lock(&self.lapsed).get(&key(self.server().url())).cloned()
    }

    fn accounts(&self) -> Vec<(String, Account)> {
        lock(&self.accounts)
            .0
            .iter()
            .map(|(s, c)| (s.clone(), account_of(c)))
            .collect()
    }
    /// Until 3b: `sync` answers `Unavailable`.
    /// Per collection instead: one syncs when its source on the server is
    /// the studio (the workspace asks [`Backend::remote_collection`]).
    fn team_sync(&self) -> bool {
        false
    }
    fn sync_status(&self, _: &CollectionKey) -> SyncStatus {
        SyncStatus::Off
    }
    fn publishes(&self) -> bool {
        true
    }
    fn remote_collection(&self, team: &str, slug: &str) -> BoxFut<'_, Option<RemoteCollection>> {
        let (server, team, slug) = (key(self.server().url()), team.to_string(), slug.to_string());
        Box::pin(async move {
            let rows = self.authed(&server, |c| c.collections(&team))?;
            let Some(row) = rows.into_iter().find(|r| r.slug == slug) else {
                return Ok(None);
            };
            let current = match row.current_version {
                Some(n) => Some(self.authed(&server, |c| c.version(&team, &slug, n))?),
                None => None,
            };
            Ok(Some(RemoteCollection {
                source: row.source.unwrap_or(CollectionSource::Git),
                current,
            }))
        })
    }
    fn remote_files(&self, team: &str, slug: &str, n: u32) -> BoxFut<'_, Vec<BundleFile>> {
        let (server, team, slug) = (key(self.server().url()), team.to_string(), slug.to_string());
        Box::pin(async move { self.authed(&server, |c| c.version_files(&team, &slug, n)) })
    }
    fn publish(
        &self,
        team: &str,
        slug: &str,
        request: PublishRequest,
    ) -> BoxFut<'_, PublishResponse> {
        let (server, team, slug) = (key(self.server().url()), team.to_string(), slug.to_string());
        Box::pin(async move {
            // Publishing needs the session: no anonymous retry.
            let token = match self.sessions.get(&server) {
                Ok(Some(t)) => t,
                Ok(None) => return Err(BackendError::SignedOut),
                Err(_) => return Err(keychain_unreadable()),
            };
            Client::new(&server, Some(token))
                .publish(&team, &slug, &request)
                .map_err(|e| self.publish_error(&server, &team, e))
        })
    }
    fn team_share(&self, team: &str, slug: &str) -> BoxFut<'_, TeamShare> {
        let (server, team, slug) = (key(self.server().url()), team.to_string(), slug.to_string());
        Box::pin(async move { self.team_call(&server, &team, |c| c.share(&team, &slug)) })
    }
    fn change_team_share(
        &self,
        team: &str,
        slug: &str,
        change: ShareChange,
    ) -> BoxFut<'_, TeamShare> {
        let (server, team, slug) = (key(self.server().url()), team.to_string(), slug.to_string());
        Box::pin(async move {
            self.team_call(&server, &team, |c| c.change_share(&team, &slug, &change))
        })
    }
    fn add_share_entry(
        &self,
        team: &str,
        slug: &str,
        entry: &str,
        notify: bool,
    ) -> BoxFut<'_, ShareEntry> {
        let (server, team, slug, entry) = (
            key(self.server().url()),
            team.to_string(),
            slug.to_string(),
            entry.to_string(),
        );
        Box::pin(async move {
            self.team_call(&server, &team, |c| {
                c.add_share_entry(&team, &slug, &entry, notify)
            })
        })
    }
    fn remove_share_entry(&self, team: &str, slug: &str, entry: &str) -> BoxFut<'_, ()> {
        let (server, team, slug, entry) = (
            key(self.server().url()),
            team.to_string(),
            slug.to_string(),
            entry.to_string(),
        );
        Box::pin(async move {
            self.team_call(&server, &team, |c| {
                c.remove_share_entry(&team, &slug, &entry)
            })
        })
    }
    fn dashboard_url(&self) -> BoxFut<'_, String> {
        let server = self.server();
        Box::pin(async move {
            if server == Server::Cloud {
                return Ok(CLOUD_APP_URL.to_string());
            }
            let url = key(server.url());
            let info = Client::new(&url, None)
                .info()
                .map_err(|e| Self::describe_error(&url, e))?;
            let app = info
                .app_url
                .map(|a| key(&a))
                .filter(|a| !a.is_empty())
                .ok_or_else(|| BackendError::Invalid("This server has no dashboard.".into()))?;
            check_server(&app).map_err(BackendError::Invalid)?;
            Ok(app)
        })
    }
    /// Needs the key's Cloud link: a folder that isn't on Cloud has no
    /// collection to sync with.
    fn sync(
        &self,
        collection: &CollectionKey,
        local: &[SyncFile],
        deleted: &[SyncDeletion],
    ) -> BoxFut<'_, SyncReport> {
        let server = key(self.server().url());
        let link = collection.cloud.clone();
        let (req, texts) = sync_request(local, deleted);
        Box::pin(async move {
            let Some(link) = link else {
                return Err(BackendError::Unavailable);
            };
            let res = self.team_call(&server, &link.team, |c| {
                c.sync(&link.team, &link.slug, &req)
            })?;
            Ok(sync_report(res, &texts))
        })
    }
    fn share_settings(&self, _: &CollectionKey) -> BoxFut<'_, ShareSettings> {
        unavailable()
    }
    fn update_share(&self, _: &CollectionKey, _: ShareUpdate) -> BoxFut<'_, ShareSettings> {
        unavailable()
    }
    fn invite(&self, _: &CollectionKey, _: &str) -> BoxFut<'_, Person> {
        unavailable()
    }
    fn remove_person(&self, _: &CollectionKey, _: &str) -> BoxFut<'_, ()> {
        unavailable()
    }
    /// The card first (a restricted link without a session stops there,
    /// before any file is sent), then the snapshot for its inputs.
    fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection> {
        let link = link.clone();
        Box::pin(async move {
            let info = self.authed(&link.server, |c| c.shared_info(&link.token))?;
            let snap = snapshot_of(self.authed(&link.server, |c| c.snapshot(&link.token))?);
            Ok(SharedCollection {
                summary: folders_line(&snap.files),
                owner_org: info.team,
                team_slug: info.team_slug,
                name: info.name,
                updated: info.updated_at,
                request_count: usize::try_from(info.request_count).unwrap_or(usize::MAX),
                version: info.version,
                access: info.access,
                variables: snap.variables,
                link,
            })
        })
    }

    fn fetch_shared(&self, link: &ShareLink) -> BoxFut<'_, SharedSnapshot> {
        let link = link.clone();
        Box::pin(async move {
            self.authed(&link.server, |c| c.snapshot(&link.token))
                .map(snapshot_of)
        })
    }

    fn latest_version(&self, link: &ShareLink) -> BoxFut<'_, u32> {
        let link = link.clone();
        Box::pin(async move {
            self.authed(&link.server, |c| c.latest(&link.token))
                .map(|l| l.version)
        })
    }

    fn received(&self) -> Vec<ReceivedShare> {
        lock(&self.received).clone()
    }

    fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
        ready(self.edit_received(|all| {
            all.retain(|r| r.link.token != share.link.token);
            all.push(share);
        }))
    }

    fn update_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
        ready(self.edit_received(|all| {
            match all.iter_mut().find(|r| r.link.token == share.link.token) {
                Some(r) => *r = share,
                None => all.push(share),
            }
        }))
    }
    fn dev(&self) -> Option<&dyn DevControls> {
        None
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod tests;

/// `rates/get.yaml` on every OS: the server's form of a path.
fn wire_path(rel: &std::path::Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The request for `local` and `deleted`, and each local text by wire path
/// (the conflicts' "mine"). Texts go only for files changed since their
/// base; a base kept before 2.13 is unknown to the server, so a file goes
/// without one and a deletion waits (it can't be checked against the
/// server's copy).
fn sync_request(
    local: &[SyncFile],
    deleted: &[SyncDeletion],
) -> (
    curl_runner_cloud_proto::sync::SyncRequest,
    std::collections::HashMap<String, String>,
) {
    use super::is_current_hash;
    use curl_runner_cloud_proto::sync::{SyncDeletion as WireDeletion, SyncFile as WireFile};
    let mut texts = std::collections::HashMap::with_capacity(local.len());
    let files = local
        .iter()
        .map(|f| {
            let path = wire_path(&f.rel);
            texts.insert(path.clone(), f.text.clone());
            let base = f.base.clone().filter(|b| is_current_hash(b));
            WireFile {
                text: (base.as_ref() != Some(&f.hash)).then(|| f.text.clone()),
                path,
                hash: f.hash.clone(),
                base,
            }
        })
        .collect();
    let deleted = deleted
        .iter()
        .filter(|d| is_current_hash(&d.base))
        .map(|d| WireDeletion {
            path: wire_path(&d.rel),
            base: d.base.clone(),
        })
        .collect();
    (
        curl_runner_cloud_proto::sync::SyncRequest { files, deleted },
        texts,
    )
}

/// The server's answer as the workspace takes it. A conflict on a file not
/// sent (deleted here) is a local deletion.
fn sync_report(
    res: curl_runner_cloud_proto::sync::SyncResponse,
    texts: &std::collections::HashMap<String, String>,
) -> SyncReport {
    SyncReport {
        pushed: res.pushed.len(),
        pulled: res
            .pulled
            .into_iter()
            .map(|f| SyncFile {
                rel: std::path::PathBuf::from(&f.path),
                hash: f.hash,
                text: f.text,
                base: None,
            })
            .collect(),
        conflicts: res
            .conflicts
            .into_iter()
            .map(|c| {
                let local = texts.get(&c.path);
                super::Conflict {
                    rel: std::path::PathBuf::from(&c.path),
                    local: local.cloned().unwrap_or_default(),
                    local_deleted: local.is_none(),
                    server: c.server_text.unwrap_or_default(),
                    server_deleted: c.deleted,
                    server_author: c.server_author,
                    server_at: c.server_at,
                }
            })
            .collect(),
        deleted: res
            .deleted
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect(),
    }
}
