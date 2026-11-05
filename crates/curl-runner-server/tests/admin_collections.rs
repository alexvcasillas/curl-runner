#![cfg(feature = "db-tests")]

mod common;

use axum::http::StatusCode;
use chrono::Duration;
use common::{TestApp, test_app_with, test_config};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use curl_runner_server::clock::Clock;
use serde_json::{Value, json};
use sqlx::PgPool;

const ROOT: &str = "root@example.com";
const ADA: &str = "ada@example.com";
const SHARE: &str = "/v1/teams/acme/collections/payments-api/share";
const DISABLE: &str = "/v1/admin/collections/acme/payments-api/disable-link";
const ENABLE: &str = "/v1/admin/collections/acme/payments-api/enable-link";

struct Ctx {
    app: TestApp,
    root: String,
    ada: String,
}

async fn setup(pool: PgPool) -> Ctx {
    let app = test_app_with(pool, test_config(&[("OPERATOR_EMAILS", ROOT)])).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    Ctx { app, root, ada }
}

async fn req(
    c: &Ctx,
    method: &str,
    path: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    c.app
        .request(method.parse().unwrap(), path, body, bearer)
        .await
}

/// Publish `slug` of `team` with `charge` as its one request file.
async fn publish_to(c: &Ctx, team: &str, slug: &str, charge: &str) {
    let files = vec![
        BundleFile {
            path: "curl-runner.yaml".into(),
            text: "name: payments".into(),
        },
        BundleFile {
            path: "charge.yaml".into(),
            text: charge.into(),
        },
    ];
    let body = json!({"hash": bundle_hash(&files), "files": files, "inputs": []});
    let (status, out) = req(
        c,
        "PUT",
        &format!("/v1/teams/{team}/collections/{slug}/versions"),
        Some(body),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{out}");
}

async fn set_access(c: &Ctx, team: &str, slug: &str, access: &str) -> String {
    let (status, body) = req(
        c,
        "PATCH",
        &format!("/v1/teams/{team}/collections/{slug}/share"),
        Some(json!({"access": access})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["link"]
        .as_str()
        .unwrap()
        .split("/s/")
        .nth(1)
        .unwrap()
        .to_owned()
}

/// Publish `payments-api`, make it public and return its share token.
async fn shared(c: &Ctx) -> String {
    publish_to(
        c,
        "acme",
        "payments-api",
        "request:\n  url: https://x.example.com/1",
    )
    .await;
    set_access(c, "acme", "payments-api", "public").await
}

async fn disable(c: &Ctx) -> (StatusCode, Value) {
    req(
        c,
        "POST",
        DISABLE,
        Some(json!({"reason": "Spam"})),
        Some(&c.root),
    )
    .await
}

async fn mails(c: &Ctx) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT to_addr FROM outbox_emails WHERE subject LIKE '%link%' ORDER BY to_addr",
    )
    .fetch_all(&c.app.state.db)
    .await
    .unwrap()
}

async fn actions(c: &Ctx) -> Vec<String> {
    sqlx::query_scalar("SELECT action FROM admin_events ORDER BY at, action")
        .fetch_all(&c.app.state.db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn disabled_link_404_for_everyone(pool: PgPool) {
    let c = setup(pool).await;
    let t = shared(&c).await;
    let bob = c.app.sign_in("bob@example.com").await;
    let (status, _) = req(&c, "GET", &format!("/v1/shared/{t}"), None, Some(&bob)).await;
    assert_eq!(status, StatusCode::OK);
    let (_, mine) = req(&c, "GET", "/v1/me/shared", None, Some(&bob)).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);

    let (status, body) = disable(&c).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["link_disabled"], true);

    for suffix in ["", "/snapshot", "/versions/latest"] {
        for bearer in [None, Some(bob.as_str()), Some(c.ada.as_str())] {
            let (status, body) =
                req(&c, "GET", &format!("/v1/shared/{t}{suffix}"), None, bearer).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{suffix} {bearer:?}");
            assert_eq!(body["error"], "not_found");
        }
    }
    // Signed-in users lose it from their list, team members from theirs too.
    let (_, mine) = req(&c, "GET", "/v1/me/shared", None, Some(&bob)).await;
    assert_eq!(mine, json!([]));
    let (_, mine) = req(&c, "GET", "/v1/me/shared", None, Some(&c.ada)).await;
    assert_eq!(mine, json!([]));

    // The team still reads and sees why the link is dead.
    let (status, share) = req(&c, "GET", SHARE, None, Some(&c.ada)).await;
    assert_eq!(status, StatusCode::OK, "{share}");
    assert_eq!(share["link_disabled"], true);
    let (status, _) = req(
        &c,
        "GET",
        "/v1/teams/acme/collections/payments-api/versions",
        None,
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(mails(&c).await, [ADA]);
    assert_eq!(actions(&c).await, ["collection.link.disable"]);
    // Twice is a conflict, and logs nothing more.
    let (status, _) = disable(&c).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(actions(&c).await.len(), 1);
    let (status, _) = req(&c, "POST", ENABLE, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = req(&c, "POST", ENABLE, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn team_cannot_reset_disabled_link(pool: PgPool) {
    let c = setup(pool).await;
    let t = shared(&c).await;
    disable(&c).await;
    let (status, body) = req(
        &c,
        "PATCH",
        SHARE,
        Some(json!({"reset_link": true})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "link_disabled");
    assert_eq!(
        body["message"],
        "This link was turned off by an admin. Contact support."
    );
    // Nothing was applied, not even the other fields sent with it.
    let (status, body) = req(
        &c,
        "PATCH",
        SHARE,
        Some(json!({"reset_link": true, "access": "off"})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let access: String =
        sqlx::query_scalar("SELECT access FROM collections WHERE slug = 'payments-api'")
            .fetch_one(&c.app.state.db)
            .await
            .unwrap();
    assert_eq!(access, "public");
    // Access mode and expiry stay the team's to change.
    let (status, body) = req(
        &c,
        "PATCH",
        SHARE,
        Some(json!({"access": "off", "expires_in_days": 5})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["link_disabled"], true);
    assert!(body["link"].as_str().unwrap().ends_with(&t));
}

#[sqlx::test]
async fn enable_restores(pool: PgPool) {
    let c = setup(pool).await;
    let t = shared(&c).await;
    disable(&c).await;
    let (status, body) = req(&c, "POST", ENABLE, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["link_disabled"], false);
    let (status, _) = req(&c, "GET", &format!("/v1/shared/{t}"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, share) = req(&c, "GET", SHARE, None, Some(&c.ada)).await;
    assert_eq!(share["link_disabled"], false);
    let (status, body) = req(
        &c,
        "PATCH",
        SHARE,
        Some(json!({"reset_link": true})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        actions(&c).await,
        ["collection.link.disable", "collection.link.enable"]
    );
    // An admin without `teams` cannot, and an unknown collection is a 404.
    let rosa = c.app.sign_in("rosa@example.com").await;
    sqlx::query(
        "INSERT INTO admins (user_id, permissions, added_at) \
         SELECT id, '{users}', now() FROM users WHERE email = 'rosa@example.com'",
    )
    .execute(&c.app.state.db)
    .await
    .unwrap();
    let (status, _) = req(
        &c,
        "POST",
        DISABLE,
        Some(json!({"reason": "x"})),
        Some(&rosa),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = req(
        &c,
        "POST",
        "/v1/admin/collections/acme/nope/disable-link",
        Some(json!({})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn opens_30d_counts_share_opens(pool: PgPool) {
    let c = setup(pool).await;
    shared(&c).await;
    let now = c.app.clock.now();
    for days in [1, 10, 29, 31, 90] {
        sqlx::query(
            "INSERT INTO share_opens (collection_id, opened_at) \
             SELECT id, $1 FROM collections",
        )
        .bind(now - Duration::days(days))
        .execute(&c.app.state.db)
        .await
        .unwrap();
    }
    let (status, body) = req(&c, "GET", "/v1/admin/collections", None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["items"][0]["opens_30d"], 3);
    // The window moves with the injectable clock.
    c.app.clock.advance(Duration::days(5));
    let (_, body) = req(&c, "GET", "/v1/admin/collections", None, Some(&c.root)).await;
    assert_eq!(body["items"][0]["opens_30d"], 2);
}

#[sqlx::test]
async fn stats_dedup_bytes(pool: PgPool) {
    let c = setup(pool).await;
    let (status, body) = req(
        &c,
        "GET",
        "/v1/admin/collections/stats",
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({"count": 0, "bytes_raw": 0, "bytes_stored": 0}));
    // Two collections with the same two files, and a second version of one
    // that changes only the request.
    publish_to(
        &c,
        "acme",
        "one",
        "request:\n  url: https://x.example.com/1",
    )
    .await;
    publish_to(
        &c,
        "acme",
        "two",
        "request:\n  url: https://x.example.com/1",
    )
    .await;
    publish_to(
        &c,
        "acme",
        "one",
        "request:\n  url: https://x.example.com/2",
    )
    .await;
    let cfg = "name: payments".len() as i64;
    let a = "request:\n  url: https://x.example.com/1".len() as i64;
    let b = "request:\n  url: https://x.example.com/2".len() as i64;
    let (_, body) = req(
        &c,
        "GET",
        "/v1/admin/collections/stats",
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(body["count"], 2);
    // Three versions of two files each.
    assert_eq!(body["bytes_raw"], 3 * cfg + a + a + b);
    // Distinct content: one config, and the two requests.
    assert_eq!(body["bytes_stored"], cfg + a + b);
}

#[sqlx::test]
async fn list_filters_search_and_csv(pool: PgPool) {
    let c = setup(pool).await;
    c.app.create_team(&c.ada, "beta").await;
    publish_to(
        &c,
        "acme",
        "payments-api",
        "request:\n  url: https://x.example.com/1",
    )
    .await;
    publish_to(
        &c,
        "acme",
        "cmd",
        "request:\n  url: https://x.example.com/1",
    )
    .await;
    publish_to(
        &c,
        "beta",
        "billing",
        "request:\n  url: https://x.example.com/1",
    )
    .await;
    set_access(&c, "acme", "payments-api", "public").await;
    sqlx::query("UPDATE collections SET name = '=1+1' WHERE slug = 'cmd'")
        .execute(&c.app.state.db)
        .await
        .unwrap();
    sqlx::query("UPDATE teams SET plan = 'team' WHERE slug = 'beta'")
        .execute(&c.app.state.db)
        .await
        .unwrap();
    set_access(&c, "beta", "billing", "restricted").await;
    disable(&c).await;

    let list = |q: &'static str| {
        let c = &c;
        async move {
            let (status, body) = req(
                c,
                "GET",
                &format!("/v1/admin/collections{q}"),
                None,
                Some(&c.root),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{body}");
            body
        }
    };
    let slugs = |b: &Value| -> Vec<String> {
        let mut v: Vec<String> = b["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| format!("{}/{}", r["team"], r["slug"]).replace('"', ""))
            .collect();
        v.sort();
        v
    };
    let all = list("").await;
    assert_eq!(
        slugs(&all),
        ["acme/cmd", "acme/payments-api", "beta/billing"]
    );
    let row = all["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["slug"] == "payments-api")
        .unwrap();
    let size = "name: payments".len() + "request:\n  url: https://x.example.com/1".len();
    assert_eq!(row["name"], "payments-api");
    assert_eq!(row["source"], "git");
    assert_eq!(row["access"], "public");
    assert_eq!(row["versions"], 1);
    assert_eq!(row["size_bytes"], size);
    assert_eq!(row["opens_30d"], 0);
    assert_eq!(row["link_disabled"], true);
    assert!(row["last_publish"].is_string());
    assert_eq!(slugs(&list("?filter=public").await), ["acme/payments-api"]);
    assert_eq!(slugs(&list("?filter=restricted").await), ["beta/billing"]);
    assert_eq!(
        slugs(&list("?filter=disabled").await),
        ["acme/payments-api"]
    );
    assert_eq!(slugs(&list("?q=BILL").await), ["beta/billing"]);
    assert_eq!(
        slugs(&list("?q=beta").await),
        ["beta/billing"],
        "team search"
    );
    assert_eq!(slugs(&list("?q=%25").await), Vec::<String>::new());
    // Paging.
    let page = list("?limit=2").await;
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    let cursor = page["next_cursor"].as_str().unwrap().to_owned();
    let (_, rest) = req(
        &c,
        "GET",
        &format!("/v1/admin/collections?limit=2&cursor={cursor}"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(rest["items"].as_array().unwrap().len(), 1);
    assert_eq!(rest["next_cursor"], Value::Null);
    let (status, _) = req(
        &c,
        "GET",
        "/v1/admin/collections?filter=nope",
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // CSV: header, a row per collection, and the formula cell is neutralised.
    let (status, csv) = req(
        &c,
        "GET",
        "/v1/admin/collections.csv?filter=all",
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let csv = csv.as_str().unwrap();
    assert!(
        csv.starts_with("team,slug,name,source,access,versions,size_bytes,opens_30d,last_publish,link_disabled\r\n"),
        "{csv}"
    );
    assert_eq!(csv.lines().count(), 4, "{csv}");
    assert!(csv.contains("acme,cmd,'=1+1,"), "{csv}");
}
