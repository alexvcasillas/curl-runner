#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, test_app};
use curl_runner_cloud_proto::file_hash;
use curl_runner_server::clock::Clock;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const OWNER: &str = "ada@example.com";
const MEMBER: &str = "sam@example.com";
const COLLECTIONS: &str = "/v1/teams/acme/collections";
const SYNC: &str = "/v1/teams/acme/collections/pay/sync";

/// `acme` with an owner and a member, and a new cloud collection `pay`.
async fn setup(pool: PgPool) -> (TestApp, String, String) {
    let app = test_app(pool).await;
    let owner = app.sign_in(OWNER).await;
    let member = app.sign_in(MEMBER).await;
    app.create_team(&owner, "acme").await;
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT t.id, u.id, 'member', now() FROM teams t, users u \
         WHERE t.slug = 'acme' AND u.email = $1",
    )
    .bind(MEMBER)
    .execute(&app.state.db)
    .await
    .unwrap();
    let (status, body) = app
        .request(
            Method::POST,
            COLLECTIONS,
            Some(json!({"slug": "pay", "name": "Payments"})),
            Some(&owner),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["source"], "cloud");
    assert_eq!(body["current_version"], Value::Null);
    (app, owner, member)
}

fn h(text: &str) -> String {
    file_hash(text)
}

/// A file the client has: `text` with the base it last synced.
fn has(path: &str, text: &str, base: Option<&str>) -> Value {
    let changed = base.map(h).as_deref() != Some(h(text).as_str());
    json!({
        "path": path,
        "hash": h(text),
        "base": base.map(h),
        "text": if changed { Some(text) } else { None },
    })
}

fn gone(path: &str, base: &str) -> Value {
    json!({"path": path, "base": h(base)})
}

async fn sync(app: &TestApp, s: &str, files: Vec<Value>, deleted: Vec<Value>) -> Value {
    let (status, body) = sync_raw(app, s, files, deleted).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn sync_raw(
    app: &TestApp,
    s: &str,
    files: Vec<Value>,
    deleted: Vec<Value>,
) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        SYNC,
        Some(json!({"files": files, "deleted": deleted})),
        Some(s),
    )
    .await
}

fn paths(v: &Value) -> Vec<&str> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().or_else(|| p["path"].as_str()).unwrap())
        .collect()
}

async fn versions(app: &TestApp) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM versions")
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn first_sync_pushes_and_a_teammate_pulls(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    let a = "request:\n  name: A\n  url: https://x\n";
    let body = sync(&app, &owner, vec![has("a.yaml", a, None)], vec![]).await;
    assert_eq!(body["version"], 1);
    assert_eq!(paths(&body["pushed"]), ["a.yaml"]);
    let (publisher, requests): (String, i32) =
        sqlx::query_as("SELECT published_by, request_count FROM versions")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!((publisher.as_str(), requests), (OWNER, 1));
    // A teammate with nothing gets the file.
    let body = sync(&app, &member, vec![], vec![]).await;
    assert_eq!(body["version"], 1);
    assert_eq!(
        body["pulled"],
        json!([{"path": "a.yaml", "hash": h(a), "text": a}])
    );
    // In sync: nothing happens, no version.
    let body = sync(&app, &member, vec![has("a.yaml", a, Some(a))], vec![]).await;
    assert_eq!(body["pushed"], json!([]));
    assert_eq!(body["pulled"], json!([]));
    assert_eq!(versions(&app).await, 1);
}

#[sqlx::test]
async fn edits_flow_both_ways(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    sync(&app, &owner, vec![has("a.yaml", "1", None)], vec![]).await;
    // Sam edits: pushed as v2.
    let body = sync(&app, &member, vec![has("a.yaml", "2", Some("1"))], vec![]).await;
    assert_eq!(body["version"], 2);
    // Ada's unchanged copy pulls it.
    let body = sync(&app, &owner, vec![has("a.yaml", "1", Some("1"))], vec![]).await;
    assert_eq!(body["pulled"][0]["text"], "2");
    // The same edit on both sides is already in sync.
    let body = sync(&app, &owner, vec![has("a.yaml", "2", Some("1"))], vec![]).await;
    assert_eq!(body["pulled"], json!([]));
    assert_eq!(body["conflicts"], json!([]));
    assert_eq!(versions(&app).await, 2);
}

#[sqlx::test]
async fn edits_on_both_sides_conflict_and_the_server_keeps_its_copy(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    sync(&app, &owner, vec![has("a.yaml", "1", None)], vec![]).await;
    app.clock.advance(chrono::Duration::minutes(5));
    let at = app.clock.now();
    sync(&app, &member, vec![has("a.yaml", "sam", Some("1"))], vec![]).await;
    app.clock.advance(chrono::Duration::minutes(5));
    // A later version that doesn't touch a.yaml doesn't move its author.
    sync(
        &app,
        &member,
        vec![has("a.yaml", "sam", Some("sam")), has("b.yaml", "b", None)],
        vec![],
    )
    .await;
    let body = sync(&app, &owner, vec![has("a.yaml", "ada", Some("1"))], vec![]).await;
    assert_eq!(body["version"], 3);
    let c = &body["conflicts"][0];
    assert_eq!(c["path"], "a.yaml");
    assert_eq!(c["server_hash"], h("sam"));
    assert_eq!(c["server_text"], "sam");
    assert_eq!(c["server_author"], MEMBER);
    assert_eq!(c["server_at"], json!(at));
    assert_eq!(c["deleted"], false);
    // b.yaml, never seen by Ada, comes down in the same sync.
    assert_eq!(paths(&body["pulled"]), ["b.yaml"]);
    assert_eq!(versions(&app).await, 3);
}

#[sqlx::test]
async fn deletions(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    sync(
        &app,
        &owner,
        vec![has("a.yaml", "1", None), has("b.yaml", "1", None)],
        vec![],
    )
    .await;
    // Sam deletes a.yaml: the server drops it.
    let body = sync(
        &app,
        &member,
        vec![has("b.yaml", "1", Some("1"))],
        vec![gone("a.yaml", "1")],
    )
    .await;
    assert_eq!(body["version"], 2);
    assert_eq!(paths(&body["pushed"]), ["a.yaml"]);
    // Ada's unchanged copy is told to delete it.
    let body = sync(
        &app,
        &owner,
        vec![has("a.yaml", "1", Some("1")), has("b.yaml", "1", Some("1"))],
        vec![],
    )
    .await;
    assert_eq!(paths(&body["deleted"]), ["a.yaml"]);
    // Sam edits b.yaml; Ada deleted it meanwhile: it comes back, not lost.
    sync(&app, &member, vec![has("b.yaml", "2", Some("1"))], vec![]).await;
    let body = sync(&app, &owner, vec![], vec![gone("b.yaml", "1")]).await;
    assert_eq!(paths(&body["pulled"]), ["b.yaml"]);
    assert_eq!(versions(&app).await, 3);
    // Ada edits a file Sam deleted: a conflict saying so.
    sync(&app, &member, vec![], vec![gone("b.yaml", "2")]).await;
    app.clock.advance(chrono::Duration::minutes(1));
    let body = sync(&app, &owner, vec![has("b.yaml", "3", Some("2"))], vec![]).await;
    let c = &body["conflicts"][0];
    assert_eq!(c["deleted"], true);
    assert_eq!(c["server_hash"], Value::Null);
    assert_eq!(c["server_author"], MEMBER);
}

#[sqlx::test]
async fn request_count_follows_the_changes(pool: PgPool) {
    let (app, owner, _) = setup(pool).await;
    let two = "requests:\n  - url: https://a\n  - url: https://b\n";
    let one = "request:\n  url: https://a\n";
    sync(
        &app,
        &owner,
        vec![
            has("a.yaml", two, None),
            has("b.yaml", one, None),
            has("curl-runner.yaml", two, None),
        ],
        vec![],
    )
    .await;
    sync(
        &app,
        &owner,
        vec![
            has("a.yaml", one, Some(two)),
            has("curl-runner.yaml", two, Some(two)),
        ],
        vec![gone("b.yaml", one)],
    )
    .await;
    let counts: Vec<i32> = sqlx::query_scalar("SELECT request_count FROM versions ORDER BY number")
        .fetch_all(&app.state.db)
        .await
        .unwrap();
    assert_eq!(counts, [3, 1]);
}

#[sqlx::test]
async fn concurrent_syncs_take_turns(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    sync(&app, &owner, vec![has("a.yaml", "1", None)], vec![]).await;
    let (x, y) = tokio::join!(
        sync(&app, &owner, vec![has("a.yaml", "ada", Some("1"))], vec![]),
        sync(&app, &member, vec![has("a.yaml", "sam", Some("1"))], vec![]),
    );
    // One wins, the other sees its version and conflicts.
    let conflicts = [&x, &y]
        .iter()
        .filter(|b| !b["conflicts"].as_array().unwrap().is_empty())
        .count();
    assert_eq!(conflicts, 1);
    assert_eq!(versions(&app).await, 2);
}

#[sqlx::test]
async fn refusals(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    // A changed file without its text.
    let mut file = has("a.yaml", "1", None);
    file["text"] = Value::Null;
    let (status, body) = sync_raw(&app, &owner, vec![file], vec![]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    // A text that isn't the hash.
    let mut file = has("a.yaml", "1", None);
    file["text"] = json!("2");
    let (status, _) = sync_raw(&app, &owner, vec![file], vec![]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Two paths differing only in case.
    let (status, body) = sync_raw(
        &app,
        &owner,
        vec![has("a.yaml", "1", None), has("A.yaml", "2", None)],
        vec![],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_bundle");
    // Unknown collection, outsider.
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/teams/acme/collections/nope/sync",
            Some(json!({"files": []})),
            Some(&owner),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let outsider = app.sign_in("eve@example.com").await;
    let (status, _) = sync_raw(&app, &outsider, vec![], vec![]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // A slug that exists.
    let (status, body) = app
        .request(
            Method::POST,
            COLLECTIONS,
            Some(json!({"slug": "pay", "name": "Again"})),
            Some(&member),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}

#[sqlx::test]
async fn only_owners_switch_the_source_and_git_collections_refuse_sync(pool: PgPool) {
    let (app, owner, member) = setup(pool).await;
    sync(&app, &owner, vec![has("a.yaml", "1", None)], vec![]).await;
    let patch = "/v1/teams/acme/collections/pay";
    let (status, body) = app
        .request(
            Method::PATCH,
            patch,
            Some(json!({"source": "git"})),
            Some(&member),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, body) = app
        .request(
            Method::PATCH,
            patch,
            Some(json!({"source": "git"})),
            Some(&owner),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = sync_raw(&app, &member, vec![], vec![]).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "wrong_source");
    assert_eq!(
        body["message"],
        "pay is published from files. Switch its source to cloud in the dashboard to sync it from the studio."
    );
}

#[sqlx::test]
async fn suspended_teams_and_paused_publishing_stop_syncs(pool: PgPool) {
    let (app, owner, _) = setup(pool).await;
    let team: Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'acme'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    sqlx::query("UPDATE teams SET suspended_at = now() WHERE id = $1")
        .bind(team)
        .execute(&app.state.db)
        .await
        .unwrap();
    let (status, body) = sync_raw(&app, &owner, vec![], vec![]).await;
    assert_eq!(status.as_u16() / 100, 4, "{body}");
    assert_eq!(body["error"], "team_suspended");
}
