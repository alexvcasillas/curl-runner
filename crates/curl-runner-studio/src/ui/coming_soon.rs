//! "Coming soon": how a build without the curl-runner server marks what
//! needs it (mockups `LocalFirst`, `WelcomeLocal`). The yellow pill, the
//! tooltip on a disabled control and the Settings card.

use super::color;
use super::settings_kit::{Observed, card, card_body, card_text, card_title};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;

/// What the pill says unless told otherwise.
pub const COMING_SOON: &str = "Coming soon";

/// The `LocalFirst` pill: 11px medium `warning` text on `warning-subtle`,
/// a `warning-border` edge, fully rounded, `0 7px`, 18px tall lines.
pub fn soon_pill() -> Observed {
    soon_pill_labelled(COMING_SOON)
}

/// [`soon_pill`] saying `label` (the sidebar's `Sync soon`).
pub fn soon_pill_labelled(label: &'static str) -> Observed {
    div()
        .id("soon-pill")
        .role(Role::Status)
        .test_support()
        .aria_label(label)
        .flex_none()
        .px(px(7.))
        .rounded_full()
        .border_1()
        .border_color(color("warning-border"))
        .bg(color("warning-subtle"))
        .text_size(px(11.))
        .line_height(px(18.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color("warning"))
        .whitespace_nowrap()
        .child(label)
}

/// The tooltip for a disabled control: `title` with the pill, then
/// `body`. Pass it to `.tooltip(…)`.
pub fn soon_tooltip(
    title: &'static str,
    body: &'static str,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |window, cx| {
        Tooltip::element(move |_, _| {
            div()
                .id("soon-tooltip")
                .role(Role::Group)
                .test_support()
                .aria_label(format!("{title}. {body}"))
                .flex()
                .flex_col()
                .gap(px(4.))
                .max_w(px(260.))
                .py(px(4.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color("text-primary"))
                        .child(title)
                        .child(soon_pill()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(relative(1.5))
                        .text_color(color("text-secondary"))
                        .child(body),
                )
        })
        .build(window, cx)
    }
}

/// A Settings card for what needs the server: `title` with the pill, then
/// `body`.
pub fn soon_card(title: &'static str, body: &'static str) -> impl IntoElement {
    card("soon-card").aria_label(title).child(
        card_body()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(card_title(title))
                    .child(soon_pill()),
            )
            .child(card_text(body)),
    )
}

/// Without the server: every server surface is marked or gone.
#[cfg(all(test, not(feature = "server")))]
mod tests {
    use super::super::app::{bindings, menus};
    use super::super::open_shared::{link_handler, pump_links};
    use super::super::palette::{PaletteAction, Target, palette_entries};
    use super::super::settings_view::AppPage;
    use super::super::state::{Connection, StateStore};
    use super::super::sync::SYNC_DELAY;
    use super::super::workspace::{Workspace, test_backend, test_secrets};
    use crate::model::backend::Backend;
    use crate::model::backend::testing::Spy;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, MenuItem, Point, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    const REQUEST: &str = "request:\n  url: https://api.example.com/a\n";

    /// A mock that is signed in: nothing but the build keeps it from
    /// syncing or sharing.
    fn signed_in() -> Arc<dyn Backend> {
        let backend = test_backend();
        crate::model::backend::testing::sign_in(&*backend);
        backend
    }

    fn open(
        cx: &mut TestAppContext,
        root: Option<PathBuf>,
        store: StateStore,
        backend: Arc<dyn Backend>,
    ) -> (AnyWindowHandle, Entity<Workspace>) {
        let out = cx.update(|cx| {
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
                cx.new(|cx| Workspace::new(root, store, test_secrets(), backend, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        out
    }

    fn collection() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.yaml"), REQUEST).unwrap();
        dir
    }

    fn shows(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        scope: &'static str,
        label: &str,
    ) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within(scope).try_find_by_label(label).is_some()
        })
        .unwrap()
    }

    fn dialog_open(cx: &mut TestAppContext, window: AnyWindowHandle) -> bool {
        cx.update_window(window, |_, window, cx| window.has_active_dialog(cx))
            .unwrap()
    }

    fn calls(backend: &Arc<dyn Backend>, prefix: &str) -> Vec<String> {
        backend
            .dev()
            .unwrap()
            .recorded()
            .into_iter()
            .filter(|c| c.starts_with(prefix))
            .collect()
    }

    #[gpui_kit::test]
    fn welcome_shared_link_card_is_disabled_with_coming_soon(cx: &mut TestAppContext) {
        let (window, ws) = open(cx, None, StateStore::none(), test_backend());
        assert!(shows(cx, window, "welcome-open-link", "Coming soon"));
        assert!(shows(
            cx,
            window,
            "welcome-open-link",
            "Open a collection someone shared with you. Arrives with the curl-runner server."
        ));
        cx.update_window(window, |_, window, cx| {
            window.click("welcome-open-link", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!dialog_open(cx, window));
        assert!(ws.read_with(cx, |ws, _| ws.open_shared_view().is_none()));
    }

    /// Welcome, the empty sidebar and the status bar promise nothing the
    /// build can't do.
    #[test]
    fn local_copy_never_mentions_sharing() {
        let local = [
            super::super::welcome::WELCOME_LEDE_LOCAL,
            super::super::sidebar::NO_COLLECTIONS_LOCAL,
            super::super::status_bar::HISTORY_NOTE,
        ];
        assert_eq!(
            local[0],
            "Write requests as YAML, run them as tests and read the results."
        );
        assert_eq!(
            local[1],
            "A collection is a folder of request files. Open one from disk or start a new one."
        );
        assert_eq!(local[2], "History stays on this device.");
        for text in local {
            let text = text.to_lowercase();
            assert!(!text.contains("shar") && !text.contains("sync"), "{text}");
        }
    }

    #[gpui_kit::test]
    fn share_button_is_disabled_and_explains(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(
            cx,
            Some(dir.path().to_path_buf()),
            StateStore::none(),
            backend.clone(),
        );
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("sidebar-sync").click("sidebar-share", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!dialog_open(cx, window));
        assert!(ws.read_with(cx, |ws, _| ws.share_view().is_none()));
        assert!(calls(&backend, "share").is_empty());
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.within("sidebar-sync").hover("sidebar-share", cx);
        })
        .unwrap();
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("soon-tooltip").is_some());
            assert!(window.try_find_by_label(
                "Sharing. Share links need the curl-runner server. Until then, share the YAML files through git."
            ).is_some());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn palette_has_no_open_shared_link(_cx: &mut TestAppContext) {
        let listed = palette_entries(None, &[], false);
        assert!(
            !listed
                .iter()
                .any(|(_, t)| *t == Target::Action(PaletteAction::OpenSharedLink))
        );
        let file = menus(None).into_iter().find(|m| m.name == "File").unwrap();
        assert!(
            !file.items.iter().any(
                |i| matches!(i, MenuItem::Action { name, .. } if name.contains("Shared Link"))
            )
        );
        assert!(!bindings().iter().any(|s| s.label == "Open a shared link"));
    }

    #[gpui_kit::test]
    fn cold_link_shows_coming_soon_notice_not_sign_in(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_connection(Some(Connection::Offline));
        // Queued before the window exists, as a cold launch from a link.
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let mut handler = link_handler(tx);
        handler(vec![
            "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm"
                .into(),
        ]);
        let (window, ws) = open(cx, None, store, test_backend());
        cx.update(|cx| pump_links(rx, window, ws.downgrade(), cx));
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            assert!(!ws.sign_in_shown());
            assert!(ws.open_shared_view().is_none());
            assert_eq!(
                ws.toasts,
                ["Shared links need the curl-runner server, which is coming soon."]
            );
        });
        assert!(!dialog_open(cx, window));
        assert!(!state.path().join("shared").exists());
    }

    #[gpui_kit::test]
    fn settings_accounts_page_is_one_coming_soon_card(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(
            cx,
            Some(dir.path().to_path_buf()),
            StateStore::none(),
            signed_in(),
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_app_settings(AppPage::Accounts, window, cx)
            });
            window.render_frame(cx);
            assert_eq!(window.within("app-settings").find_all("soon-card").len(), 1);
            for gone in ["server-card", "account-card", "sync-card"] {
                assert!(window.try_find(gone).is_none(), "{gone}");
            }
            assert!(window.try_find("credentials-card").is_some());
        })
        .unwrap();
        for gone in ["Sync now", "Sign in", "Sign out"] {
            assert!(!shows(cx, window, "app-settings", gone), "{gone}");
        }
    }

    #[gpui_kit::test]
    fn collection_settings_has_no_sharing_page(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(
            cx,
            Some(dir.path().to_path_buf()),
            StateStore::none(),
            signed_in(),
        );
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_settings(window, cx));
            window.render_frame(cx);
            assert!(window.try_find("settings-environments").is_some());
            assert!(window.try_find("settings-sharing").is_none());
            assert!(window.try_find("environments-callout").is_none());
        })
        .unwrap();
    }

    /// Received copies an earlier build left behind: no `Shared with me`,
    /// no owner's `Shared by` rows, an ordinary writable folder.
    #[gpui_kit::test]
    fn leftover_received_copies_stay_local_folders(cx: &mut TestAppContext) {
        use crate::model::backend::{ReceivedShare, SharedVar};
        use crate::model::collection_meta::CollectionMeta;
        use crate::model::received::{self, LocalCopy, ReceivedCopy};
        let state = tempfile::tempdir().unwrap();
        let base = state.path();
        let copy = collection();
        let meta = CollectionMeta {
            read_only_from: Some("Acme".into()),
            received_vars: vec![SharedVar {
                name: "REGION".into(),
                description: String::new(),
                required: false,
                secret: false,
                default: Some("eu".into()),
                locked: true,
            }],
            ..Default::default()
        };
        meta.save(base, copy.path()).unwrap();
        let other = collection();
        for root in [copy.path(), other.path()] {
            let local = LocalCopy {
                share_id: format!("acme/{}", root.display()),
                root: root.to_path_buf(),
                collection_id: "c".into(),
            };
            received::record(base, local).unwrap();
        }
        assert!(base.join("received.json").is_file());
        let store = StateStore::at(base);
        store.set_connection(Some(Connection::Offline));
        let (window, ws) = open(cx, Some(copy.path().to_path_buf()), store, signed_in());
        // Even with a copy listed, the section isn't drawn.
        ws.update(cx, |ws, _| {
            assert!(ws.received_all.is_empty());
            assert!(ws.read_only_org().is_none());
            ws.received_all.push(ReceivedCopy {
                share: ReceivedShare {
                    link: crate::model::backend::ShareLink {
                        server: "https://api.mock.curl-runner.test".into(),
                        token: "crsl_x".into(),
                    },
                    team: "Acme".into(),
                    name: "Shipping API".into(),
                    version: 1,
                    added: chrono::Utc::now().date_naive(),
                    files: Default::default(),
                },
                root: other.path().to_path_buf(),
                collection_id: "c".into(),
            });
        });
        assert!(!shows(cx, window, "sidebar", "Shared with me"));
        assert!(!shows(cx, window, "sidebar", "Shipping API"));
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_settings(window, cx));
            window.render_frame(cx);
            // Workspace › Members, Sync & hosting and Billing need the server.
            for gone in ["settings-members", "settings-sync", "settings-billing"] {
                assert!(window.try_find(gone).is_none(), "{gone}");
            }
        })
        .unwrap();
        cx.run_until_parked();
        let listed = ws.read_with(cx, |ws, cx| {
            let view = ws.settings_view.as_ref().unwrap().read(cx);
            view.rows().iter().any(|r| r.name == "REGION")
        });
        assert!(!listed, "the owner's default isn't listed");
        // An ordinary, writable folder.
        let mut c = crate::model::collection::Collection::open_in(copy.path(), Some(base)).unwrap();
        c.create_file(Path::new("b.yaml"), REQUEST).unwrap();
    }

    #[gpui_kit::test]
    fn sidebar_says_working_locally_and_never_syncs(cx: &mut TestAppContext) {
        let spy = Arc::new(Spy::new(signed_in()));
        let backend: Arc<dyn Backend> = spy.clone();
        // Welcome's foot.
        let (window, _) = open(cx, None, StateStore::none(), backend.clone());
        assert!(shows(cx, window, "sidebar", "Working locally"));
        assert!(shows(cx, window, "sidebar", "Sync soon"));
        assert!(!shows(cx, window, "sidebar", "Change"));
        // A collection's foot, through a save and the sync pause.
        let dir = collection();
        let (window, ws) = open(
            cx,
            Some(dir.path().to_path_buf()),
            StateStore::none(),
            backend.clone(),
        );
        assert!(shows(cx, window, "sidebar-sync", "Working locally"));
        assert!(shows(cx, window, "sidebar-sync", "Sync soon"));
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("a.yaml"), window, cx);
                let ix = ws.state.active.unwrap();
                ws.state.open[ix].set_text("request:\n  url: https://api.example.com/b\n".into());
                assert!(ws.save(ix, window, cx));
                ws.sync_now(window, cx);
            });
        })
        .unwrap();
        cx.executor().advance_clock(SYNC_DELAY * 3);
        cx.run_until_parked();
        assert!(spy.sent.lock().unwrap().is_empty());
        assert!(calls(&backend, "sync").is_empty());
        assert!(calls(&backend, "share").is_empty());
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("title-offline").is_none());
            assert!(window.try_find("status-sync").is_none());
        })
        .unwrap();
    }
}
