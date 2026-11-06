//! The sign-in screen (board: SignIn): where collections live (curl-runner
//! Cloud or a self-hosted server, with `Test connection`), `Continue in
//! browser` (the dashboard signs you in and sends the studio a callback)
//! and `Continue offline`. Shown on first launch, before Welcome,
//! and from Welcome's `Change` and Settings' `Sign in` / `Switch account`.

use super::app::{CloseSettings, mono_font};
use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{neutral_pill, small_button};
use super::state::Connection;
use crate::model::backend::{
    Backend, BackendError, ConnectionInfo, Server, SignInMethod, methods_text,
};
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInEvent {
    /// `Continue offline`: remember it, move on. (A sign-in finishes in
    /// the workspace, which gets the callback.)
    Done(Connection),
    /// Esc, when the screen was opened from elsewhere.
    Back,
    /// The sign-in page to open: the browser's, or a `curl-runner://`
    /// callback the studio answers itself (the mock).
    OpenUrl(String),
}

/// Where `Continue in browser` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    /// Testing the server and asking it for the sign-in page.
    Opening,
    /// The page is open: the callback finishes it.
    Waiting,
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
    phase: Phase,
    /// What Cloud reports it signs in with, once asked.
    cloud_methods: Option<Vec<SignInMethod>>,
    /// The last sign-in's error, under the primary button.
    error: Option<String>,
    /// Esc leaves (not on first launch, where there is nowhere to go).
    can_go_back: bool,
    task: Option<Task<()>>,
    test_task: Option<Task<()>>,
    methods_task: Option<Task<()>>,
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
        let mut view = SignInView {
            backend,
            pick,
            url,
            test: Test::Idle,
            phase: Phase::Idle,
            cloud_methods: None,
            error: None,
            can_go_back,
            task: None,
            test_task: None,
            methods_task: None,
            focus: cx.focus_handle(),
            _url_changes: url_changes,
        };
        view.fetch_cloud_methods(cx);
        view
    }

    /// Asks Cloud how it signs people in, for the methods line. A failure
    /// leaves the line out.
    fn fetch_cloud_methods(&mut self, cx: &mut Context<Self>) {
        if self.cloud_methods.is_some() || self.methods_task.is_some() {
            return;
        }
        let call = backend_call(cx, &self.backend, |b| b.test_connection(&Server::Cloud));
        self.methods_task = Some(cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                this.methods_task = None;
                if let Ok(info) = result {
                    this.cloud_methods = Some(info.sign_in);
                    cx.notify();
                }
            })
            .ok();
        }));
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
            if pick == Pick::Cloud {
                self.fetch_cloud_methods(cx);
            }
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

    /// `Continue in browser`: chooses the picked server and opens its
    /// sign-in page. A self-hosted address is tested first, so a typo
    /// fails here, not on first sync. The callback finishes it
    /// ([`SignInView::sign_in_failed`] when it fails).
    pub fn continue_in_browser(&mut self, cx: &mut Context<Self>) {
        if self.phase != Phase::Idle {
            return;
        }
        let server = self.server(cx);
        self.phase = Phase::Opening;
        self.error = None;
        let call = backend_call(cx, &self.backend, move |b| {
            Box::pin(async move {
                if matches!(server, Server::SelfHosted { .. }) {
                    b.test_connection(&server).await?;
                }
                b.set_server(server.clone()).await?;
                b.sign_in_url(&server).await
            })
        });
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(url) => {
                        this.phase = Phase::Waiting;
                        cx.emit(SignInEvent::OpenUrl(url));
                    }
                    Err(e) => {
                        this.phase = Phase::Idle;
                        this.error = Some(sign_in_error(&e));
                    }
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// `Cancel`: stops waiting for the browser.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.phase = Phase::Idle;
        self.task = None;
        cx.notify();
    }

    /// The callback came back and its sign-in failed: the error, under the
    /// button, and the button again.
    pub fn sign_in_failed(&mut self, e: &BackendError, cx: &mut Context<Self>) {
        self.phase = Phase::Idle;
        self.task = None;
        self.error = Some(sign_in_error(e));
        cx.notify();
    }

    /// `Continue offline`: no account either, so a signed-in studio signs
    /// out first. A failed sign-out (the mock's Offline toggle) is logged
    /// and does not keep you here.
    pub fn continue_offline(&mut self, cx: &mut Context<Self>) {
        if self.phase == Phase::Opening {
            return;
        }
        self.phase = Phase::Idle;
        if self.backend.account().is_none() {
            cx.emit(SignInEvent::Done(Connection::Offline));
            return;
        }
        let server = self.backend.server();
        let call = backend_call(cx, &self.backend, move |b| b.sign_out(server.url()));
        self.task = Some(cx.spawn(async move |this, cx| {
            if let Err(e) = call.await {
                log::warn!("sign-out before going offline failed: {e}");
            }
            this.update(cx, |_, cx| cx.emit(SignInEvent::Done(Connection::Offline)))
                .ok();
        }));
    }

    /// `CURL_RUNNER_STUDIO_SUBMIT`: presses the primary button.
    pub fn dev_submit(&mut self, cx: &mut Context<Self>) {
        self.continue_in_browser(cx);
    }

    /// What the picked server reports it signs in with, once known.
    fn methods(&self) -> Option<&[SignInMethod]> {
        match (&self.pick, &self.test) {
            (Pick::Cloud, _) => self.cloud_methods.as_deref(),
            (Pick::SelfHosted, Test::Connected(info)) => Some(&info.sign_in),
            (Pick::SelfHosted, _) => None,
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

/// `Connected · server 2.1.1 · sign-in via GitHub or email · 42 ms`.
pub fn connected_text(info: &ConnectionInfo) -> String {
    match methods_text(&info.sign_in) {
        Some(methods) => format!(
            "Connected · server {} · sign-in via {methods} · {} ms",
            info.version, info.latency_ms
        ),
        None => format!(
            "Connected · server {} · {} ms",
            info.version, info.latency_ms
        ),
    }
}

/// `Sign in with GitHub, Google or email`, from what a server reports.
pub fn methods_line(methods: &[SignInMethod]) -> Option<String> {
    methods_text(methods).map(|m| format!("Sign in with {m}"))
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
        let idle = self.phase == Phase::Idle;
        let label = if idle {
            "Continue in browser"
        } else {
            "Waiting for the browser…"
        };
        let button = div()
            .id("sign-in-continue")
            .role(Role::Button)
            .test_support()
            .aria_label(label)
            .flex()
            .flex_1()
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
            .when(!idle, |d| d.opacity(0.6))
            .when(idle, |d| {
                d.cursor_pointer()
                    .hover(|s| s.bg(color("primary-hover")))
                    .on_click(cx.listener(|this, _, _, cx| this.continue_in_browser(cx)))
            })
            .child(label);
        let primary = div().flex().gap(px(8.)).child(button).when(!idle, |d| {
            d.child(
                small_button("sign-in-cancel", "Cancel")
                    .h(px(40.))
                    .px(px(16.))
                    .text_size(px(14.))
                    .on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))
                    .child("Cancel"),
            )
        });
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
            .children(self.render_methods());
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
pub(super) fn radio(on: bool) -> Div {
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
pub(super) fn option_card(id: &'static str, label: &'static str, on: bool) -> Stateful<Div> {
    choice_card(id, label, on, !on)
}

/// [`option_card`], `live` when it answers a pointer.
pub(super) fn choice_card(
    id: &'static str,
    label: &'static str,
    on: bool,
    live: bool,
) -> Stateful<Div> {
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
            }
        })
        .when(live, |d| {
            d.cursor_pointer().hover(|s| s.bg(color("surface-100")))
        })
        .child(radio(on))
}

/// The card's name line: icon, name and the pill.
pub(super) fn option_title(icon: &'static str, name: &'static str, pill: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .child(glyph(icon).size(px(16.)).text_color(color("text-primary")))
        .child(name)
        .child(pill)
}

/// The neutral pill after a card's name.
fn tag(label: &'static str) -> Div {
    neutral_pill().text_size(px(11.)).px(px(7.)).child(label)
}

pub(super) fn option_text() -> Div {
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
                    .child(option_title(
                        icons::CLOUD,
                        "curl-runner Cloud",
                        tag("Managed"),
                    ))
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
            .child(option_title(
                icons::SERVER,
                "Self-hosted",
                tag("Open source"),
            ))
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
                    .when_some(methods_text(&info.sign_in), |d, methods| {
                        d.child(format!("sign-in via {methods}")).child(sep())
                    })
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
                    .on_click(cx.listener(|this, _, _, cx| this.continue_offline(cx)))
                    .child("Continue offline"),
            )
    }

    /// `Sign in with GitHub, Google or email`: what the picked server
    /// reports. The dashboard asks which one.
    fn render_methods(&self) -> Option<impl IntoElement + use<>> {
        let line = methods_line(self.methods()?)?;
        Some(
            div()
                .id("sign-in-methods")
                .role(Role::Status)
                .test_support()
                .aria_label(line.clone())
                .text_center()
                .text_size(px(12.))
                .text_color(color("text-tertiary"))
                .child(line),
        )
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "server")]
    use super::super::settings_view::AppPage;
    use super::super::state::{Connection, StateStore};
    use super::super::workspace::{Workspace, test_secrets};
    use crate::model::backend::{Backend, test_backend};
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

    /// The sign-in screen itself: first launch shows it with the server,
    /// and only then; without it the tests ask for it.
    fn open_sign_in(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
    ) -> Entity<super::SignInView> {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_sign_in(None, window, cx))
        })
        .unwrap()
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

    #[cfg(feature = "server")]
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

    #[cfg(feature = "server")]
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
        let view = open_sign_in(cx, window, &ws);
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_self_hosted("https://studio.api.example.com", window, cx)
            });
        })
        .unwrap();
        click(cx, window, "test-connection");
        let connected = "Connected · server 2.1.1 · sign-in via GitHub or email · 42 ms";
        assert_eq!(
            view.read_with(cx, |v, _| v.test_text()),
            Some(connected.into())
        );
        assert!(shows(cx, window, connected));
        assert!(shows(cx, window, "Sign in with GitHub or email"));
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
        let view = open_sign_in(cx, window, &ws);
        click(cx, window, "sign-in-continue");
        assert!(sign_in_shown(cx, &ws));
        assert_eq!(
            view.read_with(cx, |v, _| v.error().map(String::from)),
            Some("Sign-in failed. Try again.".into())
        );
        assert!(shows(cx, window, "Sign-in failed. Try again."));
        assert!(backend.account().is_none());
        assert!(shows(cx, window, "Continue in browser"), "not waiting");
        // The toggle clears itself: the retry works.
        click(cx, window, "sign-in-continue");
        assert!(!sign_in_shown(cx, &ws));
        assert_eq!(backend.account().unwrap().name, "Jamie Rivera");
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn settings_sign_out_returns_to_signed_out_cards(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_connection(Some(Connection::Online));
        let backend = test_backend();
        crate::model::backend::testing::sign_in(&*backend);
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

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn shortcuts_do_nothing_behind_the_sign_in_screen(cx: &mut TestAppContext) {
        use gpui_kit::component::WindowExt as _;
        let state = tempfile::tempdir().unwrap();
        let (window, ws) = open(cx, StateStore::at(state.path()), test_backend());
        assert!(sign_in_shown(cx, &ws));
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.press("cmd-,", cx);
            window.press("cmd-n", cx);
            window.press("cmd-k", cx);
            window.render_frame(cx);
            assert!(!window.has_active_dialog(cx), "no New dialog");
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            assert!(ws.sign_in_shown());
            assert!(!ws.app_settings_shown());
        });
        click(cx, window, "continue-offline");
        ws.read_with(cx, |ws, _| {
            assert!(!ws.sign_in_shown());
            assert!(!ws.app_settings_shown(), "no Settings resurfaces");
            assert!(ws.state.collection.is_none());
        });
        assert!(shows(cx, window, "Offline"), "Welcome");
        // Now the shortcuts work again.
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.press("cmd-,", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(ws.app_settings_shown()));
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn continue_offline_signs_out(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_connection(Some(Connection::Online));
        let backend = test_backend();
        crate::model::backend::testing::sign_in(&*backend);
        let (window, ws) = open(cx, store.clone(), backend.clone());
        assert!(shows(cx, window, "Connected to curl-runner Cloud"));
        click(cx, window, "sidebar-cloud-change");
        assert!(
            shows(cx, window, "Signed in as jamie@acme.dev"),
            "the status bar"
        );
        click(cx, window, "continue-offline");
        assert!(!sign_in_shown(cx, &ws));
        assert!(backend.account().is_none(), "offline is no account");
        assert_eq!(store.connection(), Some(Connection::Offline));
        assert!(shows(cx, window, "Offline"));
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn without_a_store_the_answer_holds_for_the_session(cx: &mut TestAppContext) {
        let (window, ws) = open(cx, StateStore::none(), test_backend());
        assert!(sign_in_shown(cx, &ws));
        click(cx, window, "continue-offline");
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.first_launch(window, cx);
                assert!(!ws.sign_in_shown(), "not asked again this session");
            });
        })
        .unwrap();
        assert!(shows(cx, window, "Offline"));
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn settings_server_save_tests_first_and_shows_the_error(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_connection(Some(Connection::Online));
        let backend = test_backend();
        let (window, ws) = open(cx, store, backend.clone());
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_app_settings(AppPage::Accounts, window, cx)
            });
        })
        .unwrap();
        let view = ws.read_with(cx, |ws, _| ws.app_settings.clone().unwrap());
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| v.set_self_hosted("studio", window, cx));
        })
        .unwrap();
        click(cx, window, "server-save");
        assert_eq!(backend.server(), crate::model::backend::Server::Cloud);
        assert!(shows(
            cx,
            window,
            "Enter the server's address, like https://curl-runner.example.com."
        ));
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_self_hosted("https://studio.api.example.com", window, cx)
            });
        })
        .unwrap();
        click(cx, window, "server-save");
        assert_eq!(
            backend.server(),
            crate::model::backend::Server::SelfHosted {
                url: "https://studio.api.example.com".into()
            }
        );
        assert!(!shows(
            cx,
            window,
            "Enter the server's address, like https://curl-runner.example.com."
        ));
    }

    /// Without the server, a stored `Online` answer from an earlier build
    /// opens Welcome working locally: no first-launch screen, no received
    /// copies, and nothing it left behind is rewritten or removed.
    #[cfg(not(feature = "server"))]
    #[gpui_kit::test]
    fn stored_online_connection_opens_welcome_locally(cx: &mut TestAppContext) {
        use crate::model::backend::ReceivedShare;
        use crate::model::received::{self, LocalCopy};
        let state = tempfile::tempdir().unwrap();
        let base = state.path();
        let store = StateStore::at(base);
        store.set_connection(Some(Connection::Online));
        let backend = test_backend();
        crate::model::backend::testing::sign_in_by_email(&*backend);
        let share = ReceivedShare {
            id: "acme/shipping-api".into(),
            owner_org: "Acme".into(),
            name: "Shipping API".into(),
            added: chrono::Utc::now(),
        };
        futures::executor::block_on(backend.add_received(share.clone())).unwrap();
        let copy = tempfile::tempdir().unwrap();
        let local = LocalCopy {
            share_id: share.id.clone(),
            root: copy.path().to_path_buf(),
            collection_id: "c".into(),
        };
        received::record(base, local).unwrap();
        let mock_state = base.join("mock-backend.json");
        std::fs::write(&mock_state, "{}").unwrap();
        let before = |name: &str| std::fs::read(base.join(name)).unwrap();
        let (state_json, received_json) = (before("state.json"), before("received.json"));

        let (window, ws) = open(cx, store.clone(), backend);
        assert!(!sign_in_shown(cx, &ws));
        assert!(shows(cx, window, "Working locally"));
        ws.read_with(cx, |ws, _| assert!(ws.received_all.is_empty()));

        assert_eq!(before("state.json"), state_json);
        assert_eq!(before("received.json"), received_json);
        assert_eq!(std::fs::read_to_string(&mock_state).unwrap(), "{}");
        assert_eq!(store.connection(), Some(Connection::Online));
    }

    /// What the dashboard opens in the browser, in these tests.
    #[cfg(feature = "server")]
    const BROWSER: &str = "https://app.curl-runner.test/sign-in?client=studio";

    /// The mock behind a spy whose `sign_in_url` answers with a browser
    /// page, as a real server does.
    #[cfg(feature = "server")]
    fn browser_backend() -> Arc<dyn Backend> {
        let spy = crate::model::backend::testing::Spy::new(test_backend());
        *spy.browser_url.lock().unwrap() = Some(BROWSER.into());
        Arc::new(spy)
    }

    #[cfg(feature = "server")]
    fn open_url(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
        url: &str,
    ) {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_urls(vec![url.into()], window, cx))
        })
        .unwrap();
        cx.run_until_parked();
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn continue_in_browser_then_callback_signs_in_and_resumes_link(cx: &mut TestAppContext) {
        use crate::model::backend::Server;
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let backend = browser_backend();
        let (window, ws) = open(cx, store.clone(), backend.clone());
        assert!(sign_in_shown(cx, &ws));
        // A shared link that comes in now waits for the sign-in.
        let link =
            "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm";
        open_url(cx, window, &ws, link);
        assert!(ws.read_with(cx, |ws, _| ws.pending_link.is_some()));
        assert!(shows(cx, window, "Continue in browser"));
        click(cx, window, "sign-in-continue");
        assert_eq!(cx.opened_url().as_deref(), Some(BROWSER));
        assert!(shows(cx, window, "Waiting for the browser…"));
        assert!(shows(cx, window, "Cancel"));
        assert!(sign_in_shown(cx, &ws));
        assert!(backend.account().is_none());
        // The dashboard's answer.
        let callback = format!(
            "curl-runner://auth/callback?code=crh_mock&server={}",
            Server::Cloud.url()
        );
        open_url(cx, window, &ws, &callback);
        assert!(!sign_in_shown(cx, &ws));
        assert_eq!(store.connection(), Some(Connection::Online));
        assert_eq!(backend.account().unwrap().email, "jamie@acme.dev");
        ws.read_with(cx, |ws, _| {
            assert!(ws.pending_link.is_none());
            assert!(ws.open_shared_view().is_some(), "the link opens");
        });
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn cancel_stops_waiting_for_the_browser(cx: &mut TestAppContext) {
        let (window, ws) = open(cx, StateStore::none(), browser_backend());
        click(cx, window, "sign-in-continue");
        assert!(shows(cx, window, "Waiting for the browser…"));
        click(cx, window, "sign-in-cancel");
        assert!(sign_in_shown(cx, &ws));
        assert!(!shows(cx, window, "Waiting for the browser…"));
        assert!(shows(cx, window, "Continue in browser"));
    }

    #[test]
    fn methods_read_github_google_email() {
        use super::methods_line;
        use crate::model::backend::SignInMethod::{Email, Github, Google};
        assert_eq!(
            methods_line(&[Email, Github]).as_deref(),
            Some("Sign in with GitHub or email")
        );
        assert_eq!(
            methods_line(&[Email, Google, Github]).as_deref(),
            Some("Sign in with GitHub, Google or email")
        );
        assert_eq!(
            methods_line(&[Google]).as_deref(),
            Some("Sign in with Google")
        );
        assert_eq!(methods_line(&[]), None);
    }

    #[gpui_kit::test]
    fn sign_in_lists_reported_methods(cx: &mut TestAppContext) {
        let (window, ws) = open(cx, StateStore::none(), test_backend());
        let view = open_sign_in(cx, window, &ws);
        cx.run_until_parked();
        // Cloud reports all three.
        assert!(shows(cx, window, "Sign in with GitHub, Google or email"));
        // The mock's self-hosted server reports `[email, github]`.
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.set_self_hosted("https://studio.api.example.com", window, cx)
            });
        })
        .unwrap();
        click(cx, window, "test-connection");
        assert!(shows(cx, window, "Sign in with GitHub or email"));
        assert!(!shows(cx, window, "Sign in with GitHub, Google or email"));
    }

    /// Views reach the server through `Backend` only; in `app.rs` only
    /// `app_backend` builds the mock. No other ui file names the mock or
    /// its fixtures.
    #[test]
    fn mock_is_only_named_in_app_backend() {
        fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    rust_files(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let needles = [
            concat!("Mock", "Backend"),
            concat!("backend::", "fixtures"),
            concat!("backend::", "mock"),
        ];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut files = Vec::new();
        rust_files(&dir, &mut files);
        let app = dir.join("app.rs");
        let mut offenders = Vec::new();
        for path in files.iter().filter(|p| **p != app) {
            let text = std::fs::read_to_string(path).unwrap();
            for needle in needles {
                if text.contains(needle) {
                    offenders.push(format!("{}: {needle}", path.display()));
                }
            }
        }
        assert!(offenders.is_empty(), "{offenders:?}");
        // In app.rs, the mock is named inside `app_backend` and nowhere else.
        let app = std::fs::read_to_string(app).unwrap();
        let start = app
            .find("pub fn app_backend(")
            .expect("app.rs has app_backend");
        let end = start + app[start..].find("\n}\n").expect("app_backend ends");
        let (inside, outside) = (
            &app[start..end],
            format!("{}{}", &app[..start], &app[end..]),
        );
        assert!(inside.contains(needles[0]), "app_backend builds it");
        for needle in needles {
            assert!(
                !outside.contains(needle),
                "app.rs names {needle} outside app_backend"
            );
        }
    }
}
