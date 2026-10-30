use super::*;
use crate::model::backend::{shared_vars, sync_files};
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
    block_on(b.sign_in(SignInMethod::Sso)).unwrap();
    b
}

#[test]
fn sign_in_then_account_persists_across_open() {
    let d = tempfile::tempdir().unwrap();
    let b = mock(d.path());
    assert_eq!(b.account(), None);
    assert_eq!(b.server(), Server::Cloud);
    let account = block_on(b.sign_in(SignInMethod::Sso)).unwrap();
    assert_eq!(account.name, "Jamie Rivera");
    assert_eq!(account.email, "jamie@acme.dev");
    assert_eq!(account.org.as_deref(), Some("Acme"));
    assert_eq!(account.provider, SignInMethod::Sso);
    drop(b);
    let again = mock(d.path());
    assert_eq!(again.account(), Some(account));
    block_on(again.sign_out()).unwrap();
    assert_eq!(mock(d.path()).account(), None);
}

#[test]
fn switching_server_signs_out_and_persists() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    let own = Server::SelfHosted {
        url: "https://curl-runner.example.com".into(),
    };
    let info = block_on(b.test_connection(&own)).unwrap();
    assert_eq!(
        (info.version.as_str(), info.sign_in, info.latency_ms),
        ("2.1.1", SignInMethod::Sso, 42)
    );
    let bad = Server::SelfHosted { url: "nope".into() };
    assert!(matches!(
        block_on(b.test_connection(&bad)),
        Err(BackendError::Invalid(_))
    ));
    block_on(b.set_server(own.clone())).unwrap();
    assert_eq!(b.account(), None);
    assert_eq!(mock(d.path()).server(), own);
}

#[test]
fn fail_next_sign_in_fails_once() {
    let d = tempfile::tempdir().unwrap();
    let b = mock(d.path());
    dev(&b).fail_next_sign_in(true);
    assert!(dev(&b).toggles().fail_next_sign_in);
    assert_eq!(
        block_on(b.sign_in(SignInMethod::GitHub)),
        Err(BackendError::Unauthorized)
    );
    assert_eq!(b.account(), None);
    assert!(!dev(&b).toggles().fail_next_sign_in);
    assert!(block_on(b.sign_in(SignInMethod::GitHub)).is_ok());
}

#[test]
fn offline_fails_every_call_with_offline() {
    let d = tempfile::tempdir().unwrap();
    let b = signed_in_mock(d.path());
    dev(&b).set_offline(true);
    assert!(dev(&b).offline());
    assert!(mock(d.path()).offline(), "the toggle persists");
    let k = key();
    let link = ShareLink::parse("curl-runner://open/acme/shipping-api?share=7Kx2Qm").unwrap();
    let off = Err(BackendError::Offline);
    assert_eq!(block_on(b.set_server(Server::Cloud)), off);
    assert_eq!(block_on(b.test_connection(&Server::Cloud)).map(|_| ()), off);
    assert_eq!(block_on(b.sign_in(SignInMethod::Sso)).map(|_| ()), off);
    assert_eq!(block_on(b.sign_out()), off);
    assert_eq!(block_on(b.sync(&k, &[], &[])).map(|_| ()), off);
    assert_eq!(block_on(b.share_settings(&k)).map(|_| ()), off);
    assert_eq!(
        block_on(b.update_share(&k, ShareUpdate::default())).map(|_| ()),
        off
    );
    assert_eq!(block_on(b.invite(&k, "maria@example.com")).map(|_| ()), off);
    assert_eq!(block_on(b.remove_person(&k, "maria@example.com")), off);
    assert_eq!(block_on(b.resolve_link(&link)).map(|_| ()), off);
    assert_eq!(
        block_on(b.fetch_shared(fixtures::SHIPPING_ID)).map(|_| ()),
        off
    );
    let share = ReceivedShare {
        id: fixtures::SHIPPING_ID.into(),
        collection_id: "ffff0000".into(),
        owner_org: "Acme".into(),
        name: "Shipping API".into(),
        root: "/tmp/x".into(),
        added: Utc::now(),
    };
    assert_eq!(block_on(b.add_received(share)), off);
    assert!(b.account().is_some(), "local state is kept");
    assert!(b.received().is_empty());
}

#[test]
fn offline_mid_call_fails_that_call() {
    let d = tempfile::tempdir().unwrap();
    let b = MockBackend::open(Some(d.path()), Duration::from_millis(30));
    let started = Instant::now();
    let pending = b.sign_in(SignInMethod::Sso);
    dev(&b).set_offline(true);
    assert_eq!(block_on(pending), Err(BackendError::Offline));
    assert!(started.elapsed() >= Duration::from_millis(30), "it waited");
    assert_eq!(b.account(), None);
}

#[test]
fn zero_latency_is_ready_on_first_poll() {
    let b = MockBackend::open(None, Duration::ZERO);
    let mut fut = b.sign_in(SignInMethod::Email);
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(Ok(a)) => assert_eq!(a.name, "Maria"),
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
        url.starts_with("https://studio.example.com/s/payments-api-"),
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
    assert_eq!(resolved.id, k.id);

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
        "curl-runner://open/acme/shipping-api?share=7Kx2Qm",
        "https://studio.example.com/s/shipping-api-7Kx2Qm",
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
    let snap = block_on(b.fetch_shared(fixtures::SHIPPING_ID)).unwrap();
    assert_eq!(snap.files, fixtures::shipping_snapshot());
    assert_eq!(
        block_on(b.fetch_shared("acme/nope")).map(|_| ()),
        Err(BackendError::NotFound)
    );
    let unknown = ShareLink::parse("curl-runner://open/acme/shipping-api?share=zzzzzz").unwrap();
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
    let fixture = ShareLink::parse("curl-runner://open/acme/shipping-api?share=7Kx2Qm").unwrap();
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
    let garbage = ShareLink::parse("https://studio.example.com/s/x-zzzzzz").unwrap();
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

    // Offline: changes wait.
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
    assert_eq!(b.sync_status(&k), SyncStatus::Offline(2));
    dev(&b).set_offline(false);
    assert_eq!(b.sync_status(&k), SyncStatus::Pending(2));
    assert_eq!(mock(d.path()).sync_status(&k), SyncStatus::Pending(2));
    assert_eq!(block_on(b.sync(&k, &local, &[])).unwrap().pushed, 2);
    assert!(matches!(b.sync_status(&k), SyncStatus::Synced { .. }));

    block_on(b.sign_out()).unwrap();
    assert_eq!(b.sync_status(&k), SyncStatus::Off);
}

#[test]
fn received_collection_pulls_and_never_pushes() {
    let b = MockBackend::open(None, Duration::ZERO);
    block_on(b.sign_in(SignInMethod::Email)).unwrap();
    let k = CollectionKey {
        id: "ffff0000".into(),
        name: "Shipping API".into(),
    };
    block_on(b.add_received(ReceivedShare {
        id: fixtures::SHIPPING_ID.into(),
        collection_id: k.id.clone(),
        owner_org: "Acme".into(),
        name: "Shipping API".into(),
        root: "/tmp/shared/acme-shipping-api".into(),
        added: Utc::now(),
    }))
    .unwrap();
    assert_eq!(b.received().len(), 1);
    let mut local = fixtures::shipping_snapshot();
    local[0] = SyncFile::new(local[0].rel.clone(), "stale");
    let report = block_on(b.sync(&k, &local, &[])).unwrap();
    assert_eq!(report.pushed, 0);
    assert_eq!(
        report.pulled,
        vec![fixtures::shipping_snapshot()[0].clone()]
    );
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
    assert_eq!(b.sync_status(&k), SyncStatus::Conflict(1));
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
    block_on(b.sign_in(SignInMethod::Sso)).unwrap();
    let vars = vec![
        SharedVar {
            name: "API_TOKEN".into(),
            description: String::new(),
            required: true,
            secret: true,
            default: Some("leaked".into()),
        },
        SharedVar {
            name: "BASE_URL".into(),
            description: String::new(),
            required: true,
            secret: false,
            default: Some("https://api.example.com".into()),
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
    block_on(b.sign_in(SignInMethod::Sso)).unwrap();
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
        },
    );
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
    block_on(b.fetch_shared(&shared.id)).unwrap();

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
fn received_copy_of_an_owned_share_pulls_the_owners_files() {
    let b = MockBackend::open(None, Duration::ZERO);
    block_on(b.sign_in(SignInMethod::Sso)).unwrap();
    let owner = key();
    let v1 = files(&[("charges/create.yaml", "request:\n  url: a\n")]);
    block_on(b.sync(&owner, &v1, &[])).unwrap();
    let link = block_on(b.update_share(&owner, ShareUpdate::default()))
        .unwrap()
        .link
        .unwrap();
    let shared = block_on(b.resolve_link(&ShareLink::parse(&link).unwrap())).unwrap();
    assert_eq!(block_on(b.fetch_shared(&shared.id)).unwrap().files, v1);
    let copy = CollectionKey {
        id: "c0ffee00".into(),
        name: "Payments API".into(),
    };
    block_on(b.add_received(ReceivedShare {
        id: shared.id.clone(),
        collection_id: copy.id.clone(),
        owner_org: shared.owner_org.clone(),
        name: shared.name.clone(),
        root: "/tmp/shared/acme-payments-api".into(),
        added: Utc::now(),
    }))
    .unwrap();
    // The owner pushes a change; the copy pulls it, not the fixture.
    let v2 = files(&[("charges/create.yaml", "request:\n  url: a2\n")]);
    let v2_based: Vec<SyncFile> = v2
        .iter()
        .cloned()
        .map(|f| f.with_base(Some(v1[0].hash)))
        .collect();
    assert_eq!(block_on(b.sync(&owner, &v2_based, &[])).unwrap().pushed, 1);
    let report = block_on(b.sync(&copy, &v1, &[])).unwrap();
    assert_eq!((report.pushed, report.pulled.clone()), (0, v2.clone()));
    assert!(block_on(b.sync(&copy, &v2, &[])).unwrap().pulled.is_empty());

    // Signed out, a received copy does not sync.
    block_on(b.sign_out()).unwrap();
    assert_eq!(
        block_on(b.sync(&copy, &v2, &[])).map(|_| ()),
        Err(BackendError::SignedOut)
    );
}

#[test]
fn fetch_shared_honours_expiry() {
    let b = MockBackend::open(None, Duration::ZERO);
    block_on(b.sign_in(SignInMethod::Sso)).unwrap();
    let k = key();
    block_on(b.update_share(&k, ShareUpdate::default())).unwrap();
    assert!(block_on(b.fetch_shared(&k.id)).is_ok());
    dev(&b).expire_links(true);
    for id in [k.id.as_str(), fixtures::SHIPPING_ID] {
        assert_eq!(
            block_on(b.fetch_shared(id)).map(|_| ()),
            Err(BackendError::Expired),
            "{id}"
        );
    }
    assert_eq!(
        block_on(b.fetch_shared("acme/nope")).map(|_| ()),
        Err(BackendError::NotFound)
    );
    dev(&b).expire_links(false);
    b.state().shares.get_mut(&k.id).unwrap().expires =
        Some(Utc::now().date_naive() - chrono::Duration::days(1));
    assert_eq!(
        block_on(b.fetch_shared(&k.id)).map(|_| ()),
        Err(BackendError::Expired)
    );
    // Resolving and fetching need no account.
    block_on(b.sign_out()).unwrap();
    assert!(block_on(b.fetch_shared(fixtures::SHIPPING_ID)).is_ok());
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
fn deletions_remove_owned_files_and_received_copies_are_told() {
    let b = MockBackend::open(None, Duration::ZERO);
    block_on(b.sign_in(SignInMethod::Sso)).unwrap();
    let owner = key();
    let both = files(&[("a.yaml", "a: 1\n"), ("b.yaml", "b: 1\n")]);
    block_on(b.sync(&owner, &both, &[])).unwrap();
    let link = block_on(b.update_share(&owner, ShareUpdate::default()))
        .unwrap()
        .link
        .unwrap();
    let shared = block_on(b.resolve_link(&ShareLink::parse(&link).unwrap())).unwrap();
    let copy = CollectionKey {
        id: "c0ffee00".into(),
        name: "Payments API".into(),
    };
    block_on(b.add_received(ReceivedShare {
        id: shared.id.clone(),
        collection_id: copy.id.clone(),
        owner_org: shared.owner_org.clone(),
        name: shared.name.clone(),
        root: "/tmp/shared/acme-payments-api".into(),
        added: Utc::now(),
    }))
    .unwrap();
    // The owner deletes b.yaml.
    let a = files(&[("a.yaml", "a: 1\n")]);
    let a_based: Vec<SyncFile> = a
        .iter()
        .cloned()
        .map(|f| {
            let h = f.hash;
            f.with_base(Some(h))
        })
        .collect();
    let del = |rel: &str, text: &str| SyncDeletion {
        rel: rel.into(),
        base: SyncFile::hash_of(text),
    };
    block_on(b.sync(&owner, &a_based, &[del("b.yaml", "b: 1\n")])).unwrap();
    assert!(!b.state().synced[&owner.id].files.contains_key("b.yaml"));
    // The copy synced both before: told to remove b.yaml; its own
    // deletions are ignored (read only).
    let copy_local: Vec<SyncFile> = both
        .iter()
        .cloned()
        .map(|f| {
            let h = f.hash;
            f.with_base(Some(h))
        })
        .collect();
    let report = block_on(b.sync(&copy, &copy_local, &[del("a.yaml", "a: 1\n")])).unwrap();
    assert_eq!(report.deleted, [PathBuf::from("b.yaml")]);
    assert!(b.state().synced[&owner.id].files.contains_key("a.yaml"));
    // A file the copy never synced is not "deleted".
    let unsynced = files(&[("a.yaml", "a: 1\n"), ("b.yaml", "b: 1\n")]);
    assert!(
        block_on(b.sync(&copy, &unsynced, &[]))
            .unwrap()
            .deleted
            .is_empty()
    );
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
