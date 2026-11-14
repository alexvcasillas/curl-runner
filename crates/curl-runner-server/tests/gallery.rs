#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, test_app_with, test_config};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use serde_json::{Value, json};
use sqlx::PgPool;

const OWNER: &str = "ada@example.com";
const LIST: &str = "/v1/public/collections";

/// `acme` (Team plan) owned by `OWNER`, an operator, so admin calls work.
async fn setup(pool: PgPool) -> (TestApp, String) {
    let config = test_config(&[("OPERATOR_EMAILS", OWNER)]);
    let app = test_app_with(pool, config).await;
    let s = app.sign_in(OWNER).await;
    app.create_team(&s, "acme").await;
    (app, s)
}

async fn publish(app: &TestApp, s: &str, slug: &str, description: Option<&str>) {
    let files = vec![
        BundleFile {
            path: "curl-runner.yaml".into(),
            text: "cloud:\n  collection: x\n".into(),
        },
        BundleFile {
            path: "charges/create.yaml".into(),
            text: "request:\n  name: Create charge\n  method: POST\n  url: https://api.example.com/c\n"
                .into(),
        },
        BundleFile {
            path: "charges/more.yaml".into(),
            text: "requests:\n  - name: List\n    url: https://api.example.com/c\n  - name: Refund\n    method: post\n    url: https://api.example.com/r\n"
                .into(),
        },
    ];
    let body = json!({
        "hash": bundle_hash(&files),
        "files": files,
        "inputs": [{"name": "API_KEY", "description": "Your key", "secret": true,
                    "required": true, "locked": false, "default": null}],
        "name": format!("Name of {slug}"),
        "description": description,
    });
    let (status, res) = app
        .request(
            Method::PUT,
            &format!("/v1/teams/acme/collections/{slug}/versions"),
            Some(body),
            Some(s),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{res}");
}

async fn share(app: &TestApp, s: &str, slug: &str, body: Value) -> (StatusCode, Value) {
    app.request(
        Method::PATCH,
        &format!("/v1/teams/acme/collections/{slug}/share"),
        Some(body),
        Some(s),
    )
    .await
}

async fn list(app: &TestApp, query: &str) -> Value {
    let (status, body) = app
        .request(Method::GET, &format!("{LIST}{query}"), None, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn slugs(body: &Value) -> Vec<String> {
    body["collections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["slug"].as_str().unwrap().to_string())
        .collect()
}

#[sqlx::test]
async fn listing_needs_public_and_a_description(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, "bare", None).await;
    publish(&app, &s, "pay", Some("Charges and refunds")).await;
    // Off: refused.
    let (status, body) = share(&app, &s, "pay", json!({"listed": true})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    // Public without a description: refused, saying how.
    share(&app, &s, "bare", json!({"access": "public"})).await;
    let (status, body) = share(&app, &s, "bare", json!({"listed": true})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap().contains("description"));
    // Public in the same call, with a description: listed.
    let (status, body) = share(&app, &s, "pay", json!({"access": "public", "listed": true})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["listed"], true);
    assert_eq!(body["description"], "Charges and refunds");
    assert_eq!(slugs(&list(&app, "").await), ["pay"]);
    // Off takes it out.
    let (_, body) = share(&app, &s, "pay", json!({"access": "off"})).await;
    assert_eq!(body["listed"], false);
    assert!(slugs(&list(&app, "").await).is_empty());
}

#[sqlx::test]
async fn the_list_reads_like_the_gallery(pool: PgPool) {
    let (app, s) = setup(pool).await;
    for slug in ["pay", "ship", "hooks"] {
        publish(&app, &s, slug, Some(&format!("All about {slug}"))).await;
        share(&app, &s, slug, json!({"access": "public", "listed": true})).await;
        app.clock.advance(chrono::Duration::minutes(1));
    }
    let body = list(&app, "").await;
    // Newest listed first.
    assert_eq!(slugs(&body), ["hooks", "ship", "pay"]);
    assert_eq!(
        (body["total"].as_i64(), body["teams"].as_i64()),
        (Some(3), Some(1))
    );
    let card = &body["collections"][0];
    assert_eq!(card["team"]["slug"], "acme");
    assert_eq!(card["name"], "Name of hooks");
    assert_eq!(card["requests"], 3);
    assert_eq!(card["version"], 1);
    assert!(card["link"].as_str().unwrap().contains("/s/crsl_"));
    // Search: name, description and team.
    assert_eq!(slugs(&list(&app, "?q=about%20ship").await), ["ship"]);
    assert_eq!(list(&app, "?q=acme").await["total"], 3);
    assert_eq!(list(&app, "?q=nothing").await["total"], 0);
    // Wildcards are literal.
    assert_eq!(list(&app, "?q=%25").await["total"], 0);
    assert_eq!(list(&app, "?team=acme").await["total"], 3);
    assert_eq!(list(&app, "?team=globex").await["total"], 0);
}

#[sqlx::test]
async fn pages_follow_the_cursor(pool: PgPool) {
    let (app, s) = setup(pool).await;
    for i in 0..26 {
        let slug = format!("c{i:02}");
        publish(&app, &s, &slug, Some("Desc")).await;
        share(&app, &s, &slug, json!({"access": "public", "listed": true})).await;
        app.clock.advance(chrono::Duration::minutes(1));
    }
    let first = list(&app, "").await;
    assert_eq!(first["collections"].as_array().unwrap().len(), 24);
    let cursor = first["next_cursor"].as_str().unwrap().to_string();
    let second = list(&app, &format!("?cursor={cursor}")).await;
    assert_eq!(slugs(&second), ["c01", "c00"]);
    assert_eq!(second["next_cursor"], Value::Null);
}

#[sqlx::test]
async fn detail_has_files_requests_and_inputs(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, "pay", Some("Charges")).await;
    let (status, _) = app
        .request(Method::GET, &format!("{LIST}/acme/pay"), None, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    share(&app, &s, "pay", json!({"access": "public", "listed": true})).await;
    let (status, body) = app
        .request(Method::GET, &format!("{LIST}/acme/pay"), None, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["team"]["name"], "acme");
    let files = body["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);
    let more = files
        .iter()
        .find(|f| f["path"] == "charges/more.yaml")
        .unwrap();
    assert_eq!(
        more["requests"],
        json!([{"method": "GET", "name": "List"}, {"method": "POST", "name": "Refund"}])
    );
    let config = files
        .iter()
        .find(|f| f["path"] == "curl-runner.yaml")
        .unwrap();
    assert_eq!(config["requests"], json!([]));
    assert_eq!(
        body["inputs"],
        json!([{"name": "API_KEY", "description": "Your key", "secret": true}])
    );
}

#[sqlx::test]
async fn takedowns_suspensions_and_unlisting_hide_it(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, "pay", Some("Charges")).await;
    share(&app, &s, "pay", json!({"access": "public", "listed": true})).await;
    // Admin filter and unlist.
    let (status, body) = app
        .request(
            Method::GET,
            "/v1/admin/collections?filter=listed",
            None,
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.to_string().contains("\"listed_at\""));
    let unlist = "/v1/admin/collections/acme/pay/unlist";
    let (status, body) = app
        .request(
            Method::POST,
            unlist,
            Some(json!({"reason": "spam"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(list(&app, "").await["total"], 0);
    let (status, _) = app
        .request(
            Method::POST,
            unlist,
            Some(json!({"reason": "again"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // The team can list it again; a takedown hides it.
    share(&app, &s, "pay", json!({"listed": true})).await;
    assert_eq!(list(&app, "").await["total"], 1);
    app.request(
        Method::POST,
        "/v1/admin/collections/acme/pay/disable-link",
        Some(json!({"reason": "abuse"})),
        Some(&s),
    )
    .await;
    assert_eq!(list(&app, "").await["total"], 0);
    app.request(
        Method::POST,
        "/v1/admin/collections/acme/pay/enable-link",
        None,
        Some(&s),
    )
    .await;
    assert_eq!(list(&app, "").await["total"], 1);
    // A suspended team's collections are hidden.
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/admin/teams/acme/suspend",
            Some(json!({"reason": "x", "notify": false})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(list(&app, "").await["total"], 0);
}

#[sqlx::test]
async fn the_gallery_is_rate_limited_per_ip(pool: PgPool) {
    let (app, _) = setup(pool).await;
    let mut last = StatusCode::OK;
    for _ in 0..241 {
        let (status, _) = app.request(Method::GET, LIST, None, None).await;
        last = status;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
async fn the_description_is_set_in_share(pool: PgPool) {
    let (app, s) = setup(pool).await;
    publish(&app, &s, "bare", None).await;
    // A description and the listing in one call, trimmed.
    let (status, body) = share(
        &app,
        &s,
        "bare",
        json!({"access": "public", "description": "  Charges  ", "listed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["description"], "Charges");
    assert_eq!(body["listed"], true);
    assert_eq!(slugs(&list(&app, "").await), ["bare"]);
    // Too long: refused, nothing changes.
    let long = "x".repeat(501);
    let (status, _) = share(&app, &s, "bare", json!({"description": long})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Cleared: out of the gallery.
    let (_, body) = share(&app, &s, "bare", json!({"description": " "})).await;
    assert_eq!(body["description"], Value::Null);
    assert_eq!(body["listed"], false);
    assert!(slugs(&list(&app, "").await).is_empty());
}
