//! The local-first first launch (mockup `LocalFirst`): shown once, in a
//! build without the curl-runner server, before Welcome. Cloud and
//! Self-hosted are there but wait for the server.

use super::app::mono_font;
use super::color;
use super::coming_soon::{soon_pill, soon_tooltip};
use super::icons::{self, glyph};
use super::sign_in::{choice_card, option_card, option_text, option_title};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// The board's column, as the sign-in screen's.
const COLUMN_WIDTH: f32 = 520.;
/// Where `Follow along on GitHub` goes.
const REPOSITORY_URL: &str = "https://github.com/alexvcasillas/curl-runner";

/// What the screen asks of the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstLaunchEvent {
    /// `Get started`: remember the local choice, open Welcome.
    GetStarted,
}

pub struct FirstLaunchView {
    focus: FocusHandle,
}

impl EventEmitter<FirstLaunchEvent> for FirstLaunchView {}

impl Focusable for FirstLaunchView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl FirstLaunchView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        FirstLaunchView {
            focus: cx.focus_handle(),
        }
    }
}

/// A card for what needs the server: dimmed, inert, with its tooltip.
fn disabled_card(
    id: &'static str,
    icon: &'static str,
    name: &'static str,
    text: &'static str,
) -> impl IntoElement {
    choice_card(id, name, false, false)
        .test_support()
        .opacity(0.55)
        .tooltip(soon_tooltip(
            name,
            "Needs the curl-runner server, which is not in this build.",
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap(px(4.))
                .child(option_title(icon, name, soon_pill()))
                .child(option_text().child(text)),
        )
}

impl Render for FirstLaunchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let intro = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.))
            .text_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(44.))
                    .rounded(px(11.))
                    .bg(color("primary"))
                    .text_color(color("on-primary"))
                    .font_family(mono_font())
                    .font_weight(FontWeight::MEDIUM)
                    .text_size(px(16.))
                    .child(">_"),
            )
            .child(
                div()
                    .id("first-launch-title")
                    .role(Role::Heading)
                    .test_support()
                    .aria_label("Welcome to curl-runner studio")
                    .mt(px(8.))
                    .text_size(px(24.))
                    .line_height(relative(1.2))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Welcome to curl-runner studio"),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.6))
                    .text_color(color("text-secondary"))
                    .child(
                        "Write requests as YAML, run them as tests and read the results. \
                         Everything stays on this Mac.",
                    ),
            );
        let local = option_card("first-launch-local", "On this Mac", true).child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap(px(4.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(
                            glyph(icons::FOLDER)
                                .size(px(16.))
                                .text_color(color("text-primary")),
                        )
                        .child("On this Mac"),
                )
                .child(option_text().child(
                    "Plain YAML files in folders you pick. The same files the CLI runs. \
                     No account needed.",
                )),
        );
        let choices = div()
            .id("first-launch-choice")
            .role(Role::RadioGroup)
            .aria_label("Where your collections live")
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(local)
            .child(disabled_card(
                "first-launch-cloud",
                icons::CLOUD,
                "curl-runner Cloud",
                "Sync collections and share them with the people who call your API.",
            ))
            .child(disabled_card(
                "first-launch-self-hosted",
                icons::SERVER,
                "Self-hosted",
                "The same server, open source, on your own infrastructure.",
            ));
        let primary = div()
            .id("first-launch-get-started")
            .role(Role::Button)
            .test_support()
            .aria_label("Get started")
            .flex()
            .items_center()
            .justify_center()
            .h(px(40.))
            .rounded(px(6.))
            .border_1()
            .border_color(color("primary"))
            .bg(color("primary"))
            .text_color(color("on-primary"))
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(|s| s.bg(color("primary-hover")))
            .on_click(cx.listener(|_, _, _, cx| cx.emit(FirstLaunchEvent::GetStarted)))
            .child("Get started");
        let footnote = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.))
            .text_size(px(12.))
            .line_height(relative(1.5))
            .text_color(color("text-tertiary"))
            .text_center()
            .child("Sync and sharing need the curl-runner server, which we're building now.")
            .child(
                div()
                    .id("first-launch-github")
                    .role(Role::Link)
                    .test_support()
                    .aria_label("Follow along on GitHub")
                    .cursor_pointer()
                    .text_color(color("text-secondary"))
                    .underline()
                    .on_click(|_, _, cx| cx.open_url(REPOSITORY_URL))
                    .child("Follow along on GitHub"),
            );
        let column = div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .w(px(COLUMN_WIDTH))
            .py(px(40.))
            .child(intro)
            .child(choices)
            .child(primary)
            .child(footnote);
        div()
            .id("first-launch")
            .key_context("FirstLaunch")
            .track_focus(&self.focus)
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .justify_center()
            .overflow_y_scroll()
            .bg(linear_gradient(
                180.,
                linear_color_stop(color("surface-200"), 0.),
                linear_color_stop(color("bg-100"), 0.6),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .min_h_full()
                    .child(column),
            )
    }
}

#[cfg(all(test, not(feature = "server")))]
mod tests {
    use super::super::state::{Connection, StateStore};
    use super::super::workspace::{Workspace, test_secrets};
    use crate::model::backend::test_backend;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };

    /// A window as `app::run` opens it, on `store`.
    fn open(cx: &mut TestAppContext, store: StateStore) -> (AnyWindowHandle, Entity<Workspace>) {
        cx.update(|cx| {
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
                cx.new(|cx| {
                    let mut ws =
                        Workspace::new(None, store, test_secrets(), test_backend(), window, cx);
                    ws.first_launch(window, cx);
                    ws
                })
            })
            .unwrap()
        })
    }

    fn click(cx: &mut TestAppContext, window: AnyWindowHandle, id: &'static str) {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click(id, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn shows(cx: &mut TestAppContext, window: AnyWindowHandle, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .within("workspace")
                .try_find_by_label(label)
                .is_some()
        })
        .unwrap()
    }

    fn shown(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> bool {
        ws.read_with(cx, |ws, _| ws.first_launch.is_some())
    }

    #[gpui_kit::test]
    fn local_first_shows_once(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store.clone());
        assert!(shown(cx, &ws));
        assert!(shows(cx, window, "Welcome to curl-runner studio"));
        click(cx, window, "first-launch-get-started");
        assert!(!shown(cx, &ws));
    }

    #[gpui_kit::test]
    fn cloud_and_self_hosted_are_disabled(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store.clone());
        for id in ["first-launch-cloud", "first-launch-self-hosted"] {
            click(cx, window, id);
            assert!(shown(cx, &ws), "{id} does not leave the screen");
            assert_eq!(store.connection(), None, "{id} answers nothing");
        }
        // "On this Mac" stays the choice: Get started still stores it.
        click(cx, window, "first-launch-get-started");
        assert_eq!(store.connection(), Some(Connection::Offline));
    }

    #[gpui_kit::test]
    fn get_started_stores_local_and_opens_welcome(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, store.clone());
        click(cx, window, "first-launch-get-started");
        assert_eq!(store.connection(), Some(Connection::Offline));
        assert!(!shown(cx, &ws));
        assert!(!ws.read_with(cx, |ws, _| ws.sign_in_shown()));
        assert!(shows(cx, window, "Working locally"), "Welcome's chip");
    }
}
