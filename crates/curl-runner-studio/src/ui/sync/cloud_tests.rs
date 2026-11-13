//! A folder whose collection on the server syncs from the studio (source
//! Cloud): two-way sync, the sync foot, and the periodic check.

use super::{SyncLine, Workspace};
use crate::model::backend::{
    Backend, BundleFile, CollectionSource, publishing_test_backend, testing,
};
use crate::model::publish::CloudLink;
use crate::model::settings::CheckEvery;
use crate::ui::state::{Connection, StateStore};
use crate::ui::workspace::test_secrets;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
    WindowOptions, px, size,
};
use std::sync::Arc;
use std::time::Duration;

const REQUEST: &str = "request:\n  name: A\n  url: https://api.example.com/a\n  method: GET\n";

struct Open {
    window: AnyWindowHandle,
    ws: Entity<Workspace>,
    backend: Arc<dyn Backend>,
    _dir: tempfile::TempDir,
    _state: tempfile::TempDir,
}

/// A folder linked to `acme/pay`, whose source on the server is `source`.
fn open(cx: &mut TestAppContext, source: CollectionSource, check: CheckEvery) -> Open {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.yaml"), REQUEST).unwrap();
    let backend = publishing_test_backend();
    testing::sign_in(&*backend);
    let files = vec![BundleFile {
        path: "a.yaml".into(),
        text: REQUEST.into(),
    }];
    backend
        .dev()
        .unwrap()
        .seed_published("acme", "pay", source, files, "sam@acme.dev");
    let state = tempfile::tempdir().unwrap();
    let store = StateStore::at(state.path());
    store.set_connection(Some(Connection::Online));
    let root = dir.path().to_path_buf();
    let b = backend.clone();
    let (window, ws) = cx.update(|cx| {
        gpui_kit::init(cx);
        crate::ui::app::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(1440.), px(1000.)),
            })),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Workspace::new(Some(root), store, test_secrets(), b, window, cx))
        })
        .unwrap()
    });
    cx.update_window(window, |_, window, cx| {
        ws.update(cx, |ws, cx| {
            ws.settings.sync.check = check;
            let server = ws.backend.server().url().to_string();
            ws.notes.update(cx, |m, cx| {
                m.cloud = Some(CloudLink {
                    server,
                    team: "acme".into(),
                    slug: "pay".into(),
                });
                cx.notify();
            });
            ws.refresh_publish(false, window, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    Open {
        window,
        ws,
        backend,
        _dir: dir,
        _state: state,
    }
}

fn syncs(backend: &Arc<dyn Backend>) -> usize {
    backend
        .dev()
        .unwrap()
        .recorded()
        .into_iter()
        .filter(|c| c.starts_with("sync("))
        .count()
}

fn labelled(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.try_find_by_label(label).is_some()
    })
    .unwrap()
}

#[gpui_kit::test]
fn a_cloud_source_syncs_both_ways_with_the_sync_foot(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Cloud, CheckEvery::FiveMinutes);
    assert!(o.ws.read_with(cx, |ws, _| ws.cloud_synced()));
    assert!(syncs(&o.backend) >= 1, "synced once found");
    let line = o.ws.read_with(cx, |ws, _| ws.sync_line());
    assert!(matches!(line, SyncLine::Synced(_)), "{line:?}");
    // The sync foot, not the publish one ("Syncs from the studio").
    let text = line.text(chrono::Utc::now());
    assert!(labelled(cx, o.window, &text), "{text}");
    assert!(!labelled(cx, o.window, "Syncs from the studio"));
    // Its version and who changed it, under the state.
    assert!(labelled(cx, o.window, "v1 · sam@acme.dev edited just now"));
    // The key carries the link the real server needs.
    let key = o.ws.read_with(cx, |ws, cx| ws.collection_key(cx)).unwrap();
    assert_eq!(
        key.cloud.map(|l| (l.team, l.slug)),
        Some(("acme".into(), "pay".into()))
    );
}

#[gpui_kit::test]
fn a_git_source_keeps_publishing(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Git, CheckEvery::FiveMinutes);
    assert!(!o.ws.read_with(cx, |ws, _| ws.cloud_synced()));
    assert_eq!(syncs(&o.backend), 0);
}

#[gpui_kit::test]
fn checks_for_team_changes_at_the_chosen_interval(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Cloud, CheckEvery::OneMinute);
    let first = syncs(&o.backend);
    cx.executor().advance_clock(Duration::from_secs(61));
    cx.run_until_parked();
    assert_eq!(syncs(&o.backend), first + 1);
    cx.executor().advance_clock(Duration::from_secs(61));
    cx.run_until_parked();
    assert_eq!(syncs(&o.backend), first + 2);
}

#[gpui_kit::test]
fn on_open_only_never_checks_on_a_timer(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Cloud, CheckEvery::OnOpen);
    let first = syncs(&o.backend);
    cx.executor().advance_clock(Duration::from_secs(60 * 30));
    cx.run_until_parked();
    assert_eq!(syncs(&o.backend), first);
}

fn open_general(cx: &mut TestAppContext, o: &Open) {
    cx.update_window(o.window, |_, window, cx| {
        o.ws.update(cx, |ws, cx| ws.open_settings(window, cx));
        window.render_frame(cx);
        window.click("settings-general", cx);
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn owners_switch_the_source_after_a_warning(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Git, CheckEvery::FiveMinutes);
    open_general(cx, &o);
    assert!(labelled(cx, o.window, "Cloud"));
    assert!(labelled(cx, o.window, "acme/pay on curl-runner Cloud."));
    cx.update_window(o.window, |_, window, cx| {
        window.render_frame(cx);
        window.click(("settings-source", CollectionSource::Cloud as usize), cx);
    })
    .unwrap();
    cx.run_until_parked();
    let warning = "CI publishes to pay will be refused with “synced from the studio”. Remove the publish step, or switch back to Publish first.";
    assert!(labelled(cx, o.window, warning));
    // Nothing goes until the confirm.
    let switched = |b: &Arc<dyn Backend>| {
        b.dev()
            .unwrap()
            .recorded()
            .iter()
            .any(|c| c.starts_with("set_source("))
    };
    assert!(!switched(&o.backend));
    cx.update_window(o.window, |_, window, cx| {
        window.render_frame(cx);
        window.click("settings-source-go", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(switched(&o.backend));
    // The collection syncs now, and the card says so with no confirm left.
    assert!(o.ws.read_with(cx, |ws, _| ws.cloud_synced()));
    assert!(syncs(&o.backend) >= 1);
    assert!(!labelled(cx, o.window, warning));
}

#[gpui_kit::test]
fn members_see_the_source_as_text(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Cloud, CheckEvery::FiveMinutes);
    open_general(cx, &o);
    let view =
        o.ws.read_with(cx, |ws, _| ws.settings_view.clone())
            .unwrap();
    let mut cloud = o.ws.read_with(cx, |ws, cx| ws.cloud_source(cx)).unwrap();
    assert!(cloud.owner, "the fixture account owns acme");
    cloud.owner = false;
    view.update(cx, |v, cx| v.set_cloud(Some(cloud), cx));
    assert!(labelled(cx, o.window, "Source: Sync with my team"));
    assert!(labelled(cx, o.window, "Only team owners can change this."));
}

#[gpui_kit::test]
fn open_from_cloud_makes_a_folder_that_syncs(cx: &mut TestAppContext) {
    let o = open(cx, CollectionSource::Git, CheckEvery::FiveMinutes);
    let files = vec![BundleFile {
        path: "rates/get.yaml".into(),
        text: REQUEST.into(),
    }];
    o.backend.dev().unwrap().seed_published(
        "acme",
        "rates",
        CollectionSource::Cloud,
        files,
        "sam@acme.dev",
    );
    let home = tempfile::tempdir().unwrap();
    o.ws.update(cx, |ws, _| ws.cloud_home = Some(home.path().to_path_buf()));
    cx.update_window(o.window, |_, window, cx| {
        o.ws.update(cx, |ws, cx| ws.open_from_cloud(None, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    // Both listed; only the synced one opens here.
    assert!(labelled(cx, o.window, "v1 · sam@acme.dev edited just now"));
    assert!(labelled(
        cx,
        o.window,
        "Published from files · v1. Get it from its repo."
    ));
    assert!(labelled(cx, o.window, "~/curl-runner/Acme"));
    // The open folder is `acme/pay`: Open goes back to it, not a copy.
    let here =
        o.ws.read_with(cx, |ws, cx| {
            ws.open_cloud_view
                .as_ref()
                .and_then(|v| v.upgrade())
                .map(|v| v.read(cx).is_here("pay"))
        })
        .unwrap();
    assert!(here);
    cx.update_window(o.window, |_, window, cx| {
        window.render_frame(cx);
        window.click("open-cloud-open-rates", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let dir = home.path().join("curl-runner/Acme/rates");
    assert_eq!(
        o.ws.read_with(cx, |ws, _| ws
            .state
            .collection
            .as_ref()
            .map(|c| c.root.clone())),
        Some(dir.clone())
    );
    assert!(o.ws.read_with(cx, |ws, _| ws.cloud_synced()));
}
