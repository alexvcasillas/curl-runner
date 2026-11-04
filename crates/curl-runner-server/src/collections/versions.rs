//! Version history reads, and the per-file diff between two versions.

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::{BundleFile, FileChanges, GitMeta, VersionSummary};
use sqlx::types::Json as Jsonb;

use super::collection_id;
use crate::auth::UserAuth;
use crate::error::ApiError;
use crate::state::AppState;
use crate::teams::team_access;

/// Paths added, changed and removed going from `old` to `new` (path to file hash).
pub(super) fn diff(old: &HashMap<String, String>, new: &HashMap<String, String>) -> FileChanges {
    let mut changes = FileChanges::default();
    for (path, hash) in new {
        match old.get(path) {
            None => changes.added.push(path.clone()),
            Some(prev) if prev != hash => changes.changed.push(path.clone()),
            Some(_) => {}
        }
    }
    changes.removed = old
        .keys()
        .filter(|path| !new.contains_key(*path))
        .cloned()
        .collect();
    changes.added.sort();
    changes.changed.sort();
    changes.removed.sort();
    changes
}

type SummaryRow = (
    i32,
    String,
    DateTime<Utc>,
    String,
    Option<Jsonb<GitMeta>>,
    i64,
);

/// Newest first.
pub(super) async fn list(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
) -> Result<Json<Vec<VersionSummary>>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let id = collection_id(&state.db, access.team_id, &slug).await?;
    let rows: Vec<SummaryRow> = sqlx::query_as(
        "SELECT v.number, v.hash, v.created_at, v.published_by, v.git, \
                (SELECT count(*) FROM version_files f WHERE f.version_id = v.id) \
         FROM versions v WHERE v.collection_id = $1 ORDER BY v.number DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(number, hash, published_at, publisher, git, files)| VersionSummary {
                    number: u32::try_from(number).unwrap_or_default(),
                    hash,
                    published_at,
                    publisher,
                    git: git.map(|g| g.0),
                    file_count: u32::try_from(files).unwrap_or_default(),
                },
            )
            .collect(),
    ))
}

pub(super) async fn files(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug, n)): Path<(String, String, i32)>,
) -> Result<Json<Vec<BundleFile>>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let id = collection_id(&state.db, access.team_id, &slug).await?;
    let version: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM versions WHERE collection_id = $1 AND number = $2")
            .bind(id)
            .bind(n)
            .fetch_optional(&state.db)
            .await?;
    let version = version.ok_or_else(ApiError::not_found)?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT f.path, b.text FROM version_files f JOIN blobs b ON b.hash = f.hash \
         WHERE f.version_id = $1 ORDER BY f.path",
    )
    .bind(version)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(path, text)| BundleFile { path, text })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(items: &[(&str, &str)]) -> HashMap<String, String> {
        items
            .iter()
            .map(|(p, h)| ((*p).to_owned(), (*h).to_owned()))
            .collect()
    }

    #[test]
    fn diff_reports_added_changed_removed_sorted() {
        let old = m(&[
            ("a.yaml", "1"),
            ("b.yaml", "2"),
            ("z.yaml", "9"),
            ("y.yaml", "8"),
        ]);
        let new = m(&[
            ("a.yaml", "1"),
            ("b.yaml", "3"),
            ("d.yaml", "4"),
            ("c.yaml", "5"),
        ]);
        assert_eq!(
            diff(&old, &new),
            FileChanges {
                added: vec!["c.yaml".into(), "d.yaml".into()],
                changed: vec!["b.yaml".into()],
                removed: vec!["y.yaml".into(), "z.yaml".into()],
            }
        );
    }

    #[test]
    fn diff_against_nothing_is_all_added() {
        let new = m(&[("a.yaml", "1")]);
        assert_eq!(diff(&HashMap::new(), &new).added, vec!["a.yaml"]);
    }
}
