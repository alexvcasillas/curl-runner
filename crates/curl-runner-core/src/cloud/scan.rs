//! The bundle to publish: the collection's YAML files.

use std::path::{Component, Path, PathBuf};

use curl_runner_cloud_proto::{
    BundleError, BundleFile, MAX_FILE_BYTES, bundle_hash, validate_bundle,
};

use super::gitignore::Ignores;
use crate::discover::find_yaml_files;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    TooBig,
    Unreadable,
    OutsideRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub files: Vec<BundleFile>,
    pub hash: String,
    pub skipped: Vec<(PathBuf, SkipReason)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("cannot read {path}: {source}")]
    Root {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Bundle(#[from] BundleError),
}

/// The YAML files under `root`, sorted by `/`-separated path. Hidden paths
/// (so no `.env*`), files git ignores and anything resolving outside `root`
/// are left out; unreadable and oversized files are listed in `skipped`.
pub fn scan_bundle(root: &Path) -> Result<Bundle, ScanError> {
    std::fs::read_dir(root).map_err(|source| ScanError::Root {
        path: root.to_path_buf(),
        source,
    })?;
    let real_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut ignores = Ignores::new(root);
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    for found in find_yaml_files(&[], true, root) {
        let rel = PathBuf::from(found.strip_prefix("./").unwrap_or(&found));
        if !rel.components().all(|c| {
            matches!(c, Component::Normal(_)) && !c.as_os_str().to_string_lossy().starts_with('.')
        }) {
            continue;
        }
        let key = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if ignores.is_ignored(&key) {
            continue;
        }
        let path = root.join(&rel);
        let Ok(meta) = std::fs::metadata(&path) else {
            skipped.push((rel, SkipReason::Unreadable));
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        if !path
            .canonicalize()
            .is_ok_and(|real| real.starts_with(&real_root))
        {
            skipped.push((rel, SkipReason::OutsideRoot));
            continue;
        }
        if meta.len() > MAX_FILE_BYTES as u64 {
            skipped.push((rel, SkipReason::TooBig));
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => files.push(BundleFile { path: key, text }),
            Err(_) => skipped.push((rel, SkipReason::Unreadable)),
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    validate_bundle(&files)?;
    let hash = bundle_hash(&files);
    Ok(Bundle {
        files,
        hash,
        skipped,
    })
}
