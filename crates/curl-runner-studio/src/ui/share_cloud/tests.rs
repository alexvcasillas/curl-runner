use super::{ENTRY_HELP, ShareCloudView, Workspace};
use crate::model::backend::{Backend, LinkAccess, ShareChange, publishing_test_backend, testing};
use crate::model::publish::CloudLink;
use crate::ui::state::{Connection, StateStore};
use crate::ui::workspace::test_secrets;
use futures::executor::block_on;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
    WindowOptions, px, size,
};
use std::sync::Arc;

const REQUEST: &str = "request:\n  name: A\n  url: https://api.example.com/a\n  method: GET\n";

/// A folder published as `team/pay` and linked to it, open in a window.
fn open(
    cx: &mut TestAppContext,
    team: &str,
    connection: Connection,
) -> (
    AnyWindowHandle,
    Entity<Workspace>,
    Arc<dyn Backend>,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.yaml"), REQUEST).unwrap();
    std::fs::write(
        dir.path().join("curl-runner.yaml"),
        "cloud:\n  collection: pay\n  name: Payments API\n  inputs:\n    - name: BASE_URL\n    - name: API_TOKEN\n      secret: true\n",
    )
    .unwrap();
    let backend = publishing_test_backend();
    testing::sign_in(&*backend);
    backend.dev().unwrap().seed_published(
        team,
        "pay",
        crate::model::backend::CollectionSource::Git,
        Vec::new(),
        "token CI",
    );
    let state = tempfile::tempdir().unwrap();
    let store = StateStore::at(state.path());
    store.set_connection(Some(connection));
    let root = dir.path().to_path_buf();
    let b = backend.clone();
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
            cx.new(|cx| Workspace::new(Some(root), store, test_secrets(), b, window, cx))
        })
        .unwrap()
    });
    let team = team.to_string();
    ws.update(cx, |ws, cx| {
        let server = ws.backend.server().url().to_string();
        ws.notes.update(cx, |m, cx| {
            m.cloud = Some(CloudLink {
                server,
                team,
                slug: "pay".into(),
            });
            cx.notify();
        });
    });
    cx.run_until_parked();
    (window, ws, backend, dir, state)
}

fn share(
    cx: &mut TestAppContext,
    window: AnyWindowHandle,
    ws: &Entity<Workspace>,
) -> Entity<ShareCloudView> {
    cx.update_window(window, |_, window, cx| {
        ws.update(cx, |ws, cx| ws.open_team_share(window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    ws.read_with(cx, |ws, _| ws.team_share_view())
        .expect("the dialog")
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

fn access(cx: &mut TestAppContext, v: &Entity<ShareCloudView>) -> Option<LinkAccess> {
    v.read_with(cx, |v, _| v.share.as_ref().map(|s| s.access))
}

fn chip(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Option<(LinkAccess, usize)> {
    ws.read_with(cx, |ws, _| ws.publish.share)
}

#[gpui_kit::test]
fn off_to_public_then_restricted_asks_first(cx: &mut TestAppContext) {
    let (window, ws, _b, _d, _s) = open(cx, "acme", Connection::Online);
    let v = share(cx, window, &ws);
    assert_eq!(access(cx, &v), Some(LinkAccess::Off));
    assert!(shows(cx, window, "Share Payments API"));
    assert!(shows(cx, window, "Edit in curl-runner.yaml"));
    click(cx, window, "share-cloud-public");
    assert_eq!(access(cx, &v), Some(LinkAccess::Public));
    assert_eq!(chip(cx, &ws), Some((LinkAccess::Public, 0)));
    click(cx, window, "share-cloud-restricted");
    // Not yet: the warning first.
    assert_eq!(access(cx, &v), Some(LinkAccess::Public));
    assert!(shows(cx, window, super::RESTRICT_WARNING));
    click(cx, window, "share-cloud-keep-public");
    assert_eq!(access(cx, &v), Some(LinkAccess::Public));
    click(cx, window, "share-cloud-restricted");
    click(cx, window, "share-cloud-make-restricted");
    assert_eq!(access(cx, &v), Some(LinkAccess::Restricted));
    assert_eq!(chip(cx, &ws), Some((LinkAccess::Restricted, 0)));
}

#[gpui_kit::test]
fn people_are_added_checked_and_removed(cx: &mut TestAppContext) {
    let (window, ws, b, _d, _s) = open(cx, "acme", Connection::Online);
    block_on(b.change_team_share(
        "acme",
        "pay",
        ShareChange {
            access: Some(LinkAccess::Restricted),
            ..Default::default()
        },
    ))
    .unwrap();
    let v = share(cx, window, &ws);
    let add = |cx: &mut TestAppContext, text: &str| {
        let text = text.to_string();
        cx.update_window(window, |_, window, cx| {
            v.update(cx, |v, cx| {
                v.entry.update(cx, |i, cx| i.set_value(text, window, cx));
                v.add_entry(cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
    };
    add(cx, "jane");
    assert_eq!(
        v.read_with(cx, |v, _| v.entry_error.clone()).as_deref(),
        Some(ENTRY_HELP)
    );
    add(cx, "Jane@Globex.com");
    add(cx, "*@initech.com");
    assert!(shows(cx, window, "jane@globex.com, Invited · not opened"));
    assert_eq!(
        v.read_with(cx, |v, _| v.share.clone().unwrap().entries.len()),
        2
    );
    assert_eq!(chip(cx, &ws), Some((LinkAccess::Restricted, 2)));
    // The field is cleared after an add.
    assert_eq!(
        v.read_with(cx, |v, cx| v.entry.read(cx).value().to_string()),
        ""
    );
    click(cx, window, "share-cloud-remove-0");
    assert!(!shows(cx, window, "jane@globex.com, Invited · not opened"));
    assert_eq!(chip(cx, &ws), Some((LinkAccess::Restricted, 1)));
    let recorded = b.dev().unwrap().recorded();
    assert!(
        recorded
            .iter()
            .any(|c| c == r#"add_share_entry("acme", "pay", "jane@globex.com", true)"#)
    );
}

#[gpui_kit::test]
fn reset_link_asks_first(cx: &mut TestAppContext) {
    let (window, ws, _b, _d, _s) = open(cx, "acme", Connection::Online);
    let v = share(cx, window, &ws);
    click(cx, window, "share-cloud-public");
    let before = v.read_with(cx, |v, _| v.share.clone().unwrap().link);
    click(cx, window, "share-cloud-reset");
    assert_eq!(
        v.read_with(cx, |v, _| v.share.clone().unwrap().link),
        before
    );
    click(cx, window, "share-cloud-reset-go");
    assert_ne!(
        v.read_with(cx, |v, _| v.share.clone().unwrap().link),
        before
    );
}

#[gpui_kit::test]
fn a_free_team_cannot_pick_restricted(cx: &mut TestAppContext) {
    // Jamie's account only knows Acme: another team reads as Free.
    let (window, ws, _b, _d, _s) = open(cx, "globex", Connection::Online);
    let v = share(cx, window, &ws);
    assert!(shows(cx, window, "Upgrade"));
    click(cx, window, "share-cloud-public");
    click(cx, window, "share-cloud-restricted");
    assert_eq!(access(cx, &v), Some(LinkAccess::Public));
    assert!(v.read_with(cx, |v, _| v.confirm.is_none()));
}

#[gpui_kit::test]
fn offline_is_read_only(cx: &mut TestAppContext) {
    let (window, ws, b, _d, _s) = open(cx, "acme", Connection::Offline);
    let v = share(cx, window, &ws);
    assert!(shows(cx, window, super::OFFLINE));
    click(cx, window, "share-cloud-public");
    assert_eq!(access(cx, &v), Some(LinkAccess::Off));
    assert!(
        !b.dev()
            .unwrap()
            .recorded()
            .iter()
            .any(|c| c.starts_with("change_team_share"))
    );
}

#[gpui_kit::test]
fn the_foot_share_opens_it(cx: &mut TestAppContext) {
    let (window, ws, b, d, _s) = open(cx, "acme", Connection::Online);
    // Publish the folder as it is: the foot reads up to date, with Share.
    let target = curl_runner_cloud_client::publish::check(d.path(), Some("pay"))
        .target
        .unwrap();
    block_on(b.publish("acme", "pay", target.request(None))).unwrap();
    cx.update_window(window, |_, window, cx| {
        ws.update(cx, |ws, cx| ws.refresh_publish(false, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, window, "sidebar-share");
    assert!(ws.read_with(cx, |ws, _| ws.team_share_view()).is_some());
}

#[gpui_kit::test]
fn listing_needs_public_and_a_description(cx: &mut TestAppContext) {
    let (window, ws, b, d, _s) = open(cx, "acme", Connection::Online);
    // Published without a description: the switch is there but off.
    let v = share(cx, window, &ws);
    click(cx, window, "share-cloud-public");
    assert!(shows(cx, window, "List in the gallery, off"));
    click(cx, window, "share-cloud-gallery");
    assert!(!v.read_with(cx, |v, _| v.share.clone().unwrap().listed));
    // Publish with one, then list it.
    let mut target = curl_runner_cloud_client::publish::check(d.path(), Some("pay"))
        .target
        .unwrap();
    target.description = Some("Charges and refunds".into());
    block_on(b.publish("acme", "pay", target.request(None))).unwrap();
    cx.update_window(window, |_, _, cx| {
        v.update(cx, |v, cx| v.load(cx));
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, window, "share-cloud-gallery");
    assert!(v.read_with(cx, |v, _| v.share.clone().unwrap().listed));
    assert!(shows(cx, window, "View it on curl-runner.com"));
    // Off takes it out of the gallery.
    click(cx, window, "share-cloud-off");
    assert!(!v.read_with(cx, |v, _| v.share.clone().unwrap().listed));
    assert!(!shows(cx, window, "List in the gallery, off"));
}

#[gpui_kit::test]
fn a_description_is_written_in_share_then_listed(cx: &mut TestAppContext) {
    let (window, ws, _b, _d, _s) = open(cx, "acme", Connection::Online);
    let v = share(cx, window, &ws);
    click(cx, window, "share-cloud-public");
    assert!(shows(cx, window, "0 / 500"), "the field, empty");
    // Typed and saved: not listed yet, the switch does that.
    cx.update_window(window, |_, window, cx| {
        v.update(cx, |v, cx| {
            v.description
                .update(cx, |t, cx| t.set_value("Charges and refunds", window, cx))
        })
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, window, "share-cloud-description-save");
    let shared = v.read_with(cx, |v, _| v.share.clone().unwrap());
    assert_eq!(shared.description.as_deref(), Some("Charges and refunds"));
    assert!(!shared.listed);
    assert!(shows(cx, window, "Charges and refunds"));
    assert!(!shows(cx, window, "19 / 500"), "the field closed");
    click(cx, window, "share-cloud-gallery");
    assert!(v.read_with(cx, |v, _| v.share.clone().unwrap().listed));
    // Edit opens it with the text; clicks there don't toggle the listing.
    click(cx, window, "share-cloud-description-edit");
    assert!(shows(cx, window, "19 / 500"));
    click(cx, window, "share-cloud-gallery");
    assert!(v.read_with(cx, |v, _| v.share.clone().unwrap().listed));
    click(cx, window, "share-cloud-description-cancel");
    assert!(!shows(cx, window, "19 / 500"));
}
