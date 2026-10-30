//! Where this Mac keeps each received copy: share id → the copy's folder
//! and its [`CollectionMeta::id`], in `{base}/received.json`. Local only:
//! the server's record ([`ReceivedShare`]) never carries a path.
//!
//! The sidebar's Shared with me (and everything that asks "which share is
//! this copy?") is the server's records joined with this mapping. A record
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
    /// [`ReceivedShare::id`].
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

/// Forgets the copy of share `share_id`; none is not an error.
pub fn forget(base: &Path, share_id: &str) -> std::io::Result<()> {
    let mut copies = load(Some(base));
    let before = copies.len();
    copies.retain(|c| c.share_id != share_id);
    if copies.len() == before {
        return Ok(());
    }
    save(base, &copies)
}

/// The server's `records` that have a copy here whose folder is still
/// there, in the records' order.
pub fn join(records: Vec<ReceivedShare>, local: &[LocalCopy]) -> Vec<ReceivedCopy> {
    records
        .into_iter()
        .filter_map(|share| {
            let copy = local
                .iter()
                .find(|c| c.share_id == share.id && c.root.is_dir())?;
            Some(ReceivedCopy {
                root: copy.root.clone(),
                collection_id: copy.collection_id.clone(),
                share,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share(id: &str) -> ReceivedShare {
        ReceivedShare {
            id: id.into(),
            owner_org: "Acme".into(),
            name: "Shipping API".into(),
            added: chrono::Utc::now(),
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
        assert_eq!(joined[0].share.id, "a");
        assert_eq!(joined[0].root, here);
        assert_eq!(joined[0].collection_id, "c0ffee00");
    }
}
