//! [`HttpBackend`]: the studio's Backend over the curl-runner Cloud API:
//! server info, browser sign-in, accounts. Calls run the blocking client
//! inside a boxed future, polled on GPUI's background executor.
//!
//! The team side (`sync`, sharing, people) answers `Unavailable` until
//! plan 3b; shared collections arrive in Task 6.

use super::{
    Account, AuthCallback, Backend, BackendError, BoxFut, CollectionKey, ConnectionInfo,
    DevControls, PendingSignIns, Person, ReceivedShare, Server, ShareLink, ShareSettings,
    ShareUpdate, SharedCollection, SharedSnapshot, SyncDeletion, SyncFile, SyncReport, SyncStatus,
};
use crate::model::sessions::{AccountCache, CachedAccount, SessionStore};
use curl_runner_cloud_client::{Client, CloudError, check_server, describe};
use curl_runner_cloud_proto::ErrorCode;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

/// Where the server choice is kept, under the state folder.
const SERVER_FILE: &str = "server.json";

pub struct HttpBackend {
    state: Option<PathBuf>,
    sessions: Arc<dyn SessionStore>,
    server: Mutex<Server>,
    /// Name and email of each signed-in server (no tokens), mirrored in
    /// `{state}/accounts.json`.
    accounts: Mutex<AccountCache>,
    pending: PendingSignIns,
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
            pending: PendingSignIns::default(),
        }
    }

    /// Asks `server` who the session on it is, and refreshes the cached
    /// account. A 401 signs out of that server only.
    // The workspace calls it when it checks sessions (a later task of 3a).
    pub fn refresh_account(&self, server: &str) -> BoxFut<'_, Account> {
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
            Ok(self.remember(&server, me.user.name, me.user.email))
        })
    }

    fn remember(&self, server: &str, name: Option<String>, email: String) -> Account {
        let cached = CachedAccount { name, email };
        let account = account_of(&cached);
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
            // Signed out either way; a Keychain that won't delete is
            // retried by the next sign-out.
            let _ = self.forget(&key(server));
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
            let info = Client::new(&url, None)
                .info()
                .map_err(|e| self.map_error(&url, e))?;
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
                .map_err(|e| self.map_error(&url, e))?;
            let app = info
                .app_url
                .map(|a| key(&a))
                .filter(|a| !a.is_empty())
                .ok_or_else(|| BackendError::Invalid("This server has no sign-in page.".into()))?;
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
            Ok(self.remember(&server, verified.user.name, verified.user.email))
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
            self.forget(&server)
        })
    }

    fn account(&self) -> Option<Account> {
        self.account_on(self.server().url())
    }

    fn account_on(&self, server: &str) -> Option<Account> {
        lock(&self.accounts).0.get(&key(server)).map(account_of)
    }
    fn sync_status(&self, _: &CollectionKey) -> SyncStatus {
        SyncStatus::Off
    }
    fn sync(
        &self,
        _: &CollectionKey,
        _: &[SyncFile],
        _: &[SyncDeletion],
    ) -> BoxFut<'_, SyncReport> {
        unavailable()
    }
    fn sync_shared(&self, _: &CollectionKey, _: &str, _: &[SyncFile]) -> BoxFut<'_, SyncReport> {
        unavailable()
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
    // Task 6: shared collections.
    fn resolve_link(&self, _: &ShareLink) -> BoxFut<'_, SharedCollection> {
        unavailable()
    }
    fn fetch_shared(&self, _: &str) -> BoxFut<'_, SharedSnapshot> {
        unavailable()
    }
    fn received(&self) -> Vec<ReceivedShare> {
        Vec::new()
    }
    fn add_received(&self, _: ReceivedShare) -> BoxFut<'_, ()> {
        unavailable()
    }
    fn dev(&self) -> Option<&dyn DevControls> {
        None
    }
}

#[cfg(test)]
mod tests;
