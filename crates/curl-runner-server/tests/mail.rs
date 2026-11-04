#![cfg(feature = "db-tests")]

mod common;

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use common::{NOW, test_app};
use curl_runner_server::mail::transport::SendFuture;
use curl_runner_server::mail::worker::{MAX_ATTEMPTS, run_once};
use curl_runner_server::mail::{Mail, Transport};
use sqlx::PgPool;

/// Records sends; fails the first `failures` calls.
struct FakeTransport {
    failures: Mutex<u32>,
    sent: Mutex<Vec<(String, String)>>,
}

impl FakeTransport {
    fn new(failures: u32) -> Self {
        Self {
            failures: Mutex::new(failures),
            sent: Mutex::default(),
        }
    }
    fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().unwrap().clone()
    }
}

impl Transport for FakeTransport {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        _text: &'a str,
        _html: Option<&'a str>,
    ) -> SendFuture<'a> {
        Box::pin(async move {
            let mut failures = self.failures.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return Err("relay down".into());
            }
            self.sent.lock().unwrap().push((to.into(), subject.into()));
            Ok(())
        })
    }
}

fn code_mail() -> Mail {
    Mail::SignInCode {
        to: "ada@example.com".into(),
        code: "123456".into(),
        link: "https://cloud.example.com/auth/magic?token=abc".into(),
    }
}

async fn count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM outbox_emails")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn enqueue_in_rolled_back_tx_sends_nothing(pool: PgPool) {
    let app = test_app(pool.clone()).await;
    let mut tx = pool.begin().await.unwrap();
    app.state
        .mailer
        .enqueue(&mut tx, code_mail())
        .await
        .unwrap();
    tx.rollback().await.unwrap();

    let transport = FakeTransport::new(0);
    assert_eq!(
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap(),
        0
    );
    assert!(transport.sent().is_empty());
    assert_eq!(count(&pool).await, 0);
}

#[sqlx::test]
async fn worker_sends_and_marks_sent(pool: PgPool) {
    let app = test_app(pool.clone()).await;
    let mut tx = pool.begin().await.unwrap();
    app.state
        .mailer
        .enqueue(&mut tx, code_mail())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let transport = FakeTransport::new(0);
    assert_eq!(
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        transport.sent(),
        vec![(
            "ada@example.com".to_string(),
            "Your curl-runner sign-in code: 123456".to_string()
        )]
    );
    let sent_at: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT sent_at FROM outbox_emails")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sent_at, Some(NOW.parse().unwrap()));
    // Already sent: a second pass does nothing.
    assert_eq!(
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap(),
        0
    );
    assert_eq!(transport.sent().len(), 1);
}

#[sqlx::test]
async fn worker_backs_off_on_failure(pool: PgPool) {
    let app = test_app(pool.clone()).await;
    let mut tx = pool.begin().await.unwrap();
    app.state
        .mailer
        .enqueue(&mut tx, code_mail())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let transport = FakeTransport::new(1);
    assert_eq!(
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap(),
        0
    );
    let (attempts, next, error): (i32, DateTime<Utc>, Option<String>) =
        sqlx::query_as("SELECT attempts, next_attempt_at, last_error FROM outbox_emails")
            .fetch_one(&pool)
            .await
            .unwrap();
    let now: DateTime<Utc> = NOW.parse().unwrap();
    assert_eq!(attempts, 1);
    assert_eq!(next, now + Duration::seconds(30));
    assert_eq!(error.as_deref(), Some("relay down"));

    // Not due yet.
    app.clock.advance(Duration::seconds(29));
    assert_eq!(
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap(),
        0
    );
    assert!(transport.sent().is_empty());

    app.clock.advance(Duration::seconds(2));
    assert_eq!(
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap(),
        1
    );
    assert_eq!(transport.sent().len(), 1);
}

#[sqlx::test]
async fn worker_gives_up_after_max_attempts(pool: PgPool) {
    let app = test_app(pool.clone()).await;
    let mut tx = pool.begin().await.unwrap();
    app.state
        .mailer
        .enqueue(&mut tx, code_mail())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let transport = FakeTransport::new(u32::MAX);
    for _ in 0..MAX_ATTEMPTS {
        run_once(&pool, &transport, app.clock.as_ref())
            .await
            .unwrap();
        app.clock.advance(Duration::hours(2));
    }
    let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM outbox_emails")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, MAX_ATTEMPTS);
    // Stopped: another pass leaves it alone.
    run_once(&pool, &transport, app.clock.as_ref())
        .await
        .unwrap();
    let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM outbox_emails")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, MAX_ATTEMPTS);
}
