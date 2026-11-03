//! Secret naming rules and no-overwrite file writing shared by the importers.

use crate::security::redact::is_sensitive_header;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

/// Query parameters whose values are secrets.
pub const SECRET_PARAMS: [&str; 9] = [
    "token",
    "access_token",
    "api_key",
    "apikey",
    "key",
    "secret",
    "password",
    "sig",
    "signature",
];

/// A secret header: the CLI redactor's names, plus `X-…-Key`.
pub fn secret_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    is_sensitive_header(name) || lower.ends_with("-key") || lower.ends_with("_key")
}

/// `X-Auth-Token` → `AUTH_TOKEN`, `x-api-key` → `API_KEY`.
pub fn header_var(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    screaming(lower.strip_prefix("x-").unwrap_or(&lower))
}

/// `access_token` → `ACCESS_TOKEN`; a bare key is `API_KEY`.
pub fn param_var(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "key" | "apikey" | "api_key" => "API_KEY".into(),
        other => screaming(other),
    }
}

pub fn screaming(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_');
    if out.is_empty() {
        "SECRET".into()
    } else {
        out.to_string()
    }
}

/// `path` if `taken` says it is free, else `stem-2.ext`, `stem-3.ext`, ….
pub fn unique_path(path: &Path, taken: impl Fn(&Path) -> bool) -> PathBuf {
    if !taken(path) {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| path.with_file_name(format!("{stem}-{n}{ext}")))
        .find(|p| !taken(p))
        .expect("an unbounded range finds a free name")
}

/// Writes a new file, creating its folders; an existing one fails with
/// `AlreadyExists`.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::{Path, PathBuf};

    #[test]
    fn header_and_param_names() {
        assert!(secret_header("X-Api-Key"));
        assert!(secret_header("authorization"));
        assert!(!secret_header("Content-Type"));
        assert_eq!(header_var("X-Auth-Token"), "AUTH_TOKEN");
        assert_eq!(param_var("apikey"), "API_KEY");
        assert_eq!(param_var("access_token"), "ACCESS_TOKEN");
        assert_eq!(screaming("🚀"), "SECRET");
    }

    #[test]
    fn unique_path_adds_a_number() {
        let taken = |p: &Path| p == Path::new("a/x.yaml") || p == Path::new("a/x-2.yaml");
        assert_eq!(
            unique_path(Path::new("a/x.yaml"), taken),
            PathBuf::from("a/x-3.yaml")
        );
        assert_eq!(
            unique_path(Path::new("a/y.yaml"), taken),
            PathBuf::from("a/y.yaml")
        );
    }

    #[test]
    fn write_new_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub/f.txt");
        write_new(&p, b"one").unwrap();
        assert_eq!(
            write_new(&p, b"two").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "one");
    }
}
