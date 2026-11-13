use super::{
    BackendError, CONFIG_FILE, CollectionSource, Problem, ProblemRow, PublishResponse,
    PublishStatus, Workspace, changes_line, load_cloud_config, local_hash, problem_rows,
    problems_title, suggest_slug,
};
use crate::model::backend::{Backend, publishing_test_backend, test_backend, testing};
use crate::ui::state::{Connection, StateStore};
use crate::ui::workspace::test_secrets;
use curl_runner_cloud_proto::{BundleFile, FileChanges};
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
    WindowOptions, px, size,
};
use std::path::Path;
use std::sync::Arc;

const REQUEST: &str = "request:\n  name: A\n  url: https://api.example.com/a\n  method: GET\n";

/// A git repo (a `.git` folder is enough for the dialog's default) with
/// `files`.
fn folder(files: &[(&str, &str)]) -> tempfile::TempDir {
    let d = plain_folder(files);
    std::fs::create_dir(d.path().join(".git")).unwrap();
    d
}

/// A folder that isn't a git repo.
fn plain_folder(files: &[(&str, &str)]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    for (rel, text) in files {
        let p = d.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    d
}

fn mock() -> Arc<dyn Backend> {
    publishing_test_backend()
}

fn open(
    cx: &mut TestAppContext,
    root: &Path,
    backend: Arc<dyn Backend>,
    connection: Connection,
) -> (AnyWindowHandle, Entity<Workspace>, tempfile::TempDir) {
    let state = tempfile::tempdir().unwrap();
    let store = StateStore::at(state.path());
    store.set_connection(Some(connection));
    let root = root.to_path_buf();
    let (window, ws) = cx.update(|cx| {
        gpui_kit::init(cx);
        super::super::app::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(1440.), px(1400.)),
            })),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Workspace::new(Some(root), store, test_secrets(), backend, window, cx))
        })
        .unwrap()
    });
    cx.run_until_parked();
    (window, ws, state)
}

fn shows(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.try_find_by_label(label).is_some()
    })
    .unwrap()
}

fn click(cx: &mut TestAppContext, window: AnyWindowHandle, id: &str) {
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.click(id.to_string(), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn publish(cx: &mut TestAppContext, window: AnyWindowHandle, ws: &Entity<Workspace>) {
    cx.update_window(window, |_, window, cx| {
        ws.update(cx, |ws, cx| ws.start_publish(window, cx))
    })
    .unwrap();
    cx.run_until_parked();
}

fn foot(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> String {
    ws.read_with(cx, |ws, cx| {
        let line = ws.publish_line(cx);
        let now = chrono::Utc::now();
        match line.detail(now) {
            Some(d) => format!("{}. {d}", line.text()),
            None => line.text(),
        }
    })
}

#[gpui_kit::test]
fn first_publish_picks_team_and_name_then_saves_them(cx: &mut TestAppContext) {
    let dir = folder(&[("charges/a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    assert_eq!(foot(cx, &ws), "Not published");
    publish(cx, window, &ws);
    let view = ws
        .read_with(cx, |ws, _| ws.publish_view())
        .expect("the dialog");
    let name = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(shows(
        cx,
        window,
        &format!("Put {name} on curl-runner Cloud")
    ));
    // A git repo: Publish by default.
    view.read_with(cx, |v, _| assert_eq!(v.mode, super::Mode::Publish));
    assert!(shows(cx, window, "Ready to publish"));
    assert!(shows(
        cx,
        window,
        &format!("1 request in 1 folder · {} B", REQUEST.len())
    ));
    view.read_with(cx, |v, cx| {
        assert_eq!(v.slug(cx), suggest_slug(&name));
        assert_eq!(v.blocked(cx), None);
    });
    click(cx, window, "publish-go");
    assert!(ws.read_with(cx, |ws, _| ws.publish_view()).is_none());
    assert!(
        backend
            .dev()
            .unwrap()
            .recorded()
            .iter()
            .any(|c| c.starts_with("publish(\"acme\""))
    );
    assert_eq!(foot(cx, &ws), "Published v1. just now by you");
    // The folder names its collection for CI now, and the notes link it.
    let cfg = load_cloud_config(dir.path()).unwrap().unwrap();
    assert_eq!(cfg.collection, suggest_slug(&name));
    assert_eq!(cfg.name.as_deref(), Some(name.as_str()));
    let link = ws
        .read_with(cx, |ws, cx| ws.notes.read(cx).cloud.clone())
        .unwrap();
    assert_eq!(
        (link.team.as_str(), link.slug.as_str()),
        ("acme", cfg.collection.as_str())
    );
}

#[gpui_kit::test]
fn later_publishes_ask_nothing(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    // Without the yaml block: only the notes remember.
    click(cx, window, "publish-save-yaml");
    click(cx, window, "publish-go");
    assert!(!dir.path().join(CONFIG_FILE).exists());
    std::fs::write(dir.path().join("a.yaml"), REQUEST.replace("GET", "POST")).unwrap();
    cx.update_window(window, |_, window, cx| {
        ws.update(cx, |ws, cx| ws.refresh_publish(false, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(foot(cx, &ws), "1 file not published. v1 · just now by you");
    publish(cx, window, &ws);
    assert!(ws.read_with(cx, |ws, _| ws.publish_view()).is_none());
    assert_eq!(foot(cx, &ws), "Published v2. just now by you");
}

#[gpui_kit::test]
fn problems_stop_the_publish(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", "request:\n  method: GET\n")]);
    let backend = mock();
    testing::sign_in(&*backend);
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    assert!(shows(cx, window, "Fix 1 problem to publish"));
    let blocked = ws
        .read_with(cx, |ws, _| ws.publish_view())
        .unwrap()
        .read_with(cx, |v, cx| v.blocked(cx));
    assert_eq!(blocked.as_deref(), Some("Fix the problems first."));
    // Open takes you to the file.
    click(cx, window, "publish-open-0");
    ws.read_with(cx, |ws, _| {
        let ix = ws.state.active.unwrap();
        assert_eq!(ws.state.open[ix].rel_path, Path::new("a.yaml"));
    });
}

#[gpui_kit::test]
fn a_studio_synced_name_is_refused_in_the_dialog(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let name = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    backend.dev().unwrap().seed_published(
        "acme",
        &suggest_slug(&name),
        CollectionSource::Cloud,
        Vec::new(),
        "jamie@acme.dev",
    );
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    let view = ws.read_with(cx, |ws, _| ws.publish_view()).unwrap();
    assert_eq!(
        view.read_with(cx, |v, cx| v.blocked(cx)).as_deref(),
        Some("This collection syncs from the studio.")
    );
    let wrong = cx
        .update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .try_find("publish-wrong-source".to_string())
                .is_some()
        })
        .unwrap();
    assert!(wrong);
}

#[gpui_kit::test]
fn an_existing_collection_warns_and_bumps_the_version(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let name = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    backend.dev().unwrap().seed_published(
        "acme",
        &suggest_slug(&name),
        CollectionSource::Git,
        vec![BundleFile {
            path: "old.yaml".into(),
            text: REQUEST.into(),
        }],
        "token CI",
    );
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    assert!(shows(cx, window, "Publish version 2"));
    let label = format!(
        "acme/{} already exists (v1, by CI). Publishing makes version 2 from this folder.",
        suggest_slug(&name)
    );
    assert!(shows(cx, window, &label));
}

#[gpui_kit::test]
fn plan_limit_offers_upgrade(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    backend
        .dev()
        .unwrap()
        .refuse_next_publish(BackendError::PlanLimit(
            "Free teams can have 3 collections.".into(),
        ));
    publish(cx, window, &ws);
    let v = ws.read_with(cx, |ws, _| ws.publish_view()).unwrap();
    assert_eq!(v.read_with(cx, |v, cx| v.blocked(cx)), None);
    click(cx, window, "publish-go");
    assert!(shows(cx, window, "Couldn't publish"));
    assert!(shows(cx, window, "Free teams can have 3 collections."));
    assert!(shows(cx, window, "Upgrade"));
    assert_eq!(foot(cx, &ws), "Not published");
}

#[gpui_kit::test]
fn no_team_says_so(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in_by_email(&*backend);
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    assert!(shows(cx, window, "You're not in a team yet"));
    assert!(ws.read_with(cx, |ws, _| ws.publish_view()).is_none());
}

#[gpui_kit::test]
fn signed_out_and_offline_feet(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let (window, ws, _s) = open(cx, dir.path(), mock(), Connection::Online);
    assert_eq!(foot(cx, &ws), "Sign in to publish");
    assert!(shows(cx, window, "Sign in"));
    let backend = mock();
    testing::sign_in(&*backend);
    let (_, ws, _s) = open(cx, dir.path(), backend, Connection::Offline);
    assert_eq!(foot(cx, &ws), "Working offline");
}

#[gpui_kit::test]
fn the_old_sync_mock_keeps_its_foot(cx: &mut TestAppContext) {
    let dir = folder(&[("a.yaml", REQUEST)]);
    let backend = test_backend();
    let (_, ws, _s) = open(cx, dir.path(), backend, Connection::Online);
    assert!(!ws.read_with(cx, |ws, _| ws.publish_applies()));
}

#[test]
fn problem_rows_read_like_the_board() {
    let rows = problem_rows(&[
        Problem::Secret {
            file: "auth/login.yaml".into(),
            name: "API_TOKEN".into(),
        },
        Problem::OutsideRoot("x".into()),
        Problem::InvalidSlug("Bad".into()),
        Problem::TooBig("big.yaml".into()),
    ]);
    assert_eq!(
        rows,
        [
            ProblemRow {
                file: Some("auth/login.yaml".into()),
                text: "auth/login.yaml gives the secret API_TOKEN a value. Use ${API_TOKEN} and keep the value in your Keychain.".into(),
            },
            ProblemRow {
                file: Some("big.yaml".into()),
                text: "big.yaml is larger than 1 MiB.".into(),
            },
        ]
    );
    assert_eq!(problems_title(1), "Fix 1 problem to publish");
    assert_eq!(problems_title(2), "Fix 2 problems to publish");
}

#[test]
fn success_line_counts_files() {
    let r = |added: usize, changed: usize| PublishResponse {
        collection: "pay".into(),
        version: 15,
        status: PublishStatus::Created,
        changed: FileChanges {
            added: vec!["a".into(); added],
            changed: vec!["c".into(); changed],
            removed: Vec::new(),
        },
    };
    assert_eq!(
        changes_line(&r(1, 3)),
        "3 files changed, 1 added. People with the link get it as an update."
    );
    assert_eq!(
        changes_line(&r(2, 0)),
        "2 files added. People with the link get it as an update."
    );
}

#[test]
fn local_hash_matches_the_bundle() {
    let dir = folder(&[("a.yaml", REQUEST)]);
    assert_eq!(local_hash(dir.path()).len(), 64);
}

#[gpui_kit::test]
fn a_folder_outside_git_syncs_by_default_and_starts_syncing(cx: &mut TestAppContext) {
    let dir = plain_folder(&[("charges/a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    let view = ws
        .read_with(cx, |ws, _| ws.publish_view())
        .expect("the dialog");
    view.read_with(cx, |v, _| assert_eq!(v.mode, super::Mode::Sync));
    assert!(shows(cx, window, "Start syncing"));
    assert!(shows(cx, window, "Ready to sync"));
    // No curl-runner.yaml switch for a synced collection.
    assert!(!shows(
        cx,
        window,
        &format!("Save the name and description to {CONFIG_FILE}")
    ));
    click(cx, window, "publish-go");
    assert!(ws.read_with(cx, |ws, _| ws.publish_view()).is_none());
    let calls = backend.dev().unwrap().recorded();
    assert!(
        calls.iter().any(|c| c.starts_with("create_collection(")),
        "{calls:?}"
    );
    assert!(calls.iter().any(|c| c.starts_with("sync(")), "{calls:?}");
    assert!(ws.read_with(cx, |ws, _| ws.cloud_synced()));
    assert!(!dir.path().join(CONFIG_FILE).exists(), "nothing written");
    let key = ws.read_with(cx, |ws, cx| ws.collection_key(cx)).unwrap();
    let files = testing::server_files(&*backend, &key);
    assert_eq!(files.len(), 1, "the first sync pushed the folder");
}

#[gpui_kit::test]
fn sync_refuses_a_name_the_team_has(cx: &mut TestAppContext) {
    let dir = plain_folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let name = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let team = backend.account().unwrap().teams[0].slug.clone();
    backend.dev().unwrap().seed_published(
        &team,
        &suggest_slug(&name),
        crate::model::backend::CollectionSource::Git,
        Vec::new(),
        "token CI",
    );
    let (window, ws, _s) = open(cx, dir.path(), backend.clone(), Connection::Online);
    publish(cx, window, &ws);
    let view = ws.read_with(cx, |ws, _| ws.publish_view()).unwrap();
    let blocked = view.read_with(cx, |v, cx| v.blocked(cx)).unwrap();
    assert!(blocked.contains("already exists"), "{blocked}");
    // Publish is still open to it.
    click(cx, window, "publish-mode-publish");
    let blocked = view.read_with(cx, |v, cx| v.blocked(cx));
    assert!(blocked.is_none_or(|b| !b.contains("already exists")));
}

#[gpui_kit::test]
fn the_foot_says_put_on_cloud_outside_git(cx: &mut TestAppContext) {
    let plain = plain_folder(&[("a.yaml", REQUEST)]);
    let backend = mock();
    testing::sign_in(&*backend);
    let (window, _ws, _s) = open(cx, plain.path(), backend.clone(), Connection::Online);
    assert!(shows(cx, window, "Put on Cloud"));
    let repo = folder(&[("a.yaml", REQUEST)]);
    let (window, _ws, _s) = open(cx, repo.path(), backend, Connection::Online);
    assert!(shows(cx, window, "Publish"));
    assert!(!shows(cx, window, "Put on Cloud"));
}
