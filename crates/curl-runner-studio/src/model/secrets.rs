//! Secret values of an environment. They live only in a [`SecretStore`] (the
//! macOS Keychain in the app), never on disk.

use std::collections::BTreeMap;
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
}

/// The Keychain service of a collection's environment.
pub fn service_name(collection: &str, environment: &str) -> String {
    format!("curl-runner studio · {collection} · {environment}")
}

/// An in-process [`SecretStore`] (tests, and other OSes).
#[derive(Debug, Default)]
pub struct MemorySecrets {
    items: Mutex<BTreeMap<(String, String), String>>,
}

impl MemorySecrets {
    fn items(&self) -> std::sync::MutexGuard<'_, BTreeMap<(String, String), String>> {
        self.items.lock().unwrap_or_else(|e| e.into_inner())
    }
}

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
        let bytes = security_framework::passwords::get_generic_password(service, name).ok()?;
        String::from_utf8(bytes).ok()
    }

    fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String> {
        security_framework::passwords::set_generic_password(service, name, value.as_bytes())
            .map_err(|e| e.to_string())
    }

    fn delete(&self, service: &str, name: &str) -> Result<(), String> {
        /// `errSecItemNotFound`.
        const NOT_FOUND: i32 = -25300;
        match security_framework::passwords::delete_generic_password(service, name) {
            Err(e) if e.code() != NOT_FOUND => Err(e.to_string()),
            _ => Ok(()),
        }
    }

    fn names(&self, service: &str) -> Vec<String> {
        use security_framework::item::{ItemClass, ItemSearchOptions, Limit};
        let found = ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(service)
            .load_attributes(true)
            .limit(Limit::All)
            .search()
            .unwrap_or_default();
        let mut names: Vec<String> = found
            .iter()
            .filter_map(|r| r.simplify_dict()?.remove("acct"))
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_names_collection_and_environment() {
        assert_eq!(
            service_name("Payments API", "staging"),
            "curl-runner studio · Payments API · staging"
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
