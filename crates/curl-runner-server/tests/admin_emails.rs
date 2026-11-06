#![cfg(feature = "db-tests")]

mod common;

use std::collections::HashSet;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use common::{TestApp, test_app_with, test_config};
use curl_runner_server::clock::Clock;
use curl_runner_server::mail::unsubscribe;
use curl_runner_server::mail::worker::{expand_broadcast, fan_out_due};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const ROOT: &str = "root@example.com";
const ROSA: &str = "rosa@example.com";
const SECRET: &str = "test-secret-key-test-secret-key-0123";

struct Ctx {
    app: TestApp,
    root: String,
}

async fn setup(pool: PgPool) -> Ctx {
    let app = test_app_with(pool, test_config(&[("OPERATOR_EMAILS", ROOT)])).await;
    let root = app.sign_in(ROOT).await;
    Ctx { app, root }
}

impl Ctx {
    fn db(&self) -> &PgPool {
        &self.app.state.db
    }

    fn now(&self) -> DateTime<Utc> {
        self.app.clock.now()
    }

    async fn req(&self, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.req_as(method, path, body, &self.root).await
    }

    async fn req_as(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        bearer: &str,
    ) -> (StatusCode, Value) {
        self.app
            .request(method.parse().unwrap(), path, body, Some(bearer))
            .await
    }

    async fn user(&self, email: &str, name: Option<&str>) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO users (email, name, created_at) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(email)
        .bind(name)
        .bind(self.now())
        .fetch_one(self.db())
        .await
        .unwrap()
    }

    async fn team(&self, slug: &str, name: &str) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO teams (slug, name, plan, created_at) VALUES ($1, $2, 'free', $3) RETURNING id",
        )
        .bind(slug)
        .bind(name)
        .bind(self.now())
        .fetch_one(self.db())
        .await
        .unwrap()
    }

    /// `user` joins `team` `days_ago` days before now.
    async fn join(&self, team: Uuid, user: Uuid, role: &str, days_ago: i64) {
        sqlx::query(
            "INSERT INTO memberships (team_id, user_id, role, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(team)
        .bind(user)
        .bind(role)
        .bind(self.now() - Duration::days(days_ago))
        .execute(self.db())
        .await
        .unwrap();
    }

    async fn create(&self, subject: &str, body: &str, audience: &str) -> String {
        let (status, b) = self
            .req(
                "POST",
                "/v1/admin/broadcasts",
                Some(json!({"subject": subject, "body_md": body, "audience": audience})),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{b}");
        assert_eq!(b["status"], "draft");
        b["id"].as_str().unwrap().to_owned()
    }

    async fn send_now(&self, id: &str) {
        let (status, b) = self
            .req(
                "POST",
                &format!("/v1/admin/broadcasts/{id}/send"),
                Some(json!({})),
            )
            .await;
        assert_eq!(status, StatusCode::ACCEPTED, "{b}");
        assert_eq!(b["status"], "sending");
    }

    async fn broadcast(&self, id: &str) -> Value {
        let (status, b) = self
            .req("GET", &format!("/v1/admin/broadcasts/{id}"), None)
            .await;
        assert_eq!(status, StatusCode::OK, "{b}");
        b
    }

    /// `(to_addr, user_id, subject, text_body, html_body)` of the broadcast's rows.
    async fn rows(&self, id: &str) -> Vec<(String, Option<Uuid>, String, String, String)> {
        sqlx::query_as(
            "SELECT to_addr, user_id, subject, text_body, html_body FROM outbox_emails \
             WHERE broadcast_id = $1 ORDER BY to_addr",
        )
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_all(self.db())
        .await
        .unwrap()
    }

    async fn events(&self, action: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM admin_events WHERE action = $1")
            .bind(action)
            .fetch_one(self.db())
            .await
            .unwrap()
    }
}

#[sqlx::test]
async fn markdown_escapes_html(pool: PgPool) {
    let c = setup(pool).await;
    let id = c
        .create(
            "Hi <b>{{first_name}}</b>",
            "<script>alert(1)</script> **bold** [x](javascript:alert(1)) [ok](https://example.com)",
            "everyone",
        )
        .await;
    let (status, p) = c
        .req("GET", &format!("/v1/admin/broadcasts/{id}/preview"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    let html = p["html"].as_str().unwrap();
    assert!(
        html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "{html}"
    );
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("<strong>bold</strong>"), "{html}");
    assert!(!html.contains("href=\"javascript"), "{html}");
    assert!(html.contains("href=\"https://example.com\""), "{html}");
    // Previewed as the caller by default; root has no name.
    assert_eq!(p["subject"], "Hi <b>there</b>");
    assert_eq!(p["as_user"]["email"], ROOT);
    // The preview never carries a working unsubscribe token.
    assert!(html.contains("token=preview"), "{html}");
}

#[sqlx::test]
async fn variables_replaced_per_recipient(pool: PgPool) {
    let c = setup(pool).await;
    let ada = c.user("Ada@Example.com", Some("Ada Lovelace")).await;
    let bob = c.user("bob@example.com", None).await;
    let eve = c.user("eve@example.com", Some("  <i>Eve</i> X")).await;
    let acme = c.team("acme", "Acme").await;
    let globex = c.team("globex", "Globex & Co").await;
    // Acme has a live override: its effective plan is Team.
    sqlx::query("UPDATE teams SET override_plan = 'team', override_reason = 'x' WHERE id = $1")
        .bind(acme)
        .execute(c.db())
        .await
        .unwrap();
    c.join(acme, ada, "owner", 3).await;
    c.join(globex, bob, "owner", 3).await;
    c.join(globex, eve, "member", 3).await;
    let id = c
        .create(
            "News for {{first_name}}",
            "Hi {{first_name}}, **{{team_name}}** is on {{plan}}.",
            "everyone",
        )
        .await;

    // As another user, through the preview.
    let (status, p) = c
        .req(
            "GET",
            &format!("/v1/admin/broadcasts/{id}/preview?as_user={ada}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["subject"], "News for Ada");
    assert!(
        p["text"]
            .as_str()
            .unwrap()
            .contains("Hi Ada, Acme is on Team.")
    );
    let (status, p) = c
        .req(
            "GET",
            "/v1/admin/broadcasts/{id}/preview?as_user=bob@example.com"
                .replace("{id}", &id)
                .as_str(),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["subject"], "News for there");

    c.send_now(&id).await;
    // Nothing is queued by the request itself: the worker expands it.
    assert!(c.rows(&id).await.is_empty());
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let rows = c.rows(&id).await;
    let to: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    // Lowercased, and root (no team) is included in `everyone`.
    assert_eq!(
        to,
        [
            "ada@example.com",
            "bob@example.com",
            "eve@example.com",
            ROOT
        ]
    );
    assert!(rows.iter().all(|r| r.1.is_some()), "every row has a user");
    let by = |email: &str| rows.iter().find(|r| r.0 == email).unwrap();
    let (_, _, subject, text, html) = by("ada@example.com");
    assert_eq!(subject, "News for Ada");
    assert!(text.contains("Hi Ada, Acme is on Team."), "{text}");
    assert!(html.contains("<strong>Acme</strong> is on Team."), "{html}");
    let (_, _, subject, text, html) = by("bob@example.com");
    assert_eq!(subject, "News for there");
    assert!(text.contains("Hi there, Globex & Co is on Free."), "{text}");
    assert!(html.contains("<strong>Globex &amp; Co</strong>"), "{html}");
    let (_, _, _, text, html) = by("eve@example.com");
    assert!(text.contains("Hi <i>Eve</i>, Globex"), "{text}");
    assert!(html.contains("Hi &lt;i&gt;Eve&lt;/i&gt;,"), "{html}");
    let (_, _, _, text, _) = by(ROOT);
    assert!(text.contains("Hi there, your team is on Free."), "{text}");
    // Each copy links that recipient's own unsubscribe token.
    let token = unsubscribe::token(SECRET.as_bytes(), ada);
    assert!(by("ada@example.com").3.contains(&format!(
        "http://localhost:8080/v1/unsubscribe?token={token}"
    )),);
    let b = c.broadcast(&id).await;
    assert_eq!(b["status"], "sent");
    assert_eq!(b["sent_count"], 4);
    assert_eq!(b["sent_at"], json!(c.now()));
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT kind FROM outbox_emails WHERE broadcast_id IS NOT NULL",
    )
    .fetch_all(c.db())
    .await
    .unwrap();
    assert_eq!(kinds, ["broadcast"]);
}

#[sqlx::test]
async fn multi_team_user_gets_one_email_first_team(pool: PgPool) {
    let c = setup(pool).await;
    let ada = c.user("ada@example.com", Some("Ada")).await;
    let older = c.team("older", "Older").await;
    let newer = c.team("newer", "Newer").await;
    c.join(newer, ada, "owner", 1).await;
    c.join(older, ada, "member", 5).await;
    let id = c.create("Hi", "{{team_name}}", "everyone").await;
    c.send_now(&id).await;
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let rows: Vec<_> = c
        .rows(&id)
        .await
        .into_iter()
        .filter(|r| r.1 == Some(ada))
        .collect();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].3.starts_with("Older\n"), "{}", rows[0].3);

    // For an owners' audience, the first team that puts them in it.
    let id = c.create("Hi", "{{team_name}}", "team_owners").await;
    c.send_now(&id).await;
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let rows = c.rows(&id).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].3.starts_with("Newer\n"), "{}", rows[0].3);
}

#[sqlx::test]
async fn opted_out_skipped_sign_in_codes_still_sent(pool: PgPool) {
    let c = setup(pool).await;
    c.user("ada@example.com", None).await;
    let bob = c.user("bob@example.com", None).await;
    let sus = c.user("sus@example.com", None).await;
    sqlx::query("UPDATE users SET product_emails = false WHERE id = $1")
        .bind(bob)
        .execute(c.db())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET suspended_at = $2 WHERE id = $1")
        .bind(sus)
        .bind(c.now())
        .execute(c.db())
        .await
        .unwrap();
    let (status, n) = c
        .req("GET", "/v1/admin/audience-count?audience=everyone", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{n}");
    assert_eq!(n["count"], 2, "root and ada");
    let id = c.create("Hi", "Hello", "everyone").await;
    c.send_now(&id).await;
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let to: Vec<String> = c.rows(&id).await.into_iter().map(|r| r.0).collect();
    assert_eq!(to, ["ada@example.com", ROOT]);

    // Bob still gets his sign-in code.
    let (code, _) = c.app.start_email("bob@example.com", "web").await;
    assert_eq!(code.len(), 6);
    let kind: String =
        sqlx::query_scalar("SELECT kind FROM outbox_emails WHERE to_addr = 'bob@example.com'")
            .fetch_one(c.db())
            .await
            .unwrap();
    assert_eq!(kind, "sign_in_code");
}

#[sqlx::test]
async fn unsubscribe_token_hmac_and_idempotent(pool: PgPool) {
    let c = setup(pool).await;
    let bob = c.user("bob@example.com", None).await;
    let token = unsubscribe::token(SECRET.as_bytes(), bob);
    assert_eq!(unsubscribe::verify(SECRET.as_bytes(), &token), Some(bob));
    assert_eq!(
        unsubscribe::verify(b"another-secret-another-secret-0000", &token),
        None
    );
    let opted_in = || async {
        sqlx::query_scalar::<_, bool>("SELECT product_emails FROM users WHERE id = $1")
            .bind(bob)
            .fetch_one(c.db())
            .await
            .unwrap()
    };
    let get = |t: String| {
        let c = &c;
        async move {
            c.app
                .request(
                    "GET".parse().unwrap(),
                    &format!("/v1/unsubscribe?token={t}"),
                    None,
                    None,
                )
                .await
        }
    };
    let post = |t: String, body: &'static str, content_type: Option<&'static str>| {
        let c = &c;
        async move {
            let headers: Vec<(&str, &str)> = content_type
                .map(|ct| ("content-type", ct))
                .into_iter()
                .collect();
            c.app
                .post_raw(&format!("/v1/unsubscribe?token={t}"), body, &headers)
                .await
        }
    };
    const DONE: &str = "You won't get product emails from curl-runner anymore.";

    // GET only asks: a link scanner changes nothing.
    for _ in 0..2 {
        let (status, body) = get(token.clone()).await;
        assert_eq!(status, StatusCode::OK);
        let page = body.as_str().unwrap();
        assert!(
            page.contains("Stop product emails from curl-runner?"),
            "{page}"
        );
        assert!(
            page.contains(&format!(
                "<form method=\"post\" action=\"http://localhost:8080/v1/unsubscribe?token={token}\""
            )),
            "{page}"
        );
        assert!(page.contains("&gt;_"), "the light layout");
        assert!(!page.contains('\u{2014}'));
    }
    assert!(opted_in().await);

    // Tampered, truncated, junk and missing tokens get the same pages as a
    // good one, and change nothing.
    let other = c.user("other@example.com", None).await;
    let mut forged = other.as_bytes().to_vec();
    forged.extend_from_slice(&base64_decode(&token)[16..]);
    let (_, good_confirm) = get(token.clone()).await;
    for bad in [
        base64_encode(&forged),
        token[..token.len() - 2].to_owned(),
        "nope".to_owned(),
        String::new(),
        "%ZZ&token=".to_owned(),
    ] {
        let (status, body) = get(bad.clone()).await;
        assert_eq!(status, StatusCode::OK, "{bad}");
        let page = body.as_str().unwrap();
        assert!(page.contains("Stop product emails"), "{bad}: {page}");
        assert_eq!(
            without_token(page),
            without_token(good_confirm.as_str().unwrap()),
            "{bad}"
        );
        let (status, body) = post(bad.clone(), "", None).await;
        assert_eq!(status, StatusCode::OK, "{bad}");
        assert!(body.as_str().unwrap().contains(DONE), "{bad}");
    }
    assert!(opted_in().await);
    let other_in: bool = sqlx::query_scalar("SELECT product_emails FROM users WHERE id = $1")
        .bind(other)
        .fetch_one(c.db())
        .await
        .unwrap();
    assert!(other_in, "a forged token does nothing");

    // The confirm page's form posts no fields.
    let (status, body) = post(token.clone(), "", Some("application/x-www-form-urlencoded")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_str().unwrap().contains(DONE));
    assert!(!opted_in().await);
    // Again is the same.
    let (status, body) = post(token.clone(), "", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_str().unwrap().contains(DONE));
    assert!(!opted_in().await);

    // RFC 8058 one-click, from a mail client.
    let carol = c.user("carol@example.com", None).await;
    let (status, body) = post(
        unsubscribe::token(SECRET.as_bytes(), carol),
        "List-Unsubscribe=One-Click",
        Some("application/x-www-form-urlencoded"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_str().unwrap().contains(DONE));
    let carol_in: bool = sqlx::query_scalar("SELECT product_emails FROM users WHERE id = $1")
        .bind(carol)
        .fetch_one(c.db())
        .await
        .unwrap();
    assert!(!carol_in);

    // Limited per IP, answered as a page too.
    let mut limited = None;
    for _ in 0..40 {
        let (status, body) = get(token.clone()).await;
        if status == StatusCode::TOO_MANY_REQUESTS {
            limited = Some(body);
            break;
        }
    }
    let page = limited.expect("rate limited");
    assert!(page.as_str().unwrap().contains("Too many tries."), "{page}");
}

/// `page` with the form's token value cut out.
fn without_token(page: &str) -> String {
    let (head, tail) = page.split_once("token=").expect("a token in the form");
    let rest = &tail[tail.find('"').unwrap()..];
    format!("{head}token={rest}")
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .unwrap()
}

fn base64_encode(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

#[sqlx::test]
async fn send_resumes_without_duplicates(pool: PgPool) {
    let c = setup(pool).await;
    sqlx::query(
        "INSERT INTO users (email, name, created_at) \
         SELECT 'user' || n || '@example.com', 'User ' || n, $1 FROM generate_series(1, 1100) n",
    )
    .bind(c.now())
    .execute(c.db())
    .await
    .unwrap();
    let id = c.create("Hi", "Hello {{first_name}}", "everyone").await;
    let uuid = Uuid::parse_str(&id).unwrap();
    c.send_now(&id).await;

    // One chunk, then the process "dies".
    let done = expand_broadcast(&c.app.state, uuid, 1).await.unwrap();
    assert!(!done);
    assert_eq!(c.rows(&id).await.len(), 500);
    let b = c.broadcast(&id).await;
    assert_eq!(
        (b["status"].as_str(), b["sent_count"].as_i64()),
        (Some("sending"), Some(500))
    );

    // The next tick resumes where it stopped.
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let rows = c.rows(&id).await;
    assert_eq!(rows.len(), 1101);
    let users: HashSet<Uuid> = rows.iter().map(|r| r.1.unwrap()).collect();
    assert_eq!(users.len(), 1101, "one row per user");
    let b = c.broadcast(&id).await;
    assert_eq!(
        (b["status"].as_str(), b["sent_count"].as_i64()),
        (Some("sent"), Some(1101))
    );

    // Running again, or sending again, adds nothing.
    assert!(
        expand_broadcast(&c.app.state, uuid, usize::MAX)
            .await
            .unwrap()
    );
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    assert_eq!(c.rows(&id).await.len(), 1101);
    let (status, _) = c
        .req(
            "POST",
            &format!("/v1/admin/broadcasts/{id}/send"),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = c
        .req(
            "PATCH",
            &format!("/v1/admin/broadcasts/{id}"),
            Some(json!({"subject": "Changed"})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(c.events("broadcast.send").await, 1);
}

#[sqlx::test]
async fn scheduled_sends_at_time(pool: PgPool) {
    let c = setup(pool).await;
    c.user("ada@example.com", None).await;
    let id = c.create("Later", "Hi", "everyone").await;
    let path = format!("/v1/admin/broadcasts/{id}/send");
    let (status, _) = c
        .req(
            "POST",
            &path,
            Some(json!({"at": c.now() - Duration::minutes(1)})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let at = c.now() + Duration::hours(1);
    let (status, b) = c.req("POST", &path, Some(json!({"at": at}))).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{b}");
    assert_eq!(b["status"], "scheduled");
    assert_eq!(b["scheduled_at"], json!(at));
    assert_eq!(c.events("broadcast.schedule").await, 1);
    // Still editable, and can be moved.
    let (status, b) = c
        .req(
            "PATCH",
            &format!("/v1/admin/broadcasts/{id}"),
            Some(json!({"body_md": "Hi again"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{b}");
    assert_eq!(b["status"], "scheduled");

    fan_out_due(&c.app.state, c.now()).await.unwrap();
    assert!(c.rows(&id).await.is_empty(), "not due yet");
    c.app.clock.advance(Duration::minutes(59));
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    assert!(c.rows(&id).await.is_empty(), "not due yet");
    c.app.clock.advance(Duration::minutes(1));
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let rows = c.rows(&id).await;
    assert_eq!(rows.len(), 2);
    assert!(rows[0].3.starts_with("Hi again"));
    let b = c.broadcast(&id).await;
    assert_eq!(b["status"], "sent");
    assert_eq!(b["sent_at"], json!(c.now()));
}

#[sqlx::test]
async fn audience_counts(pool: PgPool) {
    let c = setup(pool).await;
    let ada = c.user("ada@example.com", None).await; // owns paying acme
    let bob = c.user("bob@example.com", None).await; // member of acme
    let carol = c.user("carol@example.com", None).await; // owns free globex
    let dan = c.user("dan@example.com", None).await; // owns granted initech
    let erin = c.user("erin@example.com", None).await; // owns free hooli, opted out
    let acme = c.team("acme", "Acme").await;
    let globex = c.team("globex", "Globex").await;
    let initech = c.team("initech", "Initech").await;
    let hooli = c.team("hooli", "Hooli").await;
    let gone = c.team("gone", "Gone").await;
    sqlx::query(
        "UPDATE teams SET plan = 'team', stripe_subscription_id = 'sub_1', billing_status = 'active' \
         WHERE id = $1",
    )
    .bind(acme)
    .execute(c.db())
    .await
    .unwrap();
    // A cancelled subscription is not paying.
    sqlx::query(
        "UPDATE teams SET stripe_subscription_id = 'sub_2', billing_status = 'canceled' WHERE id = $1",
    )
    .bind(gone)
    .execute(c.db())
    .await
    .unwrap();
    sqlx::query("UPDATE teams SET override_plan = 'team', override_until = $2 WHERE id = $1")
        .bind(initech)
        .bind(c.now() + Duration::days(3))
        .execute(c.db())
        .await
        .unwrap();
    sqlx::query("UPDATE users SET product_emails = false WHERE id = $1")
        .bind(erin)
        .execute(c.db())
        .await
        .unwrap();
    c.join(acme, ada, "owner", 1).await;
    c.join(acme, bob, "member", 1).await;
    c.join(globex, carol, "owner", 1).await;
    c.join(initech, dan, "owner", 1).await;
    c.join(hooli, erin, "owner", 1).await;
    c.join(gone, bob, "owner", 1).await;

    let count = |audience: &'static str| {
        let c = &c;
        async move {
            let (status, b) = c
                .req(
                    "GET",
                    &format!("/v1/admin/audience-count?audience={audience}"),
                    None,
                )
                .await;
            assert_eq!(status, StatusCode::OK, "{b}");
            assert_eq!(b["audience"], audience);
            b["count"].as_i64().unwrap()
        }
    };
    assert_eq!(count("everyone").await, 5, "root, ada, bob, carol, dan");
    assert_eq!(count("team_owners").await, 4, "ada, carol, dan, bob (gone)");
    assert_eq!(count("paying_teams").await, 1, "ada");
    assert_eq!(count("free_teams").await, 2, "carol, bob (gone)");
    assert_eq!(count("granted_teams").await, 1, "dan");
    // Once the override ends, initech is Free.
    c.app.clock.advance(Duration::days(4));
    assert_eq!(count("granted_teams").await, 0);
    assert_eq!(count("free_teams").await, 3);
    let (status, _) = c
        .req("GET", "/v1/admin/audience-count?audience=nope", None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn drafts_tests_permissions_and_events(pool: PgPool) {
    let c = setup(pool).await;
    let rosa = c.app.sign_in(ROSA).await;
    let (status, b) = c
        .req(
            "POST",
            "/v1/admin/admins",
            Some(json!({"email": ROSA, "permissions": ["users"]})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{b}");

    // A draft may start empty; a bad audience is refused.
    let (status, b) = c.req("POST", "/v1/admin/broadcasts", Some(json!({}))).await;
    assert_eq!(status, StatusCode::CREATED, "{b}");
    assert_eq!(
        (b["subject"].as_str(), b["audience"].as_str()),
        (Some(""), Some("everyone"))
    );
    let id = b["id"].as_str().unwrap().to_owned();
    let (status, _) = c
        .req(
            "POST",
            "/v1/admin/broadcasts",
            Some(json!({"audience": "nope"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // An empty broadcast cannot be tested or sent.
    for action in ["test", "send"] {
        let (status, _) = c
            .req(
                "POST",
                &format!("/v1/admin/broadcasts/{id}/{action}"),
                Some(json!({})),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{action}");
    }
    let (status, b) = c
        .req(
            "PATCH",
            &format!("/v1/admin/broadcasts/{id}"),
            Some(json!({"subject": "Launch", "body_md": "Hi {{first_name}}", "audience": "team_owners"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{b}");
    assert_eq!(b["audience"], "team_owners");

    // Rosa can read but not write.
    let (status, list) = c.req_as("GET", "/v1/admin/broadcasts", None, &rosa).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["items"][0]["subject"], "Launch");
    for (method, path, body) in [
        ("POST", "/v1/admin/broadcasts".to_owned(), json!({})),
        (
            "PATCH",
            format!("/v1/admin/broadcasts/{id}"),
            json!({"subject": "x"}),
        ),
        ("POST", format!("/v1/admin/broadcasts/{id}/test"), json!({})),
        ("POST", format!("/v1/admin/broadcasts/{id}/send"), json!({})),
    ] {
        let (status, _) = c.req_as(method, &path, Some(body), &rosa).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }

    // A test goes to me only, marked, outside the broadcast's rows.
    let (status, b) = c
        .req(
            "POST",
            &format!("/v1/admin/broadcasts/{id}/test"),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{b}");
    let (to, subject, kind, broadcast): (String, String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT to_addr, subject, kind, broadcast_id FROM outbox_emails WHERE kind = 'broadcast_test'",
    )
    .fetch_one(c.db())
    .await
    .unwrap();
    assert_eq!(
        (to.as_str(), subject.as_str(), broadcast),
        (ROOT, "[Test] Launch", None)
    );
    assert_eq!(kind, "broadcast_test");
    assert_eq!(c.broadcast(&id).await["status"], "draft");

    let (status, _) = c
        .req(
            "GET",
            &format!("/v1/admin/broadcasts/{}", Uuid::new_v4()),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for action in ["broadcast.create", "broadcast.update", "broadcast.test"] {
        assert_eq!(c.events(action).await, 1, "{action}");
    }
    let target: String =
        sqlx::query_scalar("SELECT target_id FROM admin_events WHERE action = 'broadcast.update'")
            .fetch_one(c.db())
            .await
            .unwrap();
    assert_eq!(target, id);
}

#[sqlx::test]
async fn delivery_log_filters_and_cursor(pool: PgPool) {
    let c = setup(pool).await;
    let now = c.now();
    // The root sign-in code is already queued. Add a legacy row with no
    // created_at, a sent one and a failed one.
    let insert = |to: &'static str, kind: &'static str, created: Option<DateTime<Utc>>| {
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO outbox_emails (to_addr, subject, text_body, next_attempt_at, kind, created_at) \
             VALUES ($1, 'Subject ' || $1, 'x', $2, $3, $4) RETURNING id",
        )
        .bind(to)
        .bind(now)
        .bind(kind)
        .bind(created)
        .fetch_one(c.db())
    };
    let legacy = insert("old@example.com", "transactional", None)
        .await
        .unwrap();
    sqlx::query("UPDATE outbox_emails SET created_at = NULL, sent_at = $2 WHERE id = $1")
        .bind(legacy)
        .bind(now - Duration::days(400))
        .execute(c.db())
        .await
        .unwrap();
    let sent = insert(
        "sent@example.com",
        "member_invite",
        Some(now - Duration::minutes(5)),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE outbox_emails SET sent_at = $2 WHERE id = $1")
        .bind(sent)
        .bind(now)
        .execute(c.db())
        .await
        .unwrap();
    let failed = insert(
        "failed@example.com",
        "payment_failed",
        Some(now - Duration::minutes(10)),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE outbox_emails SET failed_at = $2, attempts = 8, last_error = '550 no such user' \
         WHERE id = $1",
    )
    .bind(failed)
    .bind(now)
    .execute(c.db())
    .await
    .unwrap();

    let (status, log) = c.req("GET", "/v1/admin/emails/log", None).await;
    assert_eq!(status, StatusCode::OK, "{log}");
    let items = log["items"].as_array().unwrap();
    let to: Vec<&str> = items.iter().map(|i| i["to"].as_str().unwrap()).collect();
    assert_eq!(
        to,
        [
            ROOT,
            "sent@example.com",
            "failed@example.com",
            "old@example.com"
        ],
        "newest first, NULL created_at last"
    );
    // A sign-in code's subject is never shown.
    assert_eq!(items[0]["kind"], "sign_in_code");
    assert_eq!(items[0]["subject"], "Your curl-runner sign-in code");
    assert_eq!(items[0]["status"], "queued");
    assert_eq!(items[1]["status"], "sent");
    assert_eq!(items[1]["subject"], "Subject sent@example.com");
    assert_eq!(items[2]["status"], "failed");
    assert_eq!(items[2]["attempts"], 8);
    assert_eq!(items[2]["last_error"], "550 no such user");
    assert_eq!(items[3]["created_at"], Value::Null);
    for key in [
        "id",
        "to",
        "subject",
        "kind",
        "status",
        "created_at",
        "sent_at",
        "failed_at",
        "attempts",
    ] {
        assert!(items[0].get(key).is_some(), "{key}");
    }

    // Filters.
    for (query, expect) in [
        ("status=failed", vec!["failed@example.com"]),
        ("status=sent", vec!["sent@example.com", "old@example.com"]),
        ("status=queued", vec![ROOT]),
        ("kind=member_invite", vec!["sent@example.com"]),
        ("q=FAILED", vec!["failed@example.com"]),
        ("q=%25", vec![]),
    ] {
        let (status, log) = c
            .req("GET", &format!("/v1/admin/emails/log?{query}"), None)
            .await;
        assert_eq!(status, StatusCode::OK, "{query}: {log}");
        let to: Vec<&str> = log["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["to"].as_str().unwrap())
            .collect();
        assert_eq!(to, expect, "{query}");
    }
    for bad in ["status=nope", "kind=nope", "cursor=nope", "limit=0"] {
        let (status, _) = c
            .req("GET", &format!("/v1/admin/emails/log?{bad}"), None)
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }

    // Paging walks every row once, across the NULL created_at.
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let path = match &cursor {
            Some(c) => format!("/v1/admin/emails/log?limit=1&cursor={c}"),
            None => "/v1/admin/emails/log?limit=1".to_owned(),
        };
        let (status, page) = c.req("GET", &path, None).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        for i in page["items"].as_array().unwrap() {
            seen.push(i["to"].as_str().unwrap().to_owned());
        }
        match page["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
    }
    assert_eq!(seen, to);
}

/// Records who was sent mail, with its extra headers, in order.
#[derive(Default)]
struct Recorder(std::sync::Mutex<Vec<Sent>>);

/// One sent message: the recipient and its extra headers.
type Sent = (String, Vec<(&'static str, String)>);

impl Recorder {
    fn to(&self) -> Vec<String> {
        self.0.lock().unwrap().iter().map(|s| s.0.clone()).collect()
    }

    fn headers_of(&self, to: &str) -> Vec<(&'static str, String)> {
        let sent = self.0.lock().unwrap();
        sent.iter().find(|s| s.0 == to).expect("sent").1.clone()
    }
}

impl curl_runner_server::mail::Transport for Recorder {
    fn send<'a>(
        &'a self,
        to: &'a str,
        _subject: &'a str,
        _text: &'a str,
        _html: Option<&'a str>,
        headers: &'a curl_runner_server::mail::transport::Headers,
    ) -> curl_runner_server::mail::transport::SendFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .push((to.to_owned(), headers.to_vec()));
            Ok(())
        })
    }
}

#[sqlx::test]
async fn sign_in_codes_jump_the_broadcast_queue(pool: PgPool) {
    let c = setup(pool).await;
    sqlx::query(
        "INSERT INTO users (email, created_at) \
         SELECT 'user' || n || '@example.com', $1 FROM generate_series(1, 60) n",
    )
    .bind(c.now())
    .execute(c.db())
    .await
    .unwrap();
    let id = c.create("Big news", "Hello", "everyone").await;
    c.send_now(&id).await;
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    // The root's own sign-in code from setup went out first; clear the slate.
    let transport = Recorder::default();
    c.app.clock.advance(Duration::seconds(1));
    let (code, _) = c.app.start_email("late@example.com", "web").await;
    assert_eq!(code.len(), 6);
    curl_runner_server::mail::worker::run_once(
        c.db(),
        &transport,
        c.app.clock.as_ref(),
        &c.app.state.config,
    )
    .await
    .unwrap();
    let sent = transport.to();
    assert_eq!(sent.len(), 20);
    let late = sent.iter().position(|t| t == "late@example.com");
    assert!(late.is_some_and(|i| i <= 1), "{sent:?}");
    // A sent broadcast copy keeps its subject for the log; bodies are blanked.
    // A sent sign-in code keeps nothing.
    let (subject, text): (String, String) = sqlx::query_as(
        "SELECT subject, text_body FROM outbox_emails \
         WHERE broadcast_id IS NOT NULL AND sent_at IS NOT NULL LIMIT 1",
    )
    .fetch_one(c.db())
    .await
    .unwrap();
    assert_eq!((subject.as_str(), text.as_str()), ("Big news", ""));
    let subject: String =
        sqlx::query_scalar("SELECT subject FROM outbox_emails WHERE to_addr = 'late@example.com'")
            .fetch_one(c.db())
            .await
            .unwrap();
    assert_eq!(subject, "");
}

async fn send_all(c: &Ctx, transport: &Recorder) {
    while curl_runner_server::mail::worker::run_once(
        c.db(),
        transport,
        c.app.clock.as_ref(),
        &c.app.state.config,
    )
    .await
    .unwrap()
        > 0
    {}
}

#[sqlx::test]
async fn list_unsubscribe_headers_on_broadcasts_only(pool: PgPool) {
    let c = setup(pool).await;
    let ada = c.user("Ada@Example.com", None).await;
    let id = c.create("News", "Hello", "everyone").await;
    c.send_now(&id).await;
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    let transport = Recorder::default();
    send_all(&c, &transport).await;
    let link = format!(
        "<http://localhost:8080/v1/unsubscribe?token={}>",
        unsubscribe::token(SECRET.as_bytes(), ada)
    );
    assert_eq!(
        transport.headers_of("ada@example.com"),
        [
            ("List-Unsubscribe", link),
            (
                "List-Unsubscribe-Post",
                "List-Unsubscribe=One-Click".to_owned()
            ),
        ]
    );
    // The root got a sign-in code and a broadcast copy; the code has none.
    let sent = transport.0.lock().unwrap().clone();
    let root: Vec<_> = sent.iter().filter(|s| s.0 == ROOT).collect();
    assert_eq!(root.len(), 2);
    assert!(root[0].1.is_empty(), "the sign-in code goes first, bare");
    assert_eq!(root[1].1.len(), 2);
    // A test copy is not a broadcast row: no headers either.
    let (status, _) = c
        .req(
            "POST",
            &format!("/v1/admin/broadcasts/{id}/test"),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let tests = Recorder::default();
    send_all(&c, &tests).await;
    assert_eq!(tests.to(), [ROOT]);
    assert!(tests.headers_of(ROOT).is_empty());
}

#[sqlx::test]
async fn opt_out_mid_send_skips_the_queued_copy(pool: PgPool) {
    let c = setup(pool).await;
    let ada = c.user("ada@example.com", None).await;
    let bob = c.user("bob@example.com", None).await;
    sqlx::query("DELETE FROM outbox_emails")
        .execute(c.db())
        .await
        .unwrap();
    let id = c.create("News", "Hello", "everyone").await;
    c.send_now(&id).await;
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    assert_eq!(c.rows(&id).await.len(), 3);
    // Ada unsubscribes from the first copy before hers goes out, and bob's
    // account is deleted.
    let (status, _) = c
        .app
        .post_raw(
            &format!(
                "/v1/unsubscribe?token={}",
                unsubscribe::token(SECRET.as_bytes(), ada)
            ),
            "List-Unsubscribe=One-Click",
            &[("content-type", "application/x-www-form-urlencoded")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(bob)
        .execute(c.db())
        .await
        .unwrap();
    let transport = Recorder::default();
    send_all(&c, &transport).await;
    assert_eq!(transport.to(), [ROOT]);
    let skipped: Vec<(String, Option<String>, bool, String)> = sqlx::query_as(
        "SELECT to_addr, last_error, failed_at IS NOT NULL, text_body FROM outbox_emails \
         WHERE broadcast_id = $1 AND sent_at IS NULL ORDER BY to_addr",
    )
    .bind(Uuid::parse_str(&id).unwrap())
    .fetch_all(c.db())
    .await
    .unwrap();
    assert_eq!(
        skipped,
        [
            (
                "ada@example.com".to_owned(),
                Some(curl_runner_server::mail::worker::SKIPPED_OPTED_OUT.to_owned()),
                true,
                String::new()
            ),
            (
                "bob@example.com".to_owned(),
                Some(curl_runner_server::mail::worker::SKIPPED_NO_ACCOUNT.to_owned()),
                true,
                String::new()
            ),
        ]
    );
    // They show as failed, with the reason, in the log and the counts.
    let (_, log) = c
        .req("GET", "/v1/admin/emails/log?status=failed", None)
        .await;
    assert_eq!(log["items"].as_array().unwrap().len(), 2);
    let b = c.broadcast(&id).await;
    assert_eq!(
        (b["delivered"].as_i64(), b["failed"].as_i64()),
        (Some(1), Some(2))
    );
    // A second pass leaves them alone.
    let again = Recorder::default();
    send_all(&c, &again).await;
    assert!(again.to().is_empty());
}

#[sqlx::test]
async fn scheduled_broadcast_stays_complete(pool: PgPool) {
    let c = setup(pool).await;
    let id = c.create("Later", "Hi", "everyone").await;
    let at = c.now() + Duration::hours(1);
    let (status, _) = c
        .req(
            "POST",
            &format!("/v1/admin/broadcasts/{id}/send"),
            Some(json!({"at": at})),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    for body in [json!({"subject": "  "}), json!({"body_md": ""})] {
        let (status, _) = c
            .req(
                "PATCH",
                &format!("/v1/admin/broadcasts/{id}"),
                Some(body.clone()),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let b = c.broadcast(&id).await;
    assert_eq!(
        (b["subject"].as_str(), b["body_md"].as_str()),
        (Some("Later"), Some("Hi"))
    );
    assert_eq!(c.events("broadcast.update").await, 0);

    // The worker's backstop: emptied behind the API's back, it goes back to
    // draft instead of sending, and other broadcasts still go out.
    sqlx::query("UPDATE broadcasts SET body_md = '' WHERE id = $1")
        .bind(Uuid::parse_str(&id).unwrap())
        .execute(c.db())
        .await
        .unwrap();
    let other = c.create("Now", "Hi", "everyone").await;
    c.send_now(&other).await;
    c.app.clock.advance(Duration::hours(2));
    fan_out_due(&c.app.state, c.now()).await.unwrap();
    assert_eq!(c.broadcast(&id).await["status"], "draft");
    assert!(c.rows(&id).await.is_empty());
    assert_eq!(c.broadcast(&other).await["status"], "sent");
}
