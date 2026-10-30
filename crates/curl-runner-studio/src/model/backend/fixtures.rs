//! The mock's fixed data: the accounts, and the Shipping API collection Acme
//! shares (boards OpenShared and ReadOnly). Placeholders only.

use super::{Account, SharedVar, SignInMethod, SyncFile};

/// The cloud server's address (fake).
pub const CLOUD_URL: &str = "https://cloud.curl-runner.com";
/// The version every mocked server reports (board SignIn).
pub const SERVER_VERSION: &str = "2.1.1";
/// What `Test connection` measures (board SignIn).
pub const CONNECTION_MS: u64 = 42;
/// Where web share links point.
pub const SHARE_HOST: &str = "https://studio.example.com";

/// The teammate who pushes the conflicting change (board SyncConflict).
pub const CONFLICT_AUTHOR: &str = "Sam";

/// The org that owns the shared fixture.
pub const SHIPPING_ORG: &str = "Acme";
pub const SHIPPING_NAME: &str = "Shipping API";
/// The fixture's server id ([`super::Backend::fetch_shared`]).
pub const SHIPPING_ID: &str = "acme/shipping-api";
pub const SHIPPING_OWNER_SLUG: &str = "acme";
pub const SHIPPING_SLUG: &str = "shipping-api";
/// The token of the fixture's link (board OpenShared).
pub const SHIPPING_TOKEN: &str = "7Kx2Qm";
/// The folders the summary card names (board OpenShared).
pub const SHIPPING_SUMMARY: &str = "rates, labels, tracking";
/// How long ago Acme last pushed it.
pub const SHIPPING_UPDATED_DAYS_AGO: i64 = 3;

/// The account a sign-in gets: Jamie (owner at Acme) with SSO or GitHub,
/// Maria (no org, the customer on the shared boards) with email.
pub fn account(method: SignInMethod) -> Account {
    match method {
        SignInMethod::Sso | SignInMethod::GitHub => Account {
            name: "Jamie Rivera".into(),
            email: "jamie@acme.dev".into(),
            org: Some("Acme".into()),
            role: "Owner".into(),
            provider: method,
        },
        SignInMethod::Email => Account {
            name: "Maria".into(),
            email: "maria@example.com".into(),
            org: None,
            role: "Member".into(),
            provider: method,
        },
    }
}

/// The variables the Shipping API asks for (board OpenShared), required
/// first.
pub fn shipping_variables() -> Vec<SharedVar> {
    vec![
        SharedVar {
            name: "BASE_URL".into(),
            description: "Suggested by Acme for Production. You can change it.".into(),
            required: true,
            secret: false,
            default: Some("https://api.example.com/shipping".into()),
        },
        SharedVar {
            name: "API_TOKEN".into(),
            description: "Your own token from your Acme dashboard.".into(),
            required: true,
            secret: true,
            default: None,
        },
        SharedVar {
            name: "ACCOUNT_ID".into(),
            description: "Only used by the labels requests.".into(),
            required: false,
            secret: false,
            default: None,
        },
    ]
}

/// The Shipping API's 12 request files.
pub fn shipping_snapshot() -> Vec<SyncFile> {
    SHIPPING_FILES
        .iter()
        .map(|&(rel, text)| SyncFile::new(rel, text))
        .collect()
}

/// `rates/get-rates.yaml` is the ReadOnly board's editor, line for line.
const SHIPPING_FILES: [(&str, &str); 12] = [
    (
        "rates/get-rates.yaml",
        r#"# Quote rates for a parcel across carriers
request:
  name: Get rates
  url: ${BASE_URL}/v2/rates
  method: POST
  headers:
    Authorization: Bearer ${API_TOKEN}
  body:
    from_zip: "10001"
    to_zip: "94105"
    weight_kg: 1.2
  expect:
    status: 200
    body:
      rates: "length > 0"
"#,
    ),
    (
        "rates/list-carriers.yaml",
        r"# The carriers that can quote this account
request:
  name: List carriers
  url: ${BASE_URL}/v2/carriers
  method: GET
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 200
",
    ),
    (
        "rates/validate-address.yaml",
        r#"# Check an address before quoting
request:
  name: Validate address
  url: ${BASE_URL}/v2/addresses/validate
  method: POST
  headers:
    Authorization: Bearer ${API_TOKEN}
  body:
    line1: 1 Market St
    city: San Francisco
    zip: "94105"
    country: US
  expect:
    status: 200
"#,
    ),
    (
        "labels/create-label.yaml",
        r"# Buy a label for a quoted rate
request:
  name: Create label
  url: ${BASE_URL}/v2/accounts/${ACCOUNT_ID}/labels
  method: POST
  headers:
    Authorization: Bearer ${API_TOKEN}
  body:
    carrier: ground
    weight_kg: 1.2
  expect:
    status: 201
",
    ),
    (
        "labels/get-label.yaml",
        r"request:
  name: Get label
  url: ${BASE_URL}/v2/accounts/${ACCOUNT_ID}/labels/lbl_1042
  method: GET
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 200
",
    ),
    (
        "labels/list-labels.yaml",
        r"request:
  name: List labels
  url: ${BASE_URL}/v2/accounts/${ACCOUNT_ID}/labels
  method: GET
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 200
",
    ),
    (
        "labels/void-label.yaml",
        r"# Void a label that was never used
request:
  name: Void label
  url: ${BASE_URL}/v2/accounts/${ACCOUNT_ID}/labels/lbl_1042
  method: DELETE
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 204
",
    ),
    (
        "tracking/get-tracking.yaml",
        r"request:
  name: Get tracking
  url: ${BASE_URL}/v2/tracking/1Z999
  method: GET
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 200
",
    ),
    (
        "tracking/list-events.yaml",
        r"request:
  name: List tracking events
  url: ${BASE_URL}/v2/tracking/1Z999/events
  method: GET
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 200
",
    ),
    (
        "tracking/subscribe.yaml",
        r"# Get pushed updates for a parcel
request:
  name: Subscribe to tracking
  url: ${BASE_URL}/v2/tracking/1Z999/subscriptions
  method: POST
  headers:
    Authorization: Bearer ${API_TOKEN}
  body:
    url: https://hooks.example.com/tracking
  expect:
    status: 201
",
    ),
    (
        "webhooks/create-webhook.yaml",
        r"request:
  name: Create webhook
  url: ${BASE_URL}/v2/webhooks
  method: POST
  headers:
    Authorization: Bearer ${API_TOKEN}
  body:
    url: https://hooks.example.com/shipping
    events:
      - label.created
      - tracking.updated
  expect:
    status: 201
",
    ),
    (
        "webhooks/delete-webhook.yaml",
        r"request:
  name: Delete webhook
  url: ${BASE_URL}/v2/webhooks/wh_7
  method: DELETE
  headers:
    Authorization: Bearer ${API_TOKEN}
  expect:
    status: 204
",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::lint::{LintOutcome, Severity, lint_text};

    #[test]
    fn fixture_snapshot_parses_as_curl_runner_yaml() {
        let files = shipping_snapshot();
        assert_eq!(files.len(), 12);
        for top in ["rates/", "labels/", "tracking/", "webhooks/"] {
            assert!(files.iter().any(|f| f.rel.starts_with(top)), "{top}");
        }
        for f in &files {
            assert!(f.text.contains("${BASE_URL}"), "{}", f.rel.display());
            assert!(f.text.contains("${API_TOKEN}"), "{}", f.rel.display());
            match lint_text(&f.text).expect("no crash") {
                LintOutcome::ParseError(e) => panic!("{}: {e}", f.rel.display()),
                LintOutcome::Linted { issues, .. } => {
                    let errors: Vec<_> = issues
                        .iter()
                        .filter(|i| i.severity == Severity::Error)
                        .collect();
                    assert!(errors.is_empty(), "{}: {errors:?}", f.rel.display());
                }
            }
        }
    }

    #[test]
    fn variables_match_the_board() {
        let vars = shipping_variables();
        let names: Vec<_> = vars.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["BASE_URL", "API_TOKEN", "ACCOUNT_ID"]);
        assert_eq!(
            vars[0].default.as_deref(),
            Some("https://api.example.com/shipping")
        );
        assert!(vars[0].required && vars[1].required && !vars[2].required);
        assert!(vars[1].secret && vars[1].default.is_none());
    }
}
