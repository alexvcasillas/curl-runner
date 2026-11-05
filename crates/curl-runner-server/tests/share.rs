#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{TestApp, test_app_with, test_config};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use serde_json::{Value, json};
use sqlx::PgPool;

const OWNER: &str = "ada@example.com";
const SHARE: &str = "/v1/teams/acme/collections/payments-api/share";

/// A Team-plan `acme` owned by `OWNER`, with `payments-api` published.
async fn setup(pool: PgPool) -> (TestApp, String) {
    let config = test_config(&[("OPERATOR_EMAILS", OWNER)]);
    let app = test_app_with(pool, config).await;
    let s = app.sign_in(OWNER).await;
    app.create_team(&s, "acme").await;
    set_plan(&app, &s, "team").await;
    let files = [
        ("curl-runner.yaml", "name: payments"),
        (
            "charge.yaml",
            "request:\n  url: https://api.example.com/charge",
        ),
        (
            "refund.yaml",
            "request:\n  url: https://api.example.com/refund",
        ),
    ];
    let (status, body) = publish(&app, &s, &files).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    (app, s)
}

async fn set_plan(app: &TestApp, s: &str, plan: &str) {
    let (status, body) = app
        .request(
            Method::PUT,
            "/v1/admin/teams/acme/plan",
            Some(json!({"plan": plan})),
            Some(s),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

async fn publish(app: &TestApp, s: &str, files: &[(&str, &str)]) -> (StatusCode, Value) {
    let files: Vec<BundleFile> = files
        .iter()
        .map(|(p, t)| BundleFile {
            path: (*p).into(),
            text: (*t).into(),
        })
        .collect();
    let body = json!({
        "hash": bundle_hash(&files),
        "files": files,
        "inputs": [{"name": "API_KEY", "description": null, "secret": true,
                    "required": true, "locked": false, "default": null}],
    });
    app.request(
        Method::PUT,
        "/v1/teams/acme/collections/payments-api/versions",
        Some(body),
        Some(s),
    )
    .await
}

async fn patch(app: &TestApp, s: &str, body: Value) -> (StatusCode, Value) {
    app.request(Method::PATCH, SHARE, Some(body), Some(s)).await
}

/// The share token from the owner's share link.
async fn token(app: &TestApp, s: &str) -> String {
    let (status, body) = app.request(Method::GET, SHARE, None, Some(s)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let link = body["link"].as_str().unwrap();
    link.split("/s/").nth(1).expect("token in link").to_owned()
}

async fn fetch(app: &TestApp, path: &str, bearer: Option<&str>) -> (StatusCode, Value) {
    app.request(Method::GET, path, None, bearer).await
}

async fn add_entry(app: &TestApp, s: &str, entry: &str, notify: bool) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        &format!("{SHARE}/entries"),
        Some(json!({"entry": entry, "notify": notify})),
        Some(s),
    )
    .await
}

#[sqlx::test]
async fn off_collection_link_404(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let t = token(&app, &s).await;
    for suffix in ["", "/snapshot", "/versions/latest"] {
        let (status, body) = fetch(&app, &format!("/v1/shared/{t}{suffix}"), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{suffix}");
        assert_eq!(body["error"], "not_found");
    }
    let (status, _) = fetch(&app, "/v1/shared/crsl_nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn public_link_no_auth_snapshot(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let (status, share) = patch(&app, &s, json!({"access": "public"})).await;
    assert_eq!(status, StatusCode::OK, "{share}");
    assert_eq!(share["access"], "public");
    let t = token(&app, &s).await;

    let (status, info) = fetch(&app, &format!("/v1/shared/{t}"), None).await;
    assert_eq!(status, StatusCode::OK, "{info}");
    assert_eq!(info["collection"], "payments-api");
    assert_eq!(info["team"], "acme");
    assert_eq!(info["team_slug"], "acme");
    assert_eq!(info["version"], 1);
    assert_eq!(info["request_count"], 2, "config files are not requests");
    assert_eq!(info["access"], "public");

    let (status, snap) = fetch(&app, &format!("/v1/shared/{t}/snapshot"), None).await;
    assert_eq!(status, StatusCode::OK, "{snap}");
    assert_eq!(snap["inputs"][0]["name"], "API_KEY");

    let (status, latest) = fetch(&app, &format!("/v1/shared/{t}/versions/latest"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(latest["version"], 1);
    assert_eq!(latest["hash"].as_str().unwrap().len(), 64);
}

#[sqlx::test]
async fn snapshot_excludes_nothing_published(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "public"})).await;
    let t = token(&app, &s).await;
    let (_, snap) = fetch(&app, &format!("/v1/shared/{t}/snapshot"), None).await;
    let (_, files) = fetch(
        &app,
        "/v1/teams/acme/collections/payments-api/versions/1/files",
        Some(&s),
    )
    .await;
    assert_eq!(snap["files"], files);
    assert_eq!(snap["files"].as_array().unwrap().len(), 3);
}

#[sqlx::test]
async fn restricted_needs_auth_401(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let t = token(&app, &s).await;
    for suffix in ["", "/snapshot", "/versions/latest"] {
        let (status, body) = fetch(&app, &format!("/v1/shared/{t}{suffix}"), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{suffix}");
        assert_eq!(body["error"], "unauthorized");
    }
    // A present but invalid session is a 401, never anonymous.
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some("crs_bogus")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn invalid_session_on_public_link_is_401(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "public"})).await;
    let t = token(&app, &s).await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some("crs_bogus")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn restricted_not_listed_403_names_team(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let t = token(&app, &s).await;
    let stranger = app.sign_in("eve@example.com").await;
    let (status, body) = fetch(&app, &format!("/v1/shared/{t}/snapshot"), Some(&stranger)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "forbidden");
    assert_eq!(
        body["message"],
        "Your account isn't on the list for this collection. Ask acme for access."
    );
}

#[sqlx::test]
async fn restricted_email_case_insensitive(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let (status, entry) = add_entry(&app, &s, "Bob@Client.COM", false).await;
    assert_eq!(status, StatusCode::CREATED, "{entry}");
    assert_eq!(entry["entry"], "bob@client.com");
    assert_eq!(entry["kind"], "email");
    let t = token(&app, &s).await;
    let bob = app.sign_in("bob@client.com").await;
    let (status, body) = fetch(&app, &format!("/v1/shared/{t}/snapshot"), Some(&bob)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["info"]["access"], "restricted");
}

#[sqlx::test]
async fn restricted_domain_entry_matches(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let (status, entry) = add_entry(&app, &s, "*@Client.com", false).await;
    assert_eq!(status, StatusCode::CREATED, "{entry}");
    assert_eq!(entry["kind"], "domain");
    let t = token(&app, &s).await;
    let carol = app.sign_in("carol@client.com").await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some(&carol)).await;
    assert_eq!(status, StatusCode::OK);
    // A subdomain is a different domain.
    let dan = app.sign_in("dan@eu.client.com").await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some(&dan)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn email_entry_wins_over_domain_entry(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    add_entry(&app, &s, "*@client.com", false).await;
    add_entry(&app, &s, "bob@client.com", false).await;
    let t = token(&app, &s).await;
    let bob = app.sign_in("bob@client.com").await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some(&bob)).await;
    assert_eq!(status, StatusCode::OK);
    let entry: Option<String> = sqlx::query_scalar("SELECT entry::text FROM share_opens")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(entry.as_deref(), Some("bob@client.com"));
}

#[sqlx::test]
async fn team_member_opens_own_restricted_link(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let t = token(&app, &s).await;
    // The owner is on no access list, but is a member of the owning team.
    for suffix in ["", "/snapshot", "/versions/latest"] {
        let (status, body) = fetch(&app, &format!("/v1/shared/{t}{suffix}"), Some(&s)).await;
        assert_eq!(status, StatusCode::OK, "{suffix}: {body}");
    }
    // A member of another team is still checked against the list.
    let eve = app.sign_in("eve@example.com").await;
    app.create_team(&eve, "other").await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some(&eve)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn reset_link_kills_old(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "public"})).await;
    let old = token(&app, &s).await;
    let (status, _) = patch(&app, &s, json!({"reset_link": true})).await;
    assert_eq!(status, StatusCode::OK);
    let new = token(&app, &s).await;
    assert_ne!(old, new);
    let (status, _) = fetch(&app, &format!("/v1/shared/{old}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = fetch(&app, &format!("/v1/shared/{new}"), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn expired_link_404(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let (status, share) = patch(&app, &s, json!({"access": "public", "expires_in_days": 7})).await;
    assert_eq!(status, StatusCode::OK, "{share}");
    assert!(share["expires_at"].is_string());
    let t = token(&app, &s).await;
    app.clock.advance(Duration::days(6));
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), None).await;
    assert_eq!(status, StatusCode::OK);
    app.clock.advance(Duration::days(2));
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // null clears the expiry; absent leaves it alone.
    let (_, share) = patch(&app, &s, json!({"expires_in_days": null})).await;
    assert_eq!(share["expires_at"], Value::Null);
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn customer_invite_queues_mail(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let t = token(&app, &s).await;
    add_entry(&app, &s, "bob@client.com", true).await;
    add_entry(&app, &s, "*@other.com", true).await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT to_addr, text_body FROM outbox_emails WHERE subject LIKE '%shared%'",
    )
    .fetch_all(&app.state.db)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "only the email entry is mailed");
    assert_eq!(rows[0].0, "bob@client.com");
    assert!(rows[0].1.contains(&format!("/s/{t}")));
    // No notify, no mail.
    add_entry(&app, &s, "quiet@client.com", false).await;
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox_emails WHERE subject LIKE '%shared%'")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test]
async fn entries_require_restricted_409(pool: PgPool) {
    let (app, s) = setup(pool).await;
    for access in ["off", "public"] {
        patch(&app, &s, json!({"access": access})).await;
        let (status, body) = add_entry(&app, &s, "bob@client.com", false).await;
        assert_eq!(status, StatusCode::CONFLICT, "{access}");
        assert_eq!(body["error"], "conflict");
    }
}

#[sqlx::test]
async fn invalid_entries_are_rejected(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    for bad in ["bob", "@client.com", "a@b@c.com", "*@client", "bob@"] {
        let (status, body) = add_entry(&app, &s, bad, false).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body}");
        assert_eq!(body["error"], "invalid_request");
    }
}

#[sqlx::test]
async fn share_view_lists_entries_and_opens(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    add_entry(&app, &s, "bob@client.com", false).await;
    add_entry(&app, &s, "*@other.com", false).await;
    let t = token(&app, &s).await;
    let bob = app.sign_in("bob@client.com").await;
    fetch(&app, &format!("/v1/shared/{t}"), Some(&bob)).await;
    let (_, share) = app.request(Method::GET, SHARE, None, Some(&s)).await;
    let entries = share["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    let find = |name: &str| entries.iter().find(|e| e["entry"] == name).unwrap();
    assert_eq!(find("bob@client.com")["opened"], true);
    assert_eq!(find("*@other.com")["opened"], false);

    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("{SHARE}/entries/Bob@Client.com"),
            None,
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some(&bob)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("{SHARE}/entries/bob@client.com"),
            None,
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn opens_are_recorded_once_per_user_per_day_and_never_anonymously(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "public"})).await;
    let t = token(&app, &s).await;
    let count = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM share_opens")
            .fetch_one(&app.state.db)
            .await
            .unwrap()
    };
    for _ in 0..3 {
        fetch(&app, &format!("/v1/shared/{t}"), None).await;
    }
    assert_eq!(count().await, 0, "anonymous opens are skipped");
    let bob = app.sign_in("bob@client.com").await;
    for suffix in ["", "/snapshot", ""] {
        fetch(&app, &format!("/v1/shared/{t}{suffix}"), Some(&bob)).await;
    }
    assert_eq!(count().await, 1);
    app.clock.advance(Duration::days(1));
    fetch(&app, &format!("/v1/shared/{t}"), Some(&bob)).await;
    assert_eq!(count().await, 2);
}

#[sqlx::test]
async fn shared_endpoints_are_rate_limited_per_ip(pool: PgPool) {
    let (app, _) = setup(pool).await;
    for _ in 0..120 {
        let (status, _) = fetch(&app, "/v1/shared/crsl_nope", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, body) = fetch(&app, "/v1/shared/crsl_nope/snapshot", None).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "rate_limited");
}

#[sqlx::test]
async fn free_team_cannot_set_restricted(pool: PgPool) {
    let (app, s) = setup(pool).await;
    set_plan(&app, &s, "free").await;
    let (status, body) = patch(&app, &s, json!({"access": "restricted"})).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(body["error"], "plan_limit");
    let (status, _) = patch(&app, &s, json!({"access": "public"})).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn downgraded_team_still_serves_restricted(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    add_entry(&app, &s, "bob@client.com", false).await;
    let t = token(&app, &s).await;
    set_plan(&app, &s, "free").await;

    let bob = app.sign_in("bob@client.com").await;
    let (status, body) = fetch(&app, &format!("/v1/shared/{t}/snapshot"), Some(&bob)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = add_entry(&app, &s, "new@client.com", false).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(body["error"], "plan_limit");
}

#[sqlx::test]
async fn non_members_get_404_on_team_endpoints(pool: PgPool) {
    let (app, _) = setup(pool).await;
    let eve = app.sign_in("eve@example.com").await;
    let (status, _) = app.request(Method::GET, SHARE, None, Some(&eve)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = patch(&app, &eve, json!({"access": "public"})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = add_entry(&app, &eve, "x@y.com", false).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn switching_public_to_restricted_locks_out_anonymous(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "public"})).await;
    let t = token(&app, &s).await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), None).await;
    assert_eq!(status, StatusCode::OK);
    patch(&app, &s, json!({"access": "restricted"})).await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn lookalike_domain_is_forbidden(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    add_entry(&app, &s, "*@client.com", false).await;
    let t = token(&app, &s).await;
    let x = app.sign_in("x@evilclient.com").await;
    let (status, _) = fetch(&app, &format!("/v1/shared/{t}"), Some(&x)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn corrupted_link_token_is_503_until_reset(pool: PgPool) {
    let (app, s) = setup(pool).await;
    sqlx::query("UPDATE collections SET link_token_enc = 'garbage'")
        .execute(&app.state.db)
        .await
        .unwrap();
    let (status, body) = app.request(Method::GET, SHARE, None, Some(&s)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let (status, body) = patch(&app, &s, json!({"reset_link": true})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = app.request(Method::GET, SHARE, None, Some(&s)).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn re_adding_an_entry_does_not_resend_mail(pool: PgPool) {
    let (app, s) = setup(pool).await;
    patch(&app, &s, json!({"access": "restricted"})).await;
    let (_, first) = add_entry(&app, &s, "bob@client.com", true).await;
    app.clock.advance(Duration::hours(1));
    let (status, again) = add_entry(&app, &s, "Bob@client.com", true).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(again["invited_at"], first["invited_at"]);
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox_emails WHERE subject LIKE '%shared%'")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test]
async fn patch_share_missing_collection_404_and_same_access_is_not_a_change(pool: PgPool) {
    let (app, s) = setup(pool).await;
    let (status, _) = app
        .request(
            Method::PATCH,
            "/v1/teams/acme/collections/nope/share",
            Some(json!({"access": "public"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    patch(&app, &s, json!({"access": "restricted"})).await;
    set_plan(&app, &s, "free").await;
    let (status, body) = patch(&app, &s, json!({"access": "restricted"})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
