//! The sign-in screen (board: SignIn): where collections live (curl-runner
//! Cloud or a self-hosted server, with `Test connection`), the sign-in
//! buttons and `Continue offline`. Shown on first launch, before Welcome,
//! and from Welcome's `Change` and Settings' `Sign in` / `Switch account`.

use super::app::{CloseSettings, mono_font};
use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{neutral_pill, small_button};
use super::state::Connection;
use crate::model::backend::{Backend, BackendError, ConnectionInfo, Server, SignInMethod};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::sync::Arc;

/// The board's column.
const COLUMN_WIDTH: f32 = 520.;
/// Where `Setup guide` goes: the project, whose README covers running a
/// server.
pub const SETUP_GUIDE_URL: &str = env!("CARGO_PKG_REPOSITORY");
const URL_PLACEHOLDER: &str = "https://curl-runner.example.com";

/// What the screen asks of the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInEvent {
    /// Signed in (`Online`) or `Continue offline`: remember it, move on.
    Done(Connection),
    /// Esc, when the screen was opened from elsewhere.
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    Cloud,
    SelfHosted,
}

/// `Test connection`'s line under the URL.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Test {
    Idle,
    Testing,
    Connected(ConnectionInfo),
    Failed(String),
}

pub struct SignInView {
    backend: Arc<dyn Backend>,
    pick: Pick,
    url: Entity<InputState>,
    test: Test,
    /// The sign-in in flight, by method.
    busy: Option<SignInMethod>,
    /// The last sign-in's error, under the primary button.
    error: Option<String>,
    /// Esc leaves (not on first launch, where there is nowhere to go).
    can_go_back: bool,
    task: Option<Task<()>>,
    test_task: Option<Task<()>>,
    focus: FocusHandle,
    _url_changes: Subscription,
}

impl EventEmitter<SignInEvent> for SignInView {}

impl Focusable for SignInView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SignInView {
    pub fn new(
        backend: Arc<dyn Backend>,
        can_go_back: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (pick, url) = match backend.server() {
            Server::Cloud => (Pick::Cloud, String::new()),
            Server::SelfHosted { url } => (Pick::SelfHosted, url),
        };
        let url = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(URL_PLACEHOLDER)
                .default_value(url)
        });
        // A test speaks for the address it tested only.
        let url_changes = cx.subscribe(&url, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) && this.test != Test::Idle {
                this.test = Test::Idle;
                this.test_task = None;
                cx.notify();
            }
        });
        SignInView {
            backend,
            pick,
            url,
            test: Test::Idle,
            busy: None,
            error: None,
            can_go_back,
            task: None,
            test_task: None,
            focus: cx.focus_handle(),
            _url_changes: url_changes,
        }
    }

    /// The server the picked card names.
    fn server(&self, cx: &App) -> Server {
        match self.pick {
            Pick::Cloud => Server::Cloud,
            Pick::SelfHosted => Server::SelfHosted {
                url: self.url.read(cx).value().trim().to_string(),
            },
        }
    }

    fn pick(&mut self, pick: Pick, cx: &mut Context<Self>) {
        if self.pick != pick {
            self.pick = pick;
            self.error = None;
            cx.notify();
        }
    }

    pub fn test_connection(&mut self, cx: &mut Context<Self>) {
        let server = self.server(cx);
        self.test = Test::Testing;
        let call = backend_call(cx, &self.backend, move |b| b.test_connection(&server));
        self.test_task = Some(cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                this.test = match result {
                    Ok(info) => Test::Connected(info),
                    Err(e) => Test::Failed(e.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Chooses the picked server and signs in with `method`. A self-hosted
    /// address is tested first, so a typo fails here, not on first sync.
    pub fn continue_with(&mut self, method: SignInMethod, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let server = self.server(cx);
        self.busy = Some(method);
        self.error = None;
        let call = backend_call(cx, &self.backend, move |b| {
            Box::pin(async move {
                if matches!(server, Server::SelfHosted { .. }) {
                    b.test_connection(&server).await?;
                }
                b.set_server(server).await?;
                b.sign_in(method).await
            })
        });
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(_) => cx.emit(SignInEvent::Done(Connection::Online)),
                    Err(e) => this.error = Some(sign_in_error(&e)),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// The primary button's method: what the tested server signs in with,
    /// SSO for an untested self-hosted one, GitHub on Cloud.
    fn primary_method(&self) -> SignInMethod {
        match (&self.pick, &self.test) {
            (Pick::Cloud, _) => SignInMethod::GitHub,
            (Pick::SelfHosted, Test::Connected(info)) => info.sign_in,
            (Pick::SelfHosted, _) => SignInMethod::Sso,
        }
    }

    /// `CURL_RUNNER_STUDIO_SHOW=signin`: with `url`, the Self-hosted card on
    /// it, tested.
    pub fn dev_fill(&mut self, url: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(url) = url else { return };
        self.pick = Pick::SelfHosted;
        let url = url.to_string();
        self.url.update(cx, |s, cx| s.set_value(url, window, cx));
        self.test_connection(cx);
    }

    #[cfg(test)]
    pub fn test_text(&self) -> Option<String> {
        match &self.test {
            Test::Connected(info) => Some(connected_text(info)),
            Test::Failed(e) => Some(e.clone()),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[cfg(test)]
    pub fn set_self_hosted(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.pick = Pick::SelfHosted;
        // `set_value` is not an edit: no `Change` resets the test.
        self.test = Test::Idle;
        let url = url.to_string();
        self.url.update(cx, |s, cx| s.set_value(url, window, cx));
    }
}

/// `Connected · server 2.1.1 · sign-in via SSO · 42 ms`.
pub fn connected_text(info: &ConnectionInfo) -> String {
    format!(
        "Connected · server {} · sign-in via {} · {} ms",
        info.version,
        info.sign_in.label(),
        info.latency_ms
    )
}

/// What a failed sign-in says under the button.
fn sign_in_error(e: &BackendError) -> String {
    match e {
        BackendError::Offline => "You're offline. Connect, or continue offline.".into(),
        other => other.to_string(),
    }
}

impl Render for SignInView {
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
                    .id("sign-in-title")
                    .role(Role::Heading)
                    .test_support()
                    .aria_label("Where should your collections live?")
                    .mt(px(8.))
                    .text_size(px(24.))
                    .line_height(relative(1.2))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Where should your collections live?"),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.6))
                    .text_color(color("text-secondary"))
                    .child(
                        "curl-runner is open source. Use our managed cloud, or point the studio \
                         at a server you run yourself.",
                    ),
            );
        let choices = div()
            .id("server-choice")
            .role(Role::RadioGroup)
            .aria_label("Server")
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(self.render_cloud(cx))
            .child(self.render_self_hosted(cx));
        let method = self.primary_method();
        let label = match self.busy {
            Some(_) => "Signing in…".to_string(),
            None => format!("Continue with {}", method.label()),
        };
        let primary = div()
            .id("sign-in-continue")
            .role(Role::Button)
            .test_support()
            .aria_label(label.clone())
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
            .when(self.busy.is_some(), |d| d.opacity(0.6))
            .when(self.busy.is_none(), |d| {
                d.cursor_pointer()
                    .hover(|s| s.bg(color("primary-hover")))
                    .on_click(cx.listener(move |this, _, _, cx| this.continue_with(method, cx)))
            })
            .child(label);
        let error = self.error.clone().map(|e| {
            div()
                .id("sign-in-error")
                .role(Role::Alert)
                .test_support()
                .aria_label(e.clone())
                .mt(px(-14.))
                .text_size(px(12.))
                .text_color(color("danger"))
                .text_center()
                .child(e)
        });
        let rule = || div().flex_1().h(px(1.)).bg(color("border-100"));
        let or = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .text_size(px(12.))
            .text_color(color("text-gutter"))
            .child(rule())
            .child("or")
            .child(rule());
        let column = div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .w(px(COLUMN_WIDTH))
            .py(px(40.))
            .child(intro)
            .child(choices)
            .child(primary)
            .children(error)
            .child(or)
            .child(self.render_offline(cx))
            .child(self.render_cloud_methods(cx));
        div()
            .id("sign-in")
            .key_context("SignIn")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &CloseSettings, _, cx| {
                if this.can_go_back {
                    cx.emit(SignInEvent::Back);
                }
            }))
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .justify_center()
            .overflow_y_scroll()
            // The board's glow from the top: a radial in CSS, the nearest
            // GPUI draws is linear.
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

/// The 16px radio: a ring, or a 5px ring around a dark dot when `on`.
fn radio(on: bool) -> Div {
    div()
        .flex_none()
        .mt(px(2.))
        .size(px(16.))
        .rounded_full()
        .map(|d| {
            if on {
                d.border(px(5.)).border_color(color("text-primary"))
            } else {
                d.border_1().border_color(color("text-disabled"))
            }
        })
}

/// A choice card: `.opt`, outlined in `text-primary` when picked.
fn option_card(id: &'static str, label: &'static str, on: bool) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::RadioButton)
        .aria_label(label)
        .flex()
        .gap(px(12.))
        .p(px(16.))
        .rounded(px(8.))
        .border_1()
        .map(|d| {
            if on {
                d.border_color(color("text-primary"))
                    .bg(color("surface-raised"))
            } else {
                d.border_color(color("border-200"))
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-100")))
            }
        })
        .child(radio(on))
}

/// The card's name line: icon, name and the pill.
fn option_title(icon: &'static str, name: &'static str, pill: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .child(glyph(icon).size(px(16.)).text_color(color("text-primary")))
        .child(name)
        .child(neutral_pill().text_size(px(11.)).px(px(7.)).child(pill))
}

fn option_text() -> Div {
    div()
        .text_size(px(12.))
        .line_height(relative(1.5))
        .text_color(color("text-secondary"))
}

/// The ` · ` between the connection facts.
fn sep() -> Div {
    div().text_color(color("text-disabled")).child("·")
}

impl SignInView {
    fn render_cloud(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let on = self.pick == Pick::Cloud;
        option_card("server-cloud", "curl-runner Cloud", on)
            .when(!on, |d| {
                d.on_click(cx.listener(|this, _, _, cx| this.pick(Pick::Cloud, cx)))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap(px(4.))
                    .child(option_title(icons::CLOUD, "curl-runner Cloud", "Managed"))
                    .child(
                        option_text().child(
                            "Nothing to run. Sign in with GitHub or email and start sharing.",
                        ),
                    ),
            )
    }

    fn render_self_hosted(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let on = self.pick == Pick::SelfHosted;
        let text = option_text()
            .flex()
            .flex_wrap()
            .child("Your own curl-runner server, deployed with Docker.\u{a0}");
        let text = text.child(
            div()
                .id("setup-guide")
                .role(Role::Link)
                .text_color(color("accent-blue"))
                .cursor_pointer()
                .on_click(|_, _, cx| cx.open_url(SETUP_GUIDE_URL))
                .child("Setup guide"),
        );
        let head = div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(option_title(icons::SERVER, "Self-hosted", "Open source"))
            .child(text);
        let body = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(px(12.))
            .child(head)
            .when(on, |d| {
                d.child(self.render_url(cx)).children(self.render_test())
            });
        option_card("server-self-hosted", "Self-hosted", on)
            .when(!on, |d| {
                d.on_click(cx.listener(|this, _, _, cx| this.pick(Pick::SelfHosted, cx)))
            })
            .child(body)
    }

    fn render_url(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let border = match &self.test {
            Test::Connected(_) => color("success"),
            Test::Failed(_) => color("danger"),
            _ => color("border-200"),
        };
        let testing = self.test == Test::Testing;
        let button = small_button(
            "test-connection",
            if testing {
                "Testing…"
            } else {
                "Test connection"
            },
        )
        .h(px(32.))
        .px(px(12.))
        .text_size(px(13.))
        .when(testing, |d| d.opacity(0.6))
        .when(!testing, |d| {
            d.on_click(cx.listener(|this, _, _, cx| this.test_connection(cx)))
        })
        .child(if testing {
            "Testing…"
        } else {
            "Test connection"
        });
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .child("Server URL")
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(mono_font())
                            .text_color(color("text-primary"))
                            .child(
                                Input::new(&self.url)
                                    .aria_label("Server URL")
                                    .min_h(px(32.))
                                    .text_size(px(12.))
                                    .border_color(border),
                            ),
                    )
                    .child(button),
            )
    }

    fn render_test(&self) -> Option<impl IntoElement + use<>> {
        let line = div()
            .id("connection-status")
            .role(Role::Status)
            .test_support()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .text_size(px(12.))
            .text_color(color("text-secondary"));
        match &self.test {
            Test::Idle | Test::Testing => None,
            Test::Connected(info) => Some(
                line.aria_label(connected_text(info))
                    .child(
                        glyph(icons::CHECK)
                            .size(px(13.))
                            .text_color(color("success")),
                    )
                    .child(div().text_color(color("success")).child("Connected"))
                    .child(sep())
                    .child(format!("server {}", info.version))
                    .child(sep())
                    .child(format!("sign-in via {}", info.sign_in.label()))
                    .child(sep())
                    .child(format!("{} ms", info.latency_ms)),
            ),
            Test::Failed(e) => Some(
                line.aria_label(e.clone())
                    .child(
                        glyph(icons::CROSS)
                            .size(px(13.))
                            .text_color(color("danger")),
                    )
                    .child(div().text_color(color("danger")).child(e.clone())),
            ),
        }
    }

    fn render_offline(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(16.))
            .py(px(14.))
            .rounded(px(8.))
            .border_1()
            .border_color(color("border-100"))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(32.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(color("border-200"))
                    .child(
                        glyph(icons::FOLDER)
                            .size(px(16.))
                            .text_color(color("text-secondary")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Use offline"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child("Local YAML files only. Nothing syncs, no account needed."),
                    ),
            )
            .child(
                small_button("continue-offline", "Continue offline")
                    .on_click(
                        cx.listener(|_, _, _, cx| cx.emit(SignInEvent::Done(Connection::Offline))),
                    )
                    .child("Continue offline"),
            )
    }

    /// `Picked Cloud?` GitHub · Email: signs in to Cloud.
    fn render_cloud_methods(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let method = |id: &'static str,
                      icon: &'static str,
                      label: &'static str,
                      m: SignInMethod,
                      cx: &mut Context<Self>| {
            small_button(id, label)
                .gap(px(6.))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.pick(Pick::Cloud, cx);
                    this.continue_with(m, cx);
                }))
                .child(glyph(icon).size(px(15.)))
                .child(label)
        };
        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(16.))
            .text_size(px(12.))
            .child(
                div()
                    .text_color(color("text-tertiary"))
                    .child("Picked Cloud?"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(method(
                        "sign-in-github",
                        icons::GITHUB,
                        "GitHub",
                        SignInMethod::GitHub,
                        cx,
                    ))
                    .child(method(
                        "sign-in-email",
                        icons::MAIL,
                        "Email",
                        SignInMethod::Email,
                        cx,
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::super::settings_view::AppPage;
    use super::super::state::{Connection, StateStore};
    use super::super::workspace::{Workspace, test_secrets};
    use crate::model::backend::{Backend, SignInMethod, test_backend};
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::sync::Arc;

    /// A window on Welcome, as `app::run` opens it: the sign-in screen
    /// first when the store has no answer.
    fn open(
        cx: &mut TestAppContext,
        store: StateStore,
        backend: Arc<dyn Backend>,
    ) -> (AnyWindowHandle, Entity<Workspace>) {
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
                    let mut ws = Workspace::new(None, store, test_secrets(), backend, window, cx);
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

    fn sign_in_shown(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> bool {
        ws.read_with(cx, |ws, _| ws.sign_in_shown())
    }

    #[gpui_kit::test]
    fn first_launch_shows_sign_in_once(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let backend = test_backend();
        let (window, ws) = open(cx, store.clone(), backend.clone());
        assert!(sign_in_shown(cx, &ws));
        assert!(shows(cx, window, "Where should your collections live?"));
        assert!(shows(cx, window, "Not signed in"), "the status bar");
        // Cloud is picked: its primary button signs in with GitHub.
        click(cx, window, "sign-in-continue");
        assert!(!sign_in_shown(cx, &ws));
        assert_eq!(store.connection(), Some(Connection::Online));
        assert_eq!(backend.account().unwrap().email, "jamie@acme.dev");
        assert!(shows(cx, window, "Connected to curl-runner Cloud"));

        // The next launch goes straight to Welcome.
        let (_, ws) = open(cx, store, backend);
        assert!(!sign_in_shown(cx, &ws));
    }

    #[gpui_kit::test]
    fn continue_offline_skips_next_launch(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let backend = test_backend();
        let (window, ws) = open(cx, store.clone(), backend.clone());
        click(cx, window, "continue-offline");
        assert!(!sign_in_shown(cx, &ws));
        assert_eq!(store.connection(), Some(Connection::Offline));
        assert!(backend.account().is_none());
        assert!(shows(cx, window, "Offline"), "Welcome's chip");

        let (window, ws) = open(cx, store, backend);
        assert!(!sign_in_shown(cx, &ws));
        // `Change` brings the screen back; Esc leaves it, as it was answered.
        click(cx, window, "sidebar-cloud-change");
        assert!(sign_in_shown(cx, &ws));
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.press("escape", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(!sign_in_shown(cx, &ws));
    }

    #[gpui_kit::test]
    fn self_hosted_test_connection_shows_info(cx: &mut TestAppContext) {
        let backend = test_backend();
        let (window, ws) = open(cx, StateStore::none(), backend.clone());
        let view = ws.read_with(cx, |ws, _| ws.sign_in.clone().unwrap());
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_self_hosted("https://studio.api.example.com", window, cx)
            });
        })
        .unwrap();
        click(cx, window, "test-connection");
        let connected = "Connected · server 2.1.1 · sign-in via SSO · 42 ms";
        assert_eq!(
            view.read_with(cx, |v, _| v.test_text()),
            Some(connected.into())
        );
        assert!(shows(cx, window, connected));
        assert!(shows(cx, window, "Continue with SSO"));
        // Not a server address: the error, in place of the facts.
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.set_self_hosted("studio", window, cx));
        })
        .unwrap();
        assert_eq!(
            view.read_with(cx, |v, _| v.test_text()),
            None,
            "a new address"
        );
        click(cx, window, "test-connection");
        let failed = view.read_with(cx, |v, _| v.test_text()).unwrap();
        assert!(failed.starts_with("Enter the server's address"), "{failed}");
        // Signing in tests first, so the bad address never becomes the server.
        click(cx, window, "sign-in-continue");
        assert!(sign_in_shown(cx, &ws));
        assert_eq!(backend.server(), crate::model::backend::Server::Cloud);
    }

    #[gpui_kit::test]
    fn sign_in_failure_toggle_shows_error(cx: &mut TestAppContext) {
        let backend = test_backend();
        backend.dev().unwrap().fail_next_sign_in(true);
        let (window, ws) = open(cx, StateStore::none(), backend.clone());
        let view = ws.read_with(cx, |ws, _| ws.sign_in.clone().unwrap());
        click(cx, window, "sign-in-github");
        assert!(sign_in_shown(cx, &ws));
        assert_eq!(
            view.read_with(cx, |v, _| v.error().map(String::from)),
            Some("Sign-in failed. Try again.".into())
        );
        assert!(shows(cx, window, "Sign-in failed. Try again."));
        assert!(backend.account().is_none());
        // The toggle clears itself: the retry works.
        click(cx, window, "sign-in-email");
        assert!(!sign_in_shown(cx, &ws));
        assert_eq!(backend.account().unwrap().name, "Maria");
    }

    #[gpui_kit::test]
    fn settings_sign_out_returns_to_signed_out_cards(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_connection(Some(Connection::Online));
        let backend = test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        let (window, ws) = open(cx, store, backend.clone());
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_app_settings(AppPage::Accounts, window, cx)
            });
        })
        .unwrap();
        assert!(shows(cx, window, "Jamie Rivera"));
        assert!(shows(cx, window, "Switch account"));
        click(cx, window, "sign-out");
        assert!(backend.account().is_none());
        assert!(shows(cx, window, "Not signed in"));
        assert!(shows(cx, window, "Sign in"));
        assert!(!shows(cx, window, "Jamie Rivera"));
        // Sign in: the screen, then back to Settings › Accounts & sync.
        click(cx, window, "sign-in");
        assert!(sign_in_shown(cx, &ws));
        click(cx, window, "sign-in-continue");
        ws.read_with(cx, |ws, cx| {
            assert!(!ws.sign_in_shown());
            let page = ws.app_settings.as_ref().map(|v| v.read(cx).page());
            assert_eq!(page, Some(AppPage::Accounts));
        });
        assert!(shows(cx, window, "Jamie Rivera"));
    }

    /// Views reach the server through `Backend` only; `app.rs` alone builds
    /// the mock.
    #[test]
    fn ui_never_imports_mock_backend() {
        let needle = concat!("Mock", "Backend");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") || path.ends_with("app.rs") {
                continue;
            }
            if std::fs::read_to_string(&path).unwrap().contains(needle) {
                offenders.push(path);
            }
        }
        assert!(offenders.is_empty(), "{offenders:?}");
        let app = std::fs::read_to_string(dir.join("app.rs")).unwrap();
        assert!(app.contains(needle), "app.rs builds it");
    }
}
