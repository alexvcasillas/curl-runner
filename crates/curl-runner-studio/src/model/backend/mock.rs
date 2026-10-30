//! [`MockBackend`]: the server, in process. Its state (sign-in, server
//! choice, shares, people, received shares, server copies of synced files
//! and the dev toggles) persists in `{base}/mock-backend.json`.
//!
//! Every call behaves like a network: it resolves after `latency` and
//! fails with [`BackendError::Offline`] if the offline toggle is on when it
//! resolves (so turning Offline on mid-call fails that call).

use super::fixtures::{self, CONFLICT_AUTHOR};
use super::{
    Access, Account, Backend, BackendError, BoxFut, CollectionKey, Conflict, ConnectionInfo,
    DevControls, DevToggles, Expiry, Person, ReceivedShare, Role, Server, ShareLink, ShareSettings,
    ShareUpdate, SharedCollection, SharedSnapshot, SharedVar, SignInMethod, SyncFile, SyncReport,
    SyncStatus,
};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

pub struct MockBackend {
    /// `None`: memory only (tests).
    base: Option<PathBuf>,
    latency: Duration,
    state: Mutex<State>,
    log: Mutex<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct State {
    server: Server,
    account: Option<Account>,
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
    slug: String,
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
    /// Local changes the last (offline) attempt could not push.
    pending: usize,
    /// Conflicts the last sync returned.
    conflicts: usize,
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
        let state = base
            .and_then(|b| std::fs::read(b.join(STATE_FILE)).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        MockBackend {
            base: base.map(Path::to_path_buf),
            latency,
            state: Mutex::new(state),
            log: Mutex::default(),
        }
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
        if !cfg!(any(test, debug_assertions)) {
            return;
        }
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        if log.len() >= RECORD_CAP {
            log.remove(0);
        }
        log.push(call);
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

fn online(s: &State) -> Result<(), BackendError> {
    if s.toggles.offline {
        Err(BackendError::Offline)
    } else {
        Ok(())
    }
}

fn signed_in(s: &State) -> Result<Account, BackendError> {
    online(s)?;
    s.account.clone().ok_or(BackendError::Unauthorized)
}

/// `rates/get-rates.yaml` on every OS.
fn rel_key(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn slug(name: &str) -> String {
    let file = crate::model::templates::file_name_for(name);
    file.trim_end_matches(".yaml").to_string()
}

/// 6 base62 chars from the collection, a counter and the clock.
fn new_token(id: &str, seq: u64) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut h = SyncFile::hash_of(&format!("{id}:{seq}:{nanos}"));
    (0..6)
        .map(|_| {
            let c = ALPHABET[(h % 62) as usize] as char;
            h /= 62;
            c
        })
        .collect()
}

fn expiry_date(e: Expiry) -> Option<NaiveDate> {
    match e {
        Expiry::Days(n) => Utc::now()
            .date_naive()
            .checked_add_days(Days::new(u64::from(n))),
        Expiry::Never => None,
    }
}

/// A secret never carries a default, whatever the caller sent.
fn scrub(vars: Vec<SharedVar>) -> Vec<SharedVar> {
    vars.into_iter()
        .map(|mut v| {
            if v.secret {
                v.default = None;
            }
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
        link: Some(ShareLink::web(&share.slug, &share.token)),
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
            owner_org: account.org.clone().unwrap_or_else(|| account.name.clone()),
            slug: slug(&key.name),
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

fn fixture_collection() -> SharedCollection {
    let files = fixtures::shipping_snapshot();
    SharedCollection {
        id: fixtures::SHIPPING_ID.into(),
        owner_org: fixtures::SHIPPING_ORG.into(),
        name: fixtures::SHIPPING_NAME.into(),
        summary: fixtures::SHIPPING_SUMMARY.into(),
        updated: Utc::now() - chrono::Duration::days(fixtures::SHIPPING_UPDATED_DAYS_AGO),
        request_count: files.len(),
        variables: fixtures::shipping_variables(),
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
            if s.server != server {
                s.account = None;
                s.server = server;
            }
            Ok(())
        })
    }

    fn test_connection(&self, server: &Server) -> BoxFut<'_, ConnectionInfo> {
        self.record(format!("test_connection({server:?})"));
        let server = server.clone();
        self.call(false, move |s| {
            online(s)?;
            let sign_in = match &server {
                Server::Cloud => SignInMethod::GitHub,
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
                    SignInMethod::Sso
                }
            };
            Ok(ConnectionInfo {
                version: fixtures::SERVER_VERSION.into(),
                sign_in,
                latency_ms: fixtures::CONNECTION_MS,
            })
        })
    }

    fn sign_in(&self, method: SignInMethod) -> BoxFut<'_, Account> {
        self.record(format!("sign_in({method:?})"));
        self.call(true, move |s| {
            online(s)?;
            if s.toggles.fail_next_sign_in {
                s.toggles.fail_next_sign_in = false;
                return Err(BackendError::Unauthorized);
            }
            let account = fixtures::account(method);
            s.account = Some(account.clone());
            Ok(account)
        })
    }

    fn sign_out(&self) -> BoxFut<'_, ()> {
        self.record("sign_out()".into());
        self.call(true, |s| {
            online(s)?;
            s.account = None;
            Ok(())
        })
    }

    fn account(&self) -> Option<Account> {
        self.state().account.clone()
    }

    fn sync_status(&self, collection: &CollectionKey) -> SyncStatus {
        let s = self.state();
        if s.account.is_none() {
            return SyncStatus::Off;
        }
        let Some(col) = s.synced.get(&collection.id) else {
            return SyncStatus::Off;
        };
        if s.toggles.offline {
            return SyncStatus::Offline(col.pending);
        }
        if col.conflicts > 0 {
            return SyncStatus::Conflict(col.conflicts);
        }
        if col.pending > 0 {
            return SyncStatus::Pending(col.pending);
        }
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

    fn sync(&self, collection: &CollectionKey, local: &[SyncFile]) -> BoxFut<'_, SyncReport> {
        self.record(format!("sync({collection:?}, {local:?})"));
        let (key, local) = (collection.clone(), local.to_vec());
        self.call(true, move |s| {
            let received = s.received.iter().any(|r| r.collection_id == key.id);
            let col = s.synced.get(&key.id);
            let changed: Vec<&SyncFile> = if received {
                Vec::new()
            } else {
                local
                    .iter()
                    .filter(|f| {
                        col.and_then(|c| c.files.get(&rel_key(&f.rel)))
                            .is_none_or(|sf| sf.hash != f.hash)
                    })
                    .collect()
            };
            if s.toggles.offline {
                if s.account.is_some() {
                    s.synced.entry(key.id.clone()).or_default().pending = changed.len();
                }
                return Err(BackendError::Offline);
            }
            signed_in(s)?;
            let now = Utc::now();
            let mut report = SyncReport::default();
            if received {
                // Read only: the owner's copy wins; nothing is pushed.
                let by_rel: BTreeMap<String, u64> =
                    local.iter().map(|f| (rel_key(&f.rel), f.hash)).collect();
                let snapshot = fixtures::shipping_snapshot();
                report.pulled = snapshot
                    .iter()
                    .filter(|f| by_rel.get(&rel_key(&f.rel)) != Some(&f.hash))
                    .cloned()
                    .collect();
                let col = s.synced.entry(key.id.clone()).or_default();
                col.files = snapshot
                    .into_iter()
                    .map(|f| {
                        let file = ServerFile {
                            hash: f.hash,
                            text: f.text,
                            author: fixtures::SHIPPING_ORG.into(),
                            at: now,
                        };
                        (rel_key(&f.rel), file)
                    })
                    .collect();
                col.synced_at = Some(now);
                col.pending = 0;
                col.conflicts = 0;
                return Ok(report);
            }
            let author = signed_in(s)?.name;
            // With nothing changed, a forced conflict takes the first file.
            let conflicted = s
                .toggles
                .conflict_next_sync
                .then(|| changed.first().copied().or(local.first()))
                .flatten()
                .cloned();
            let changed: Vec<SyncFile> = changed.into_iter().cloned().collect();
            let col = s.synced.entry(key.id.clone()).or_default();
            for f in &changed {
                if conflicted.as_ref().is_some_and(|c| c.rel == f.rel) {
                    continue;
                }
                let file = ServerFile {
                    text: f.text.clone(),
                    hash: f.hash,
                    author: author.clone(),
                    at: now,
                };
                col.files.insert(rel_key(&f.rel), file);
                report.pushed += 1;
            }
            if let Some(c) = conflicted {
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
                });
            }
            col.synced_at = Some(now);
            col.pending = 0;
            col.conflicts = report.conflicts.len();
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
        self.call(true, move |s| {
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
        })
    }

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

    /// Anyone with a link may resolve it: the mock does not check
    /// `InvitedOnly` against the caller.
    fn resolve_link(&self, link: &ShareLink) -> BoxFut<'_, SharedCollection> {
        self.record(format!("resolve_link({link:?})"));
        let link = link.clone();
        self.call(false, move |s| {
            online(s)?;
            let fixture = link.token == fixtures::SHIPPING_TOKEN
                && link.collection_slug == fixtures::SHIPPING_SLUG
                && (link.owner_slug.is_empty() || link.owner_slug == fixtures::SHIPPING_OWNER_SLUG);
            if fixture {
                if s.toggles.expire_links {
                    return Err(BackendError::Expired);
                }
                return Ok(fixture_collection());
            }
            let (id, share) = s
                .shares
                .iter()
                .find(|(_, sh)| sh.token == link.token && sh.slug == link.collection_slug)
                .ok_or(BackendError::NotFound)?;
            let today = Utc::now().date_naive();
            if s.toggles.expire_links || share.expires.is_some_and(|d| d < today) {
                return Err(BackendError::Expired);
            }
            let col = s.synced.get(id);
            let files = col.map(|c| &c.files);
            Ok(SharedCollection {
                id: id.clone(),
                owner_org: share.owner_org.clone(),
                name: share.name.clone(),
                summary: files.map(|f| folders_line(f.keys())).unwrap_or_default(),
                updated: col.and_then(|c| c.synced_at).unwrap_or_else(Utc::now),
                request_count: files.map_or(0, BTreeMap::len),
                variables: share.variables.clone(),
            })
        })
    }

    fn fetch_shared(&self, id: &str) -> BoxFut<'_, SharedSnapshot> {
        self.record(format!("fetch_shared({id:?})"));
        let id = id.to_string();
        self.call(false, move |s| {
            online(s)?;
            if id == fixtures::SHIPPING_ID {
                return Ok(SharedSnapshot {
                    files: fixtures::shipping_snapshot(),
                });
            }
            if !s.shares.contains_key(&id) {
                return Err(BackendError::NotFound);
            }
            let files = s.synced.get(&id).map(|c| &c.files);
            Ok(SharedSnapshot {
                files: files
                    .into_iter()
                    .flatten()
                    .map(|(rel, f)| SyncFile {
                        rel: rel.into(),
                        hash: f.hash,
                        text: f.text.clone(),
                    })
                    .collect(),
            })
        })
    }

    fn received(&self) -> Vec<ReceivedShare> {
        self.state().received.clone()
    }

    fn add_received(&self, share: ReceivedShare) -> BoxFut<'_, ()> {
        self.record(format!("add_received({share:?})"));
        self.call(true, move |s| {
            online(s)?;
            s.received.retain(|r| r.id != share.id);
            s.received.push(share);
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

    fn reset(&self) {
        let mut state = self.state();
        *state = State::default();
        self.save(&state);
    }

    fn toggles(&self) -> DevToggles {
        self.state().toggles
    }

    fn recorded(&self) -> Vec<String> {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
