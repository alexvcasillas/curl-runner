//! Saved sessions: `{"servers": {"<api url>": {"session", "email"}}}` in the
//! user's config directory. The file holds session tokens, so it is written
//! with mode 0600 (Unix) through a temp file and a rename, and tokens are
//! never printed.

use super::Env;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const TOKEN_ENV: &str = "CURL_RUNNER_TOKEN";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub session: String,
    pub email: String,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("session", &"<redacted>")
            .field("email", &self.email)
            .finish()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub servers: BTreeMap<String, Entry>,
}

/// `$XDG_CONFIG_HOME/curl-runner/credentials.json`, else `~/.config/...`;
/// `%APPDATA%\curl-runner\credentials.json` on Windows.
pub fn path(env: Env) -> Option<PathBuf> {
    let set = |k: &str| env(k).filter(|v| !v.is_empty());
    let base = if cfg!(windows) {
        PathBuf::from(set("APPDATA")?)
    } else if let Some(x) = set("XDG_CONFIG_HOME") {
        PathBuf::from(x)
    } else {
        PathBuf::from(set("HOME")?).join(".config")
    };
    Some(base.join("curl-runner").join("credentials.json"))
}

impl Credentials {
    /// A missing file is empty; an unreadable or corrupt one is an error.
    pub fn load(path: &Path) -> Result<Credentials, String> {
        match std::fs::read(path) {
            Ok(b) => serde_json::from_slice(&b).map_err(|_| {
                format!(
                    "{} is not valid. Remove it and sign in again.",
                    path.display()
                )
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Credentials::default()),
            Err(e) => Err(format!("Cannot read {}: {e}.", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let err = |e: std::io::Error| format!("Cannot write {}: {e}.", path.display());
        let dir = path.parent().ok_or("Bad credentials path.")?;
        if !dir.exists() {
            let mut b = std::fs::DirBuilder::new();
            b.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                b.mode(0o700);
            }
            b.create(dir).map_err(err)?;
        }
        let tmp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let write = || -> std::io::Result<()> {
            use std::io::Write;
            let mut f = opts.open(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
            std::fs::rename(&tmp, path)
        };
        write().map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            err(e)
        })
    }
}

/// The token to call `server` with: `CURL_RUNNER_TOKEN` (a `crt_` or `crs_`
/// token) wins over the saved session.
/// A corrupt credentials file is an error, not a silent sign-out.
#[allow(dead_code)] // used by publish and pull
pub fn token_for(env: Env, server: &str) -> Result<Option<String>, String> {
    if let Some(t) = env(TOKEN_ENV).filter(|t| !t.is_empty()) {
        return Ok(Some(t));
    }
    let Some(p) = path(env) else { return Ok(None) };
    let creds = Credentials::load(&p).map_err(|_| {
        format!(
            "Can't read {}. Fix or delete it, then run curl-runner login.",
            p.display()
        )
    })?;
    Ok(creds.servers.get(server).map(|e| e.session.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry {
            session: "crs_secret".into(),
            email: "a@example.com".into(),
        }
    }

    #[test]
    fn path_follows_xdg_then_home() {
        if cfg!(windows) {
            return;
        }
        let xdg = |k: &str| (k == "XDG_CONFIG_HOME").then(|| "/x".to_string());
        assert_eq!(
            path(&xdg).unwrap(),
            PathBuf::from("/x/curl-runner/credentials.json")
        );
        let home = |k: &str| (k == "HOME").then(|| "/h".to_string());
        assert_eq!(
            path(&home).unwrap(),
            PathBuf::from("/h/.config/curl-runner/credentials.json")
        );
        assert_eq!(path(&|_| None), None);
    }

    #[test]
    fn save_load_roundtrip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub/credentials.json");
        assert!(Credentials::load(&p).unwrap().servers.is_empty());
        let mut c = Credentials::default();
        c.servers.insert("https://api.example.com".into(), entry());
        c.save(&p).unwrap();
        let back = Credentials::load(&p).unwrap();
        assert_eq!(back.servers["https://api.example.com"], entry());
        assert!(!p.with_extension("json.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn new_directory_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("curl-runner/credentials.json");
        Credentials::default().save(&p).unwrap();
        let mode = std::fs::metadata(p.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("credentials.json");
        Credentials::default().save(&p).unwrap();
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn corrupt_file_is_an_error_and_debug_hides_the_token() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("credentials.json");
        std::fs::write(&p, "nope").unwrap();
        assert!(Credentials::load(&p).is_err());
        assert!(!format!("{:?}", entry()).contains("crs_secret"));
    }

    #[test]
    fn token_env_overrides_file() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().to_string_lossy().into_owned();
        let p =
            path(&|k| (k == "XDG_CONFIG_HOME" || k == "APPDATA").then(|| home.clone())).unwrap();
        let mut c = Credentials::default();
        c.servers.insert("https://s".into(), entry());
        c.save(&p).unwrap();
        let file = |k: &str| (k == "XDG_CONFIG_HOME" || k == "APPDATA").then(|| home.clone());
        assert_eq!(
            token_for(&file, "https://s").unwrap().as_deref(),
            Some("crs_secret")
        );
        assert_eq!(token_for(&file, "https://other").unwrap(), None);
        let both = |k: &str| match k {
            TOKEN_ENV => Some("crt_pub".to_string()),
            _ => file(k),
        };
        assert_eq!(
            token_for(&both, "https://s").unwrap().as_deref(),
            Some("crt_pub")
        );
        std::fs::write(&p, "nope").unwrap();
        assert!(
            token_for(&file, "https://s")
                .unwrap_err()
                .contains("Fix or delete it")
        );
    }
}
