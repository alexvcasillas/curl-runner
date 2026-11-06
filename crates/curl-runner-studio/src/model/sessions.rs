//! Cloud sessions: one `crs_` token per server, kept in the Keychain (service
//! `curl-runner Cloud`, account = the server URL). The signed-in person's name
//! and email sit in `{state}/accounts.json` for display; it never holds tokens.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::secrets::SecretStore;

/// The Keychain service of every Cloud session.
pub const SERVICE: &str = "curl-runner Cloud";

const FILE: &str = "accounts.json";

/// Where the session token of each server is kept.
pub trait SessionStore: Send + Sync {
    /// `Ok(None)` is signed out; `Err` is a store that could not be read
    /// (a locked Keychain), which is not the same thing.
    fn get(&self, server: &str) -> io::Result<Option<String>>;
    fn set(&self, server: &str, token: &str) -> io::Result<()>;
    /// Signs `server` out only; a missing session is not an error.
    fn delete(&self, server: &str) -> io::Result<()>;
    /// The servers signed in to, from the account cache (the Keychain can't
    /// list accounts cheaply), sorted.
    // Settings → Accounts lists them in 3b; only the tests call it now.
    #[cfg_attr(not(test), allow(dead_code))]
    fn servers(&self) -> Vec<String>;
}

/// Who is signed in to a server, for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedAccount {
    pub name: Option<String>,
    pub email: String,
}

/// `{state}/accounts.json`: the account of each server. No tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountCache(pub BTreeMap<String, CachedAccount>);

impl AccountCache {
    /// Missing or unreadable is empty. `state` `None` keeps nothing.
    pub fn load(state: Option<&Path>) -> Self {
        state
            .and_then(|s| std::fs::read(s.join(FILE)).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Writes atomically; `state` `None` is a no-op.
    pub fn save(&self, state: Option<&Path>) -> io::Result<()> {
        let Some(state) = state else { return Ok(()) };
        std::fs::create_dir_all(state)?;
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        crate::model::write_atomic(&state.join(FILE), &json)
    }

    /// Forgets `server` and saves; a missing one is not an error.
    pub fn remove(state: Option<&Path>, server: &str) -> io::Result<()> {
        let mut cache = Self::load(state);
        if cache.0.remove(server).is_some() {
            cache.save(state)?;
        }
        Ok(())
    }
}

/// [`SessionStore`] over a [`SecretStore`] (the Keychain in the app).
pub struct SecretSessions<S: SecretStore> {
    secrets: S,
    state: Option<PathBuf>,
}

impl<S: SecretStore> SecretSessions<S> {
    pub fn new(secrets: S, state: Option<&Path>) -> Self {
        SecretSessions {
            secrets,
            state: state.map(Path::to_path_buf),
        }
    }
}

impl<S: SecretStore> SessionStore for SecretSessions<S> {
    fn get(&self, server: &str) -> io::Result<Option<String>> {
        self.secrets
            .try_get(SERVICE, server)
            .map_err(io::Error::other)
    }

    fn set(&self, server: &str, token: &str) -> io::Result<()> {
        self.secrets
            .set(SERVICE, server, token)
            .map_err(io::Error::other)
    }

    fn delete(&self, server: &str) -> io::Result<()> {
        self.secrets
            .delete(SERVICE, server)
            .map_err(io::Error::other)?;
        AccountCache::remove(self.state.as_deref(), server)
    }

    fn servers(&self) -> Vec<String> {
        AccountCache::load(self.state.as_deref())
            .0
            .into_keys()
            .collect()
    }
}

/// Sessions in the login Keychain: the app's, with the `server` feature.
#[cfg(all(target_os = "macos", feature = "server"))]
pub type KeychainSessions = SecretSessions<crate::model::secrets::KeychainSecrets>;

/// In-process sessions (tests, and other OSes).
#[cfg(any(test, not(target_os = "macos")))]
pub type MemorySessions = SecretSessions<crate::model::secrets::MemorySecrets>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::secrets::MemorySecrets;

    const A: &str = "https://api.curl-runner.com";
    const B: &str = "https://cloud.example.test";

    fn account(email: &str) -> CachedAccount {
        CachedAccount {
            name: None,
            email: email.into(),
        }
    }

    #[test]
    fn memory_sessions_are_per_server() {
        let s = MemorySessions::new(MemorySecrets::default(), None);
        s.set(A, "crs_a").unwrap();
        s.set(B, "crs_b").unwrap();
        assert_eq!(s.get(A).unwrap().as_deref(), Some("crs_a"));
        assert_eq!(s.get(B).unwrap().as_deref(), Some("crs_b"));
        assert_eq!(s.get("https://other.test").unwrap(), None);
        s.set(A, "crs_a2").unwrap();
        assert_eq!(s.get(A).unwrap().as_deref(), Some("crs_a2"));
    }

    #[test]
    fn account_cache_round_trips_without_tokens() {
        let d = tempfile::tempdir().unwrap();
        let s = MemorySessions::new(MemorySecrets::default(), Some(d.path()));
        s.set(A, "crs_secret").unwrap();
        let mut cache = AccountCache::load(Some(d.path()));
        assert!(cache.0.is_empty());
        cache.0.insert(
            A.into(),
            CachedAccount {
                name: Some("Ada".into()),
                email: "ada@example.test".into(),
            },
        );
        cache.0.insert(B.into(), account("b@example.test"));
        cache.save(Some(d.path())).unwrap();
        let text = std::fs::read_to_string(d.path().join("accounts.json")).unwrap();
        assert!(!text.contains("crs_"), "{text}");
        assert_eq!(AccountCache::load(Some(d.path())), cache);
        assert_eq!(s.servers(), [A, B]);
        std::fs::write(d.path().join("accounts.json"), "{not json").unwrap();
        assert!(AccountCache::load(Some(d.path())).0.is_empty());
        assert!(AccountCache::load(None).0.is_empty());
    }

    #[test]
    fn delete_leaves_other_servers() {
        let d = tempfile::tempdir().unwrap();
        let s = MemorySessions::new(MemorySecrets::default(), Some(d.path()));
        let mut cache = AccountCache::default();
        for (server, token) in [(A, "crs_a"), (B, "crs_b")] {
            s.set(server, token).unwrap();
            cache.0.insert(server.into(), account("x@example.test"));
        }
        cache.save(Some(d.path())).unwrap();
        s.delete(A).unwrap();
        s.delete(A).unwrap();
        assert_eq!(s.get(A).unwrap(), None);
        assert_eq!(s.get(B).unwrap().as_deref(), Some("crs_b"));
        assert_eq!(s.servers(), [B]);
    }

    /// A Keychain that can't be read.
    struct Locked;

    impl SecretStore for Locked {
        fn get(&self, _: &str, _: &str) -> Option<String> {
            None
        }
        fn try_get(&self, _: &str, _: &str) -> Result<Option<String>, String> {
            Err("locked".into())
        }
        fn set(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn names(&self, _: &str) -> Vec<String> {
            vec![]
        }
    }

    #[test]
    fn a_locked_keychain_is_an_error_not_signed_out() {
        let s = SecretSessions::new(Locked, None);
        assert!(s.get(A).is_err());
    }
}
