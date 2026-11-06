//! A round trip against a running curl-runner Cloud server. Ignored by
//! default; run it by hand with:
//!
//! ```sh
//! CURL_RUNNER_LIVE_SERVER=http://localhost:8090 \
//! CURL_RUNNER_LIVE_PUBLIC='curl-runner://open?server=…&share=…' \
//! CURL_RUNNER_LIVE_RESTRICTED='curl-runner://open?server=…&share=…' \
//! CURL_RUNNER_LIVE_HANDOFF=crh_… \
//! cargo test -p curl-runner-studio --features server live_server_round_trip -- --ignored --nocapture
//! ```
//!
//! - `CURL_RUNNER_LIVE_SERVER`: the API URL.
//! - `CURL_RUNNER_LIVE_PUBLIC`: a public share link on it (deep-link form).
//! - `CURL_RUNNER_LIVE_RESTRICTED`: a restricted share link on it.
//! - `CURL_RUNNER_LIVE_HANDOFF`: a fresh `crh_` code, minted for an account
//!   allowed on the restricted collection. Codes are single use.
//!
//! Any of them missing: the test says so and passes. It prints names and
//! versions only, never a token or a code.

use super::*;
use crate::model::backend::{AuthCallback, ReceivedShare, Server, ShareLink, SyncFile};
use crate::model::secrets::MemorySecrets;
use crate::model::sessions::MemorySessions;
use futures::executor::block_on;

const VARS: [&str; 4] = [
    "CURL_RUNNER_LIVE_SERVER",
    "CURL_RUNNER_LIVE_PUBLIC",
    "CURL_RUNNER_LIVE_RESTRICTED",
    "CURL_RUNNER_LIVE_HANDOFF",
];

#[test]
#[ignore = "needs a running server; see the doc comment"]
fn live_server_round_trip() {
    let values: Vec<String> = VARS
        .iter()
        .filter_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
        .collect();
    let [server, public, restricted, handoff] =
        <[String; 4]>::try_from(values).unwrap_or_else(|_| {
            eprintln!("skipped: set {} to run it", VARS.join(", "));
            Default::default()
        });
    if server.is_empty() {
        return;
    }
    let state = tempfile::tempdir().unwrap();
    let sessions = Arc::new(MemorySessions::new(
        MemorySecrets::default(),
        Some(state.path()),
    ));
    let b = HttpBackend::open(Some(state.path()), sessions);

    // Public: no sign-in.
    let public = ShareLink::parse(&public).expect("CURL_RUNNER_LIVE_PUBLIC parses");
    let shared = block_on(b.resolve_link(&public)).expect("the public link resolves");
    assert!(!shared.name.is_empty(), "it has a name");
    assert!(shared.version >= 1, "it has a version");
    eprintln!("public: {} v{}", shared.name, shared.version);
    let snap = block_on(b.fetch_shared(&public)).expect("the public link fetches");
    assert!(!snap.files.is_empty(), "the snapshot has files");
    assert_eq!(snap.version, shared.version);
    let latest = block_on(b.latest_version(&public)).expect("latest version");
    assert_eq!(latest, shared.version, "latest is the version it opened");

    // Restricted: signed out first.
    let restricted = ShareLink::parse(&restricted).expect("CURL_RUNNER_LIVE_RESTRICTED parses");
    assert_eq!(
        block_on(b.resolve_link(&restricted)).map(|s| s.name),
        Err(BackendError::SignedOut),
        "a restricted link asks for sign-in"
    );

    // Sign in the way the studio does: start, then the callback.
    let url = block_on(b.sign_in_url(&Server::SelfHosted {
        url: server.clone(),
    }))
    .expect("the sign-in page");
    assert!(url.ends_with("/sign-in?client=studio"), "{url}");
    let account = block_on(b.complete_sign_in(&AuthCallback {
        server: server.clone(),
        code: handoff,
    }))
    .expect("the handoff code redeems");
    assert!(account.email.contains('@'), "an account email");
    eprintln!("signed in as {}", account.email);

    let shared = block_on(b.resolve_link(&restricted)).expect("the restricted link resolves");
    let snap = block_on(b.fetch_shared(&restricted)).expect("the restricted link fetches");
    assert!(!snap.files.is_empty(), "the restricted snapshot has files");
    eprintln!("restricted: {} v{}", shared.name, shared.version);

    // Added on this Mac: kept in the state folder, keyed by token.
    let record = ReceivedShare {
        link: restricted.clone(),
        team: shared.owner_org.clone(),
        name: shared.name.clone(),
        version: snap.version,
        added: chrono::Utc::now().date_naive(),
        files: snap
            .files
            .iter()
            .map(|f| (f.rel.clone(), SyncFile::hash_of(&f.text)))
            .collect(),
    };
    block_on(b.add_received(record.clone())).expect("add_received");
    assert_eq!(b.received(), [record]);
}
