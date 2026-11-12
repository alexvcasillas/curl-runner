//! [`MockBackend`]: the server, in process. Its state (sign-in, server
//! choice, shares, people, received shares, server copies of synced files
//! and the dev toggles) persists in `{base}/mock-backend.json`.
//!
//! Every call behaves like a network: it resolves after `latency` and
//! fails with [`BackendError::Offline`] if the offline toggle is on when it
//! resolves (so turning Offline on mid-call fails that call).

use super::fixtures::{self, CONFLICT_AUTHOR};
use super::{
    Access, Account, AuthCallback, Backend, BackendError, BoxFut, CollectionKey, Conflict,
    ConnectionInfo, DevControls, DevToggles, Expiry, LinkAccess, PendingSignIns, Person,
    ReceivedShare, Role, Server, ShareLink, ShareSettings, ShareUpdate, SharedCollection,
    SharedSnapshot, SharedVar, SignInMethod, SyncDeletion, SyncFile, SyncReport, SyncStatus,
};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// How long every mock call takes in the app.
pub const MOCK_LATENCY: Duration = Duration::from_millis(120);

/// The state file under the studio's data folder.
const STATE_FILE: &str = "mock-backend.json";
/// Recorded arguments kept (oldest dropped first).
const RECORD_CAP: usize = 2000;
/// How long a new link lasts (the Share board's default).
const DEFAULT_EXPIRY_DAYS: u32 = 30;
/// The server the mock's links name.
pub const MOCK_SERVER: &str = "https://api.mock.curl-runner.test";

pub struct MockBackend {
    /// `None`: memory only (tests).
    base: Option<PathBuf>,
    latency: Duration,
    state: Mutex<State>,
    log: Mutex<VecDeque<String>>,
    /// While held, `update_share` answers wait (the change is already made)
    /// and `add_received` waits before it is made.
    gate: Arc<Gate>,
    /// The sign-ins started and not yet answered (not persisted).
    pending: PendingSignIns,
}

/// [`DevControls::hold_updates`]: held, and the answers waiting on it.
#[derive(Default)]
struct Gate {
    inner: Mutex<(bool, Vec<Waker>)>,
}

impl Gate {
    #[cfg(test)]
    fn set(&self, held: bool) {
        let wakers = {
            let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            g.0 = held;
            if held {
                Vec::new()
            } else {
                std::mem::take(&mut g.1)
            }
        };
        wakers.into_iter().for_each(Waker::wake);
    }
}

/// Resolves once the gate is open.
struct GateOpen(Arc<Gate>);

impl Future for GateOpen {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut g = self.0.inner.lock().unwrap_or_else(|e| e.into_inner());
        if g.0 {
            g.1.push(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct State {
    server: Server,
    /// The session on each server, by API URL (no trailing `/`).
    accounts: BTreeMap<String, Account>,
    /// Sessions the server refused, by API URL ([`Backend::lapsed`]).
    lapsed: BTreeMap<String, Account>,
    toggles: DevToggles,
    /// Owned shares, by collection id.
    shares: BTreeMap<String, OwnedShare>,
    received: Vec<ReceivedShare>,
    /// The server's copy of each synced collection, by collection id.
    synced: BTreeMap<String, ServerCollection>,
    /// Bumped per generated token.
    seq: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnedShare {
    name: String,
    owner_org: String,
    token: String,
    access: Access,
    expires: Option<NaiveDate>,
    people: Vec<Person>,
    variables: Vec<SharedVar>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ServerCollection {
    /// By `/`-joined relative path.
    files: BTreeMap<String, ServerFile>,
    synced_at: Option<DateTime<Utc>>,
    /// Bumped by each sync that changes a file: what a share of it
    /// publishes.
    version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerFile {
    text: String,
    hash: u64,
    author: String,
    at: DateTime<Utc>,
}

impl MockBackend {
    /// The mock whose state lives in `{base}/mock-backend.json`; missing or
    /// corrupt is the first-launch state. `base` `None` keeps it in memory.
    pub fn open(base: Option<&Path>, latency: Duration) -> MockBackend {
        let state: State = base
            .and_then(|b| std::fs::read(b.join(STATE_FILE)).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let state = keep_toggles(state, cfg!(debug_assertions));
        MockBackend {
            base: base.map(Path::to_path_buf),
            latency,
            state: Mutex::new(state),
            log: Mutex::default(),
            gate: Arc::default(),
            pending: PendingSignIns::default(),
        }
    }

    /// With `clock` as the time sign-ins expire by (tests).
    #[cfg(test)]
    pub fn with_clock(mut self, clock: super::pending::Clock) -> MockBackend {
        self.pending = PendingSignIns::with_clock(clock);
        self
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Best effort: an unwritable state file only loses the mock's memory.
    fn save(&self, state: &State) {
        let Some(base) = &self.base else { return };
        let Ok(json) = serde_json::to_string_pretty(state) else {
            return;
        };
        let _ = std::fs::create_dir_all(base);
        let _ = crate::model::write_atomic(&base.join(STATE_FILE), &json);
    }

    fn record(&self, call: String) {
        if !cfg!(test) {
            return;
        }
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        if log.len() >= RECORD_CAP {
            log.pop_front();
        }
        log.push_back(call);
    }

    /// A call that resolves after the latency: `f` runs then, on the state,
    /// which is saved afterwards when `save` is set.
    fn call<'a, T: Send + 'a>(
        &'a self,
        save: bool,
        f: impl FnOnce(&mut State) -> Result<T, BackendError> + Send + 'a,
    ) -> BoxFut<'a, T> {
        let delay = Delay::new(self.latency);
        Box::pin(async move {
            delay.await;
            let mut state = self.state();
            let out = f(&mut state);
            if save {
                self.save(&state);
            }
            out
        })
    }

    fn set_toggle(&self, f: impl FnOnce(&mut DevToggles)) {
        let mut state = self.state();
        f(&mut state.toggles);
        self.save(&state);
    }
}

/// The dev toggles survive a relaunch only in debug builds (the menu that
/// sets them is debug only): a release build starts with them all off.
fn keep_toggles(mut state: State, debug: bool) -> State {
    if !debug {
        state.toggles = DevToggles::default();
    }
    state
}

fn online(s: &State) -> Result<(), BackendError> {
    if s.toggles.offline {
        Err(BackendError::Offline)
    } else {
        Ok(())
    }
}

/// `server` as the sessions are keyed: no trailing `/`.
fn server_key(server: &str) -> String {
    server.trim().trim_end_matches('/').to_string()
}

/// The account on the state's server.
fn account_of(s: &State) -> Option<&Account> {
    s.accounts.get(&server_key(s.server.url()))
}

fn signed_in(s: &State) -> Result<Account, BackendError> {
    online(s)?;
    account_of(s).cloned().ok_or(BackendError::SignedOut)
}

/// `rates/get-rates.yaml` on every OS.
fn rel_key(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// `crsl_` and 6 base62 chars from the collection, a counter and the clock.
fn new_token(id: &str, seq: u64) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut h = SyncFile::hash_of(&format!("{id}:{seq}:{nanos}"));
    let tail: String = (0..6)
        .map(|_| {
            let c = ALPHABET[(h % 62) as usize] as char;
            h /= 62;
            c
        })
        .collect();
    format!("crsl_{tail}")
}

fn expiry_date(e: Expiry) -> Option<NaiveDate> {
    match e {
        Expiry::Days(n) => Utc::now()
            .date_naive()
            .checked_add_days(Days::new(u64::from(n))),
        Expiry::Never => None,
    }
}

/// A secret never carries a default (nor a lock), whatever the caller
/// sent; a lock needs a default.
fn scrub(vars: Vec<SharedVar>) -> Vec<SharedVar> {
    vars.into_iter()
        .map(|mut v| {
            if v.secret {
                v.default = None;
            }
            v.locked &= v.default.is_some();
            v
        })
        .collect()
}

fn owner(account: &Account) -> Person {
    Person {
        name: account.name.clone(),
        email: account.email.clone(),
        role: Role::Owner,
        invited: false,
    }
}

fn settings_of(share: &OwnedShare) -> ShareSettings {
    ShareSettings {
        link: Some(format!(
            "curl-runner://open?server={MOCK_SERVER}&share={}",
            share.token
        )),
        access: share.access,
        expires: share.expires,
        people: share.people.clone(),
        variables: share.variables.clone(),
    }
}

/// The share of `key`, made on first use (a link that lasts 30 days).
fn share_mut<'s>(s: &'s mut State, key: &CollectionKey, account: &Account) -> &'s mut OwnedShare {
    let seq = s.seq;
    let share = s
        .shares
        .entry(key.id.clone())
        .or_insert_with(|| OwnedShare {
            name: key.name.clone(),
            owner_org: fixtures::OWNER_TEAM.into(),
            token: new_token(&key.id, seq),
            access: Access::AnyoneWithLink,
            expires: expiry_date(Expiry::Days(DEFAULT_EXPIRY_DAYS)),
            people: vec![owner(account)],
            variables: Vec::new(),
        });
    s.seq += 1;
    share
}

/// The server's side of a conflict: `local` with its first value changed.
/// The first integer is doubled (`2500` → `5000`, as on the board); with
/// none, the first scalar gets ` (edited)`; with no scalar, a comment is
/// added. Deterministic.
pub(super) fn server_side(local: &str) -> String {
    let lines: Vec<&str> = local.split_inclusive('\n').collect();
    let value_of = |line: &str| -> Option<(usize, String)> {
        let body = line.trim_end_matches(['\n', '\r']);
        let trimmed = body.trim_start();
        if trimmed.starts_with('#') {
            return None;
        }
        let colon = body.find(": ")?;
        let value = body[colon + 2..].trim();
        (!value.is_empty() && !value.starts_with(['|', '>', '&', '*', '#']))
            .then(|| (colon + 2, value.to_string()))
    };
    let rewrite = |i: usize, at: usize, new: &str| {
        let line = lines[i];
        let end = &line[line.trim_end_matches(['\n', '\r']).len()..];
        let mut out: String = lines[..i].concat();
        out.push_str(&line[..at]);
        out.push_str(new);
        out.push_str(end);
        out.push_str(&lines[i + 1..].concat());
        out
    };
    let values: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| value_of(l).map(|(at, v)| (i, at, v)))
        .collect();
    if let Some((i, at, n)) = values
        .iter()
        .find_map(|(i, at, v)| v.parse::<i64>().ok().map(|n| (*i, *at, n)))
    {
        return rewrite(i, at, &n.saturating_mul(2).to_string());
    }
    if let Some((i, at, v)) = values.first() {
        let edited = match v.strip_suffix('"').filter(|_| v.starts_with('"')) {
            Some(inner) => format!("{inner} (edited)\""),
            None => format!("{v} (edited)"),
        };
        return rewrite(*i, *at, &edited);
    }
    let mut out = local.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("# changed on the server\n");
    out
}

/// What a link's token opens.
enum Found<'s> {
    Fixture,
    Owned(&'s str, &'s OwnedShare),
}

/// The share behind `link`, if the caller may open it: not expired, and a
/// restricted one (`InvitedOnly`) only with a session on `link.server`.
fn open_share<'s>(s: &'s State, link: &ShareLink) -> Result<Found<'s>, BackendError> {
    online(s)?;
    let found = if link.token == fixtures::SHIPPING_TOKEN {
        Found::Fixture
    } else {
        let (id, share) = s
            .shares
            .iter()
            .find(|(_, sh)| sh.token == link.token)
            .ok_or(BackendError::NotFound)?;
        Found::Owned(id, share)
    };
    let expired = match &found {
        Found::Fixture => false,
        Found::Owned(_, share) => share.expires.is_some_and(|d| d < Utc::now().date_naive()),
    };
    if s.toggles.expire_links || expired {
        return Err(BackendError::Expired);
    }
    let restricted = matches!(&found, Found::Owned(_, sh) if sh.access == Access::InvitedOnly);
    if restricted && !s.accounts.contains_key(&server_key(&link.server)) {
        return Err(BackendError::SignedOut);
    }
    Ok(found)
}

/// What `found` holds now: its version, files and variables.
fn snapshot_of(s: &State, found: &Found<'_>) -> SharedSnapshot {
    match found {
        Found::Fixture => SharedSnapshot {
            files: fixtures::shipping_snapshot(),
            variables: fixtures::shipping_variables(),
            version: fixtures::SHIPPING_VERSION,
            published_at: Utc::now() - chrono::Duration::days(fixtures::SHIPPING_UPDATED_DAYS_AGO),
        },
        Found::Owned(id, share) => {
            let col = s.synced.get(*id);
            SharedSnapshot {
                files: col
                    .into_iter()
                    .flat_map(|c| &c.files)
                    .map(|(rel, f)| SyncFile {
                        rel: rel.into(),
                        hash: f.hash,
                        text: f.text.clone(),
                        base: None,
                    })
                    .collect(),
                variables: share.variables.clone(),
                version: col.map_or(0, |c| c.version),
                published_at: col.and_then(|c| c.synced_at).unwrap_or_else(Utc::now),
            }
        }
    }
}

fn link_access(access: Access) -> LinkAccess {
    match access {
        Access::AnyoneWithLink => LinkAccess::Public,
        Access::InvitedOnly => LinkAccess::Restricted,
    }
}

/// The top folders of `rels`, in order of first use, at most three.
fn folders_line<'r>(rels: impl Iterator<Item = &'r String>) -> String {
    let mut tops: Vec<&str> = Vec::new();
    for rel in rels {
        if let Some((top, _)) = rel.split_once('/')
            && !tops.contains(&top)
        {
            tops.push(top);
        }
    }
    tops.truncate(3);
    tops.join(", ")
}

impl Backend for MockBackend {
    fn server(&self) -> Server {
        self.state().server.clone()
    }

    fn set_server(&self, server: Server) -> BoxFut<'_, ()> {
        self.record(format!("set_server({server:?})"));
        self.call(true, move |s| {
            online(s)?;
            s.server = server;
            Ok(())
        })
    }

    fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo> {
        self.record(format!("test_connection({server:?})"));
        let server = server.clone();
        self.call(false, move |s| {
            online(s)?;
            let sign_in = match &server {
                Server::Cloud => vec![
                    SignInMethod::Github,
                    SignInMethod::Google,
                    SignInMethod::Email,
                ],
                Server::SelfHosted { url } => {
                    let host = url
                        .strip_prefix("https://")
                        .or_else(|| url.strip_prefix("http://"))
                        .unwrap_or("");
                    if host.is_empty() || host.starts_with('/') || url.contains(char::is_whitespace)
                    {
                        return Err(BackendError::Invalid(
                            "Enter the server's address, like https://curl-runner.example.com."
                                .into(),
                        ));
                    }
                    // Out of order on purpose: the studio sorts them.
                    vec![SignInMethod::Email, SignInMethod::Github]
                }
            };
            Ok(ConnectionInfo {
                version: fixtures::SERVER_VERSION.into(),
                sign_in,
                latency_ms: fixtures::CONNECTION_MS,
            })
        })
    }

    /// The callback itself: the studio opens it, as the dashboard would.
    fn sign_in_url(&self, server: &Server) -> BoxFut<'_, String> {
        self.record(format!("sign_in_url({server:?})"));
        let url = server.url().to_string();
        if let Err(why) = curl_runner_cloud_client::check_server(url.trim_end_matches('/')) {
            return Box::pin(async { Err(BackendError::Invalid(why)) });
        }
        let started = self.call(false, |s| online(s));
        Box::pin(async move {
            started.await?;
            self.pending.start(&url);
            Ok(format!(
                "curl-runner://auth/callback?code={}&server={url}",
                fixtures::MOCK_CODE
            ))
        })
    }

    fn complete_sign_in(&self, callback: &AuthCallback) -> BoxFut<'_, Account> {
        if let Err(e) = self.pending.redeem(callback) {
            return Box::pin(async { Err(e) });
        }
        // Past the checks: this is where a server gets the code.
        self.record(format!("redeem({:?})", callback.server));
        let (server, code) = (server_key(&callback.server), callback.code.clone());
        self.call(true, move |s| {
            online(s)?;
            if s.toggles.fail_next_sign_in {
                s.toggles.fail_next_sign_in = false;
                return Err(BackendError::Unauthorized);
            }
            let account = fixtures::account(&code);
            s.lapsed.remove(&server);
            s.accounts.insert(server, account.clone());
            Ok(account)
        })
    }

    /// Local first: forgetting the account needs no network, so it works
    /// offline too.
    fn sign_out(&self, server: &str) -> BoxFut<'_, ()> {
        self.record(format!("sign_out({server:?})"));
        let server = server_key(server);
        self.call(true, move |s| {
            s.accounts.remove(&server);
            s.lapsed.remove(&server);
            Ok(())
        })
    }

    fn account(&self) -> Option<Account> {
        account_of(&self.state()).cloned()
    }

    fn account_on(&self, server: &str) -> Option<Account> {
        self.state().accounts.get(&server_key(server)).cloned()
    }

    fn refresh_account(&self, server: &str) -> BoxFut<'_, Account> {
        self.record(format!("refresh_account({server:?})"));
        let server = server_key(server);
        self.call(false, move |s| {
            online(s)?;
            s.accounts
                .get(&server)
                .cloned()
                .ok_or(BackendError::SignedOut)
        })
    }

    fn lapsed(&self) -> Option<Account> {
        let s = self.state();
        s.lapsed.get(&server_key(s.server.url())).cloned()
    }

    fn accounts(&self) -> Vec<(String, Account)> {
        let s = self.state();
        s.accounts
            .iter()
            .map(|(k, a)| (k.clone(), a.clone()))
            .collect()
    }

    fn sync_status(&self, collection: &CollectionKey) -> SyncStatus {
        let s = self.state();
        if account_of(&s).is_none() {
            return SyncStatus::Off;
        }
        let Some(col) = s.synced.get(&collection.id) else {
            return SyncStatus::Off;
        };
        match col.synced_at {
            Some(at) => {
                let done = s.synced.values().filter(|c| c.synced_at.is_some());
                SyncStatus::Synced {
                    at,
                    collections: done.clone().count(),
                    requests: done.map(|c| c.files.len()).sum(),
                }
            }
            None => SyncStatus::Off,
        }
    }

    fn sync(
        &self,
        collection: &CollectionKey,
        local: &[SyncFile],
        deleted: &[SyncDeletion],
    ) -> BoxFut<'_, SyncReport> {
        // Paths and hashes only: the log stays small, and request text is
        // the caller's to check.
        let files: Vec<_> = local.iter().map(|f| (&f.rel, f.hash, f.base)).collect();
        self.record(format!("sync({collection:?}, {files:?}, {deleted:?})"));
        let (key, local, deleted) = (collection.clone(), local.to_vec(), deleted.to_vec());
        self.call(true, move |s| {
            let author = signed_in(s)?.name;
            let col = s.synced.get(&key.id);
            // Already the same on the server (whatever the base): nothing
            // to push, pull or report.
            let changed: Vec<&SyncFile> = local
                .iter()
                .filter(|f| {
                    col.and_then(|c| c.files.get(&rel_key(&f.rel)))
                        .is_none_or(|sf| sf.hash != f.hash)
                })
                .collect();
            let now = Utc::now();
            let mut report = SyncReport::default();
            // With nothing changed, a forced conflict takes the first file.
            let forced = s
                .toggles
                .conflict_next_sync
                .then(|| changed.first().copied().or(local.first()))
                .flatten()
                .cloned();
            let changed: Vec<SyncFile> = changed.into_iter().cloned().collect();
            let col = s.synced.entry(key.id.clone()).or_default();
            let before = col.files.clone();
            for d in &deleted {
                let k = rel_key(&d.rel);
                match col.files.get(&k) {
                    Some(sf) if sf.hash == d.base => {
                        col.files.remove(&k);
                    }
                    // Changed on the server since: theirs stays.
                    Some(sf) => report.conflicts.push(Conflict {
                        rel: d.rel.clone(),
                        local: String::new(),
                        server: sf.text.clone(),
                        server_author: sf.author.clone(),
                        server_at: sf.at,
                        local_deleted: true,
                        server_deleted: false,
                    }),
                    None => {}
                }
            }
            for f in &changed {
                if forced.as_ref().is_some_and(|c| c.rel == f.rel) {
                    continue;
                }
                let k = rel_key(&f.rel);
                if let Some(sf) = col.files.get(&k)
                    && f.base != Some(sf.hash)
                {
                    if f.base == Some(f.hash) {
                        // Unchanged here, newer there: pull theirs.
                        report.pulled.push(SyncFile {
                            rel: f.rel.clone(),
                            hash: sf.hash,
                            text: sf.text.clone(),
                            base: None,
                        });
                    } else {
                        // Changed on both sides: theirs stays, the client
                        // is told.
                        report.conflicts.push(Conflict {
                            rel: f.rel.clone(),
                            local: f.text.clone(),
                            server: sf.text.clone(),
                            server_author: sf.author.clone(),
                            server_at: sf.at,
                            local_deleted: false,
                            server_deleted: false,
                        });
                    }
                    continue;
                }
                let file = ServerFile {
                    text: f.text.clone(),
                    hash: f.hash,
                    author: author.clone(),
                    at: now,
                };
                col.files.insert(k, file);
                report.pushed += 1;
            }
            if let Some(c) = forced {
                let at = now - chrono::Duration::hours(1);
                let server = server_side(&c.text);
                col.files.insert(
                    rel_key(&c.rel),
                    ServerFile {
                        hash: SyncFile::hash_of(&server),
                        text: server.clone(),
                        author: CONFLICT_AUTHOR.into(),
                        at,
                    },
                );
                report.conflicts.push(Conflict {
                    rel: c.rel,
                    local: c.text,
                    server,
                    server_author: CONFLICT_AUTHOR.into(),
                    server_at: at,
                    local_deleted: false,
                    server_deleted: false,
                });
            }
            if col.files != before {
                col.version += 1;
            }
            col.synced_at = Some(now);
            s.toggles.conflict_next_sync = false;
            Ok(report)
        })
    }

    fn share_settings(&self, collection: &CollectionKey) -> BoxFut<'_, ShareSettings> {
        self.record(format!("share_settings({collection:?})"));
        let key = collection.clone();
        self.call(false, move |s| {
            let account = signed_in(s)?;
            Ok(match s.shares.get(&key.id) {
                Some(share) => settings_of(share),
                None => ShareSettings {
                    link: None,
                    access: Access::default(),
                    expires: None,
                    people: vec![owner(&account)],
                    variables: Vec::new(),
                },
            })
        })
    }

    fn update_share(
        &self,
        collection: &CollectionKey,
        update: ShareUpdate,
    ) -> BoxFut<'_, ShareSettings> {
        self.record(format!("update_share({collection:?}, {update:?})"));
        let key = collection.clone();
        let answer = self.call(true, move |s| {
            let account = signed_in(s)?;
            let seq = s.seq;
            let share = share_mut(s, &key, &account);
            if let Some(access) = update.access {
                share.access = access;
            }
            if let Some(e) = update.expires {
                share.expires = expiry_date(e);
            }
            if let Some(vars) = update.variables {
                share.variables = scrub(vars);
            }
            if update.reset_link {
                share.token = new_token(&key.id, seq + 1);
            }
            let settings = settings_of(share);
            s.seq += 1;
            Ok(settings)
        });
        let gate = GateOpen(self.gate.clone());
        Box::pin(async move {
            let out = answer.await;
            gate.await;
            out
        })
    }

    /// Inviting to a collection that has no share yet creates its link.
    fn invite(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, Person> {
        self.record(format!("invite({collection:?}, {email:?})"));
        let (key, email) = (collection.clone(), email.trim().to_string());
        self.call(true, move |s| {
            let account = signed_in(s)?;
            let valid = email
                .split_once('@')
                .is_some_and(|(user, host)| !user.is_empty() && host.contains('.'))
                && !email.contains(char::is_whitespace);
            if !valid {
                return Err(BackendError::Invalid("Enter an email address.".into()));
            }
            let share = share_mut(s, &key, &account);
            if share
                .people
                .iter()
                .any(|p| p.email.eq_ignore_ascii_case(&email))
            {
                return Err(BackendError::Invalid(format!(
                    "{email} already has access."
                )));
            }
            let user = email.split('@').next().unwrap_or_default();
            let mut name: Vec<char> = user.chars().collect();
            if let Some(c) = name.first_mut() {
                *c = c.to_ascii_uppercase();
            }
            let person = Person {
                name: name.into_iter().collect(),
                email,
                role: Role::Viewer,
                invited: true,
            };
            share.people.push(person.clone());
            Ok(person)
        })
    }

    fn remove_person(&self, collection: &CollectionKey, email: &str) -> BoxFut<'_, ()> {
        self.record(format!("remove_person({collection:?}, {email:?})"));
        let (key, email) = (collection.clone(), email.trim().to_string());
        self.call(true, move |s| {
            signed_in(s)?;
            let share = s.shares.get_mut(&key.id).ok_or(BackendError::NotFound)?;
            let at = share
                .people
                .iter()
                .position(|p| p.email.eq_ignore_ascii_case(&email))
                .ok_or(BackendError::NotFound)?;
            if share.people[at].role == Role::Owner {
                return Err(BackendError::Invalid("The owner keeps access.".into()));
            }
            share.people.remove(at);
            Ok(())
        })
    }

    /// A restricted link (`InvitedOnly`) needs a session on its server;
    /// the mock does not check the invite list against it.
    fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection> {
        self.record(format!("resolve_link({link:?})"));
        let link = link.clone();
        self.call(false, move |s| {
            let found = open_share(s, &link)?;
            let snap = snapshot_of(s, &found);
            let (owner_org, name, access) = match found {
                Found::Fixture => (
                    fixtures::SHIPPING_ORG.to_string(),
                    fixtures::SHIPPING_NAME.to_string(),
                    LinkAccess::Public,
                ),
                Found::Owned(_, share) => (
                    share.owner_org.clone(),
                    share.name.clone(),
                    link_access(share.access),
                ),
            };
            let summary = match found {
                Found::Fixture => fixtures::SHIPPING_SUMMARY.to_string(),
                Found::Owned(..) => {
                    let rels: Vec<String> = snap.files.iter().map(|f| rel_key(&f.rel)).collect();
                    folders_line(rels.iter())
                }
            };
            Ok(SharedCollection {
                team_slug: crate::model::shared_copy::slug(&owner_org),
                owner_org,
                name,
                summary,
                updated: snap.published_at,
                request_count: snap.files.len(),
                version: snap.version,
                access,
                variables: snap.variables,
                link,
            })
        })
    }

    fn fetch_shared(&self, link: &ShareLink) -> BoxFut<'_, SharedSnapshot> {
        self.record(format!("fetch_shared({link:?})"));
        let link = link.clone();
        self.call(false, move |s| {
            let found = open_share(s, &link)?;
            Ok(snapshot_of(s, &found))
        })
    }

    fn latest_version(&self, link: &ShareLink) -> BoxFut<'_, u32> {
        self.record(format!("latest_version({link:?})"));
        let link = link.clone();
        self.call(false, move |s| {
            let found = open_share(s, &link)?;
            Ok(snapshot_of(s, &found).version)
        })
    }

    fn received(&self) -> Vec<ReceivedShare> {
        self.state().received.clone()
    }

    fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
        self.record(format!("add_received({share:?})"));
        // Held, it waits before it is made (tests: a close mid-Add, or
        // offline by the time it lands).
        let gate = GateOpen(self.gate.clone());
        Box::pin(async move {
            gate.await;
            self.call(true, move |s| {
                online(s)?;
                s.received.retain(|r| r.link.token != share.link.token);
                s.received.push(share);
                Ok(())
            })
            .await
        })
    }

    fn update_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
        self.record(format!("update_received({share:?})"));
        self.call(true, move |s| {
            match s
                .received
                .iter_mut()
                .find(|r| r.link.token == share.link.token)
            {
                Some(r) => *r = share,
                None => s.received.push(share),
            }
            Ok(())
        })
    }

    fn dev(&self) -> Option<&dyn DevControls> {
        Some(self)
    }
}

impl DevControls for MockBackend {
    fn set_offline(&self, on: bool) {
        self.set_toggle(|t| t.offline = on);
    }

    fn offline(&self) -> bool {
        self.state().toggles.offline
    }

    fn conflict_next_sync(&self, on: bool) {
        self.set_toggle(|t| t.conflict_next_sync = on);
    }

    fn expire_links(&self, on: bool) {
        self.set_toggle(|t| t.expire_links = on);
    }

    fn fail_next_sign_in(&self, on: bool) {
        self.set_toggle(|t| t.fail_next_sign_in = on);
    }

    fn expire_session(&self) {
        let mut state = self.state();
        let server = server_key(state.server.url());
        if let Some(a) = state.accounts.remove(&server) {
            state.lapsed.insert(server, a);
        }
        self.save(&state);
    }

    fn reset(&self) {
        let mut state = self.state();
        *state = State::default();
        self.save(&state);
    }

    fn toggles(&self) -> DevToggles {
        self.state().toggles
    }

    #[cfg(test)]
    fn hold_updates(&self, held: bool) {
        self.gate.set(held);
    }

    #[cfg(test)]
    fn recorded(&self) -> Vec<String> {
        let log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.iter().cloned().collect()
    }
}

/// Resolves once `deadline` has passed, without a runtime: the first poll
/// starts one short-lived thread that sleeps until then and wakes the
/// latest waker. A zero latency is ready at once and starts nothing.
struct Delay {
    deadline: Instant,
    waker: Option<Arc<Mutex<Option<Waker>>>>,
}

impl Delay {
    fn new(latency: Duration) -> Delay {
        Delay {
            deadline: Instant::now() + latency,
            waker: None,
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() >= self.deadline {
            return Poll::Ready(());
        }
        let lock = |slot: &Mutex<Option<Waker>>| {
            let mut w = slot.lock().unwrap_or_else(|e| e.into_inner());
            *w = Some(cx.waker().clone());
        };
        match &self.waker {
            Some(slot) => lock(slot),
            None => {
                let slot = Arc::new(Mutex::new(Some(cx.waker().clone())));
                let (thread_slot, deadline) = (slot.clone(), self.deadline);
                let started = std::thread::Builder::new()
                    .name("mock-backend-delay".into())
                    .spawn(move || {
                        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
                        let waker = thread_slot.lock().unwrap_or_else(|e| e.into_inner()).take();
                        if let Some(w) = waker {
                            w.wake();
                        }
                    });
                if started.is_err() {
                    // No thread: poll again soon rather than never.
                    cx.waker().wake_by_ref();
                }
                self.waker = Some(slot);
            }
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests;
