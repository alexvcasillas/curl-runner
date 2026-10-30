//! Secret values of an environment. They live only in a [`SecretStore`] (the
//! macOS Keychain in the app), never on disk.

#[cfg(any(test, not(target_os = "macos")))]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Mutex;

/// Where secret values are kept: one item per `service` (collection and
/// environment, see [`service_name`]) and `name` (the variable).
pub trait SecretStore: Send + Sync {
    fn get(&self, service: &str, name: &str) -> Option<String>;
    fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String>;
    /// Removing a missing item is not an error.
    fn delete(&self, service: &str, name: &str) -> Result<(), String>;
    /// The names stored under `service`, sorted.
    fn names(&self, service: &str) -> Vec<String>;

    /// [`SecretStore::get`] that tells a missing item (`Ok(None)`) from a
    /// store that could not be read (a locked Keychain).
    fn try_get(&self, service: &str, name: &str) -> Result<Option<String>, String> {
        Ok(self.get(service, name))
    }

    /// [`SecretStore::names`] that reports a store that could not be read.
    fn try_names(&self, service: &str) -> Result<Vec<String>, String> {
        Ok(self.names(service))
    }
}

/// `errSecItemNotFound`.
#[cfg(target_os = "macos")]
const NOT_FOUND: i32 = -25300;

/// The Keychain service of a collection's environment. `id` is the
/// collection's [`CollectionMeta::id`](crate::model::collection_meta::CollectionMeta::id):
/// two collections with the same name keep apart.
pub fn service_name(collection: &str, environment: &str, id: &str) -> String {
    format!("curl-runner studio · {collection} · {environment} · {id}")
}

/// The service of the collection at `root` (named after its folder, as the
/// workspace names it) and `environment`, its id picked on first use.
#[cfg(test)]
pub fn service_for(base: &std::path::Path, root: &std::path::Path, environment: &str) -> String {
    let name = root
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let meta = crate::model::collection_meta::CollectionMeta::load_or_init(Some(base), root);
    service_name(&name, environment, &meta.id)
}

/// An in-process [`SecretStore`] (tests, and other OSes).
#[cfg(any(test, not(target_os = "macos")))]
#[derive(Debug, Default)]
pub struct MemorySecrets {
    items: Mutex<BTreeMap<(String, String), String>>,
}

#[cfg(any(test, not(target_os = "macos")))]
impl MemorySecrets {
    fn items(&self) -> std::sync::MutexGuard<'_, BTreeMap<(String, String), String>> {
        self.items.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// How many items it holds, across every service (tests).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.items().len()
    }
}

#[cfg(any(test, not(target_os = "macos")))]
impl SecretStore for MemorySecrets {
    fn get(&self, service: &str, name: &str) -> Option<String> {
        self.items()
            .get(&(service.to_string(), name.to_string()))
            .cloned()
    }

    fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
        self.items()
            .insert((service.to_string(), name.to_string()), value.to_string());
        Ok(())
    }

    fn delete(&self, service: &str, name: &str) -> Result<(), String> {
        self.items()
            .remove(&(service.to_string(), name.to_string()));
        Ok(())
    }

    fn names(&self, service: &str) -> Vec<String> {
        self.items()
            .keys()
            .filter(|(s, _)| s == service)
            .map(|(_, n)| n.clone())
            .collect()
    }
}

/// Generic-password items in the user's login Keychain (account = name).
#[cfg(target_os = "macos")]
#[derive(Debug, Default, Clone, Copy)]
pub struct KeychainSecrets;

#[cfg(target_os = "macos")]
impl SecretStore for KeychainSecrets {
    fn get(&self, service: &str, name: &str) -> Option<String> {
        self.try_get(service, name).ok().flatten()
    }

    fn try_get(&self, service: &str, name: &str) -> Result<Option<String>, String> {
        match security_framework::passwords::get_generic_password(service, name) {
            Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
            Err(e) if e.code() == NOT_FOUND => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
        security_framework::passwords::set_generic_password(service, name, value.as_bytes())
            .map_err(|e| e.to_string())
    }

    fn delete(&self, service: &str, name: &str) -> Result<(), String> {
        match security_framework::passwords::delete_generic_password(service, name) {
            Err(e) if e.code() != NOT_FOUND => Err(e.to_string()),
            _ => Ok(()),
        }
    }

    fn names(&self, service: &str) -> Vec<String> {
        self.try_names(service).unwrap_or_default()
    }

    fn try_names(&self, service: &str) -> Result<Vec<String>, String> {
        use security_framework::item::{ItemClass, ItemSearchOptions, Limit};
        let found = match ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(service)
            .load_attributes(true)
            .limit(Limit::All)
            .search()
        {
            Ok(found) => found,
            Err(e) if e.code() == NOT_FOUND => Vec::new(),
            Err(e) => return Err(e.to_string()),
        };
        let mut names: Vec<String> = found
            .iter()
            .filter_map(|r| r.simplify_dict()?.remove("acct"))
            .collect();
        names.sort();
        names.dedup();
        Ok(names)
    }
}

/// A read-through cache over another store: each item is read once until
/// [`CachedSecrets::invalidate`]. Writes go through and update the cache.
/// The studio resolves variables with it so typing never reads the Keychain.
pub struct CachedSecrets {
    inner: std::sync::Arc<dyn SecretStore>,
    cache: Mutex<HashMap<(String, String), Option<String>>>,
}

impl CachedSecrets {
    pub fn new(inner: std::sync::Arc<dyn SecretStore>) -> Self {
        CachedSecrets {
            inner,
            cache: Mutex::default(),
        }
    }

    /// Forget every cached value (after an environment switch or a secret
    /// edited elsewhere).
    pub fn invalidate(&self) {
        self.cache().clear();
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Option<String>>> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl SecretStore for CachedSecrets {
    fn get(&self, service: &str, name: &str) -> Option<String> {
        self.try_get(service, name).ok().flatten()
    }

    /// A failed read is not cached: the next one asks again.
    fn try_get(&self, service: &str, name: &str) -> Result<Option<String>, String> {
        let key = (service.to_string(), name.to_string());
        if let Some(v) = self.cache().get(&key) {
            return Ok(v.clone());
        }
        let v = self.inner.try_get(service, name)?;
        self.cache().insert(key, v.clone());
        Ok(v)
    }

    fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
        self.inner.set(service, name, value)?;
        self.cache().insert(
            (service.to_string(), name.to_string()),
            Some(value.to_string()),
        );
        Ok(())
    }

    fn delete(&self, service: &str, name: &str) -> Result<(), String> {
        self.inner.delete(service, name)?;
        self.cache()
            .insert((service.to_string(), name.to_string()), None);
        Ok(())
    }

    fn names(&self, service: &str) -> Vec<String> {
        self.inner.names(service)
    }

    fn try_names(&self, service: &str) -> Result<Vec<String>, String> {
        self.inner.try_names(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts reads of the store under it.
    struct Counting(MemorySecrets, std::sync::atomic::AtomicUsize);

    impl SecretStore for Counting {
        fn get(&self, service: &str, name: &str) -> Option<String> {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.0.get(service, name)
        }
        fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
            self.0.set(service, name, value)
        }
        fn delete(&self, service: &str, name: &str) -> Result<(), String> {
            self.0.delete(service, name)
        }
        fn names(&self, service: &str) -> Vec<String> {
            self.0.names(service)
        }
    }

    #[test]
    fn cached_secrets_read_once_until_invalidated() {
        let inner = std::sync::Arc::new(Counting(MemorySecrets::default(), Default::default()));
        inner.0.set("s", "SECRET_A", "1").unwrap();
        let cached = CachedSecrets::new(inner.clone());
        let reads = || inner.1.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(cached.get("s", "SECRET_A").as_deref(), Some("1"));
        assert_eq!(cached.get("s", "SECRET_A").as_deref(), Some("1"));
        assert_eq!(cached.get("s", "SECRET_MISSING"), None);
        assert_eq!(cached.get("s", "SECRET_MISSING"), None);
        assert_eq!(reads(), 2, "one read per item");
        inner.0.set("s", "SECRET_A", "2").unwrap();
        assert_eq!(cached.get("s", "SECRET_A").as_deref(), Some("1"), "cached");
        cached.invalidate();
        assert_eq!(cached.get("s", "SECRET_A").as_deref(), Some("2"));
        cached.set("s", "SECRET_A", "3").unwrap();
        assert_eq!(cached.get("s", "SECRET_A").as_deref(), Some("3"));
        cached.delete("s", "SECRET_A").unwrap();
        assert_eq!(cached.get("s", "SECRET_A"), None);
        assert_eq!(reads(), 3);
    }

    /// A Keychain that can't be read until it is unlocked.
    struct Lockable(MemorySecrets, std::sync::atomic::AtomicBool);

    impl SecretStore for Lockable {
        fn get(&self, service: &str, name: &str) -> Option<String> {
            self.try_get(service, name).ok().flatten()
        }
        fn try_get(&self, service: &str, name: &str) -> Result<Option<String>, String> {
            if self.1.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("locked".into());
            }
            Ok(self.0.get(service, name))
        }
        fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
            self.0.set(service, name, value)
        }
        fn delete(&self, service: &str, name: &str) -> Result<(), String> {
            self.0.delete(service, name)
        }
        fn names(&self, service: &str) -> Vec<String> {
            self.0.names(service)
        }
    }

    #[test]
    fn a_failed_read_is_reported_and_not_cached() {
        let inner = std::sync::Arc::new(Lockable(MemorySecrets::default(), true.into()));
        inner.0.set("s", "SECRET_A", "1").unwrap();
        let cached = CachedSecrets::new(inner.clone());
        assert!(cached.try_get("s", "SECRET_A").is_err());
        assert_eq!(cached.get("s", "SECRET_A"), None);
        inner.1.store(false, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(cached.try_get("s", "SECRET_A"), Ok(Some("1".into())));
        assert_eq!(MemorySecrets::default().try_names("s"), Ok(vec![]));
    }

    #[test]
    fn same_folder_name_gets_its_own_service() {
        let base = tempfile::tempdir().unwrap();
        let (p, q) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (a, b) = (p.path().join("api"), q.path().join("api"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let (sa, sb) = (
            service_for(base.path(), &a, "staging"),
            service_for(base.path(), &b, "staging"),
        );
        assert!(
            sa.starts_with("curl-runner studio · api · staging · "),
            "{sa}"
        );
        assert_ne!(sa, sb);
        assert_eq!(sa, service_for(base.path(), &a, "staging"));
    }

    #[test]
    fn service_name_names_collection_and_environment() {
        assert_eq!(
            service_name("Payments API", "staging", "0a1b2c3d"),
            "curl-runner studio · Payments API · staging · 0a1b2c3d"
        );
    }

    #[test]
    fn memory_secrets_round_trip() {
        let s = MemorySecrets::default();
        s.set("svc", "B", "2").unwrap();
        s.set("svc", "A", "1").unwrap();
        s.set("other", "C", "3").unwrap();
        assert_eq!(s.get("svc", "A").as_deref(), Some("1"));
        assert_eq!(s.names("svc"), ["A", "B"]);
        s.delete("svc", "A").unwrap();
        s.delete("svc", "missing").unwrap();
        assert_eq!(s.get("svc", "A"), None);
        assert_eq!(s.names("svc"), ["B"]);
    }

    /// Touches the real login Keychain under a throwaway service, removed
    /// afterwards. Run by hand: `cargo test -p curl-runner-studio keychain -- --ignored`.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "uses the real Keychain"]
    fn keychain_round_trip() {
        let k = KeychainSecrets;
        let svc = format!("curl-runner studio test · {}", std::process::id());
        k.set(&svc, "SECRET_A", "value-a").unwrap();
        k.set(&svc, "SECRET_B", "value-b").unwrap();
        let got = (k.get(&svc, "SECRET_A"), k.names(&svc));
        k.delete(&svc, "SECRET_A").unwrap();
        k.delete(&svc, "SECRET_B").unwrap();
        k.delete(&svc, "SECRET_B").unwrap();
        assert_eq!(got.0.as_deref(), Some("value-a"));
        assert_eq!(got.1, ["SECRET_A", "SECRET_B"]);
        assert!(k.names(&svc).is_empty());
    }
}
