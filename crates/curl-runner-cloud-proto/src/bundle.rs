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
    TrailingDotOrSpace,
    ReservedName,
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
            Self::BadChar => "path has a character not allowed in file names",
            Self::TrailingDotOrSpace => "path has a segment ending in a dot or space",
            Self::ReservedName => "path has a segment that is a reserved device name",
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
    /// Two paths that differ only in letter case, which collide on
    /// case-insensitive file systems.
    CaseCollision(String, String),
    /// A path that is a file in one entry and a folder in another
    /// (`a.yaml` and `a.yaml/b.yaml`), compared case-insensitively.
    FileFolderConflict(String, String),
    Path {
        path: String,
        error: PathError,
    },
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFiles => write!(f, "bundle has more than {MAX_FILES} files"),
            Self::FileTooLarge(p) => write!(f, "{p} is larger than {MAX_FILE_BYTES} bytes"),
            Self::BundleTooLarge => write!(f, "bundle is larger than {MAX_BUNDLE_BYTES} bytes"),
            Self::DuplicatePath(p) => write!(f, "{p} appears more than once"),
            Self::CaseCollision(a, b) => write!(f, "{a} and {b} differ only in letter case"),
            Self::FileFolderConflict(a, b) => {
                write!(f, "{a} is a file and {b} needs it to be a folder")
            }
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
    let hashes: Vec<String> = files.iter().map(|f| file_hash(&f.text)).collect();
    let pairs: Vec<(&str, &str)> = files
        .iter()
        .zip(&hashes)
        .map(|(f, h)| (f.path.as_str(), h.as_str()))
        .collect();
    bundle_hash_from_file_hashes(&pairs)
}

/// [`bundle_hash`] from `(path, file_hash)` pairs, for callers that already
/// hashed each file. The single home of the formula.
pub fn bundle_hash_from_file_hashes(files: &[(&str, &str)]) -> String {
    let mut sorted: Vec<&(&str, &str)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut h = Sha256::new();
    for (path, file_hash) in sorted {
        h.update(path.as_bytes());
        h.update([0]);
        h.update(file_hash.as_bytes());
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
    if p.contains(['\\', '\0', ':', '<', '>', '"', '|', '?', '*'])
        || p.chars().any(char::is_control)
    {
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
        if c.ends_with(['.', ' ']) {
            return Err(PathError::TrailingDotOrSpace);
        }
        if is_reserved_device_name(c) {
            return Err(PathError::ReservedName);
        }
    }
    let lower = p.to_ascii_lowercase();
    if !(lower.ends_with(".yaml") || lower.ends_with(".yml")) {
        return Err(PathError::NotYaml);
    }
    Ok(())
}

/// Windows device names, which cannot be file names with or without an
/// extension (`CON`, `con.yaml`, `COM1.tar.yml`).
fn is_reserved_device_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ');
    let upper = stem.to_ascii_uppercase();
    match upper.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            let b = upper.as_bytes();
            b.len() == 4
                && (upper.starts_with("COM") || upper.starts_with("LPT"))
                && (b'1'..=b'9').contains(&b[3])
        }
    }
}

pub fn validate_bundle(files: &[BundleFile]) -> Result<(), BundleError> {
    if files.len() > MAX_FILES {
        return Err(BundleError::TooManyFiles);
    }
    let mut seen = std::collections::HashSet::with_capacity(files.len());
    let mut folded = std::collections::HashMap::with_capacity(files.len());
    let mut total = 0usize;
    for f in files {
        validate_path(&f.path).map_err(|error| BundleError::Path {
            path: f.path.clone(),
            error,
        })?;
        if !seen.insert(f.path.as_str()) {
            return Err(BundleError::DuplicatePath(f.path.clone()));
        }
        if let Some(other) = folded.insert(f.path.to_lowercase(), f.path.as_str()) {
            return Err(BundleError::CaseCollision(
                other.to_string(),
                f.path.clone(),
            ));
        }
        if f.text.len() > MAX_FILE_BYTES {
            return Err(BundleError::FileTooLarge(f.path.clone()));
        }
        total += f.text.len();
        if total > MAX_BUNDLE_BYTES {
            return Err(BundleError::BundleTooLarge);
        }
    }
    for f in files {
        let lower = f.path.to_lowercase();
        for (i, _) in lower.match_indices('/') {
            if let Some(file) = folded.get(&lower[..i]) {
                return Err(BundleError::FileFolderConflict(
                    file.to_string(),
                    f.path.clone(),
                ));
            }
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
    fn bundle_hash_from_file_hashes_matches_bundle_hash() {
        let files = [f("b.yaml", "2"), f("a.yaml", "1")];
        let hashes: Vec<String> = files.iter().map(|x| file_hash(&x.text)).collect();
        let pairs: Vec<(&str, &str)> = files
            .iter()
            .zip(&hashes)
            .map(|(x, h)| (x.path.as_str(), h.as_str()))
            .collect();
        assert_eq!(bundle_hash_from_file_hashes(&pairs), bundle_hash(&files));
    }

    #[test]
    fn bundle_hash_changes_with_path_or_text() {
        let base = bundle_hash(&[f("a.yaml", "1")]);
        assert_ne!(base, bundle_hash(&[f("b.yaml", "1")]));
        assert_ne!(base, bundle_hash(&[f("a.yaml", "2")]));
    }

    #[test]
    fn file_and_folder_with_the_same_name_conflict() {
        for (a, b) in [("a.yaml", "a.yaml/b.yaml"), ("X/a.yml", "x/A.yml/c.yml")] {
            let r = validate_bundle(&[f(a, "1"), f(b, "2")]);
            assert!(
                matches!(r, Err(BundleError::FileFolderConflict(_, _))),
                "{a} {b}"
            );
            let r = validate_bundle(&[f(b, "2"), f(a, "1")]);
            assert!(
                matches!(r, Err(BundleError::FileFolderConflict(_, _))),
                "{b} {a}"
            );
        }
        assert!(validate_bundle(&[f("a.yaml", "1"), f("a/b.yaml", "2")]).is_ok());
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
            "a:b.yaml",
            "c:/a.yaml",
            "a<b.yaml",
            "a>b.yaml",
            "a\"b.yaml",
            "a|b.yaml",
            "a?b.yaml",
            "a*b.yaml",
            "a\tb.yaml",
            "dir./a.yaml",
            "dir /a.yaml",
            "a.yaml ",
            "a.yaml.",
            "CON.yaml",
            "con.yaml",
            "dir/nul.yml",
            "Aux/a.yaml",
            "prn/a.yaml",
            "com1.yaml",
            "COM9.yaml",
            "lpt1.yaml",
            "LPT9/a.yaml",
            "con.tar.yaml",
        ] {
            assert!(validate_path(p).is_err(), "{p:?} should be rejected");
        }
        assert!(validate_path(&format!("{}.yaml", "a".repeat(510))).is_err());
    }

    #[test]
    fn validate_path_accepts() {
        for p in [
            "a.yaml",
            "users/Get User.YML",
            "console.yaml",
            "com0.yaml",
            "com10.yaml",
            "lpt.yaml",
            "nul-check/a.yaml",
            "a.b.yaml",
        ] {
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

        let case = vec![f("Users/a.yaml", ""), f("users/A.yaml", "")];
        assert_eq!(
            validate_bundle(&case),
            Err(BundleError::CaseCollision(
                "Users/a.yaml".into(),
                "users/A.yaml".into()
            ))
        );

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
