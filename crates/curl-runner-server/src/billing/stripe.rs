//! The Stripe REST client. Form-encoded requests, bearer auth, a pinned API
//! version. The secret key is never logged: errors carry the response body
//! with the key scrubbed, and `Debug` redacts it.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Method;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::{Billing, BillingError, CheckoutRequest, Price, Subscription};

/// The Stripe API version every request is made against.
pub const STRIPE_VERSION: &str = "2026-10-28.endive";

const API_BASE: &str = "https://api.stripe.com";
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_BODY_CHARS: usize = 500;

pub struct StripeBilling {
    http: reqwest::Client,
    secret_key: String,
    base_url: String,
}

impl fmt::Debug for StripeBilling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripeBilling")
            .field("secret_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .finish()
    }
}

type Form = Vec<(String, String)>;

fn pair(key: &str, value: impl ToString) -> (String, String) {
    (key.to_owned(), value.to_string())
}

impl StripeBilling {
    pub fn new(secret_key: String) -> Self {
        Self::with_base_url(secret_key, API_BASE)
    }

    /// For tests, which point it at a local stub.
    pub fn with_base_url(secret_key: String, base_url: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .expect("reqwest client"),
            secret_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        form: Option<Form>,
    ) -> Result<T, BillingError> {
        let mut req = self
            .http
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(&self.secret_key)
            .header("Stripe-Version", STRIPE_VERSION);
        if let Some(form) = form {
            req = req.form(&form);
        }
        let res = req
            .send()
            .await
            .map_err(|e| BillingError::Unreachable(self.scrub(&e.without_url().to_string())))?;
        let status = res.status();
        let body = res
            .text()
            .await
            .map_err(|e| BillingError::Unreachable(self.scrub(&e.without_url().to_string())))?;
        if !status.is_success() {
            return Err(BillingError::Api {
                status: status.as_u16(),
                body: self.scrub(&body),
            });
        }
        serde_json::from_str(&body)
            .map_err(|e| BillingError::Unreachable(format!("unexpected response: {e}")))
    }

    /// Drop the key from text that may be logged, and cap its length.
    fn scrub(&self, text: &str) -> String {
        let text = text.replace(&self.secret_key, "<redacted>");
        text.chars().take(MAX_BODY_CHARS).collect()
    }
}

#[derive(Deserialize)]
struct UrlResponse {
    url: String,
}

#[derive(Deserialize)]
struct SubscriptionResponse {
    id: String,
    customer: String,
    status: String,
    cancel_at: Option<i64>,
    #[serde(default)]
    metadata: std::collections::HashMap<String, String>,
    items: Items,
}

#[derive(Deserialize)]
struct Items {
    data: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    quantity: Option<u32>,
    /// With the pinned API version the period lives on the item.
    current_period_end: Option<i64>,
    price: PriceId,
}

#[derive(Deserialize)]
struct PriceId {
    id: String,
}

#[derive(Deserialize)]
struct PriceResponse {
    unit_amount: Option<i64>,
    currency: String,
    recurring: Option<Recurring>,
}

#[derive(Deserialize)]
struct InvoiceList {
    data: Vec<InvoiceId>,
}

#[derive(Deserialize)]
struct InvoiceId {
    id: String,
}

#[derive(Deserialize)]
struct Recurring {
    interval: String,
}

impl SubscriptionResponse {
    fn into_subscription(self) -> Result<Subscription, BillingError> {
        let item = self
            .items
            .data
            .into_iter()
            .next()
            .ok_or_else(|| BillingError::Unreachable("subscription has no items".to_owned()))?;
        Ok(Subscription {
            id: self.id,
            customer: self.customer,
            status: self.status,
            price: item.price.id,
            quantity: item.quantity.unwrap_or(1),
            team_id: self
                .metadata
                .get("team_id")
                .and_then(|v| v.parse::<Uuid>().ok()),
            cancel_at: self.cancel_at,
            current_period_end: item.current_period_end,
        })
    }
}

/// The form body of `POST /v1/checkout/sessions`.
fn checkout_form(req: &CheckoutRequest) -> Form {
    let mut form = vec![
        pair("mode", "subscription"),
        pair("line_items[0][price]", &req.price),
        pair("line_items[0][quantity]", req.quantity),
        pair("client_reference_id", req.team_id),
        pair("subscription_data[metadata][team_id]", req.team_id),
        pair("success_url", &req.success_url),
        pair("cancel_url", &req.cancel_url),
        pair("automatic_tax[enabled]", "true"),
        pair("tax_id_collection[enabled]", "true"),
        pair("billing_address_collection", "required"),
        pair("allow_promotion_codes", "true"),
    ];
    match &req.customer {
        Some(customer) => {
            form.push(pair("customer", customer));
            form.push(pair("customer_update[address]", "auto"));
            form.push(pair("customer_update[name]", "auto"));
        }
        None => form.push(pair("customer_email", &req.email)),
    }
    form
}

#[async_trait]
impl Billing for StripeBilling {
    async fn checkout_url(&self, req: CheckoutRequest) -> Result<String, BillingError> {
        let session: UrlResponse = self
            .call(
                Method::POST,
                "/v1/checkout/sessions",
                Some(checkout_form(&req)),
            )
            .await?;
        Ok(session.url)
    }

    async fn portal_url(&self, customer: &str, return_url: &str) -> Result<String, BillingError> {
        let form = vec![pair("customer", customer), pair("return_url", return_url)];
        let session: UrlResponse = self
            .call(Method::POST, "/v1/billing_portal/sessions", Some(form))
            .await?;
        Ok(session.url)
    }

    async fn subscription(&self, id: &str) -> Result<Subscription, BillingError> {
        let res: SubscriptionResponse = self
            .call(Method::GET, &format!("/v1/subscriptions/{id}"), None)
            .await?;
        res.into_subscription()
    }

    async fn set_quantity(&self, subscription: &str, quantity: u32) -> Result<(), BillingError> {
        let path = format!("/v1/subscriptions/{subscription}");
        let current: SubscriptionResponse = self.call(Method::GET, &path, None).await?;
        let item = current
            .items
            .data
            .first()
            .ok_or_else(|| BillingError::Unreachable("subscription has no items".to_owned()))?;
        let form = vec![
            pair("items[0][id]", &item.id),
            pair("items[0][quantity]", quantity),
            pair("proration_behavior", "create_prorations"),
        ];
        let _: serde_json::Value = self.call(Method::POST, &path, Some(form)).await?;
        Ok(())
    }

    async fn cancel_subscription(&self, id: &str) -> Result<(), BillingError> {
        let _: serde_json::Value = self
            .call(Method::DELETE, &format!("/v1/subscriptions/{id}"), None)
            .await?;
        Ok(())
    }

    async fn price(&self, id: &str) -> Result<Price, BillingError> {
        let res: PriceResponse = self
            .call(Method::GET, &format!("/v1/prices/{id}"), None)
            .await?;
        let unit_amount = res
            .unit_amount
            .ok_or_else(|| BillingError::Unreachable("price has no unit_amount".to_owned()))?;
        Ok(Price {
            unit_amount,
            currency: res.currency,
            interval: res.recurring.map(|r| r.interval).unwrap_or_default(),
        })
    }

    async fn send_latest_invoice(&self, customer: &str) -> Result<Option<String>, BillingError> {
        // Only for its form encoding of the query.
        let mut url = reqwest::Url::parse("http://stripe.invalid/v1/invoices").expect("static url");
        // Open invoices only: drafts can't be sent, and paid ones are receipts.
        url.query_pairs_mut()
            .append_pair("customer", customer)
            .append_pair("status", "open")
            .append_pair("limit", "1");
        let path = format!("{}?{}", url.path(), url.query().unwrap_or_default());
        let list: InvoiceList = self.call(Method::GET, &path, None).await?;
        let Some(invoice) = list.data.into_iter().next() else {
            return Ok(None);
        };
        let path = format!("/v1/invoices/{}/send", invoice.id);
        let _: serde_json::Value = self.call(Method::POST, &path, Some(Vec::new())).await?;
        Ok(Some(invoice.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use testkit::{Request, Response, TestServer};

    type Seen = Arc<Mutex<Vec<(String, String, String, Vec<(String, String)>)>>>;

    /// A stub that records `(method, url, body, headers)` and answers `reply(method, url)`.
    fn stub(
        reply: impl Fn(&str, &str) -> (u16, String) + Send + Sync + 'static,
    ) -> (TestServer, Seen) {
        let seen: Seen = Arc::default();
        let log = seen.clone();
        let server = TestServer::with_handler(move |req: Request| {
            log.lock().unwrap().push((
                req.method.clone(),
                req.url.clone(),
                String::from_utf8_lossy(&req.body).into_owned(),
                req.headers.clone(),
            ));
            let (status, body) = reply(&req.method, &req.url);
            Response::from_data(body.into_bytes())
                .with_status_code(status)
                .with_header("Content-Type", "application/json")
        });
        (server, seen)
    }

    fn request(customer: Option<&str>) -> CheckoutRequest {
        CheckoutRequest {
            team_id: Uuid::nil(),
            team_slug: "acme".into(),
            customer: customer.map(str::to_owned),
            email: "ada@example.com".into(),
            price: "price_m".into(),
            quantity: 3,
            success_url: "https://app.test/ok".into(),
            cancel_url: "https://app.test/no".into(),
        }
    }

    #[tokio::test]
    async fn checkout_is_form_encoded_with_version_and_key() {
        let (server, seen) = stub(|_, _| (200, r#"{"url":"https://checkout.test/x"}"#.into()));
        let billing = StripeBilling::with_base_url("sk_test_abc".into(), &server.base_url);
        let url = billing.checkout_url(request(None)).await.unwrap();
        assert_eq!(url, "https://checkout.test/x");
        let (method, path, body, headers) = seen.lock().unwrap()[0].clone();
        assert_eq!(
            (method.as_str(), path.as_str()),
            ("POST", "/v1/checkout/sessions")
        );
        assert!(body.contains("client_reference_id="), "{body}");
        assert!(body.contains("automatic_tax%5Benabled%5D=true"), "{body}");
        assert!(body.contains("line_items%5B0%5D%5Bquantity%5D=3"), "{body}");
        assert!(body.contains("customer_email=ada%40example.com"), "{body}");
        assert!(!body.contains("customer_update"), "{body}");
        let header = |name: &str| {
            headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(header("authorization"), Some("Bearer sk_test_abc"));
        assert_eq!(header("stripe-version"), Some(STRIPE_VERSION));
    }

    #[tokio::test]
    async fn checkout_with_customer_updates_address_and_name() {
        let (server, seen) = stub(|_, _| (200, r#"{"url":"u"}"#.into()));
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        billing.checkout_url(request(Some("cus_1"))).await.unwrap();
        let body = seen.lock().unwrap()[0].2.clone();
        assert!(body.contains("customer=cus_1"), "{body}");
        assert!(body.contains("customer_update%5Baddress%5D=auto"), "{body}");
        assert!(body.contains("customer_update%5Bname%5D=auto"), "{body}");
        assert!(!body.contains("customer_email"), "{body}");
    }

    const SUB: &str = r#"{"id":"sub_1","customer":"cus_1","status":"active","cancel_at":1800000000,
        "metadata":{"team_id":"00000000-0000-0000-0000-000000000000"},
        "items":{"data":[{"id":"si_1","quantity":4,"current_period_end":1793000000,
        "price":{"id":"price_m"}}]}}"#;

    #[tokio::test]
    async fn subscription_reads_period_end_from_the_item() {
        let (server, _) = stub(|_, _| (200, SUB.into()));
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        let sub = billing.subscription("sub_1").await.unwrap();
        assert_eq!(sub.current_period_end, Some(1_793_000_000));
        assert_eq!(sub.cancel_at, Some(1_800_000_000));
        assert_eq!((sub.quantity, sub.price.as_str()), (4, "price_m"));
        assert_eq!(sub.team_id, Some(Uuid::nil()));
    }

    #[tokio::test]
    async fn set_quantity_prorates_the_first_item() {
        let (server, seen) = stub(|_, _| (200, SUB.into()));
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        billing.set_quantity("sub_1", 5).await.unwrap();
        let (method, path, body, _) = seen.lock().unwrap()[1].clone();
        assert_eq!(
            (method.as_str(), path.as_str()),
            ("POST", "/v1/subscriptions/sub_1")
        );
        assert!(body.contains("items%5B0%5D%5Bid%5D=si_1"), "{body}");
        assert!(body.contains("items%5B0%5D%5Bquantity%5D=5"), "{body}");
        assert!(
            body.contains("proration_behavior=create_prorations"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn cancel_deletes_the_subscription() {
        let (server, seen) = stub(|_, _| (200, "{}".into()));
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        billing.cancel_subscription("sub_9").await.unwrap();
        let (method, path, ..) = seen.lock().unwrap()[0].clone();
        assert_eq!(
            (method.as_str(), path.as_str()),
            ("DELETE", "/v1/subscriptions/sub_9")
        );
    }

    #[tokio::test]
    async fn send_latest_invoice_lists_one_then_sends_it() {
        let (server, seen) = stub(|method, _| match method {
            "GET" => (200, r#"{"data":[{"id":"in_7"}]}"#.into()),
            _ => (200, r#"{"id":"in_7"}"#.into()),
        });
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        let sent = billing.send_latest_invoice("cus_1").await.unwrap();
        assert_eq!(sent.as_deref(), Some("in_7"));
        let seen = seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            (seen[0].0.as_str(), seen[0].1.as_str()),
            ("GET", "/v1/invoices?customer=cus_1&status=open&limit=1")
        );
        assert_eq!(
            (seen[1].0.as_str(), seen[1].1.as_str()),
            ("POST", "/v1/invoices/in_7/send")
        );
    }

    #[tokio::test]
    async fn send_latest_invoice_without_invoices_sends_nothing() {
        let (server, seen) = stub(|_, _| (200, r#"{"data":[]}"#.into()));
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        assert_eq!(billing.send_latest_invoice("cus_1").await.unwrap(), None);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn send_latest_invoice_encodes_the_customer() {
        let (server, seen) = stub(|_, _| (200, r#"{"data":[]}"#.into()));
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        billing.send_latest_invoice("cus_1&limit=99").await.unwrap();
        let url = seen.lock().unwrap()[0].1.clone();
        assert_eq!(
            url,
            "/v1/invoices?customer=cus_1%26limit%3D99&status=open&limit=1"
        );
    }

    #[tokio::test]
    async fn error_body_is_kept_without_the_key() {
        let (server, _) = stub(|_, _| {
            (
                401,
                r#"{"error":{"message":"Invalid API Key provided: sk_test_abc"}}"#.into(),
            )
        });
        let billing = StripeBilling::with_base_url("sk_test_abc".into(), &server.base_url);
        let err = billing.price("price_m").await.unwrap_err();
        let BillingError::Api { status, body } = &err else {
            panic!("{err:?}")
        };
        assert_eq!(*status, 401);
        assert!(body.contains("Invalid API Key"), "{body}");
        assert!(!body.contains("sk_test_abc"), "{body}");
        assert!(!format!("{billing:?}").contains("sk_test_abc"));
    }

    #[tokio::test]
    async fn price_reads_amount_currency_and_interval() {
        let (server, _) = stub(|_, _| {
            (
                200,
                r#"{"unit_amount":900,"currency":"usd","recurring":{"interval":"month"}}"#.into(),
            )
        });
        let billing = StripeBilling::with_base_url("k".into(), &server.base_url);
        let price = billing.price("price_m").await.unwrap();
        assert_eq!(
            price,
            Price {
                unit_amount: 900,
                currency: "usd".into(),
                interval: "month".into()
            }
        );
    }
}
