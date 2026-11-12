use super::*;
use crate::model::backend::{FOREIGN_CALLBACK, shared_vars, sync_files, testing};
use crate::model::collection_meta::{CollectionMeta, VarMeta};
use crate::model::secrets::{MemorySecrets, SecretStore, service_name};
use futures::executor::block_on;

fn mock(base: &Path) -> MockBackend {
    MockBackend::open(Some(base), Duration::ZERO)
}

fn dev(b: &MockBackend) -> &dyn DevControls {
    b.dev().expect("the mock has dev controls")
}

fn key() -> CollectionKey {
    CollectionKey {
        id: "a1b2c3d4".into(),
        name: "Payments API".into(),
    }
}

fn signed_in_mock(base: &Path) -> MockBackend {
    let b = mock(base);
    testing::sign_in(&b);
    b
}

#[test]
fn sign_in_then_account_persists_across_open() {
    let d = tempfile::tempdir().unwrap();
    let b = mock(d.path());
    assert_eq!(b.account(), None);
    assert_eq!(b.server(), Server::Cloud);
    let account = testing::sign_in(&b);
    assert_eq!(account.name, "Jamie Rivera");
    assert_eq!(account.email, "jamie@acme.dev");
    assert_eq!(account.provider, Some(SignInMethod::Github));
    drop(b);
    let again = mock(d.path());
    assert_eq!(again.account(), Some(account));
    testing::sign_out(&again);
    assert_eq!(mock(d.path()).account(), None);
}

#[test]
fn switching_server_keeps_sessions_and_persists() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let own = Server::SelfHosted {
        url: "https://curl-runner.example.com".into(),
    };
    let info = block_on(b.test_connection(&own)).unwrap();
    assert_eq!(
        (info.version.as_str(), info.sign_in, info.latency_ms),
        ("2.1.1", vec![SignInMethod::Email, SignInMethod::Github], 42)
    );
    let bad = Server::SelfHosted { url: "nope".into() };
    assert!(matches!(
        block_on(b.test_connection(&bad)),
        Err(BackendError::Invalid(_))
    ));
    block_on(b.set_server(own.clone())).unwrap();
    assert_eq!(b.account(), None, "no session on the new server");
    assert!(b.account_on(Server::Cloud.url()).is_some(), "Cloud's stays");
    assert_eq!(mock(d.path()).server(), own);
}

#[test]
fn fail_next_sign_in_fails_once() {
    let d = tempfile::tempdir().unwrap();
    let b = mock(d.path());
    dev(&b).fail_next_sign_in(true);
    assert!(dev(&b).toggles().fail_next_sign_in);
    let url = block_on(b.sign_in_url(&Server::Cloud)).unwrap();
    let cb = AuthCallback::parse(&url).unwrap();
    assert_eq!(
        block_on(b.complete_sign_in(&cb)),
        Err(BackendError::Unauthorized)
    );
    assert_eq!(b.account(), None);
    assert!(!dev(&b).toggles().fail_next_sign_in);
    testing::sign_in(&b);
}

#[test]
fn offline_fails_every_call_with_offline() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    dev(&b).set_offline(true);
    assert!(dev(&b).offline());
    assert!(mock(d.path()).offline(), "the toggle persists");
    let k = key();
    let link = ShareLink::parse(
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm",
    )
    .unwrap();
    let off = Err(BackendError::Offline);
    assert_eq!(block_on(b.set_server(Server::Cloud)), off);
    assert_eq!(block_on(b.test_connection(&Server::Cloud)).map(|_| ()), off);
    assert_eq!(block_on(b.sign_in_url(&Server::Cloud)).map(|_| ()), off);
    assert_eq!(block_on(b.sync(&k, &[], &[])).map(|_| ()), off);
    assert_eq!(block_on(b.share_settings(&k)).map(|_| ()), off);
    assert_eq!(
        block_on(b.update_share(&k, ShareUpdate::default())).map(|_| ()),
        off
    );
    assert_eq!(block_on(b.invite(&k, "maria@example.com")).map(|_| ()), off);
    assert_eq!(block_on(b.remove_person(&k, "maria@example.com")), off);
    assert_eq!(block_on(b.resolve_link(&link)).map(|_| ()), off);
    assert_eq!(block_on(b.fetch_shared(&link)).map(|_| ()), off);
    assert_eq!(
        block_on(b.latest_version(&link)),
        Err(BackendError::Offline)
    );
    assert_eq!(block_on(b.add_received(received(&link, 1))), off);
    assert!(b.account().is_some(), "local state is kept");
    assert!(b.received().is_empty());
    // Signing out is local first: it works offline.
    assert_eq!(block_on(b.sign_out(Server::Cloud.url())), Ok(()));
    assert_eq!(b.account(), None);
    assert_eq!(mock(d.path()).account(), None, "and persists");
}

#[test]
fn dev_toggles_survive_a_relaunch_only_in_debug_builds() {
    let state = State {
        toggles: DevToggles {
            offline: true,
            conflict_next_sync: true,
            expire_links: true,
            fail_next_sign_in: true,
        },
        ..State::default()
    };
    assert_eq!(keep_toggles(state.clone(), true).toggles, state.toggles);
    let release = keep_toggles(state.clone(), false);
    assert_eq!(release.toggles, DevToggles::default());
    assert_eq!(release.server, state.server, "only the toggles go");
}

#[test]
fn offline_mid_call_fails_that_call() {
    let d = tempfile::tempdir().unwrap();
    let b = MockBackend::open(Some(d.path()), Duration::from_millis(30));
    let started = Instant::now();
    let pending = b.sign_in_url(&Server::Cloud);
    dev(&b).set_offline(true);
    assert_eq!(block_on(pending), Err(BackendError::Offline));
    assert!(started.elapsed() >= Duration::from_millis(30), "it waited");
    assert_eq!(b.account(), None);
}

#[test]
fn zero_latency_is_ready_on_first_poll() {
    let b = MockBackend::open(None, Duration::ZERO);
    let mut fut = b.sign_in_url(&Server::Cloud);
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(Ok(url)) => assert!(url.starts_with("curl-runner://auth/callback?")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn share_link_round_trip_and_invite() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let k = key();
    let fresh = block_on(b.share_settings(&k)).unwrap();
    assert_eq!(fresh.link, None);
    assert_eq!(fresh.people.len(), 1);
    assert_eq!(fresh.people[0].role, Role::Owner);

    let shared = block_on(b.update_share(&k, ShareUpdate::default())).unwrap();
    let url = shared.link.clone().expect("a link");
    assert!(
        url.starts_with("curl-runner://open?server=https://api.mock.curl-runner.test&share=crsl_"),
        "{url}"
    );
    assert_eq!(shared.access, Access::AnyoneWithLink);
    let in_30 = Utc::now().date_naive() + chrono::Duration::days(30);
    assert_eq!(shared.expires, Some(in_30));

    // The link resolves to the collection, persisted across open.
    let b = mock(d.path());
    let link = ShareLink::parse(&url).unwrap();
    let resolved = block_on(b.resolve_link(&link)).unwrap();
    assert_eq!(resolved.name, "Payments API");
    assert_eq!(resolved.owner_org, "Acme");
    assert_eq!(resolved.link, link);
    assert_eq!(resolved.access, LinkAccess::Public);

    let person = block_on(b.invite(&k, " maria@example.com ")).unwrap();
    assert_eq!(person.name, "Maria");
    assert_eq!(person.email, "maria@example.com");
    assert_eq!((person.role, person.invited), (Role::Viewer, true));
    assert!(matches!(
        block_on(b.invite(&k, "maria@example.com")),
        Err(BackendError::Invalid(_))
    ));
    assert!(matches!(
        block_on(b.invite(&k, "not an email")),
        Err(BackendError::Invalid(_))
    ));
    let people = block_on(b.share_settings(&k)).unwrap().people;
    assert_eq!(people.len(), 2);

    block_on(b.remove_person(&k, "maria@example.com")).unwrap();
    assert_eq!(
        block_on(b.remove_person(&k, "maria@example.com")),
        Err(BackendError::NotFound)
    );
    assert!(matches!(
        block_on(b.remove_person(&k, "jamie@acme.dev")),
        Err(BackendError::Invalid(_))
    ));

    // Access, expiry and a new token.
    let update = ShareUpdate {
        access: Some(Access::InvitedOnly),
        expires: Some(Expiry::Never),
        reset_link: true,
        ..Default::default()
    };
    let changed = block_on(b.update_share(&k, update)).unwrap();
    assert_eq!(changed.access, Access::InvitedOnly);
    assert_eq!(changed.expires, None);
    assert_ne!(changed.link, Some(url));
    assert_eq!(
        block_on(b.resolve_link(&link)).map(|_| ()),
        Err(BackendError::NotFound),
        "the old link stops working"
    );
}

#[test]
fn signed_out_share_and_sync_say_signed_out() {
    let b = MockBackend::open(None, Duration::ZERO);
    let k = key();
    assert_eq!(
        block_on(b.share_settings(&k)).map(|_| ()),
        Err(BackendError::SignedOut)
    );
    assert_eq!(
        block_on(b.sync(&k, &[], &[])).map(|_| ()),
        Err(BackendError::SignedOut)
    );
    assert_eq!(b.sync_status(&k), SyncStatus::Off);
}

#[test]
fn fixture_link_resolves_and_fetches() {
    let b = MockBackend::open(None, Duration::ZERO);
    for text in [
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm",
        "curl-runner://open/acme/shipping-api?share=crsl_7Kx2Qm&server=https%3A%2F%2Fapi.mock.curl-runner.test",
    ] {
        let c = block_on(b.resolve_link(&ShareLink::parse(text).unwrap())).unwrap();
        assert_eq!(c.owner_org, "Acme");
        assert_eq!(c.name, "Shipping API");
        assert_eq!(c.summary, "rates, labels, tracking");
        assert_eq!(c.request_count, 12);
        assert_eq!(c.variables, fixtures::shipping_variables());
        let ago = Utc::now() - c.updated;
        assert_eq!(ago.num_days(), 3);
    }
    let fixture = ShareLink::parse(
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm",
    )
    .unwrap();
    let snap = block_on(b.fetch_shared(&fixture)).unwrap();
    assert_eq!(snap.files, fixtures::shipping_snapshot());
    assert_eq!(snap.variables, fixtures::shipping_variables());
    assert_eq!(snap.version, fixtures::SHIPPING_VERSION);
    assert_eq!(
        block_on(b.latest_version(&fixture)),
        Ok(fixtures::SHIPPING_VERSION)
    );
    let unknown = ShareLink::parse(
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_zzzzzz",
    )
    .unwrap();
    assert_eq!(
        block_on(b.fetch_shared(&unknown)).map(|_| ()),
        Err(BackendError::NotFound)
    );
    assert_eq!(
        block_on(b.resolve_link(&unknown)).map(|_| ()),
        Err(BackendError::NotFound)
    );
}

#[test]
fn expired_links_resolve_to_expired() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let own = block_on(b.update_share(&key(), ShareUpdate::default()))
        .unwrap()
        .link
        .unwrap();
    let fixture = ShareLink::parse(
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm",
    )
    .unwrap();
    let own = ShareLink::parse(&own).unwrap();
    dev(&b).expire_links(true);
    assert_eq!(
        block_on(b.resolve_link(&fixture)).map(|_| ()),
        Err(BackendError::Expired)
    );
    assert_eq!(
        block_on(b.resolve_link(&own)).map(|_| ()),
        Err(BackendError::Expired)
    );
    let garbage = ShareLink::parse(
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_zzzzzz",
    )
    .unwrap();
    assert_eq!(
        block_on(b.resolve_link(&garbage)).map(|_| ()),
        Err(BackendError::NotFound),
        "an unknown link stays unknown"
    );
    dev(&b).expire_links(false);
    assert!(block_on(b.resolve_link(&fixture)).is_ok());

    // A link past its date is expired without the toggle.
    let mut state = b.state();
    state.shares.get_mut("a1b2c3d4").unwrap().expires =
        Some(Utc::now().date_naive() - chrono::Duration::days(1));
    drop(state);
    assert_eq!(
        block_on(b.resolve_link(&own)).map(|_| ()),
        Err(BackendError::Expired)
    );
}

fn files(texts: &[(&str, &str)]) -> Vec<SyncFile> {
    texts.iter().map(|(r, t)| SyncFile::new(*r, *t)).collect()
}

#[test]
fn sync_pushes_and_reports_pending_counts() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let k = key();
    assert_eq!(b.sync_status(&k), SyncStatus::Off, "never synced");
    let mut local = files(&[
        ("charges/create.yaml", "request:\n  url: a\n"),
        ("charges/list.yaml", "request:\n  url: b\n"),
        ("auth/login.yaml", "request:\n  url: c\n"),
    ]);
    let report = block_on(b.sync(&k, &local, &[])).unwrap();
    assert_eq!(report.pushed, 3);
    assert!(report.pulled.is_empty() && report.conflicts.is_empty());
    let SyncStatus::Synced {
        collections,
        requests,
        ..
    } = b.sync_status(&k)
    else {
        panic!("{:?}", b.sync_status(&k));
    };
    assert_eq!((collections, requests), (1, 3));
    assert_eq!(
        block_on(b.sync(&k, &local, &[])).unwrap().pushed,
        0,
        "no change"
    );

    // Offline: the call fails and the server says only what it has (what
    // waits is the client's to count).
    let base = |f: &SyncFile| Some(f.hash);
    local[0] =
        SyncFile::new("charges/create.yaml", "request:\n  url: a2\n").with_base(base(&local[0]));
    local[1] =
        SyncFile::new("charges/list.yaml", "request:\n  url: b2\n").with_base(base(&local[1]));
    dev(&b).set_offline(true);
    assert_eq!(
        block_on(b.sync(&k, &local, &[])).map(|_| ()),
        Err(BackendError::Offline)
    );
    assert!(matches!(
        b.sync_status(&k),
        SyncStatus::Synced { requests: 3, .. }
    ));
    dev(&b).set_offline(false);
    assert_eq!(block_on(b.sync(&k, &local, &[])).unwrap().pushed, 2);
    assert!(matches!(b.sync_status(&k), SyncStatus::Synced { .. }));

    testing::sign_out(&b);
    assert_eq!(b.sync_status(&k), SyncStatus::Off);
}

fn received(link: &ShareLink, version: u32) -> ReceivedShare {
    ReceivedShare {
        link: link.clone(),
        team: "Acme".into(),
        name: "Shipping API".into(),
        version,
        added: Utc::now().date_naive(),
        files: BTreeMap::new(),
    }
}

#[test]
fn received_records_are_kept_by_token() {
    let d = tempfile::tempdir().unwrap();
    let b = mock(d.path());
    let link = ShareLink {
        server: MOCK_SERVER.into(),
        token: fixtures::SHIPPING_TOKEN.into(),
    };
    block_on(b.add_received(received(&link, 1))).unwrap();
    block_on(b.add_received(received(&link, 2))).unwrap();
    assert_eq!(b.received(), [received(&link, 2)]);
    block_on(b.update_received(received(&link, 3))).unwrap();
    assert_eq!(mock(d.path()).received(), [received(&link, 3)], "persisted");
}

#[test]
fn a_restricted_link_needs_a_session_on_its_server() {
    let b = signed_in_mock(tempfile::tempdir().unwrap().path());
    let update = ShareUpdate {
        access: Some(Access::InvitedOnly),
        ..Default::default()
    };
    let url = block_on(b.update_share(&key(), update))
        .unwrap()
        .link
        .unwrap();
    let link = ShareLink::parse(&url).unwrap();
    // Signed in to the studio's server, not the link's.
    let signed_out = Err(BackendError::SignedOut);
    assert_eq!(block_on(b.resolve_link(&link)).map(|_| ()), signed_out);
    assert_eq!(block_on(b.fetch_shared(&link)).map(|_| ()), signed_out);
    assert_eq!(
        block_on(b.latest_version(&link)),
        Err(BackendError::SignedOut)
    );
    block_on(b.sign_in_url(&Server::for_url(&link.server))).unwrap();
    block_on(b.complete_sign_in(&AuthCallback {
        server: link.server.clone(),
        code: fixtures::MOCK_CODE.into(),
    }))
    .unwrap();
    let c = block_on(b.resolve_link(&link)).unwrap();
    assert_eq!(c.access, LinkAccess::Restricted);
}

#[test]
fn a_file_the_server_already_has_is_left_alone_whatever_the_base() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let k = key();
    block_on(b.sync(&k, &files(&[("a.yaml", "x: 1\n")]), &[])).unwrap();
    // Another client pushes x: 2 on top of x: 1.
    let h = SyncFile::hash_of;
    let other = SyncFile::new("a.yaml", "x: 2\n").with_base(Some(h("x: 1\n")));
    assert_eq!(block_on(b.sync(&k, &[other], &[])).unwrap().pushed, 1);
    // This client made the same edit: server == local != base.
    let same = SyncFile::new("a.yaml", "x: 2\n").with_base(Some(h("x: 1\n")));
    let report = block_on(b.sync(&k, &[same], &[])).unwrap();
    assert_eq!(report, SyncReport::default(), "no push, pull nor conflict");
    assert_eq!(b.state().synced[&k.id].files["a.yaml"].text, "x: 2\n");
}

#[test]
fn conflict_next_sync_returns_one_conflict_with_sam_as_author() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let k = key();
    let base_text = "request:\n  name: Create charge\n  body:\n    amount: 2500\n";
    block_on(b.sync(
        &k,
        &files(&[("charges/create.yaml", base_text), ("a.yaml", "x: 1\n")]),
        &[],
    ))
    .unwrap();
    let edited = "request:\n  name: Create charge\n  body:\n    amount: 2600\n";
    let base = |t: &str| Some(SyncFile::hash_of(t));
    let local = vec![
        SyncFile::new("a.yaml", "x: 2\n").with_base(base("x: 1\n")),
        SyncFile::new("charges/create.yaml", edited).with_base(base(base_text)),
        SyncFile::new("z.yaml", "y: 1\n"),
    ];
    dev(&b).conflict_next_sync(true);
    let report = block_on(b.sync(&k, &local, &[])).unwrap();
    assert_eq!(report.conflicts.len(), 1);
    let c = &report.conflicts[0];
    assert_eq!(c.rel, PathBuf::from("a.yaml"), "the first changed file");
    assert_eq!(c.local, "x: 2\n");
    assert_eq!(c.server, "x: 4\n", "one value changed");
    assert_eq!(c.server_author, "Sam");
    let ago = Utc::now() - c.server_at;
    assert_eq!(ago.num_minutes(), 60);
    assert_eq!(report.pushed, 2, "the others still push");
    assert!(matches!(b.sync_status(&k), SyncStatus::Synced { .. }));
    assert!(!dev(&b).toggles().conflict_next_sync, "one shot");

    // Unresolved, the next sync reports it again and keeps theirs.
    let mut local: Vec<SyncFile> = local
        .into_iter()
        .map(|f| {
            let h = f.hash;
            f.with_base(Some(h))
        })
        .collect();
    local[0].base = base("x: 1\n");
    let still = block_on(b.sync(&k, &local, &[])).unwrap();
    assert_eq!((still.pushed, still.conflicts.len()), (0, 1));
    assert_eq!(still.conflicts[0].server, "x: 4\n");
    // "Keep mine" (the base becomes theirs): the next sync pushes mine.
    local[0].base = base("x: 4\n");
    let again = block_on(b.sync(&k, &local, &[])).unwrap();
    assert_eq!((again.pushed, again.conflicts.len()), (1, 0));
    assert!(matches!(b.sync_status(&k), SyncStatus::Synced { .. }));
}

#[test]
fn server_side_changes_one_value_deterministically() {
    let board = "request:\n  name: Create charge\n  body:\n    amount: 2500\n    currency: eur\n";
    let server = server_side(board);
    assert_eq!(server, board.replace("2500", "5000"));
    assert_eq!(server_side(board), server);
    assert_eq!(
        server_side("# note: 1\nrequest:\n  name: \"Get\"\n"),
        "# note: 1\nrequest:\n  name: \"Get (edited)\"\n"
    );
    assert_eq!(server_side("a:\n"), "a:\n# changed on the server\n");
    let get_rates = &fixtures::shipping_snapshot()[0].text;
    let changed = server_side(get_rates);
    assert_ne!(&changed, get_rates);
    let diff = get_rates
        .lines()
        .zip(changed.lines())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(diff, 1);
}

#[test]
fn shared_var_default_never_set_for_secret() {
    let b = MockBackend::open(None, Duration::ZERO);
    testing::sign_in(&b);
    let vars = vec![
        SharedVar {
            name: "API_TOKEN".into(),
            description: String::new(),
            required: true,
            secret: true,
            default: Some("leaked".into()),
            locked: false,
        },
        SharedVar {
            name: "BASE_URL".into(),
            description: String::new(),
            required: true,
            secret: false,
            default: Some("https://api.example.com".into()),
            locked: false,
        },
    ];
    let update = ShareUpdate {
        variables: Some(vars),
        ..Default::default()
    };
    let settings = block_on(b.update_share(&key(), update)).unwrap();
    assert_eq!(settings.variables[0].default, None);
    assert_eq!(
        settings.variables[1].default.as_deref(),
        Some("https://api.example.com")
    );

    // The payload builder never offers one either, even when asked.
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    std::fs::write(
        root.join("get.yaml"),
        "request:\n  url: ${BASE_URL}/x\n  headers:\n    Authorization: Bearer ${SECRET_KEY}\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".env"),
        "BASE_URL=https://api.example.com\nSECRET_KEY=plain\n",
    )
    .unwrap();
    let ask = ["BASE_URL".to_string(), "SECRET_KEY".to_string()];
    let vars = shared_vars(root, &CollectionMeta::default(), &ask);
    let secret = vars.iter().find(|v| v.name == "SECRET_KEY").unwrap();
    assert!(secret.secret);
    assert_eq!(secret.default, None);
    let base = vars.iter().find(|v| v.name == "BASE_URL").unwrap();
    assert_eq!(base.default.as_deref(), Some("https://api.example.com"));
    assert_eq!(
        shared_vars(root, &CollectionMeta::default(), &[])
            .iter()
            .find(|v| v.name == "BASE_URL")
            .unwrap()
            .default,
        None,
        "only when ticked"
    );
}

#[test]
fn corrupt_state_file_falls_back() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("mock-backend.json"), "{not json").unwrap();
    let b = mock(d.path());
    assert_eq!(*b.state(), State::default());
    assert_eq!(b.account(), None);
    // And recovers: the next save rewrites it.
    testing::sign_in(&b);
    assert!(mock(d.path()).account().is_some());
}

#[test]
fn reset_returns_to_first_launch() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    block_on(b.update_share(&key(), ShareUpdate::default())).unwrap();
    dev(&b).expire_links(true);
    dev(&b).reset();
    assert_eq!(*b.state(), State::default());
    assert_eq!(*mock(d.path()).state(), State::default());
}

#[test]
fn no_secret_reaches_backend() {
    const SECRET: &str = "sk-test-value";
    let data = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("payments");
    std::fs::create_dir_all(root.join("charges")).unwrap();
    std::fs::write(
        root.join("charges/create.yaml"),
        "request:\n  url: ${BASE_URL}/v1/charges\n  method: POST\n  headers:\n    Authorization: Bearer ${API_TOKEN}\n",
    )
    .unwrap();
    // A stray plain copy of the secret, too.
    std::fs::write(
        root.join(".env"),
        format!("BASE_URL=https://api.example.com\nAPI_TOKEN={SECRET}\n"),
    )
    .unwrap();

    let mut meta = CollectionMeta::load_or_init(Some(data.path()), &root);
    meta.set_secret("API_TOKEN", true);
    meta.vars.insert(
        "API_TOKEN".into(),
        VarMeta {
            description: "Your own token.".into(),
            required_for_shared: true,
            share_default: true,
            // Even locked: a secret never carries a lock (nor a default).
            share_locked: true,
        },
    );
    // A non-secret locked without a shared default carries no lock.
    meta.vars
        .entry("ACCOUNT_ID".into())
        .or_default()
        .share_locked = true;
    let keychain = MemorySecrets::default();
    let service = service_name("payments", "Default", &meta.id);
    keychain.set(&service, "API_TOKEN", SECRET).unwrap();
    assert_eq!(keychain.get(&service, "API_TOKEN").as_deref(), Some(SECRET));

    let b = signed_in_mock(data.path());
    let k = CollectionKey {
        id: meta.id.clone(),
        name: "payments".into(),
    };
    // The owner even ticks "Share default" on the secret.
    let ticked = ["BASE_URL".to_string(), "API_TOKEN".to_string()];
    let vars = shared_vars(&root, &meta, &ticked);
    for v in &vars {
        assert!(
            !v.locked,
            "{}: no lock without a shared non-secret default",
            v.name
        );
    }
    // A forged payload: the mock drops a lock with no default too.
    let mut forged = vars.clone();
    forged.push(SharedVar {
        name: "FORGED".into(),
        description: String::new(),
        required: false,
        secret: true,
        default: Some("forged-default".into()),
        locked: true,
    });
    let stored = block_on(b.update_share(
        &k,
        ShareUpdate {
            variables: Some(forged),
            ..Default::default()
        },
    ))
    .unwrap();
    let f = stored
        .variables
        .iter()
        .find(|v| v.name == "FORGED")
        .unwrap();
    assert!(f.default.is_none() && !f.locked, "{f:?}");
    let update = ShareUpdate {
        variables: Some(vars),
        ..Default::default()
    };
    let settings = block_on(b.update_share(&k, update)).unwrap();
    block_on(b.share_settings(&k)).unwrap();
    block_on(b.invite(&k, "maria@example.com")).unwrap();
    let local = sync_files(&root);
    assert_eq!(local.len(), 1);
    block_on(b.sync(&k, &local, &[])).unwrap();
    dev(&b).conflict_next_sync(true);
    block_on(b.sync(&k, &local, &[])).unwrap();
    let link = ShareLink::parse(settings.link.as_deref().unwrap()).unwrap();
    let shared = block_on(b.resolve_link(&link)).unwrap();
    block_on(b.fetch_shared(&shared.link)).unwrap();

    let recorded = dev(&b).recorded();
    assert!(recorded.len() >= 7, "{recorded:?}");
    let sync = recorded.iter().find(|r| r.starts_with("sync(")).unwrap();
    assert!(sync.contains("charges/create.yaml"), "{sync}");
    assert!(
        !sync.contains("Bearer"),
        "no request text in the log: {sync}"
    );
    assert!(
        recorded
            .iter()
            .any(|r| r.contains("https://api.example.com")),
        "the ticked non-secret default is sent"
    );
    for r in &recorded {
        assert!(!r.contains(SECRET), "{r}");
    }
    let state = serde_json::to_string(&*b.state()).unwrap();
    assert!(!state.contains(SECRET));
}

#[test]
fn an_owned_share_publishes_each_change_as_a_version() {
    let b = MockBackend::open(None, Duration::ZERO);
    testing::sign_in(&b);
    let owner = key();
    let v1 = files(&[("charges/create.yaml", "request:\n  url: a\n")]);
    block_on(b.sync(&owner, &v1, &[])).unwrap();
    let url = block_on(b.update_share(&owner, ShareUpdate::default()))
        .unwrap()
        .link
        .unwrap();
    let link = ShareLink::parse(&url).unwrap();
    let shared = block_on(b.resolve_link(&link)).unwrap();
    assert_eq!(shared.version, 1);
    assert_eq!(shared.summary, "charges");
    let snap = block_on(b.fetch_shared(&link)).unwrap();
    assert_eq!((snap.files, snap.version), (v1.clone(), 1));
    // The owner pushes a change: a new version, with its files.
    let v2 = files(&[("charges/create.yaml", "request:\n  url: a2\n")]);
    let v2_based: Vec<SyncFile> = v2
        .iter()
        .cloned()
        .map(|f| f.with_base(Some(v1[0].hash)))
        .collect();
    assert_eq!(block_on(b.sync(&owner, &v2_based, &[])).unwrap().pushed, 1);
    assert_eq!(block_on(b.latest_version(&link)), Ok(2));
    // Nothing changed: no new version.
    block_on(b.sync(&owner, &v2, &[])).unwrap();
    assert_eq!(block_on(b.latest_version(&link)), Ok(2));
    // The owner's variables come along, as shared now.
    let var = SharedVar {
        name: "CURRENCY".into(),
        description: String::new(),
        required: true,
        secret: false,
        default: Some("eur".into()),
        locked: true,
    };
    let update = ShareUpdate {
        variables: Some(vec![var.clone()]),
        ..ShareUpdate::default()
    };
    block_on(b.update_share(&owner, update)).unwrap();
    let snapshot = block_on(b.fetch_shared(&link)).unwrap();
    assert_eq!((snapshot.files, snapshot.variables), (v2, vec![var]));
    // A public link needs no account.
    testing::sign_out(&b);
    assert!(block_on(b.fetch_shared(&link)).is_ok());
}

#[test]
fn fetch_shared_honours_expiry() {
    let b = MockBackend::open(None, Duration::ZERO);
    testing::sign_in(&b);
    let k = key();
    let url = block_on(b.update_share(&k, ShareUpdate::default()))
        .unwrap()
        .link
        .unwrap();
    let own = ShareLink::parse(&url).unwrap();
    let fixture = ShareLink {
        server: MOCK_SERVER.into(),
        token: fixtures::SHIPPING_TOKEN.into(),
    };
    assert!(block_on(b.fetch_shared(&own)).is_ok());
    dev(&b).expire_links(true);
    for link in [&own, &fixture] {
        assert_eq!(
            block_on(b.fetch_shared(link)).map(|_| ()),
            Err(BackendError::Expired),
            "{link:?}"
        );
    }
    dev(&b).expire_links(false);
    b.state().shares.get_mut(&k.id).unwrap().expires =
        Some(Utc::now().date_naive() - chrono::Duration::days(1));
    assert_eq!(
        block_on(b.fetch_shared(&own)).map(|_| ()),
        Err(BackendError::Expired)
    );
    // Resolving and fetching need no account.
    testing::sign_out(&b);
    assert!(block_on(b.fetch_shared(&fixture)).is_ok());
}

#[test]
fn server_copy_changed_since_base_is_a_conflict_and_kept() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let k = key();
    block_on(b.sync(&k, &files(&[("a.yaml", "x: 1\n")]), &[])).unwrap();
    // Another client pushes x: 3 on top of x: 1.
    let other = SyncFile::new("a.yaml", "x: 3\n").with_base(Some(SyncFile::hash_of("x: 1\n")));
    assert_eq!(block_on(b.sync(&k, &[other], &[])).unwrap().pushed, 1);
    // This client, still on base x: 1, edited to x: 2.
    let mine = SyncFile::new("a.yaml", "x: 2\n").with_base(Some(SyncFile::hash_of("x: 1\n")));
    let report = block_on(b.sync(&k, &[mine], &[])).unwrap();
    assert_eq!(report.pushed, 0);
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].server, "x: 3\n");
    assert_eq!(report.conflicts[0].local, "x: 2\n");
    assert_eq!(
        b.state().synced[&k.id].files["a.yaml"].text,
        "x: 3\n",
        "theirs kept"
    );
}

#[test]
fn deletions_remove_owned_files() {
    let b = MockBackend::open(None, Duration::ZERO);
    testing::sign_in(&b);
    let owner = key();
    let both = files(&[("a.yaml", "a: 1\n"), ("b.yaml", "b: 1\n")]);
    block_on(b.sync(&owner, &both, &[])).unwrap();
    let del = SyncDeletion {
        rel: "b.yaml".into(),
        base: SyncFile::hash_of("b: 1\n"),
    };
    block_on(b.sync(&owner, &[], &[del])).unwrap();
    assert!(!b.state().synced[&owner.id].files.contains_key("b.yaml"));
    assert!(b.state().synced[&owner.id].files.contains_key("a.yaml"));
    assert_eq!(b.state().synced[&owner.id].version, 2);
}

#[test]
fn owned_pulls_newer_and_refuses_stale_deletions() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let k = key();
    let h = SyncFile::hash_of;
    block_on(b.sync(
        &k,
        &files(&[("a.yaml", "a: 1\n"), ("b.yaml", "b: 1\n")]),
        &[],
    ))
    .unwrap();
    // Another device pushes a: 2 and b: 2.
    let other = vec![
        SyncFile::new("a.yaml", "a: 2\n").with_base(Some(h("a: 1\n"))),
        SyncFile::new("b.yaml", "b: 2\n").with_base(Some(h("b: 1\n"))),
    ];
    assert_eq!(block_on(b.sync(&k, &other, &[])).unwrap().pushed, 2);
    // This one never touched a (local == base): it pulls theirs, no
    // conflict. It deleted b, which the server changed since: a conflict.
    let mine = vec![SyncFile::new("a.yaml", "a: 1\n").with_base(Some(h("a: 1\n")))];
    let gone = [SyncDeletion {
        rel: "b.yaml".into(),
        base: h("b: 1\n"),
    }];
    let report = block_on(b.sync(&k, &mine, &gone)).unwrap();
    assert_eq!(report.pushed, 0);
    assert_eq!(report.pulled, vec![SyncFile::new("a.yaml", "a: 2\n")]);
    assert_eq!(report.conflicts.len(), 1);
    let c = &report.conflicts[0];
    assert!(c.local_deleted && !c.server_deleted);
    assert_eq!(
        (c.rel.clone(), c.server.as_str()),
        (PathBuf::from("b.yaml"), "b: 2\n")
    );
    assert!(
        b.state().synced[&k.id].files.contains_key("b.yaml"),
        "theirs kept"
    );
}

fn callback(server: &str) -> AuthCallback {
    AuthCallback {
        server: server.into(),
        code: "crh_mock".into(),
    }
}

#[test]
fn sign_in_round_trip() {
    let d = tempfile::tempdir().unwrap();
    let b = mock(d.path());
    let url = block_on(b.sign_in_url(&Server::Cloud)).unwrap();
    assert_eq!(
        url,
        format!(
            "curl-runner://auth/callback?code=crh_mock&server={}",
            Server::Cloud.url()
        )
    );
    let cb = AuthCallback::parse(&url).unwrap();
    let account = block_on(b.complete_sign_in(&cb)).unwrap();
    assert_eq!(account.name, "Jamie Rivera");
    assert_eq!(account.email, "jamie@acme.dev");
    assert_eq!(account.provider, Some(SignInMethod::Github));
    assert_eq!(b.account(), Some(account.clone()));
    assert_eq!(b.account_on(Server::Cloud.url()), Some(account.clone()));
    // A callback works once.
    assert_eq!(
        block_on(b.complete_sign_in(&cb)),
        Err(BackendError::Invalid(FOREIGN_CALLBACK.into()))
    );
    // Sessions are per server: switching servers signs nothing out.
    let own = Server::SelfHosted {
        url: "https://curl-runner.example.com".into(),
    };
    block_on(b.set_server(own.clone())).unwrap();
    assert_eq!(b.account(), None);
    assert_eq!(b.account_on(Server::Cloud.url()), Some(account.clone()));
    drop(b);
    let again = mock(d.path());
    assert_eq!(again.server(), own);
    assert_eq!(again.account_on(Server::Cloud.url()), Some(account));
    block_on(again.sign_out(Server::Cloud.url())).unwrap();
    assert_eq!(mock(d.path()).account_on(Server::Cloud.url()), None);
}

/// Review Focus 1: a callback the studio didn't ask for (a link on a web
/// page) is refused, and nothing is sent anywhere.
#[test]
fn callback_without_pending_sign_in_is_refused() {
    let b = MockBackend::open(None, Duration::ZERO);
    let refused = Err(BackendError::Invalid(FOREIGN_CALLBACK.into()));
    let complete = |server: &str| block_on(b.complete_sign_in(&callback(server))).map(|_| ());
    assert_eq!(complete(Server::Cloud.url()), refused, "nothing pending");
    // Pending on Cloud: another server's callback is still refused.
    block_on(b.sign_in_url(&Server::Cloud)).unwrap();
    assert_eq!(complete("https://evil.test"), refused);
    // Plain http, even pending, is refused: the code would travel in clear.
    let http = Server::SelfHosted {
        url: "http://curl-runner.example.com".into(),
    };
    assert!(block_on(b.sign_in_url(&http)).is_err(), "refused up front");
    assert_eq!(complete("http://curl-runner.example.com"), refused);
    assert_eq!(b.account_on("https://evil.test"), None);
    assert_eq!(b.account_on("http://curl-runner.example.com"), None);
    let sent = dev(&b).recorded();
    assert!(
        !sent.iter().any(|c| c.starts_with("redeem")),
        "no code was sent: {sent:?}"
    );
    // Loopback http is fine (local servers), and a trailing `/` too.
    let local = Server::SelfHosted {
        url: "http://localhost:8080/".into(),
    };
    block_on(b.sign_in_url(&local)).unwrap();
    assert_eq!(complete("http://localhost:8080"), Ok(()));
    assert!(b.account_on("http://localhost:8080").is_some());
    // Cloud's own sign-in still waits, and works.
    assert_eq!(complete(Server::Cloud.url()), Ok(()));
}

#[test]
fn pending_sign_in_expires_after_10_minutes() {
    let now = Arc::new(Mutex::new(Instant::now()));
    let clock = {
        let now = now.clone();
        Arc::new(move || *now.lock().unwrap())
    };
    let b = MockBackend::open(None, Duration::ZERO).with_clock(clock);
    let advance = |secs: u64| *now.lock().unwrap() += Duration::from_secs(secs);
    block_on(b.sign_in_url(&Server::Cloud)).unwrap();
    advance(10 * 60 + 1);
    assert_eq!(
        block_on(b.complete_sign_in(&callback(Server::Cloud.url()))).map(|_| ()),
        Err(BackendError::Invalid(FOREIGN_CALLBACK.into()))
    );
    assert_eq!(b.account(), None);
    block_on(b.sign_in_url(&Server::Cloud)).unwrap();
    advance(10 * 60 - 1);
    assert!(block_on(b.complete_sign_in(&callback(Server::Cloud.url()))).is_ok());
}

#[test]
fn sign_in_url_refuses_plain_http_off_loopback() {
    let b = mock(tempfile::tempdir().unwrap().path());
    let own = |url: &str| Server::SelfHosted { url: url.into() };
    let err = block_on(b.sign_in_url(&own("http://curl.example.com"))).unwrap_err();
    assert!(
        matches!(&err, BackendError::Invalid(m) if m.contains("https")),
        "{err:?}"
    );
    assert_ne!(err, BackendError::Invalid(FOREIGN_CALLBACK.into()));
    // Loopback http is fine.
    block_on(b.sign_in_url(&own("http://127.0.0.1:9"))).unwrap();
}

#[test]
fn an_expired_session_lapses_until_signed_in_or_out() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    assert_eq!(b.lapsed(), None);
    dev(&b).expire_session();
    assert_eq!(b.account(), None, "signed out");
    assert_eq!(b.lapsed().unwrap().email, "jamie@acme.dev");
    // Signing in again clears it.
    testing::sign_in(&b);
    assert_eq!(b.lapsed(), None);
    assert!(b.account().is_some());
    // So does signing out.
    dev(&b).expire_session();
    testing::sign_out(&b);
    assert_eq!(b.lapsed(), None);
    // Nothing to expire: nothing lapses.
    dev(&b).expire_session();
    assert_eq!(b.lapsed(), None);
}

#[test]
fn accounts_lists_every_server_signed_in() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let own = Server::SelfHosted {
        url: "https://curl-runner.example.com".into(),
    };
    block_on(b.set_server(own.clone())).unwrap();
    testing::sign_in_by_email(&b);
    let accounts = b.accounts();
    let servers: Vec<&str> = accounts.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(servers, [Server::Cloud.url(), own.url()]);
    assert_eq!(accounts[0].1.email, "jamie@acme.dev");
    assert_eq!(accounts[1].1.email, "maria@example.com");
}

fn bundle(text: &str) -> PublishRequest {
    let files = vec![BundleFile {
        path: "a.yaml".into(),
        text: text.into(),
    }];
    PublishRequest {
        hash: curl_runner_cloud_proto::bundle_hash(&files),
        files,
        inputs: Vec::new(),
        name: None,
        description: None,
        git: None,
    }
}

#[test]
fn publishing_mode_publishes_instead_of_syncing() {
    let b = MockBackend::open(None, Duration::ZERO).publishing();
    assert!(b.publishes() && !b.team_sync());
    assert!(!MockBackend::open(None, Duration::ZERO).publishes());
    assert_eq!(
        block_on(b.publish("acme", "pay", bundle("x"))).unwrap_err(),
        BackendError::SignedOut
    );
    testing::sign_in(&b);
    assert_eq!(block_on(b.remote_collection("acme", "pay")).unwrap(), None);
    let r = block_on(b.publish("acme", "pay", bundle("x"))).unwrap();
    assert_eq!((r.version, r.status), (1, PublishStatus::Created));
    assert_eq!(r.changed.added, ["a.yaml"]);
    let again = block_on(b.publish("acme", "pay", bundle("x"))).unwrap();
    assert_eq!((again.version, again.status), (1, PublishStatus::Unchanged));
    let v2 = block_on(b.publish("acme", "pay", bundle("y"))).unwrap();
    assert_eq!((v2.version, v2.changed.changed.len()), (2, 1));
    let remote = block_on(b.remote_collection("acme", "pay"))
        .unwrap()
        .unwrap();
    let current = remote.current.unwrap();
    assert_eq!(
        (current.number, current.publisher.as_str()),
        (2, "jamie@acme.dev")
    );
    assert_eq!(
        block_on(b.remote_files("acme", "pay", 1)).unwrap()[0].text,
        "x"
    );
}

#[test]
fn publishing_refusals() {
    let b = MockBackend::open(None, Duration::ZERO).publishing();
    testing::sign_in(&b);
    assert!(matches!(
        block_on(b.publish("globex", "pay", bundle("x"))),
        Err(BackendError::Invalid(_))
    ));
    dev(&b).seed_published(
        "acme",
        "studio",
        CollectionSource::Cloud,
        Vec::new(),
        "jamie@acme.dev",
    );
    assert!(matches!(
        block_on(b.publish("acme", "studio", bundle("x"))),
        Err(BackendError::WrongSource(_))
    ));
    dev(&b).refuse_next_publish(BackendError::PlanLimit("Upgrade.".into()));
    assert_eq!(
        block_on(b.publish("acme", "pay", bundle("x"))).unwrap_err(),
        BackendError::PlanLimit("Upgrade.".into())
    );
    assert!(block_on(b.publish("acme", "pay", bundle("x"))).is_ok());
}
