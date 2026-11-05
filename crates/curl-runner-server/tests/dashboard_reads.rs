#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{TestApp, test_app, test_app_with, test_config};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use serde_json::{Value, json};
use sqlx::PgPool;

const OWNER: &str = "ada@example.com";

/// A Team-plan `acme` owned by `OWNER`.
async fn setup(pool: PgPool) -> (TestApp, String) {
    let app = test_app_with(pool, test_config(&[("OPERATOR_EMAILS", OWNER)])).await;
    let s = app.sign_in(OWNER).await;
    app.create_team(&s, "acme").await;
    let (status, body) = app
        .request(
            Method::PUT,
            "/v1/admin/teams/acme/plan",
            Some(json!({"plan": "team"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (app, s)
}

async fn publish(app: &TestApp, s: &str, team: &str, slug: &str) -> (StatusCode, Value) {
    let files = vec![
        BundleFile {
            path: "curl-runner.yaml".into(),
            text: "name: payments".into(),
        },
        BundleFile {
            path: "charge.yaml".into(),
            text: "request:\n  url: https://api.example.com/charge".into(),
        },
    ];
    let body = json!({
        "hash": bundle_hash(&files),
        "files": files,
        "inputs": [{"name": "API_KEY", "description": "The key", "secret": true,
                    "required": true, "locked": false, "default": null}],
    });
    app.request(
        Method::PUT,
        &format!("/v1/teams/{team}/collections/{slug}/versions"),
        Some(body),
        Some(s),
    )
    .await
}

async fn share(app: &TestApp, s: &str, slug: &str, body: Value) -> Value {
    let (status, body) = app
        .request(
            Method::PATCH,
            &format!("/v1/teams/acme/collections/{slug}/share"),
            Some(body),
            Some(s),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn add_entry(app: &TestApp, s: &str, slug: &str, entry: &str) {
    let (status, body) = app
        .request(
            Method::POST,
            &format!("/v1/teams/acme/collections/{slug}/share/entries"),
            Some(json!({"entry": entry, "notify": false})),
            Some(s),
        )
        .await;
    assert!(status.is_success(), "{status} {body}");
}

fn token_of(link: &str) -> &str {
    link.split("/s/").nth(1).expect("token in link")
}

async fn shared_list(app: &TestApp, s: &str) -> Vec<Value> {
    let (status, body) = app
        .request(Method::GET, "/v1/me/shared", None, Some(s))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body.as_array().expect("array").clone()
}

#[sqlx::test]
async fn info_includes_app_url(pool: PgPool) {
    let app = test_app_with(
        pool,
        test_config(&[("PUBLIC_APP_URL", "https://app.example.com")]),
    )
    .await;
    let (status, body) = app.request(Method::GET, "/v1/info", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["app_url"], "https://app.example.com");
}

#[sqlx::test]
async fn identities_lists_email_and_oauth(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("jane@example.com").await;
    let (status, body) = app
        .request(Method::GET, "/v1/me/identities", None, Some(&s))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["provider"], "email");
    assert_eq!(body[0]["email"], "jane@example.com");

    app.clock.advance(Duration::hours(1));
    app.providers
        .set_identity("583231", "jane@example.com", Some("Jane"));
    let (status, started) = app
        .request(
            Method::POST,
            "/v1/auth/oauth/github/start",
            Some(json!({"client": "web"})),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let url = reqwest::Url::parse(started["url"].as_str().unwrap()).unwrap();
    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .unwrap();
    let (status, signed) = app
        .request(
            Method::POST,
            "/v1/auth/oauth/github/callback",
            Some(json!({"code": "c", "state": state, "binding": started["binding"]})),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{signed}");

    let (status, body) = app
        .request(Method::GET, "/v1/me/identities", None, Some(&s))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{body}");
    assert_eq!(rows[0]["provider"], "email");
    assert_eq!(rows[1]["provider"], "github");
    assert_eq!(rows[1]["email"], "jane@example.com");
    assert!(rows[1]["created_at"].is_string());

    let (status, _) = app
        .request(Method::GET, "/v1/me/identities", None, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn shared_lists_restricted_entry_and_opened_public(pool: PgPool) {
    let (app, ada) = setup(pool).await;
    // `older` is restricted with jane on the list; `newer` is public and opened.
    assert_eq!(
        publish(&app, &ada, "acme", "older").await.0,
        StatusCode::CREATED
    );
    app.clock.advance(Duration::hours(1));
    assert_eq!(
        publish(&app, &ada, "acme", "newer").await.0,
        StatusCode::CREATED
    );
    // A restricted collection jane is not on, and a public one she never opened.
    assert_eq!(
        publish(&app, &ada, "acme", "hidden").await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        publish(&app, &ada, "acme", "unopened").await.0,
        StatusCode::CREATED
    );
    share(&app, &ada, "older", json!({"access": "restricted"})).await;
    add_entry(&app, &ada, "older", "jane@example.com").await;
    share(&app, &ada, "hidden", json!({"access": "restricted"})).await;
    let public = share(&app, &ada, "newer", json!({"access": "public"})).await;
    share(&app, &ada, "unopened", json!({"access": "public"})).await;

    let jane = app.sign_in("jane@example.com").await;
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/v1/shared/{}", token_of(public["link"].as_str().unwrap())),
            None,
            Some(&jane),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let rows = shared_list(&app, &jane).await;
    let slugs: Vec<&str> = rows
        .iter()
        .map(|r| r["collection"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, ["newer", "older"], "{rows:?}");
    assert_eq!(rows[0]["team"], "acme");
    assert_eq!(rows[0]["team_slug"], "acme");
    assert_eq!(rows[1]["team_slug"], "acme");
    assert_eq!(rows[0]["name"], "newer");
    assert_eq!(rows[0]["version"], 1);
    assert_eq!(rows[0]["access"], "public");
    assert_eq!(rows[1]["access"], "restricted");
    assert!(rows[0]["updated_at"].is_string());
    assert_eq!(rows[0]["link"], public["link"]);
    assert!(
        rows[0]["link"]
            .as_str()
            .unwrap()
            .starts_with("http://localhost:3000/s/crsl_")
    );

    let (status, _) = app.request(Method::GET, "/v1/me/shared", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn shared_excludes_own_team_off_and_expired(pool: PgPool) {
    let (app, ada) = setup(pool).await;
    for slug in ["own", "off", "expiring"] {
        assert_eq!(
            publish(&app, &ada, "acme", slug).await.0,
            StatusCode::CREATED
        );
    }
    // Own team: the owner opens their public link and is on its list.
    let own = share(&app, &ada, "own", json!({"access": "public"})).await;
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/v1/shared/{}", token_of(own["link"].as_str().unwrap())),
            None,
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    // Off: the entry stays, the link is dead.
    share(&app, &ada, "off", json!({"access": "restricted"})).await;
    add_entry(&app, &ada, "off", "jane@example.com").await;
    share(&app, &ada, "off", json!({"access": "off"})).await;
    // Expired.
    share(
        &app,
        &ada,
        "expiring",
        json!({"access": "restricted", "expires_in_days": 1}),
    )
    .await;
    add_entry(&app, &ada, "expiring", "jane@example.com").await;

    let jane = app.sign_in("jane@example.com").await;
    assert_eq!(shared_list(&app, &jane).await.len(), 1, "before expiry");
    assert!(shared_list(&app, &ada).await.is_empty());
    app.clock.advance(Duration::days(2));
    assert!(shared_list(&app, &jane).await.is_empty());
}

#[sqlx::test]
async fn shared_domain_entry_matches_case_insensitively(pool: PgPool) {
    let (app, ada) = setup(pool).await;
    assert_eq!(
        publish(&app, &ada, "acme", "payments-api").await.0,
        StatusCode::CREATED
    );
    share(&app, &ada, "payments-api", json!({"access": "restricted"})).await;
    add_entry(&app, &ada, "payments-api", "*@Customer.com").await;
    let jane = app.sign_in("jane@customer.com").await;
    let rows = shared_list(&app, &jane).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["collection"], "payments-api");
    let other = app.sign_in("bob@example.com").await;
    assert!(shared_list(&app, &other).await.is_empty());
}

#[sqlx::test]
async fn version_detail_has_inputs(pool: PgPool) {
    let (app, ada) = setup(pool).await;
    assert_eq!(
        publish(&app, &ada, "acme", "payments-api").await.0,
        StatusCode::CREATED
    );
    let (status, body) = app
        .request(
            Method::GET,
            "/v1/teams/acme/collections/payments-api/versions/1",
            None,
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["number"], 1);
    assert_eq!(body["file_count"], 2);
    assert!(body["hash"].is_string());
    assert!(body["published_at"].is_string());
    assert!(body["publisher"].is_string());
    assert_eq!(body["inputs"][0]["name"], "API_KEY");
    assert_eq!(body["inputs"][0]["secret"], true);
    for bad in ["2", "x"] {
        let (status, _) = app
            .request(
                Method::GET,
                &format!("/v1/teams/acme/collections/payments-api/versions/{bad}"),
                None,
                Some(&ada),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{bad}");
    }
}

#[sqlx::test]
async fn version_detail_other_team_404(pool: PgPool) {
    let (app, ada) = setup(pool).await;
    assert_eq!(
        publish(&app, &ada, "acme", "payments-api").await.0,
        StatusCode::CREATED
    );
    let bob = app.sign_in("bob@example.com").await;
    app.create_team(&bob, "globex").await;
    // Bob reads his own team: no such collection there.
    let (status, _) = app
        .request(
            Method::GET,
            "/v1/teams/globex/collections/payments-api/versions/1",
            None,
            Some(&bob),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Bob reads acme: not a member.
    let (status, _) = app
        .request(
            Method::GET,
            "/v1/teams/acme/collections/payments-api/versions/1",
            None,
            Some(&bob),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
