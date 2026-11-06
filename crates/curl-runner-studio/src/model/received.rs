//! Where this Mac keeps each received copy: share token → the copy's
//! folder and its [`CollectionMeta::id`], in `{base}/received.json`. The
//! backend's record ([`ReceivedShare`]) never carries a path.
//!
//! The sidebar's Shared with me (and everything that asks "which share is
//! this copy?") is the backend's records joined with this mapping. A record
//! with no copy on this Mac (added on another one, or its folder gone) is
//! left out: there is nothing here to open.
//!
//! [`CollectionMeta::id`]: crate::model::collection_meta::CollectionMeta::id

use crate::model::backend::ReceivedShare;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE: &str = "received.json";

/// One copy on this Mac.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCopy {
    /// The share's token ([`ReceivedShare::link`]).
    pub share_id: String,
    pub root: PathBuf,
    /// The copy's `CollectionMeta::id`.
    pub collection_id: String,
}

/// A received share with its copy on this Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedCopy {
    pub share: ReceivedShare,
    pub root: PathBuf,
    pub collection_id: String,
}

/// The copies recorded here; missing or unreadable is none. `base` `None`
/// keeps nothing.
pub fn load(base: Option<&Path>) -> Vec<LocalCopy> {
    base.and_then(|b| std::fs::read(b.join(FILE)).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save(base: &Path, copies: &[LocalCopy]) -> std::io::Result<()> {
    std::fs::create_dir_all(base)?;
    let json = serde_json::to_string_pretty(copies).map_err(std::io::Error::other)?;
    crate::model::write_atomic(&base.join(FILE), &json)
}

/// Records `copy`, replacing any earlier copy of the same share.
pub fn record(base: &Path, copy: LocalCopy) -> std::io::Result<()> {
    let mut copies = load(Some(base));
    copies.retain(|c| c.share_id != copy.share_id);
    copies.push(copy);
    save(base, &copies)
}

/// Forgets the copy of the share with token `share_id`; none is not an
/// error.
pub fn forget(base: &Path, share_id: &str) -> std::io::Result<()> {
    let mut copies = load(Some(base));
    let before = copies.len();
    copies.retain(|c| c.share_id != share_id);
    if copies.len() == before {
        return Ok(());
    }
    save(base, &copies)
}

/// The backend's `records` that have a copy here whose folder is still
/// there, in the records' order.
pub fn join(records: Vec<ReceivedShare>, local: &[LocalCopy]) -> Vec<ReceivedCopy> {
    records
        .into_iter()
        .filter_map(|share| {
            let copy = local
                .iter()
                .find(|c| c.share_id == share.link.token && c.root.is_dir())?;
            Some(ReceivedCopy {
                root: copy.root.clone(),
                collection_id: copy.collection_id.clone(),
                share,
            })
        })
        .collect()
}

/// The files of the copy at `root` the customer changed since the version
/// `share` records: a YAML file whose hash differs from the one received,
/// one added, one removed, by the same scan sync uses. Sorted.
pub fn local_changes(root: &Path, share: &ReceivedShare) -> Vec<PathBuf> {
    use std::collections::BTreeSet;
    let scan = crate::model::backend::scan_sync_files(root);
    // Unreadable or too big to hash: there, and changed to be safe.
    let unhashed = scan.unreadable.iter().chain(&scan.too_big);
    let present: BTreeSet<&Path> = scan
        .files
        .iter()
        .map(|f| f.rel.as_path())
        .chain(unhashed.clone().map(PathBuf::as_path))
        .collect();
    let mut changed: BTreeSet<PathBuf> = scan
        .files
        .iter()
        .filter(|f| share.files.get(&f.rel) != Some(&f.hash))
        .map(|f| f.rel.clone())
        .chain(unhashed.cloned())
        .collect();
    changed.extend(
        share
            .files
            .keys()
            .filter(|rel| !present.contains(rel.as_path()))
            .cloned(),
    );
    changed.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share(token: &str) -> ReceivedShare {
        ReceivedShare {
            link: crate::model::backend::ShareLink {
                server: "https://api.x.test".into(),
                token: token.into(),
            },
            team: "Acme".into(),
            name: "Shipping API".into(),
            version: 1,
            added: chrono::Utc::now().date_naive(),
            files: Default::default(),
        }
    }

    #[test]
    fn records_round_trip_and_replace_by_share() {
        let d = tempfile::tempdir().unwrap();
        assert!(load(Some(d.path())).is_empty());
        assert!(load(None).is_empty());
        let copy = |id: &str, root: &str| LocalCopy {
            share_id: id.into(),
            root: d.path().join(root),
            collection_id: "c0ffee00".into(),
        };
        record(d.path(), copy("a", "one")).unwrap();
        record(d.path(), copy("b", "two")).unwrap();
        record(d.path(), copy("a", "three")).unwrap();
        assert_eq!(load(Some(d.path())), [copy("b", "two"), copy("a", "three")]);
        forget(d.path(), "b").unwrap();
        forget(d.path(), "nope").unwrap();
        assert_eq!(load(Some(d.path())), [copy("a", "three")]);
        std::fs::write(d.path().join(FILE), "{not json").unwrap();
        assert!(load(Some(d.path())).is_empty());
    }

    #[test]
    fn join_keeps_records_with_a_copy_here() {
        let d = tempfile::tempdir().unwrap();
        let here = d.path().join("here");
        std::fs::create_dir(&here).unwrap();
        let local = [
            LocalCopy {
                share_id: "a".into(),
                root: here.clone(),
                collection_id: "c0ffee00".into(),
            },
            LocalCopy {
                share_id: "gone".into(),
                root: d.path().join("gone"),
                collection_id: "c0ffee01".into(),
            },
        ];
        let joined = join(vec![share("elsewhere"), share("a"), share("gone")], &local);
        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].share.link.token, "a");
        assert_eq!(joined[0].root, here);
        assert_eq!(joined[0].collection_id, "c0ffee00");
    }

    #[test]
    fn local_changes_counts_edits_adds_and_removes() {
        use crate::model::backend::SyncFile;
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let files = [
            ("a.yaml", "request:\n  url: https://a\n"),
            ("rates/b.yaml", "request:\n  url: https://b\n"),
            ("rates/c.yaml", "request:\n  url: https://c\n"),
            ("curl-runner.yaml", "timeout: 5000\n"),
        ];
        std::fs::create_dir(root.join("rates")).unwrap();
        let mut received = share("a");
        for (rel, text) in files {
            std::fs::write(root.join(rel), text).unwrap();
            received
                .files
                .insert(PathBuf::from(rel), SyncFile::hash_of(text));
        }
        // As received: nothing changed. The .env and hidden files never count.
        std::fs::write(root.join(".env"), "API_TOKEN=x\n").unwrap();
        assert!(local_changes(root, &received).is_empty());
        // One edited, one removed, one added (in a new folder).
        std::fs::write(root.join("rates/b.yaml"), "request:\n  url: https://mine\n").unwrap();
        std::fs::remove_file(root.join("a.yaml")).unwrap();
        std::fs::create_dir(root.join("mine")).unwrap();
        std::fs::write(root.join("mine/new.yml"), "request:\n  url: https://n\n").unwrap();
        std::fs::write(root.join(".hidden.yaml"), "x: 1\n").unwrap();
        assert_eq!(
            local_changes(root, &received),
            [
                PathBuf::from("a.yaml"),
                PathBuf::from("mine/new.yml"),
                PathBuf::from("rates/b.yaml"),
            ]
        );
        // Changed back: no longer counted.
        std::fs::write(root.join("rates/b.yaml"), "request:\n  url: https://b\n").unwrap();
        assert_eq!(local_changes(root, &received).len(), 2);
    }
}
