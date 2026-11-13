//! Two-way sync of a collection whose source is `cloud`: the wire types of
//! `POST /v1/teams/{team}/collections/{slug}/sync` and the per-file
//! three-way merge the server runs.
//!
//! Every hash is [`crate::file_hash`] (SHA-256 hex). A client sends each
//! file it has with the hash it last synced (`base`), and the text only for
//! files it changed since; one round trip pushes, pulls and finds conflicts.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A file the client has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncFile {
    pub path: String,
    pub hash: String,
    /// The hash this client last synced, `None` when it never did.
    pub base: Option<String>,
    /// Sent only when `hash` differs from `base`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A file the client synced before and has since deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncDeletion {
    pub path: String,
    pub base: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncRequest {
    pub files: Vec<SyncFile>,
    #[serde(default)]
    pub deleted: Vec<SyncDeletion>,
}

/// A server file to write locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PulledFile {
    pub path: String,
    pub hash: String,
    pub text: String,
}

/// A file changed on both sides. The server kept its copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncConflict {
    pub path: String,
    /// `None` when the server deleted it.
    pub server_hash: Option<String>,
    pub server_text: Option<String>,
    /// Who made the server's copy (an email), and when.
    pub server_author: String,
    pub server_at: DateTime<Utc>,
    /// The server deleted the file; the client changed it.
    pub deleted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncResponse {
    /// The current version after the sync; `0` while the collection has none.
    pub version: u32,
    /// Paths the server took from the client.
    pub pushed: Vec<String>,
    pub pulled: Vec<PulledFile>,
    pub conflicts: Vec<SyncConflict>,
    /// Paths the server no longer has that the client synced before: remove
    /// them locally.
    pub deleted: Vec<String>,
}

/// What one sync does, decided from hashes alone. Paths are sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Merge {
    /// Client files the server takes: `(path, hash)`.
    pub push: Vec<(String, String)>,
    /// Server files the client deleted and the server still had unchanged.
    pub remove: Vec<String>,
    /// Server files to send back.
    pub pull: Vec<String>,
    /// Paths to tell the client to delete.
    pub delete: Vec<String>,
    /// Changed on both sides; the server keeps its copy (or its deletion).
    pub conflicts: Vec<String>,
}

impl Merge {
    /// Whether the server's files change.
    pub fn changes_server(&self) -> bool {
        !self.push.is_empty() || !self.remove.is_empty()
    }
}

/// The three-way merge of `request` against the server's files (`path →
/// hash`). Per file, with L the client's hash, B its base and S the server's:
/// L = S → nothing; B = S → take the client's; L = B → the server's goes back
/// (a pull, or a delete when S is gone); otherwise a conflict. A deletion
/// applies only while S = B; a server copy changed since comes back.
/// Server files the client never mentioned are pulled.
///
/// Assumes paths are unique across `files` and `deleted`.
pub fn merge(server: &HashMap<String, String>, request: &SyncRequest) -> Merge {
    let mut out = Merge::default();
    let mut seen: BTreeMap<&str, ()> = BTreeMap::new();
    for f in &request.files {
        seen.insert(&f.path, ());
        let s = server.get(&f.path);
        if s == Some(&f.hash) {
            continue;
        }
        if s == f.base.as_ref() {
            out.push.push((f.path.clone(), f.hash.clone()));
        } else if f.base.as_ref() == Some(&f.hash) {
            match s {
                Some(_) => out.pull.push(f.path.clone()),
                None => out.delete.push(f.path.clone()),
            }
        } else {
            out.conflicts.push(f.path.clone());
        }
    }
    for d in &request.deleted {
        seen.insert(&d.path, ());
        match server.get(&d.path) {
            None => {}
            Some(s) if *s == d.base => out.remove.push(d.path.clone()),
            Some(_) => out.pull.push(d.path.clone()),
        }
    }
    for path in server.keys() {
        if !seen.contains_key(path.as_str()) {
            out.pull.push(path.clone());
        }
    }
    for v in [
        &mut out.remove,
        &mut out.pull,
        &mut out.delete,
        &mut out.conflicts,
    ] {
        v.sort();
    }
    out.push.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(files: &[(&str, &str)]) -> HashMap<String, String> {
        files
            .iter()
            .map(|(p, h)| (p.to_string(), h.to_string()))
            .collect()
    }

    fn file(path: &str, hash: &str, base: Option<&str>) -> SyncFile {
        SyncFile {
            path: path.into(),
            hash: hash.into(),
            base: base.map(Into::into),
            text: None,
        }
    }

    fn req(files: Vec<SyncFile>, deleted: &[(&str, &str)]) -> SyncRequest {
        SyncRequest {
            files,
            deleted: deleted
                .iter()
                .map(|(p, b)| SyncDeletion {
                    path: p.to_string(),
                    base: b.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn in_sync_does_nothing() {
        let m = merge(
            &server(&[("a.yaml", "1")]),
            &req(vec![file("a.yaml", "1", Some("1"))], &[]),
        );
        assert_eq!(m, Merge::default());
        assert!(!m.changes_server());
    }

    #[test]
    fn same_edit_on_both_sides_is_in_sync() {
        let m = merge(
            &server(&[("a.yaml", "2")]),
            &req(vec![file("a.yaml", "2", Some("1"))], &[]),
        );
        assert_eq!(m, Merge::default());
    }

    #[test]
    fn local_edit_is_pushed() {
        let m = merge(
            &server(&[("a.yaml", "1")]),
            &req(vec![file("a.yaml", "2", Some("1"))], &[]),
        );
        assert_eq!(m.push, [("a.yaml".to_string(), "2".to_string())]);
        assert!(m.changes_server());
    }

    #[test]
    fn new_local_file_is_pushed() {
        let m = merge(&server(&[]), &req(vec![file("a.yaml", "1", None)], &[]));
        assert_eq!(m.push.len(), 1);
    }

    #[test]
    fn server_edit_is_pulled() {
        let m = merge(
            &server(&[("a.yaml", "2")]),
            &req(vec![file("a.yaml", "1", Some("1"))], &[]),
        );
        assert_eq!(m.pull, ["a.yaml"]);
        assert!(!m.changes_server());
    }

    #[test]
    fn server_deletion_of_an_unchanged_file_deletes_locally() {
        let m = merge(
            &server(&[]),
            &req(vec![file("a.yaml", "1", Some("1"))], &[]),
        );
        assert_eq!(m.delete, ["a.yaml"]);
    }

    #[test]
    fn edits_on_both_sides_conflict() {
        let m = merge(
            &server(&[("a.yaml", "3")]),
            &req(vec![file("a.yaml", "2", Some("1"))], &[]),
        );
        assert_eq!(m.conflicts, ["a.yaml"]);
        assert!(!m.changes_server());
    }

    #[test]
    fn local_edit_of_a_server_deleted_file_conflicts() {
        let m = merge(
            &server(&[]),
            &req(vec![file("a.yaml", "2", Some("1"))], &[]),
        );
        assert_eq!(m.conflicts, ["a.yaml"]);
    }

    #[test]
    fn new_on_both_sides_with_different_text_conflicts() {
        let m = merge(
            &server(&[("a.yaml", "3")]),
            &req(vec![file("a.yaml", "2", None)], &[]),
        );
        assert_eq!(m.conflicts, ["a.yaml"]);
    }

    #[test]
    fn deletion_applies_only_while_the_server_is_unchanged() {
        let m = merge(
            &server(&[("a.yaml", "1"), ("b.yaml", "5")]),
            &req(vec![], &[("a.yaml", "1"), ("b.yaml", "4"), ("c.yaml", "9")]),
        );
        assert_eq!(m.remove, ["a.yaml"]);
        // Changed on the server since: it comes back.
        assert_eq!(m.pull, ["b.yaml"]);
        assert!(m.changes_server());
    }

    #[test]
    fn server_files_the_client_never_had_are_pulled() {
        let m = merge(
            &server(&[("b.yaml", "1"), ("a.yaml", "2")]),
            &req(vec![], &[]),
        );
        assert_eq!(m.pull, ["a.yaml", "b.yaml"]);
    }

    #[test]
    fn text_is_left_out_of_the_wire_when_absent() {
        let json = serde_json::to_string(&file("a.yaml", "1", None)).unwrap();
        assert_eq!(json, r#"{"path":"a.yaml","hash":"1","base":null}"#);
        let back: SyncRequest = serde_json::from_str(r#"{"files":[]}"#).unwrap();
        assert!(back.deleted.is_empty());
    }
}
