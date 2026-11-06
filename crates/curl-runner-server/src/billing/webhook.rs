//! `POST /v1/billing/webhook`: Stripe events. An event only says "look at this
//! subscription": the handler re-reads it from Stripe and applies what it finds,
//! so a late or out-of-order event cannot undo a newer state.

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use curl_runner_cloud_proto::ErrorCode;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;

use super::BillingError;
use super::sync::{apply_missing, apply_subscription};
use crate::error::ApiError;
use crate::state::AppState;

/// Raw body cap.
const MAX_BODY: usize = 1024 * 1024;
/// How far a signed timestamp may be from now, in seconds.
const TOLERANCE_SECS: i64 = 300;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/billing/webhook", post(webhook))
        .layer(DefaultBodyLimit::max(MAX_BODY))
}

fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message)
}

/// Whether `header` (`t=<unix>,v1=<hex>[,v1=<hex>...]`) is a valid signature of
/// `body` under `secret`, made at most 5 minutes from `now`.
pub fn verify_signature(header: &str, body: &[u8], secret: &str, now: i64) -> bool {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", t)) => timestamp = t.parse::<i64>().ok(),
            Some(("v1", sig)) => signatures.push(sig),
            _ => {}
        }
    }
    let Some(timestamp) = timestamp else {
        return false;
    };
    if now.abs_diff(timestamp) > TOLERANCE_SECS.unsigned_abs() {
        return false;
    }
    signatures.iter().any(|sig| {
        let Some(expected) = decode_hex(sig) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        // Constant time.
        mac.verify_slice(&expected).is_ok()
    })
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// The subscription an event is about, for the event types this server handles.
/// `None` for other types, and for events with no subscription (a one-off invoice).
/// Paths are those of the pinned API version `2026-10-28.endive`.
fn subscription_id(kind: &str, object: &Value) -> Option<String> {
    let id = match kind {
        "checkout.session.completed" => object.get("subscription"),
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => object.get("id"),
        "invoice.payment_failed" | "invoice.paid" => {
            object.pointer("/parent/subscription_details/subscription")
        }
        _ => None,
    }?;
    id.as_str().map(str::to_owned)
}

fn is_handled(kind: &str) -> bool {
    matches!(
        kind,
        "checkout.session.completed"
            | "customer.subscription.created"
            | "customer.subscription.updated"
            | "customer.subscription.deleted"
            | "invoice.payment_failed"
            | "invoice.paid"
    )
}

async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let (Some(billing), Some(config)) = (&state.billing, &state.config.billing) else {
        return Err(ApiError::not_found());
    };
    let now = state.clock.now();
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            state.metrics.bad_signature(now);
            invalid("Missing signature.")
        })?;
    if !verify_signature(signature, &body, &config.webhook_secret, now.timestamp()) {
        state.metrics.bad_signature(now);
        return Err(invalid("Invalid signature."));
    }
    let event: Value =
        serde_json::from_slice(&body).map_err(|_| invalid("The body is not valid JSON."))?;
    let (Some(event_id), Some(kind)) = (
        event.get("id").and_then(Value::as_str),
        event.get("type").and_then(Value::as_str),
    ) else {
        return Err(invalid("The event has no id or type."));
    };
    if !is_handled(kind) {
        return Ok(Json(json!({})));
    }

    // The event row and the effects share one transaction: if Stripe cannot be
    // reached, nothing is recorded and Stripe's retry gets another go. A
    // concurrent delivery of the same event waits on the row, then sees it done.
    let mut tx = state.db.begin().await?;
    let fresh = sqlx::query(
        "INSERT INTO stripe_events (id, type, received_at) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(event_id)
    .bind(kind)
    .bind(now)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !fresh {
        state.metrics.duplicate_event(now);
        return Ok(Json(json!({})));
    }
    let object = event.pointer("/data/object").unwrap_or(&Value::Null);
    if let Some(id) = subscription_id(kind, object) {
        // One event per subscription at a time, so the re-read below happens
        // after any earlier event's changes are committed and the last one to
        // apply always saw the newest Stripe state.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        let applied = match billing.subscription(&id).await {
            Ok(sub) => apply_subscription(&state, &mut tx, &sub, now).await?,
            // Stripe no longer knows it: as good as canceled.
            Err(BillingError::Api { status: 404, .. }) => {
                apply_missing(&state, &mut tx, &id, now).await?
            }
            Err(err) => {
                tracing::warn!("webhook {kind} {event_id}: {err}");
                return Err(ApiError::unavailable());
            }
        };
        if let Some(applied) = applied {
            sqlx::query(
                "UPDATE stripe_events SET team_id = $2, summary = $3, mrr_delta_cents = $4, \
                 mrr_before_cents = $5 WHERE id = $1",
            )
            .bind(event_id)
            .bind(applied.team_id)
            .bind(&applied.summary)
            .bind(applied.mrr_delta_cents)
            .bind(applied.mrr_before_cents)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(Json(json!({})))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "whsec_test";
    const BODY: &[u8] = br#"{"id":"evt_1"}"#;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn sig(secret: &str, t: i64, body: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{t}.").as_bytes());
        mac.update(body);
        hex(&mac.finalize().into_bytes())
    }

    #[test]
    fn accepts_a_valid_signature() {
        let t = 1_800_000_000;
        let header = format!("t={t},v1={}", sig(SECRET, t, BODY));
        assert!(verify_signature(&header, BODY, SECRET, t));
        assert!(verify_signature(&header, BODY, SECRET, t + 300));
        assert!(verify_signature(&header, BODY, SECRET, t - 300));
    }

    #[test]
    fn rejects_stale_or_future_timestamps() {
        let t = 1_800_000_000;
        let header = format!("t={t},v1={}", sig(SECRET, t, BODY));
        assert!(!verify_signature(&header, BODY, SECRET, t + 301));
        assert!(!verify_signature(&header, BODY, SECRET, t - 301));
    }

    #[test]
    fn rejects_wrong_secret_body_and_timestamp() {
        let t = 1_800_000_000;
        let good = sig(SECRET, t, BODY);
        assert!(!verify_signature(
            &format!("t={t},v1={}", sig("other", t, BODY)),
            BODY,
            SECRET,
            t
        ));
        assert!(!verify_signature(
            &format!("t={t},v1={good}"),
            b"{}",
            SECRET,
            t
        ));
        // Signature made for another timestamp.
        assert!(!verify_signature(
            &format!("t={},v1={good}", t + 1),
            BODY,
            SECRET,
            t
        ));
    }

    #[test]
    fn rejects_malformed_headers() {
        let t = 1_800_000_000;
        let good = sig(SECRET, t, BODY);
        for header in [
            String::new(),
            "garbage".into(),
            format!("v1={good}"),
            format!("t={t}"),
            format!("t=abc,v1={good}"),
            format!("t={t},v1=zz"),
            format!("t={t},v1={}", &good[..good.len() - 1]),
            format!("t={t},v0={good}"),
        ] {
            assert!(!verify_signature(&header, BODY, SECRET, t), "{header}");
        }
    }

    #[test]
    fn any_of_several_v1_signatures_may_match() {
        let t = 1_800_000_000;
        let header = format!("t={t},v1=deadbeef,v1={}", sig(SECRET, t, BODY));
        assert!(verify_signature(&header, BODY, SECRET, t));
    }

    #[test]
    fn subscription_ids_come_from_the_2026_10_28_paths() {
        let session = json!({"id": "cs_1", "subscription": "sub_1"});
        assert_eq!(
            subscription_id("checkout.session.completed", &session).as_deref(),
            Some("sub_1")
        );
        let sub = json!({"id": "sub_2", "object": "subscription"});
        for kind in [
            "customer.subscription.created",
            "customer.subscription.updated",
            "customer.subscription.deleted",
        ] {
            assert_eq!(subscription_id(kind, &sub).as_deref(), Some("sub_2"));
        }
        let invoice = json!({
            "id": "in_1",
            "subscription": "sub_old_path",
            "parent": {"subscription_details": {"subscription": "sub_3"}}
        });
        for kind in ["invoice.paid", "invoice.payment_failed"] {
            assert_eq!(subscription_id(kind, &invoice).as_deref(), Some("sub_3"));
        }
        // The removed top-level field is not read.
        let old = json!({"id": "in_1", "subscription": "sub_old_path"});
        assert_eq!(subscription_id("invoice.paid", &old), None);
        // One-off invoices and checkout in payment mode have none.
        assert_eq!(
            subscription_id("invoice.paid", &json!({"parent": null})),
            None
        );
        assert_eq!(
            subscription_id("checkout.session.completed", &json!({"subscription": null})),
            None
        );
        assert_eq!(subscription_id("customer.created", &sub), None);
    }
}
