//! Bundle model, hashing and validation.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_FILES: usize = 2000;
pub const MAX_FILE_BYTES: usize = 1 << 20;
pub const MAX_BUNDLE_BYTES: usize = 25 << 20;
const MAX_PATH_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleFile {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    Empty,
    TooLong,
    EmptyComponent,
    DotComponent,
    Hidden,
    BadChar,
    NotYaml,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "path is empty",
            Self::TooLong => "path is longer than 512 bytes",
            Self::EmptyComponent => "path has an empty segment",
            Self::DotComponent => "path has a . or .. segment",
            Self::Hidden => "path has a segment starting with a dot",
            Self::BadChar => "path has a backslash or NUL",
            Self::NotYaml => "path must end in .yaml or .yml",
        })
    }
}

impl std::error::Error for PathError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    TooManyFiles,
    FileTooLarge(String),
    BundleTooLarge,
    DuplicatePath(String),
    Path { path: String, error: PathError },
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFiles => write!(f, "bundle has more than {MAX_FILES} files"),
            Self::FileTooLarge(p) => write!(f, "{p} is larger than {MAX_FILE_BYTES} bytes"),
            Self::BundleTooLarge => write!(f, "bundle is larger than {MAX_BUNDLE_BYTES} bytes"),
            Self::DuplicatePath(p) => write!(f, "{p} appears more than once"),
            Self::Path { path, error } => write!(f, "{path}: {error}"),
        }
    }
}

impl std::error::Error for BundleError {}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

/// Lowercase hex SHA-256 of the UTF-8 bytes.
pub fn file_hash(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

/// Order-independent hash of a bundle: files sorted by path (byte order),
/// each contributing `path \0 file_hash \n`.
pub fn bundle_hash(files: &[BundleFile]) -> String {
    let mut sorted: Vec<&BundleFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut h = Sha256::new();
    for f in sorted {
        h.update(f.path.as_bytes());
        h.update([0]);
        h.update(file_hash(&f.text).as_bytes());
        h.update(b"\n");
    }
    hex(&h.finalize())
}

pub fn validate_path(p: &str) -> Result<(), PathError> {
    if p.is_empty() {
        return Err(PathError::Empty);
    }
    if p.len() > MAX_PATH_BYTES {
        return Err(PathError::TooLong);
    }
    if p.contains(['\\', '\0']) {
        return Err(PathError::BadChar);
    }
    for c in p.split('/') {
        if c.is_empty() {
            return Err(PathError::EmptyComponent);
        }
        if c == "." || c == ".." {
            return Err(PathError::DotComponent);
        }
        if c.starts_with('.') {
            return Err(PathError::Hidden);
        }
    }
    let lower = p.to_ascii_lowercase();
    if !(lower.ends_with(".yaml") || lower.ends_with(".yml")) {
        return Err(PathError::NotYaml);
    }
    Ok(())
}

pub fn validate_bundle(files: &[BundleFile]) -> Result<(), BundleError> {
    if files.len() > MAX_FILES {
        return Err(BundleError::TooManyFiles);
    }
    let mut seen = std::collections::HashSet::with_capacity(files.len());
    let mut total = 0usize;
    for f in files {
        validate_path(&f.path).map_err(|error| BundleError::Path {
            path: f.path.clone(),
            error,
        })?;
        if !seen.insert(f.path.as_str()) {
            return Err(BundleError::DuplicatePath(f.path.clone()));
        }
        if f.text.len() > MAX_FILE_BYTES {
            return Err(BundleError::FileTooLarge(f.path.clone()));
        }
        total += f.text.len();
        if total > MAX_BUNDLE_BYTES {
            return Err(BundleError::BundleTooLarge);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, text: &str) -> BundleFile {
        BundleFile {
            path: path.into(),
            text: text.into(),
        }
    }

    #[test]
    fn file_hash_is_lowercase_hex_sha256() {
        assert_eq!(
            file_hash(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn bundle_hash_ignores_order() {
        let a = f("a.yaml", "1");
        let b = f("b.yaml", "2");
        assert_eq!(bundle_hash(&[a.clone(), b.clone()]), bundle_hash(&[b, a]));
    }

    #[test]
    fn bundle_hash_changes_with_path_or_text() {
        let base = bundle_hash(&[f("a.yaml", "1")]);
        assert_ne!(base, bundle_hash(&[f("b.yaml", "1")]));
        assert_ne!(base, bundle_hash(&[f("a.yaml", "2")]));
    }

    #[test]
    fn validate_path_rejects() {
        for p in [
            "../a.yaml",
            "/a.yaml",
            "a/.env.yaml",
            "a\\b.yaml",
            "a.json",
            "",
            "a//b.yaml",
            "./a.yaml",
            "a/\0.yaml",
        ] {
            assert!(validate_path(p).is_err(), "{p:?} should be rejected");
        }
        assert!(validate_path(&format!("{}.yaml", "a".repeat(510))).is_err());
    }

    #[test]
    fn validate_path_accepts() {
        for p in ["a.yaml", "users/Get User.YML"] {
            assert!(validate_path(p).is_ok(), "{p:?} should be accepted");
        }
    }

    #[test]
    fn validate_bundle_limits() {
        let many: Vec<_> = (0..MAX_FILES + 1)
            .map(|i| f(&format!("f{i}.yaml"), ""))
            .collect();
        assert!(matches!(
            validate_bundle(&many),
            Err(BundleError::TooManyFiles)
        ));

        let big = vec![f("a.yaml", &"x".repeat(MAX_FILE_BYTES + 1))];
        assert!(matches!(
            validate_bundle(&big),
            Err(BundleError::FileTooLarge(_))
        ));

        let dup = vec![f("a.yaml", ""), f("a.yaml", "")];
        assert!(matches!(
            validate_bundle(&dup),
            Err(BundleError::DuplicatePath(_))
        ));

        let bad = vec![f("a.txt", "")];
        assert!(matches!(
            validate_bundle(&bad),
            Err(BundleError::Path { .. })
        ));

        let total: Vec<_> = (0..26)
            .map(|i| f(&format!("f{i}.yaml"), &"x".repeat(MAX_FILE_BYTES)))
            .collect();
        assert!(matches!(
            validate_bundle(&total),
            Err(BundleError::BundleTooLarge)
        ));

        assert!(validate_bundle(&[f("a.yaml", "ok")]).is_ok());
    }
}
