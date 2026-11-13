#![cfg(feature = "db-tests")]

mod common;

use std::io::Write;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{TestApp, test_app};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use curl_runner_server::app;
use flate2::Compression;
use flate2::write::GzEncoder;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const PUBLISH: &str = "/v1/teams/acme/collections/payments-api/versions";

fn body_for(files: &[(&str, &str)]) -> Value {
    let files: Vec<BundleFile> = files
        .iter()
        .map(|(p, t)| BundleFile {
            path: (*p).into(),
            text: (*t).into(),
        })
        .collect();
    json!({
        "hash": bundle_hash(&files),
        "files": files,
        "inputs": [],
    })
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::fast());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

/// PUT `body` to `path`, optionally gzipped, through a fresh router on the app's state.
async fn put_raw(
    app: &TestApp,
    path: &str,
    bearer: &str,
    body: Vec<u8>,
    gzipped: bool,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(Method::PUT)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .header(header::CONTENT_TYPE, "application/json");
    if gzipped {
        req = req.header(header::CONTENT_ENCODING, "gzip");
    }
    let res = app_router(app)
        .oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn app_router(test: &TestApp) -> axum::Router {
    app(test.state.clone())
}

async fn publish(app: &TestApp, bearer: &str, files: &[(&str, &str)]) -> (StatusCode, Value) {
    put_raw(
        app,
        PUBLISH,
        bearer,
        body_for(files).to_string().into_bytes(),
        false,
    )
    .await
}

async fn setup(pool: PgPool) -> (TestApp, String) {
    let app = test_app(pool).await;
    let s = app.sign_in("a@example.com").await;
    app.create_team(&s, "acme").await;
    (app, s)
}

async fn count(app: &TestApp, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn first_publish_creates_git_collection_v1(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let (status, body) = publish(&app, &s, &[("a.yaml", "x: 1"), ("b/c.yaml", "y: 2")]).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["status"], "created");
    assert_eq!(body["version"], 1);
    assert_eq!(body["collection"], "payments-api");
    assert_eq!(body["changed"]["added"], json!(["a.yaml", "b/c.yaml"]));

    let (status, list) = app
        .request(Method::GET, "/v1/teams/acme/collections", None, Some(&s))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list[0]["slug"], "payments-api");
    assert_eq!(list[0]["name"], "payments-api");
    assert_eq!(list[0]["source"], "git");
    assert_eq!(list[0]["access"], "off");
    assert_eq!(list[0]["current_version"], 1);
    assert!(list[0]["updated_at"].is_string());
    assert_eq!(list[0]["publisher"], "a@example.com");
    // Published from files without git details: the dashboard says so.
    assert_eq!(list[0]["git"], Value::Null);

    // A fresh link token, hashed, and recoverable with the secret key.
    let (hash, enc): (String, String) =
        sqlx::query_as("SELECT link_hash, link_token_enc FROM collections")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    let key = app.state.config.secret_key.as_bytes();
    let token = curl_runner_server::crypto::decrypt_link_token(key, &enc).unwrap();
    assert!(token.starts_with("crsl_"));
    assert_eq!(hash, curl_runner_server::hash::secret_hash(key, &token));

    let (status, versions) = app.request(Method::GET, PUBLISH, None, Some(&s)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(versions[0]["number"], 1);
    assert_eq!(versions[0]["file_count"], 2);
    assert_eq!(versions[0]["publisher"], "a@example.com");
    let (status, files) = app
        .request(Method::GET, &format!("{PUBLISH}/1/files"), None, Some(&s))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        files,
        json!([{"path": "a.yaml", "text": "x: 1"}, {"path": "b/c.yaml", "text": "y: 2"}])
    );
}

#[sqlx::test]
async fn same_payload_twice_is_unchanged_same_version(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, &[("a.yaml", "x: 1")]).await;
    let (status, body) = publish(&app, &s, &[("a.yaml", "x: 1")]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "unchanged");
    assert_eq!(body["version"], 1);
    assert_eq!(count(&app, "versions").await, 1);
}

#[sqlx::test]
async fn changed_payload_v2_with_changes(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(
        &app,
        &s,
        &[("keep.yaml", "1"), ("edit.yaml", "1"), ("gone.yaml", "1")],
    )
    .await;
    let mut body = body_for(&[("keep.yaml", "1"), ("edit.yaml", "2"), ("new.yaml", "1")]);
    body["name"] = json!("Payments API");
    body["description"] = json!("All the payment calls");
    let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    assert_eq!(res["version"], 2);
    assert_eq!(res["changed"]["added"], json!(["new.yaml"]));
    assert_eq!(res["changed"]["changed"], json!(["edit.yaml"]));
    assert_eq!(res["changed"]["removed"], json!(["gone.yaml"]));
    let (name, desc, current): (String, Option<String>, i32) =
        sqlx::query_as("SELECT name, description, current_version FROM collections")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(
        (name.as_str(), desc.as_deref(), current),
        ("Payments API", Some("All the payment calls"), 2)
    );
    // Version 1 is untouched.
    let (_, files) = app
        .request(Method::GET, &format!("{PUBLISH}/1/files"), None, Some(&s))
        .await;
    assert_eq!(files.as_array().unwrap().len(), 3);
}

#[sqlx::test]
async fn bad_hash_400(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let mut body = body_for(&[("a.yaml", "1")]);
    body["hash"] = json!("0".repeat(64));
    let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["error"], "invalid_bundle");
    assert_eq!(count(&app, "collections").await, 0);
}

#[sqlx::test]
async fn path_traversal_400(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let (status, res) = publish(&app, &s, &[("../a.yaml", "1")]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["error"], "invalid_bundle");
    assert!(res["message"].as_str().unwrap().contains("../a.yaml"));
}

#[sqlx::test]
async fn too_many_files_400(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let names: Vec<String> = (0..2001).map(|i| format!("f{i}.yaml")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "")).collect();
    let (status, res) = publish(&app, &s, &files).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["error"], "invalid_bundle");
}

#[sqlx::test]
async fn malformed_requests_are_400(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let (status, res) = put_raw(&app, PUBLISH, &s, b"{nope".to_vec(), false).await;
    assert_eq!(
        (status, &res["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_request"))
    );
    let (status, res) = put_raw(&app, PUBLISH, &s, b"not gzip".to_vec(), true).await;
    assert_eq!(
        (status, &res["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_request"))
    );
    let (status, res) = put_raw(
        &app,
        "/v1/teams/acme/collections/Bad_Slug/versions",
        &s,
        body_for(&[("a.yaml", "1")]).to_string().into_bytes(),
        false,
    )
    .await;
    assert_eq!(
        (status, &res["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_bundle"))
    );
    // NUL cannot be stored in Postgres text.
    let (status, res) = publish(&app, &s, &[("a.yaml", "a\u{0}b")]).await;
    assert_eq!(
        (status, &res["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_bundle"))
    );
}

fn input(name: &str, secret: bool, default: Option<&str>) -> Value {
    json!({
        "name": name,
        "description": null,
        "secret": secret,
        "required": false,
        "locked": false,
        "default": default,
    })
}

#[sqlx::test]
async fn bad_inputs_are_400_invalid_bundle(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let long = "A".repeat(129);
    for (inputs, needle) in [
        (
            json!([input("API_KEY", true, Some("sk_live_123"))]),
            "API_KEY",
        ),
        (json!([input("", false, None)]), "empty name"),
        (json!([input(&long, false, None)]), "longer than 128"),
        (json!([input("1BAD", false, None)]), "1BAD"),
        (json!([input("A-B", false, None)]), "A-B"),
        (
            json!([input("TOKEN", false, None), input("TOKEN", false, None)]),
            "more than once",
        ),
    ] {
        let mut body = body_for(&[("a.yaml", "1")]);
        body["inputs"] = inputs;
        let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{res}");
        assert_eq!(res["error"], "invalid_bundle");
        assert!(res["message"].as_str().unwrap().contains(needle), "{res}");
    }
    assert_eq!(count(&app, "collections").await, 0);

    // A secret without a default and a plain input with one are fine.
    let mut body = body_for(&[("a.yaml", "1")]);
    body["inputs"] = json!([
        input("API_KEY", true, None),
        input("BASE_URL", false, Some("https://api.example.com"))
    ]);
    let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
}

#[sqlx::test]
async fn name_and_description_bounds_400(pool: PgPool) {
    let (app, s) = setup(pool).await;
    for (field, value) in [
        ("name", "x".repeat(101)),
        ("name", "  ".to_string()),
        ("description", "x".repeat(501)),
    ] {
        let mut body = body_for(&[("a.yaml", "1")]);
        body[field] = json!(value);
        let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}: {res}");
        assert_eq!(res["error"], "invalid_bundle");
    }
    let mut body = body_for(&[("a.yaml", "1")]);
    body["name"] = json!("é".repeat(100));
    body["description"] = json!("x".repeat(500));
    let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
}

#[sqlx::test]
async fn zip_bomb_400_invalid_bundle(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let bomb = gzip(&vec![b' '; 52 << 20]);
    let (status, res) = put_raw(&app, PUBLISH, &s, bomb, true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["error"], "invalid_bundle");
}

#[sqlx::test]
async fn cloud_source_409_wrong_source_with_message(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, &[("a.yaml", "1")]).await;
    let (status, _) = app
        .request(
            Method::PATCH,
            "/v1/teams/acme/collections/payments-api",
            Some(json!({"source": "cloud"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, res) = publish(&app, &s, &[("a.yaml", "2")]).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(res["error"], "wrong_source");
    assert_eq!(
        res["message"],
        "payments-api is synced from the studio. Switch its source to git in the dashboard to publish from CI."
    );
    // Switching back keeps the current version and allows publishing again.
    let (_, patched) = app
        .request(
            Method::PATCH,
            "/v1/teams/acme/collections/payments-api",
            Some(json!({"source": "git"})),
            Some(&s),
        )
        .await;
    assert_eq!(patched["current_version"], 1);
    assert_eq!(
        publish(&app, &s, &[("a.yaml", "2")]).await.0,
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn restricted_on_free_402(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, &[("a.yaml", "1")]).await;
    sqlx::query("UPDATE collections SET access = 'restricted'")
        .execute(&app.state.db)
        .await
        .unwrap();
    let (status, res) = publish(&app, &s, &[("a.yaml", "2")]).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(res["error"], "plan_limit");
    assert_eq!(
        res["message"],
        "payments-api is restricted, which needs the Team plan. Upgrade, or set its access to public or off."
    );
    assert_eq!(count(&app, "versions").await, 1);
    // On the Team plan it publishes.
    sqlx::query("UPDATE teams SET plan = 'team'")
        .execute(&app.state.db)
        .await
        .unwrap();
    assert_eq!(
        publish(&app, &s, &[("a.yaml", "2")]).await.0,
        StatusCode::CREATED
    );
}

/// Spawn one publish of `text` through its own router and connection checkout.
fn spawn_publish(
    app: &TestApp,
    bearer: &str,
    text: &'static str,
) -> tokio::task::JoinHandle<(StatusCode, Value)> {
    let state = app.state.clone();
    let bearer = bearer.to_owned();
    tokio::spawn(async move {
        let req = Request::builder()
            .method(Method::PUT)
            .uri(PUBLISH)
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body_for(&[("a.yaml", text)]).to_string()))
            .unwrap();
        let res = curl_runner_server::app(state).oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    })
}

/// Wait until `n` backends are blocked on a lock. The polling query paces the
/// loop; a publish that never blocks (no lock taken) fails the assertion.
async fn wait_blocked(
    app: &TestApp,
    n: i64,
    tasks: &[tokio::task::JoinHandle<(StatusCode, Value)>],
) {
    for _ in 0..5000 {
        let blocked: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE wait_event_type = 'Lock' AND datname = current_database()",
        )
        .fetch_one(&app.state.db)
        .await
        .unwrap();
        if blocked >= n {
            return;
        }
        assert!(
            tasks.iter().all(|t| !t.is_finished()),
            "a publish finished without waiting for the lock"
        );
        tokio::task::yield_now().await;
    }
    panic!("publishes never blocked on the lock");
}

#[sqlx::test]
async fn concurrent_publishes_get_consecutive_numbers(pool: PgPool) {
    let (app, s) = setup(pool).await;
    assert_eq!(
        publish(&app, &s, &[("a.yaml", "0")]).await.0,
        StatusCode::CREATED
    );
    // Hold the collection row so both publishes are in flight, and parked on
    // the row lock, at the same moment. Without the lock in the publish they
    // would not park, and the wait below fails.
    let mut holder = app.state.db.begin().await.unwrap();
    sqlx::query("SELECT id FROM collections WHERE slug = 'payments-api' FOR UPDATE")
        .execute(&mut *holder)
        .await
        .unwrap();
    let tasks = vec![
        spawn_publish(&app, &s, "one"),
        spawn_publish(&app, &s, "two"),
    ];
    wait_blocked(&app, 2, &tasks).await;
    holder.commit().await.unwrap();
    let mut numbers = Vec::new();
    for t in tasks {
        let (status, body) = t.await.unwrap();
        assert_eq!(status, StatusCode::CREATED, "{body}");
        numbers.push(body["version"].as_i64().unwrap());
    }
    numbers.sort_unstable();
    assert_eq!(numbers, vec![2, 3]);
    assert_eq!(count(&app, "versions").await, 3);
}

#[sqlx::test]
async fn concurrent_first_publishes_do_not_fail(pool: PgPool) {
    let (app, s) = setup(pool).await;
    // Another transaction is mid-way through creating the collection. Both
    // publishes park on its uncommitted row, then must find it, not error.
    let mut creator = app.state.db.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO collections (team_id, slug, name) \
         SELECT id, 'payments-api', 'payments-api' FROM teams WHERE slug = 'acme'",
    )
    .execute(&mut *creator)
    .await
    .unwrap();
    let tasks = vec![
        spawn_publish(&app, &s, "one"),
        spawn_publish(&app, &s, "two"),
    ];
    wait_blocked(&app, 2, &tasks).await;
    creator.commit().await.unwrap();
    let mut numbers = Vec::new();
    for t in tasks {
        let (status, body) = t.await.unwrap();
        assert_eq!(status, StatusCode::CREATED, "{body}");
        numbers.push(body["version"].as_i64().unwrap());
    }
    numbers.sort_unstable();
    assert_eq!(numbers, vec![1, 2]);
    assert_eq!(count(&app, "collections").await, 1);
}

#[sqlx::test]
async fn gzip_body_accepted(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let body = gzip(body_for(&[("a.yaml", "x: 1")]).to_string().as_bytes());
    let (status, res) = put_raw(&app, PUBLISH, &s, body, true).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    assert_eq!(res["version"], 1);
}

#[sqlx::test]
async fn blobs_deduplicated(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, &[("a.yaml", "same"), ("b.yaml", "same")]).await;
    publish(&app, &s, &[("a.yaml", "same"), ("c.yaml", "other")]).await;
    assert_eq!(count(&app, "blobs").await, 2);
    assert_eq!(count(&app, "version_files").await, 4);
}

#[sqlx::test]
async fn other_team_404(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, &[("a.yaml", "1")]).await;
    let outsider = app.sign_in("b@example.com").await;
    app.create_team(&outsider, "other").await;
    let (status, _) = publish(&app, &outsider, &[("a.yaml", "2")]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for path in [
        "/v1/teams/acme/collections",
        PUBLISH,
        &format!("{PUBLISH}/1/files"),
    ] {
        let (status, _) = app.request(Method::GET, path, None, Some(&outsider)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    let (status, _) = app
        .request(
            Method::PATCH,
            "/v1/teams/acme/collections/payments-api",
            Some(json!({"source": "cloud"})),
            Some(&outsider),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(
            Method::DELETE,
            "/v1/teams/acme/collections/payments-api",
            None,
            Some(&outsider),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn mint_token(app: &TestApp, session: &str, team: &str) -> String {
    let (status, body) = app
        .request(
            Method::POST,
            &format!("/v1/teams/{team}/tokens"),
            Some(json!({"name": "ci"})),
            Some(session),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["token"].as_str().unwrap().to_owned()
}

#[sqlx::test]
async fn publish_token_reads_collections_versions_and_files(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let token = mint_token(&app, &s, "acme").await;
    let (status, res) = publish(&app, &token, &[("a.yaml", "x: 1")]).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");

    let (status, list) = app
        .request(
            Method::GET,
            "/v1/teams/acme/collections",
            None,
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list[0]["slug"], "payments-api");
    let (status, versions) = app.request(Method::GET, PUBLISH, None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{versions}");
    assert_eq!(versions[0]["number"], 1);
    assert_eq!(versions[0]["publisher"], "token ci");
    let (status, files) = app
        .request(
            Method::GET,
            &format!("{PUBLISH}/1/files"),
            None,
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(files, json!([{"path": "a.yaml", "text": "x: 1"}]));

    // Read-only: the token cannot change or delete the collection.
    let (status, _) = app
        .request(
            Method::PATCH,
            "/v1/teams/acme/collections/payments-api",
            Some(json!({"source": "cloud"})),
            Some(&token),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Another team's token gets 404 on all three.
    let outsider = app.sign_in("b@example.com").await;
    app.create_team(&outsider, "other").await;
    let other = mint_token(&app, &outsider, "other").await;
    for path in [
        "/v1/teams/acme/collections",
        PUBLISH,
        &format!("{PUBLISH}/1/files"),
    ] {
        let (status, _) = app.request(Method::GET, path, None, Some(&other)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[sqlx::test]
async fn publish_rate_limited_per_team(pool: PgPool) {
    let (app, s) = setup(pool).await;
    for i in 0..60 {
        let (status, res) = publish(&app, &s, &[("a.yaml", &i.to_string())]).await;
        assert_eq!(status, StatusCode::CREATED, "{i}: {res}");
    }
    let (status, res) = publish(&app, &s, &[("a.yaml", "late")]).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{res}");
    assert_eq!(res["error"], "rate_limited");
    // Another team is unaffected, and the window resets.
    let other = app.sign_in("b@example.com").await;
    app.create_team(&other, "other").await;
    let (status, _) = put_raw(
        &app,
        "/v1/teams/other/collections/payments-api/versions",
        &other,
        body_for(&[("a.yaml", "1")]).to_string().into_bytes(),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    app.clock.advance(chrono::Duration::minutes(1));
    let (status, _) = publish(&app, &s, &[("a.yaml", "late")]).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn delete_is_owner_only_and_keeps_blobs(pool: PgPool) {
    let (app, owner) = setup(pool).await;
    publish(&app, &owner, &[("a.yaml", "1")]).await;
    // Add a plain member directly.
    let member = app.sign_in("m@example.com").await;
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT t.id, u.id, 'member', now() FROM teams t, users u \
         WHERE t.slug = 'acme' AND u.email = 'm@example.com'",
    )
    .execute(&app.state.db)
    .await
    .unwrap();
    let path = "/v1/teams/acme/collections/payments-api";
    let (status, res) = app.request(Method::DELETE, path, None, Some(&member)).await;
    assert_eq!(
        (status, &res["error"]),
        (StatusCode::FORBIDDEN, &json!("forbidden"))
    );
    let (status, _) = app.request(Method::DELETE, path, None, Some(&owner)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(count(&app, "collections").await, 0);
    assert_eq!(count(&app, "versions").await, 0);
    assert_eq!(count(&app, "version_files").await, 0);
    assert_eq!(count(&app, "blobs").await, 1);
    let (status, _) = app.request(Method::DELETE, path, None, Some(&owner)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn escaping_heavy_bundle_within_25_mib_is_accepted(pool: PgPool) {
    let (app, s) = setup(pool).await;
    // 24 MiB of text whose JSON form is about 48 MiB: over the old 26 MiB cap.
    let text = "\n".repeat(1 << 20);
    let names: Vec<String> = (0..24).map(|i| format!("f{i}.yaml")).collect();
    // Distinct texts, so each file is its own blob.
    let texts: Vec<String> = (0..24)
        .map(|i| format!("{}{}", "x".repeat(i), &text[i..]))
        .collect();
    let files: Vec<(&str, &str)> = names
        .iter()
        .zip(&texts)
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let json = body_for(&files).to_string();
    assert!(json.len() > 26 << 20, "{}", json.len());
    let (status, res) = put_raw(&app, PUBLISH, &s, gzip(json.as_bytes()), true).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    // Over 25 MiB of real text still fails, with the bundle message (24 escaped
    // files plus 2 plain ones: 26 MiB of text, about 50 MiB of JSON).
    let big: Vec<String> = (0..26)
        .map(|i| {
            if i < 24 {
                format!("{}{}", "y".repeat(i), &text[i..])
            } else {
                format!("{i}{}", "z".repeat((1 << 20) - 2))
            }
        })
        .collect();
    let names: Vec<String> = (0..26).map(|i| format!("g{i}.yaml")).collect();
    let files: Vec<(&str, &str)> = names
        .iter()
        .zip(&big)
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let (status, res) = put_raw(
        &app,
        PUBLISH,
        &s,
        gzip(body_for(&files).to_string().as_bytes()),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["error"], "invalid_bundle");
    let message = res["message"].as_str().unwrap();
    assert!(message.contains("bundle is larger"), "{message}");
}

#[sqlx::test]
async fn non_numeric_version_is_json_404(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, &[("a.yaml", "1")]).await;
    let (status, res) = app
        .request(
            Method::GET,
            &format!("{PUBLISH}/latest/files"),
            None,
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(res["error"], "not_found");
}

#[sqlx::test]
async fn concurrent_identical_publishes_create_once(pool: PgPool) {
    let (app, s) = setup(pool).await;
    assert_eq!(
        publish(&app, &s, &[("a.yaml", "0")]).await.0,
        StatusCode::CREATED
    );
    let mut holder = app.state.db.begin().await.unwrap();
    sqlx::query("SELECT id FROM collections WHERE slug = 'payments-api' FOR UPDATE")
        .execute(&mut *holder)
        .await
        .unwrap();
    let tasks = vec![
        spawn_publish(&app, &s, "same"),
        spawn_publish(&app, &s, "same"),
    ];
    wait_blocked(&app, 2, &tasks).await;
    holder.commit().await.unwrap();
    let mut results = Vec::new();
    for t in tasks {
        let (status, body) = t.await.unwrap();
        results.push((
            status.as_u16(),
            body["status"].as_str().unwrap().to_owned(),
            body["version"].as_i64().unwrap(),
        ));
    }
    results.sort();
    assert_eq!(
        results,
        vec![
            (200, "unchanged".to_owned(), 2),
            (201, "created".to_owned(), 2)
        ]
    );
    assert_eq!(count(&app, "versions").await, 2);
}

#[sqlx::test]
async fn list_carries_the_current_versions_git_details(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let mut body = body_for(&[("a.yaml", "x: 1")]);
    body["git"] = json!({"commit": "4f2a9c1aa", "ref": "refs/heads/main", "repo_url": "https://github.com/acme/api"});
    let (status, res) = put_raw(&app, PUBLISH, &s, body.to_string().into_bytes(), false).await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
    let (_, list) = app
        .request(Method::GET, "/v1/teams/acme/collections", None, Some(&s))
        .await;
    assert_eq!(list[0]["git"]["commit"], "4f2a9c1aa");
    assert_eq!(list[0]["git"]["ref"], "refs/heads/main");
    assert_eq!(list[0]["git"]["repo_url"], "https://github.com/acme/api");
}
