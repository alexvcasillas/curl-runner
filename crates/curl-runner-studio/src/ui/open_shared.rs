//! Opening a shared link (board: OpenShared): paste one (Welcome's card,
//! ⌘L, the palette) or receive one (`curl-runner://`, see [`link_handler`]),
//! review what it opens, fill in your own credentials and add your own
//! copy to this studio.
//!
//! Add writes nothing until the snapshot is in hand and every path in it
//! checks out. The files land in a hidden sibling folder first and move into
//! place in one rename; a later step that fails takes the folder, its notes
//! and its Keychain items back out. Secret values go to the Keychain only:
//! never to disk, a log, a toast or the backend.

use super::app::mono_font;
use super::icons::{self, glyph};
use super::share_dialog::initials;
use super::state::Connection;
use super::workspace::{Workspace, notify_error};
use super::{backend_call, color};
use crate::model::backend::{
    AuthCallback, Backend, BackendError, FOREIGN_CALLBACK, Server, ShareLink, SharedCollection,
    SharedVar,
};
use crate::model::environments::is_secret;
use crate::model::history::relative_time;
use crate::model::secrets::SecretStore;
use crate::model::shared_copy::{add_shared, already_added, save_inputs};
use chrono::Local;
use futures::StreamExt as _;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

/// The board's dialog.
const DIALOG_W: f32 = 600.;
/// About the credentials step's height: the dialog sits centred on it.
const DIALOG_H: f32 = 600.;
const FILL_LABEL: &str = "Fill in the variables these requests use";
const PASTE_TITLE: &str = "Open a shared link";
const PASTE_SUBTITLE: &str = "Paste a curl-runner link someone sent you. Adds your own copy.";
const PASTE_HINT: &str = "curl-runner://open?… or https://cloud.curl-runner.com/s/…";
const ALREADY_ADDED: &str = "Already in your studio — open it?";

/// `Acme shared Shipping API with you`.
pub fn title_text(shared: &SharedCollection) -> String {
    format!("{} shared {} with you", shared.owner_org, shared.name)
}

/// What a restricted link asks first: `Sign in to api.curl-runner.com to
/// open this collection.`
pub fn sign_in_text(link: &ShareLink) -> String {
    format!("Sign in to {} to open this collection.", link.host())
}

/// The header's subtitle.
pub fn subtitle_text(org: &str) -> String {
    format!("You can run every request with your own credentials. Only {org} can edit it.")
}

/// The note under the variables.
pub fn keychain_note(org: &str) -> String {
    format!("Saved to your keychain on this computer. Never sent to {org} or synced.")
}

/// `12 requests · rates, labels, tracking · updated 3 days ago`.
pub fn summary_text(shared: &SharedCollection, now: chrono::DateTime<Local>) -> String {
    let n = shared.request_count;
    let requests = if n == 1 {
        "1 request".to_string()
    } else {
        format!("{n} requests")
    };
    let updated = format!(
        "updated {}",
        relative_time(shared.updated.with_timezone(&Local), now)
    );
    [requests, shared.summary.clone(), updated]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Who a link came from, for an error before it resolves: the link names
/// no one.
const LINK_OWNER: &str = "whoever shared it";

/// The flow's steps; `Paste` comes before the board's two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Paste,
    Review,
    Credentials,
}

/// What the view asks of whoever shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenSharedEvent {
    Close,
    /// Added (or already there): open the collection at this root.
    Open(PathBuf),
    /// A restricted link: sign in to its server, then open it again.
    SignIn(ShareLink),
    /// The inputs an update asked for are saved.
    Saved,
}

/// One variable's input.
struct Field {
    var: SharedVar,
    input: Entity<InputState>,
    revealed: bool,
}

impl Field {
    fn secret(&self) -> bool {
        self.var.secret || is_secret(&self.var.name, &[])
    }

    /// The owner locked its default: shown, not asked for.
    fn locked(&self) -> bool {
        self.var.locked && !self.secret() && self.var.default.is_some()
    }
}

pub struct OpenSharedView {
    /// The dialog's own focus: an Add that finishes closes the dialog only
    /// while focus is still in it (no dialog opened over it since).
    pub(super) focus: FocusHandle,
    /// Added already (see [`OpenSharedView::added`]).
    done: bool,
    backend: Arc<dyn Backend>,
    secrets: Arc<dyn SecretStore>,
    /// The studio's state dir: shared copies go under `{base}/shared/`.
    base: Option<PathBuf>,
    step: Step,
    link: Entity<InputState>,
    /// The link as opened, for the `Opened from a link` strip.
    opened: Option<String>,
    /// It came in as a deep link: the strip shows before it resolves.
    deep: bool,
    shared: Option<SharedCollection>,
    /// This studio's copy of the share, when it has one.
    existing: Option<PathBuf>,
    fields: Vec<Field>,
    error: Option<String>,
    /// Resolving or adding. While adding, nothing closes the dialog.
    busy: bool,
    /// Resolving the link (an Add runs detached: it always finishes).
    task: Option<Task<()>>,
    /// A link that came in while busy: opened once it is done.
    queued: Option<String>,
    /// Where an Add that finished goes, even with this view gone.
    workspace: WeakEntity<Workspace>,
    /// A restricted link and no session on its server: `Sign in to {host}
    /// to open this collection.`
    sign_in_for: Option<ShareLink>,
    /// Dev screenshots: once resolved, go to the credentials step with
    /// these values.
    dev_fill: Option<Vec<(String, String)>>,
    /// Asking for an updated copy's new inputs (no link, no Add): its
    /// folder and Keychain service, where **Save** writes them.
    inputs_for: Option<(PathBuf, String)>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<OpenSharedEvent> for OpenSharedView {}

impl OpenSharedView {
    pub fn new(
        backend: Arc<dyn Backend>,
        secrets: Arc<dyn SecretStore>,
        base: Option<PathBuf>,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let link = cx
            .new(|cx| InputState::new(window, cx).placeholder("https://cloud.curl-runner.com/s/…"));
        let events =
            cx.subscribe_in(
                &link,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        let had = this.error.take().is_some() | this.sign_in_for.take().is_some();
                        if this.step == Step::Paste && had {
                            cx.notify();
                        }
                    }
                    InputEvent::PressEnter { .. } => this.open_link(window, cx),
                    _ => {}
                },
            );
        OpenSharedView {
            focus: cx.focus_handle(),
            done: false,
            backend,
            secrets,
            base,
            step: Step::Paste,
            link,
            opened: None,
            deep: false,
            shared: None,
            existing: None,
            fields: Vec::new(),
            error: None,
            busy: false,
            task: None,
            queued: None,
            workspace,
            sign_in_for: None,
            dev_fill: None,
            inputs_for: None,
            _subscriptions: vec![events],
        }
    }

    #[cfg(test)]
    pub fn step(&self) -> Step {
        self.step
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Each field's variable name and whether it is masked.
    #[cfg(all(test, feature = "server"))]
    pub fn fields(&self) -> Vec<(String, bool)> {
        self.fields
            .iter()
            .map(|f| (f.var.name.clone(), f.secret()))
            .collect()
    }

    /// A link from outside (a `curl-runner://` event): shown and opened.
    /// While busy it waits its turn, so an Add in flight keeps its step.
    pub fn receive(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            self.queued = Some(url.to_string());
            return;
        }
        self.deep = true;
        self.paste(url, window, cx);
        self.open_link(window, cx);
    }

    /// Puts `text` in the link field (the Paste step).
    pub fn paste(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.step = Step::Paste;
        self.link
            .update(cx, |s, cx| s.set_value(text.to_string(), window, cx));
    }

    /// `Open link`: parse, then ask the server what it opens.
    pub fn open_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let text = self.link.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.opened = Some(text.clone());
        self.shared = None;
        self.existing = None;
        self.error = None;
        self.sign_in_for = None;
        self.busy = true;
        let org = LINK_OWNER;
        // A landing URL asks its host where the API is: off the UI thread.
        let backend = self.backend.clone();
        let call = cx.background_executor().spawn(async move {
            let link = ShareLink::parse(&text)
                .map_err(|e| (BackendError::Invalid(e.to_string()), None))?;
            backend
                .resolve_link(&link)
                .await
                .map_err(|e| (e, Some(link)))
        });
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(shared) => this.resolved(shared, window, cx),
                    // Restricted: sign in to its server first, once.
                    Err((BackendError::SignedOut, Some(link))) => {
                        this.step = Step::Paste;
                        this.sign_in_for = Some(link);
                    }
                    Err((e, _)) => {
                        this.step = Step::Paste;
                        this.error = Some(e.link_text(org));
                    }
                }
                this.take_queued(window, cx);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn resolved(&mut self, shared: SharedCollection, window: &mut Window, cx: &mut Context<Self>) {
        self.existing = already_added(&*self.backend, self.base.as_deref(), &shared.link.token);
        self.inputs_for = None;
        self.set_fields(shared.variables.clone(), window, cx);
        self.shared = Some(shared);
        self.step = Step::Review;
        if let Some(fill) = self.dev_fill.take() {
            self.continue_to_credentials(window, cx);
            for (name, value) in fill {
                self.fill(&name, &value, window, cx);
            }
            if let Some(f) = self.fields.iter().find(|f| f.secret()) {
                f.input.read(cx).focus_handle(cx).focus(window, cx);
            }
        }
    }

    /// After an update, the inputs its new version requires, in the
    /// Credentials step: **Save** writes them into the copy at `root`
    /// (secrets to `service` in the Keychain, the rest to its `.env`), as
    /// an Add does.
    pub fn ask_inputs(
        &mut self,
        shared: SharedCollection,
        root: PathBuf,
        service: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        self.set_fields(shared.variables.clone(), window, cx);
        self.shared = Some(shared);
        self.existing = None;
        self.error = None;
        self.inputs_for = Some((root, service));
        self.continue_to_credentials(window, cx);
    }

    /// One field per variable, the required ones first.
    fn set_fields(&mut self, vars: Vec<SharedVar>, window: &mut Window, cx: &mut Context<Self>) {
        let mut vars = vars;
        vars.sort_by_key(|v| !v.required);
        self._subscriptions.truncate(1);
        self.fields = vars
            .into_iter()
            .map(|var| {
                let secret = var.secret || is_secret(&var.name, &[]);
                let placeholder = match (secret, var.required) {
                    (true, _) => "Paste your own value",
                    (false, true) => "Required",
                    (false, false) => "Leave empty to skip",
                };
                let default = var.default.clone().filter(|_| !secret).unwrap_or_default();
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .masked(secret)
                        .placeholder(placeholder)
                        .default_value(default)
                });
                let events =
                    cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
                        match event {
                            InputEvent::Change => {
                                this.error = None;
                                cx.notify();
                            }
                            InputEvent::PressEnter { .. } => this.add(window, cx),
                            _ => {}
                        }
                    });
                self._subscriptions.push(events);
                Field {
                    var,
                    input,
                    revealed: false,
                }
            })
            .collect();
    }

    /// `Continue`: to the credentials, the first empty required field
    /// focused.
    pub fn continue_to_credentials(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shared.is_none() {
            return;
        }
        self.step = Step::Credentials;
        let first = self
            .fields
            .iter()
            .filter(|f| !f.locked())
            .find(|f| f.var.required && f.input.read(cx).value().trim().is_empty())
            .or(self.fields.iter().find(|f| !f.locked()));
        if let Some(f) = first {
            f.input.read(cx).focus_handle(cx).focus(window, cx);
        }
        cx.notify();
    }

    pub fn back(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.inputs_for.is_some() {
            return;
        }
        self.error = None;
        self.step = match self.step {
            Step::Credentials => Step::Review,
            _ => Step::Paste,
        };
        cx.notify();
    }

    /// Types `value` into `name`'s field.
    pub fn fill(&mut self, name: &str, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.fields.iter().find(|f| f.var.name == name) {
            f.input
                .update(cx, |s, cx| s.set_value(value.to_string(), window, cx));
        }
        cx.notify();
    }

    fn toggle_reveal(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.fields.get_mut(ix) {
            f.revealed = !f.revealed;
            let masked = !f.revealed;
            f.input.update(cx, |s, cx| s.set_masked(masked, window, cx));
        }
        cx.notify();
    }

    /// Every required variable has a value.
    pub fn ready(&self, cx: &App) -> bool {
        self.fields
            .iter()
            .all(|f| !f.var.required || f.locked() || !f.input.read(cx).value().trim().is_empty())
    }

    /// An Add is in flight: the dialog stays until it is done.
    /// The copy was added while this dialog sat under another one: Add
    /// now only closes it.
    pub fn added(&mut self, cx: &mut Context<Self>) {
        self.done = true;
        self.busy = false;
        cx.notify();
    }

    pub fn adding(&self) -> bool {
        self.busy && self.step == Step::Credentials
    }

    /// Opens the link that came in while busy, if one did.
    fn take_queued(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = self.queued.take() {
            self.receive(&url, window, cx);
        }
    }

    /// `Add to my studio`: fetch, write, record, then open. It runs
    /// detached, holding everything it needs, so closing the dialog (or the
    /// view going) never leaves a half-added copy: it finishes and opens,
    /// or rolls back.
    pub fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.done {
            cx.emit(OpenSharedEvent::Close);
            return;
        }
        if self.busy || self.step != Step::Credentials || !self.ready(cx) {
            return;
        }
        if self.inputs_for.is_some() {
            return self.save_inputs(cx);
        }
        let Some(shared) = self.shared.clone() else {
            return;
        };
        let Some(base) = self.base.clone() else {
            self.error =
                Some("There's no place to keep shared collections on this computer.".into());
            cx.notify();
            return;
        };
        // A locked default is the owner's: nothing goes to `.env` for it.
        let values = self.values(cx);
        self.busy = true;
        self.error = None;
        let (backend, secrets) = (self.backend.clone(), self.secrets.clone());
        let job = cx.background_spawn(async move {
            add_shared(&*backend, &*secrets, &base, &shared, &values).await
        });
        let workspace = self.workspace.clone();
        cx.spawn_in(window, async move |this, cx| {
            let outcome = job.await;
            let shown = this
                .update(cx, |this, cx| {
                    this.busy = false;
                    if let Err(e) = &outcome {
                        this.error = Some(e.clone());
                    }
                    cx.notify();
                })
                .is_ok();
            workspace
                .update_in(cx, |ws, window, cx| {
                    ws.add_finished(outcome, shown, window, cx)
                })
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The values typed, but a locked default's (the owner's).
    fn values(&self, cx: &App) -> Vec<(String, String)> {
        self.fields
            .iter()
            .filter(|f| !f.locked())
            .map(|f| {
                let v = f.input.read(cx).value();
                (f.var.name.clone(), v.trim().to_string())
            })
            .collect()
    }

    /// **Save**, for an updated copy's new inputs: written as the form
    /// masked them (a masked field's value only ever to the Keychain).
    fn save_inputs(&mut self, cx: &mut Context<Self>) {
        let Some((root, service)) = self.inputs_for.clone() else {
            return;
        };
        let vars: Vec<SharedVar> = self
            .fields
            .iter()
            .map(|f| SharedVar {
                secret: f.secret(),
                ..f.var.clone()
            })
            .collect();
        let values = self.values(cx);
        self.busy = true;
        self.error = None;
        let secrets = self.secrets.clone();
        let job =
            cx.background_spawn(
                async move { save_inputs(&root, &service, &vars, &values, &*secrets) },
            );
        cx.spawn(async move |this, cx| {
            let saved = job.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match saved {
                    Ok(()) => cx.emit(OpenSharedEvent::Saved),
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// `Sign in`, for a restricted link: the workspace signs in to the
    /// link's server in the browser, then opens the link again.
    pub fn sign_in(&mut self, cx: &mut Context<Self>) {
        if let Some(link) = self.sign_in_for.clone().filter(|_| !self.busy) {
            self.error = None;
            cx.emit(OpenSharedEvent::SignIn(link));
        }
    }

    /// The sign-in for a restricted link couldn't start.
    pub fn sign_in_failed(&mut self, e: &BackendError, cx: &mut Context<Self>) {
        self.error = Some(format!("Couldn't sign in: {e}"));
        cx.notify();
    }

    /// `Open it`, for a share already in this studio.
    fn open_existing(&mut self, cx: &mut Context<Self>) {
        if let Some(root) = self.existing.clone() {
            cx.emit(OpenSharedEvent::Open(root));
        }
    }

    /// Dev screenshots (`CURL_RUNNER_STUDIO_FILL` with a link): the
    /// credentials step, filled with `NAME=value` pairs (`,`-separated).
    pub fn dev_credentials(&mut self, fill: &str) {
        self.dev_fill = Some(
            fill.split(',')
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (k.trim().to_string(), v.to_string()))
                .collect(),
        );
    }
}

/// The board's `.btn`: 32px, 13px text.
fn dialog_button(id: &'static str, label: impl Into<SharedString>) -> Button {
    Button::new(id)
        .h(px(32.))
        .px(px(12.))
        .text_size(px(13.))
        .label(label.into())
}

/// 16px stepper disc.
fn disc(n: &'static str, current: bool) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(16.))
        .rounded_full()
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .map(|d| {
            if current {
                d.bg(color("primary")).text_color(color("on-primary"))
            } else {
                d.border_1()
                    .border_color(color("border-200"))
                    .text_color(color("text-tertiary"))
            }
        })
        .child(n)
}

/// The small outlined chip: `Your own copy`, `from Acme`.
fn chip(text: impl Into<SharedString>, fg: &str) -> Div {
    div()
        .flex_none()
        .px(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(color("border-200"))
        .text_size(px(11.))
        .line_height(px(16.))
        .text_color(color(fg))
        .child(text.into())
}

impl OpenSharedView {
    fn render_strip(&self) -> Option<impl IntoElement + use<>> {
        let shown = self.step != Step::Paste || self.deep;
        let url = self.opened.clone().filter(|_| shown)?;
        Some(
            div()
                .id("open-shared-from")
                .test_support()
                .aria_label(url.clone())
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.))
                .px(px(24.))
                .py(px(10.))
                .border_b_1()
                .border_color(color("border-100"))
                .bg(color("surface-raised"))
                .text_size(px(12.))
                .text_color(color("text-tertiary"))
                .child(glyph(icons::LINK).size(px(13.)).flex_none())
                .child(div().flex_none().child("Opened from a link"))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(mono_font())
                        .child(url),
                ),
        )
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (title, subtitle, mark) = match &self.shared {
            Some(s) if self.step != Step::Paste => (
                title_text(s),
                subtitle_text(&s.owner_org),
                div()
                    .bg(linear_gradient(
                        135.,
                        linear_color_stop(color("text-primary"), 0.),
                        linear_color_stop(color("text-secondary").opacity(0.8), 1.),
                    ))
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color("on-primary"))
                    .child(initials(&s.owner_org)),
            ),
            _ => (
                PASTE_TITLE.to_string(),
                PASTE_SUBTITLE.to_string(),
                div()
                    .border_1()
                    .border_color(color("border-200"))
                    .text_color(color("text-secondary"))
                    .child(glyph(icons::LINK).size(px(16.))),
            ),
        };
        div()
            .flex()
            .items_start()
            .gap(px(12.))
            .px(px(24.))
            .pt(px(20.))
            .pb(px(16.))
            .child(
                mark.flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(36.))
                    .rounded_full(),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(
                        div()
                            .id("open-shared-title")
                            .test_support()
                            .aria_label(title.clone())
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(relative(1.5))
                            .text_color(color("text-secondary"))
                            .child(subtitle),
                    ),
            )
            .child(
                div()
                    .id("open-shared-close")
                    .role(Role::Button)
                    .test_support()
                    .aria_label("Close")
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(28.))
                    .rounded(px(6.))
                    .text_color(color("text-secondary"))
                    .child(glyph(icons::CROSS).size(px(14.)))
                    .map(|d| {
                        if self.adding() {
                            d.opacity(0.5).cursor_not_allowed()
                        } else {
                            d.cursor_pointer()
                                .hover(|s| {
                                    s.bg(color("surface-200")).text_color(color("text-primary"))
                                })
                                .on_click(
                                    cx.listener(|_, _, _, cx| cx.emit(OpenSharedEvent::Close)),
                                )
                        }
                    }),
            )
    }

    fn render_error(&self) -> Option<impl IntoElement + use<>> {
        let e = self.error.clone()?;
        Some(
            div()
                .id("open-shared-error")
                .role(Role::Alert)
                .test_support()
                .aria_label(e.clone())
                .flex()
                .items_start()
                .gap(px(10.))
                .px(px(14.))
                .py(px(10.))
                .border_1()
                .border_color(color("danger-border"))
                .bg(color("danger-subtle"))
                .rounded(px(8.))
                .text_size(px(12.))
                .line_height(relative(1.5))
                .child(
                    glyph(icons::ERROR)
                        .size(px(14.))
                        .flex_none()
                        .mt(px(1.))
                        .text_color(color("danger")),
                )
                .child(div().flex_1().min_w_0().child(e)),
        )
    }

    fn render_stepper(&self) -> impl IntoElement + use<> {
        let review_done = self.step == Step::Credentials;
        let item = |id: &'static str, on: bool| {
            div()
                .id(id)
                .test_support()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_color(color(if on { "text-primary" } else { "text-secondary" }))
        };
        div()
            .flex()
            .gap(px(16.))
            .text_size(px(12.))
            .child(
                item("open-shared-step-review", !review_done)
                    .when(!review_done, |d| d.aria_label("Review, current step"))
                    .child(if review_done {
                        glyph(icons::CHECK)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("success"))
                            .into_any_element()
                    } else {
                        disc("1", true).into_any_element()
                    })
                    .child("Review"),
            )
            .child(
                item("open-shared-step-credentials", review_done)
                    .when(review_done, |d| {
                        d.aria_label("Your credentials, current step")
                    })
                    .when(!review_done, |d| d.text_color(color("text-tertiary")))
                    .child(disc("2", review_done))
                    .child("Your credentials"),
            )
    }

    fn render_summary(&self, shared: &SharedCollection) -> impl IntoElement + use<> {
        let summary = summary_text(shared, Local::now());
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(14.))
            .py(px(12.))
            .rounded(px(8.))
            .border_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
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
                            .size(px(14.))
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
                            .font_weight(FontWeight::MEDIUM)
                            .child(shared.name.clone()),
                    )
                    .child(
                        div()
                            .id("open-shared-summary")
                            .test_support()
                            .aria_label(summary.clone())
                            .truncate()
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(summary),
                    ),
            )
            .child(chip("Your own copy", "text-secondary"))
    }

    fn render_paste(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let focused = self.link.read(cx).focus_handle(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                field_box(focused, window)
                    .child(
                        glyph(icons::LINK)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("text-secondary")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(mono_font())
                            .text_size(px(12.))
                            .child(Input::new(&self.link).appearance(false).h(px(34.))),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child(PASTE_HINT),
            )
            .children(self.sign_in_for.as_ref().map(|link| {
                let text = sign_in_text(link);
                div()
                    .id("open-shared-sign-in-note")
                    .role(Role::Alert)
                    .test_support()
                    .aria_label(text.clone())
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .mt(px(10.))
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(color("info-border"))
                    .bg(color("info-subtle"))
                    .text_size(px(12.))
                    .child(
                        glyph(icons::LOCK)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("accent-blue")),
                    )
                    .child(text)
            }))
    }

    fn render_review(&self, shared: &SharedCollection) -> Div {
        let rows = self.fields.iter().map(|f| {
            let secret = f.secret();
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .min_h(px(28.))
                .text_size(px(12.))
                .child(
                    div()
                        .w(px(120.))
                        .flex_none()
                        .font_family(mono_font())
                        .text_color(color("warning"))
                        .child(f.var.name.clone()),
                )
                .child(
                    div()
                        .w(px(64.))
                        .flex_none()
                        .text_size(px(11.))
                        .text_color(color(if f.var.required {
                            "danger"
                        } else {
                            "text-tertiary"
                        }))
                        .child(if f.var.required {
                            "Required"
                        } else {
                            "Optional"
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(color("text-tertiary"))
                        .child(f.var.description.clone()),
                )
                .when(secret, |d| d.child(chip("secret", "text-secondary")))
        });
        let body = div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(self.render_stepper())
            .child(self.render_summary(shared));
        match &self.existing {
            Some(_) => body.child(
                div()
                    .id("open-shared-existing")
                    .test_support()
                    .aria_label(ALREADY_ADDED)
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(color("info-border"))
                    .bg(color("info-subtle"))
                    .text_size(px(12.))
                    .child(
                        glyph(icons::FOLDER)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("accent-blue")),
                    )
                    .child(ALREADY_ADDED),
            ),
            None => body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("You'll fill in these variables next"),
                    )
                    .child(div().flex().flex_col().children(rows)),
            ),
        }
    }

    fn render_credentials(
        &self,
        shared: &SharedCollection,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let org = shared.owner_org.clone();
        let rows = self
            .fields
            .iter()
            .enumerate()
            .map(|(ix, f)| self.render_field(ix, f, &org, window, cx))
            .collect::<Vec<_>>();
        div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(self.render_stepper())
            .child(self.render_summary(shared))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(FILL_LABEL),
                    )
                    .children(rows),
            )
            .child(
                div()
                    .id("open-shared-note")
                    .test_support()
                    .aria_label(keychain_note(&org))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(color("border-100"))
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(
                        glyph(icons::LOCK)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("success")),
                    )
                    .child(keychain_note(&org)),
            )
    }

    fn render_field(
        &self,
        ix: usize,
        f: &Field,
        org: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let secret = f.secret();
        let value = f.input.read(cx).value();
        let from_owner = f
            .var
            .default
            .as_deref()
            .is_some_and(|d| !secret && !value.is_empty() && d == value.as_ref());
        let focus = f.input.read(cx).focus_handle(cx);
        let reveal = secret.then(|| {
            div()
                .id(("open-shared-reveal", ix))
                .role(Role::Button)
                .test_support()
                .aria_label(if f.revealed {
                    "Hide value"
                } else {
                    "Reveal value"
                })
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(24.))
                .rounded(px(4.))
                .text_color(color("text-secondary"))
                .cursor_pointer()
                .hover(|s| s.text_color(color("text-primary")))
                .child(
                    glyph(if f.revealed {
                        icons::EYE_OFF
                    } else {
                        icons::EYE
                    })
                    .size(px(14.)),
                )
                .on_click(
                    cx.listener(move |this, _, window, cx| this.toggle_reveal(ix, window, cx)),
                )
        });
        let field = if f.locked() {
            // Read only: the owner's value and who set it.
            let set_by = format!("Set by {org}");
            div()
                .id(("open-shared-locked", ix))
                .role(Role::Label)
                .test_support()
                .aria_label(set_by.clone())
                .flex()
                .items_center()
                .gap(px(8.))
                .h(px(36.))
                .px(px(10.))
                .rounded(px(6.))
                .border_1()
                .border_color(color("border-100"))
                .bg(color("surface-100"))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis()
                        .font_family(mono_font())
                        .text_size(px(12.))
                        .text_color(color("text-secondary"))
                        .child(f.var.default.clone().unwrap_or_default()),
                )
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(glyph(icons::LOCK).size(px(12.)))
                        .child(set_by),
                )
                .into_any_element()
        } else {
            field_box(focus, window)
                // The Kit's input pads its own text: 10px in all, as the board.
                .pl(px(2.))
                .pr(px(if secret || from_owner { 6. } else { 10. }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .font_family(mono_font())
                        .text_size(px(12.))
                        .child(Input::new(&f.input).appearance(false).h(px(34.))),
                )
                .when(from_owner, |d| {
                    d.child(chip(format!("from {org}"), "text-tertiary"))
                })
                .children(reveal)
                .into_any_element()
        };
        div()
            .id(("open-shared-var", ix))
            .test_support()
            .aria_label(f.var.name.clone())
            .flex()
            .items_start()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .w(px(120.))
                    .gap(px(2.))
                    .pt(px(9.))
                    .child(
                        div()
                            .font_family(mono_font())
                            .text_size(px(12.))
                            .text_color(color("warning"))
                            .child(f.var.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(color(if f.var.required {
                                "danger"
                            } else {
                                "text-tertiary"
                            }))
                            .child(if f.var.required {
                                "Required"
                            } else {
                                "Optional"
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(field)
                    .when(!f.var.description.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .line_height(relative(1.45))
                                .text_color(color("text-tertiary"))
                                .child(f.var.description.clone()),
                        )
                    }),
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let back = (self.step != Step::Paste && self.inputs_for.is_none()).then(|| {
            Button::new("open-shared-back")
                .ghost()
                .h(px(32.))
                .px(px(4.))
                .text_size(px(13.))
                .text_color(color("text-secondary"))
                .label("Back")
                .disabled(self.busy)
                .on_click(cx.listener(|this, _, _, cx| this.back(cx)))
        });
        let empty = self.link.read(cx).value().trim().is_empty();
        let primary = match self.step {
            Step::Paste if self.sign_in_for.is_some() => {
                dialog_button("open-shared-sign-in", "Sign in")
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx)))
            }
            Step::Paste => dialog_button(
                "open-shared-open",
                if self.busy { "Opening…" } else { "Open link" },
            )
            .disabled(self.busy || empty)
            .on_click(cx.listener(|this, _, window, cx| this.open_link(window, cx))),
            Step::Review if self.existing.is_some() => {
                dialog_button("open-shared-existing-open", "Open it")
                    .on_click(cx.listener(|this, _, _, cx| this.open_existing(cx)))
            }
            Step::Review => dialog_button("open-shared-continue", "Continue").on_click(
                cx.listener(|this, _, window, cx| this.continue_to_credentials(window, cx)),
            ),
            Step::Credentials if self.inputs_for.is_some() => dialog_button(
                "open-shared-save",
                if self.busy { "Saving…" } else { "Save" },
            )
            .disabled(self.busy || !self.ready(cx))
            .on_click(cx.listener(|this, _, window, cx| this.add(window, cx))),
            Step::Credentials => dialog_button(
                "open-shared-add",
                if self.busy {
                    "Adding…"
                } else {
                    "Add to my studio"
                },
            )
            .disabled(self.busy || !self.ready(cx))
            .on_click(cx.listener(|this, _, window, cx| this.add(window, cx))),
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .pl(px(24.))
            .pr(px(16.))
            .py(px(12.))
            .border_t_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .children(back)
            .child(div().flex_1())
            .child(
                dialog_button("open-shared-cancel", "Cancel")
                    .outline()
                    .disabled(self.adding())
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(OpenSharedEvent::Close))),
            )
            .child(primary.primary())
    }
}

/// The board's `.inp`: 36px, outlined; blue with a ring while focused.
fn field_box(focus: FocusHandle, window: &Window) -> Div {
    let focused = focus.is_focused(window);
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(36.))
        .pl(px(10.))
        .pr(px(10.))
        .border_1()
        .border_color(color(if focused { "accent-blue" } else { "border-200" }))
        .rounded(px(6.))
        .bg(color("bg-100"))
        .when(focused, |d| {
            d.shadow(vec![BoxShadow {
                color: color("accent-blue").opacity(0.18),
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(3.),
                inset: false,
            }])
        })
}

impl Render for OpenSharedView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shared = self.shared.clone();
        let body = match (self.step, &shared) {
            (Step::Review, Some(s)) => self.render_review(s),
            (Step::Credentials, Some(s)) => self.render_credentials(s, window, cx),
            _ => self.render_paste(window, cx),
        };
        let title = match &shared {
            Some(s) if self.step != Step::Paste => title_text(s),
            _ => PASTE_TITLE.to_string(),
        };
        let room = window.viewport_size().height - px(140.);
        div()
            .id("open-shared")
            .track_focus(&self.focus)
            .role(Role::Dialog)
            .test_support()
            .aria_label(title)
            .flex()
            .flex_col()
            .text_size(px(13.))
            .children(self.render_strip())
            .child(self.render_header(cx))
            .child(
                div()
                    .id("open-shared-body")
                    .max_h(room.max(px(240.)))
                    .overflow_y_scroll()
                    .px(px(24.))
                    .pb(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .children(self.render_error())
                    .child(body),
            )
            .child(self.render_footer(cx))
    }
}

impl Workspace {
    /// The Open a shared link flow (Welcome's card, ⌘L, the palette, a
    /// `curl-runner://` link). With `link`, it opens that link at once;
    /// an open flow takes the new link instead of stacking another.
    pub(super) fn open_shared(
        &mut self,
        link: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<OpenSharedView> {
        if let Some(view) = self.open_shared_view() {
            match link {
                // An Add in flight finishes first, open or not; this link
                // opens next.
                Some(link) if view.read(cx).adding() => self.wait_for_link(link),
                // The dialog shows this link now: a sign-in out for another
                // one no longer opens that one.
                Some(link) => {
                    self.forget_link_sign_in();
                    view.update(cx, |v, cx| v.receive(&link, window, cx))
                }
                None => {}
            }
            return view;
        }
        let (backend, secrets) = (self.backend.clone(), self.secrets.clone());
        let base = self.store.base().map(Path::to_path_buf);
        let ws = cx.entity().downgrade();
        let view = cx.new(|cx| OpenSharedView::new(backend, secrets, base, ws, window, cx));
        self._open_shared_events = Some(cx.subscribe_in(
            &view,
            window,
            |this, _, event: &OpenSharedEvent, window, cx| {
                if let OpenSharedEvent::SignIn(link) = event {
                    this.sign_in_for_link(link.clone(), window, cx);
                    return;
                }
                window.close_dialog(cx);
                this.open_shared = None;
                this._open_shared_events = None;
                // Closed while a link's sign-in was out: abandoned, so a
                // later sign-in to that server opens nothing.
                if this.link_sign_in.take().is_some() {
                    this.pending_link = None;
                }
                match event {
                    OpenSharedEvent::Open(root) => this.open_received(root.clone(), window, cx),
                    // An updated copy's new inputs: the runs read them now.
                    OpenSharedEvent::Saved => {
                        this.invalidate_secrets();
                        this.refresh_vars(cx);
                        cx.notify();
                    }
                    _ => {}
                }
            },
        ));
        if let Some(link) = link {
            view.update(cx, |v, cx| v.receive(&link, window, cx));
        }
        let content = view.clone();
        let first_build = std::cell::Cell::new(true);
        window.open_dialog(cx, move |d, window, cx| {
            if first_build.replace(false) {
                // The link field to paste into, else the dialog itself, so
                // the Add's close finds focus in it (a deep link opens
                // straight on the review).
                let focus = if content.read(cx).step == Step::Paste {
                    content.read(cx).link.read(cx).focus_handle(cx)
                } else {
                    content.read(cx).focus.clone()
                };
                focus.focus(window, cx);
            }
            // Centred on the credentials step, so the steps don't jump.
            let top = ((window.viewport_size().height - px(DIALOG_H)) / 2.).max(px(40.));
            // Esc and the backdrop wait for an Add in flight.
            let closable = !content.read(cx).adding();
            d.entrance(DialogEntrance::None)
                .keyboard(closable)
                .overlay_closable(closable)
                .w(px(DIALOG_W))
                .margin_top(top)
                .p_0()
                .close_button(false)
                .on_ok(|_, _, _| false)
                .child(content.clone())
        });
        // Its own handle: `window.focused` here would still be whatever had
        // focus before the dialog's first frame.
        self.open_shared_focus = Some(view.read(cx).focus.clone());
        self.open_shared = Some(view.downgrade());
        view
    }

    /// A restricted link's `Sign in`: the browser sign-in for the link's
    /// server (not the studio's own). The link waits in `pending_link` and
    /// opens again once that server's callback is redeemed, while its
    /// dialog is still open.
    ///
    /// Latest user action wins: a link already waiting (another link's
    /// sign-in, or one that came in during an Add) is replaced, since the
    /// user just asked for this one.
    fn sign_in_for_link(&mut self, link: ShareLink, window: &mut Window, cx: &mut Context<Self>) {
        let server = Server::for_url(&link.server);
        self.pending_link = Some(link.url());
        self.link_sign_in = Some(server.url().trim_end_matches('/').to_string());
        let call = backend_call(cx, &self.backend, move |b| b.sign_in_url(&server));
        cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| {
                let opened = result.and_then(|url| this.open_sign_in_page(&url, window, cx));
                if let Err(e) = opened {
                    this.link_sign_in = None;
                    this.pending_link = None;
                    match this.open_shared_view() {
                        Some(view) => view.update(cx, |v, cx| v.sign_in_failed(&e, cx)),
                        None => notify_error(format!("Couldn't sign in: {e}"), window, cx),
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// `url` opens next (once an Add or the sign-in screen is done).
    /// Latest user action wins: a link waiting before, and a sign-in out
    /// for it, are dropped.
    fn wait_for_link(&mut self, url: String) {
        self.link_sign_in = None;
        self.pending_link = Some(url);
    }

    /// A restricted link's sign-in no longer opens that link.
    fn forget_link_sign_in(&mut self) {
        if self.link_sign_in.take().is_some() {
            self.pending_link = None;
        }
    }

    /// The open flow's view.
    pub(super) fn open_shared_view(&self) -> Option<Entity<OpenSharedView>> {
        self.open_shared.as_ref().and_then(WeakEntity::upgrade)
    }

    /// An Add finished, whether or not its dialog is still open (`shown`).
    /// Added: close the dialog and open the copy. Failed with nobody to
    /// tell: a notification. Then any link that came in meanwhile.
    fn add_finished(
        &mut self,
        outcome: Result<PathBuf, String>,
        shown: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match outcome {
            Ok(root) => {
                // Closed mid-Add, its view can outlive the dialog a while;
                // and a dialog opened over it is not this one to close.
                // Nothing focused (the studio in the background) counts too:
                // another dialog would have taken the focus.
                let on_top = self.open_shared_focus.take().is_some_and(|f| {
                    f.contains_focused(window, cx) || window.focused(cx).is_none()
                });
                if self.open_shared_view().is_some() && on_top {
                    window.close_dialog(cx);
                    self.open_shared = None;
                    self._open_shared_events = None;
                } else if let Some(view) = self.open_shared_view() {
                    // Still open under another dialog: it stays wired, so its
                    // Close works, and it won't add a second copy.
                    view.update(cx, |v, cx| v.added(cx));
                }
                // Listed now, whether or not the copy opens (unsaved
                // changes may keep the current one).
                self.refresh_received_all();
                cx.notify();
                self.open_received(root, window, cx);
            }
            Err(e) if !shown => notify_error(e, window, cx),
            Err(_) => {}
        }
        if let Some(link) = self.pending_link.take() {
            self.open_shared(Some(link), window, cx);
        }
    }

    /// Opens a received collection, once unsaved changes are settled.
    fn open_received(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_secrets();
        self.settle_unsaved(
            Rc::new(move |this, window, cx| {
                if this.open_collection(&root, window, cx) {
                    this.remember_folder(&root);
                }
            }),
            window,
            cx,
        );
    }

    /// `curl-runner://` links from the system: the first one opens.
    pub(super) fn open_urls(
        &mut self,
        urls: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(url) = urls
            .into_iter()
            .find(|u| u.trim_start().starts_with("curl-runner://"))
        else {
            return;
        };
        // Without the server the app still opens, and says so.
        if !crate::features::SERVER {
            self.notify_soon(SHARED_LINKS_SOON, window, cx);
            return;
        }
        // The dashboard's answer to a sign-in: never a share link, even when
        // it is malformed.
        if url.trim_start().starts_with("curl-runner://auth/") {
            if let Some(callback) = AuthCallback::parse(&url) {
                self.complete_sign_in(callback, window, cx);
            }
            return;
        }
        // The sign-in screen answers first; the link waits for it.
        if self.sign_in_shown() {
            self.wait_for_link(url);
        } else {
            self.open_shared(Some(url), window, cx);
        }
    }
}

impl Workspace {
    /// The dashboard sent the studio back with a handoff code: the backend
    /// redeems it (or refuses a callback this studio didn't ask for), then
    /// the sign-in screen closes and a link that waited for it opens.
    pub(super) fn complete_sign_in(
        &mut self,
        callback: AuthCallback,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let server = callback.server.trim_end_matches('/').to_string();
        let call = backend_call(cx, &self.backend, move |b| b.complete_sign_in(&callback));
        // Detached: a later callback (a foreign one, say) must not cancel
        // this one's redemption.
        cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| {
                this.sign_in_completed(&server, result.map(|_| ()), window, cx);
            })
            .ok();
        })
        .detach();
    }

    fn sign_in_completed(
        &mut self,
        server: &str,
        result: Result<(), BackendError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A restricted link waited for this server's sign-in. It opens only
        // on success with its dialog still up; refused, or the dialog gone
        // (closed, Esc), the link is forgotten, so no later sign-in to that
        // server pops it open.
        let waiting = self.link_sign_in.as_deref() == Some(server);
        if waiting {
            self.link_sign_in = None;
        }
        // …and only while the dialog still asks for that very link (a link
        // pasted or received since shows instead, and wins).
        let shown = self.open_shared_view().is_some_and(|v| {
            v.read(cx).sign_in_for.as_ref().map(ShareLink::url) == self.pending_link
        });
        let for_link = waiting && result.is_ok() && shown;
        if waiting && !for_link {
            self.pending_link = None;
        }
        if waiting
            && let Err(e) = &result
            && let Some(view) = self.open_shared_view()
        {
            view.update(cx, |v, cx| v.sign_in_failed(e, cx));
            return;
        }
        match result {
            // A callback for a server that isn't the current one (a
            // restricted link's, or a late one): its session is kept (it is
            // valid), the screen is left as it is; a link that waited for
            // it opens.
            Ok(()) if self.backend.server().url().trim_end_matches('/') != server => {
                if let Some(view) = &self.app_settings {
                    view.update(cx, |_, cx| cx.notify());
                }
                if for_link && let Some(link) = self.pending_link.take() {
                    self.open_shared(Some(link), window, cx);
                }
            }
            Ok(()) => {
                self.signed_in_as(Connection::Online);
                if self.sign_in.is_some() {
                    // Back where it came from, then any waiting link.
                    self.close_sign_in(window, cx);
                } else if let Some(link) = self.pending_link.take() {
                    self.open_shared(Some(link), window, cx);
                }
                if let Some(view) = &self.app_settings {
                    view.update(cx, |_, cx| cx.notify());
                }
                self.start_sync(window, cx);
                self.sync_chrome(window, cx);
                cx.notify();
            }
            // Not ours: dropped, with a toast; a sign-in in progress still
            // waits for its own callback.
            Err(BackendError::Invalid(text)) if text == FOREIGN_CALLBACK => {
                #[cfg(test)]
                self.toasts.push(text.clone());
                crate::ui::workspace::push_toast(window, Notification::error(text), cx);
            }
            Err(e) => match &self.sign_in {
                Some(view) => view.update(cx, |v, cx| v.sign_in_failed(&e, cx)),
                None => notify_error(format!("Couldn't sign in: {e}"), window, cx),
            },
        }
    }
}

/// What a `curl-runner://` link gets in a build without the server.
const SHARED_LINKS_SOON: &str = "Shared links need the curl-runner server, which is coming soon.";

impl Workspace {
    /// A `Coming soon` notice.
    fn notify_soon(&mut self, text: &'static str, window: &mut Window, cx: &mut App) {
        #[cfg(test)]
        self.toasts.push(text.into());
        crate::ui::workspace::push_toast(window, Notification::info(text), cx);
    }
}

/// What `Application::on_open_urls` calls: the links wait in `tx` until the
/// workspace takes them (a cold launch from a link has no window yet).
pub fn link_handler(tx: UnboundedSender<Vec<String>>) -> impl FnMut(Vec<String>) + 'static {
    move |urls| {
        let _ = tx.unbounded_send(urls);
    }
}

/// Hands each batch of queued links to the workspace, for as long as it is
/// there.
pub(super) fn pump_links(
    mut rx: UnboundedReceiver<Vec<String>>,
    window: AnyWindowHandle,
    workspace: WeakEntity<Workspace>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        while let Some(urls) = rx.next().await {
            let handled = cx.update_window(window, |_, window, cx| {
                workspace.update(cx, |ws, cx| ws.open_urls(urls, window, cx))
            });
            if !matches!(handled, Ok(Ok(()))) {
                break;
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{
        Local, PASTE_HINT, SharedCollection, keychain_note, subtitle_text, summary_text, title_text,
    };
    use chrono::Utc;

    fn shipping() -> SharedCollection {
        SharedCollection {
            link: crate::model::backend::ShareLink {
                server: "https://api.curl-runner.com".into(),
                token: "crsl_x".into(),
            },
            owner_org: "Acme".into(),
            team_slug: "acme".into(),
            name: "Shipping API".into(),
            summary: "rates, labels, tracking".into(),
            updated: Utc::now() - chrono::Duration::days(3),
            request_count: 12,
            version: 1,
            access: crate::model::backend::LinkAccess::Public,
            variables: Vec::new(),
        }
    }

    #[test]
    fn copy_reads_like_the_board() {
        let s = shipping();
        assert_eq!(title_text(&s), "Acme shared Shipping API with you");
        assert_eq!(
            subtitle_text("Acme"),
            "You can run every request with your own credentials. Only Acme can edit it."
        );
        assert_eq!(
            keychain_note("Acme"),
            "Saved to your keychain on this computer. Never sent to Acme or synced."
        );
        assert_eq!(
            summary_text(&s, Local::now()),
            "12 requests · rates, labels, tracking · updated 3 days ago"
        );
        assert_eq!(
            super::sign_in_text(&s.link),
            "Sign in to api.curl-runner.com to open this collection."
        );
    }

    #[test]
    fn paste_hint_names_both_forms() {
        assert!(
            PASTE_HINT.contains("cloud.curl-runner.com/s/"),
            "{PASTE_HINT}"
        );
        assert!(PASTE_HINT.contains("curl-runner://open"), "{PASTE_HINT}");
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_backend};
    use super::{OpenSharedView, Step};
    #[cfg(feature = "server")]
    use super::{link_handler, pump_links};
    use crate::model::backend::Backend;
    use crate::model::secrets::{MemorySecrets, SecretStore, service_name};
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;
    use std::sync::Arc;

    const PATH_LINK: &str = "curl-runner://open/acme/shipping-api?share=crsl_7Kx2Qm&server=https%3A%2F%2Fapi.mock.curl-runner.test";
    const DEEP_LINK: &str =
        "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_7Kx2Qm";
    const TOKEN: &str = "sk-live-customer-4242";

    struct Studio {
        state: tempfile::TempDir,
        secrets: Arc<MemorySecrets>,
        backend: Arc<dyn Backend>,
        window: AnyWindowHandle,
        ws: Entity<Workspace>,
    }

    impl Studio {
        fn shared_dir(&self) -> std::path::PathBuf {
            self.state.path().join("shared")
        }
    }

    fn studio(cx: &mut TestAppContext) -> Studio {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let secrets = Arc::new(MemorySecrets::default());
        let backend = test_backend();
        let (s, b) = (secrets.clone() as Arc<dyn SecretStore>, backend.clone());
        let (window, ws) = cx.update(|cx| {
            gpui_kit::init(cx);
            super::super::app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(1440.), px(900.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Workspace::new(None, store, s, b, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        Studio {
            state,
            secrets,
            backend,
            window,
            ws,
        }
    }

    fn open_flow(cx: &mut TestAppContext, s: &Studio) -> Entity<OpenSharedView> {
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_shared(None, window, cx))
        })
        .unwrap()
    }

    fn paste_and_open(
        cx: &mut TestAppContext,
        s: &Studio,
        view: &Entity<OpenSharedView>,
        link: &str,
    ) {
        let view = view.clone();
        cx.update_window(s.window, |_, window, cx| {
            view.update(cx, |v, cx| v.paste(link, window, cx));
            window.render_frame(cx);
            window.click("open-shared-open", cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn click(cx: &mut TestAppContext, s: &Studio, id: &str) {
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.click(id.to_string(), cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    fn fill(
        cx: &mut TestAppContext,
        s: &Studio,
        view: &Entity<OpenSharedView>,
        name: &str,
        value: &str,
    ) {
        let view = view.clone();
        cx.update_window(s.window, |_, window, cx| {
            view.update(cx, |v, cx| v.fill(name, value, window, cx));
        })
        .unwrap();
    }

    fn labelled(cx: &mut TestAppContext, s: &Studio, id: &str, label: &str) -> bool {
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window
                .try_find(id.to_string())
                .is_some_and(|e| e.label() == Some(label))
        })
        .unwrap()
    }

    fn dialog_open(cx: &mut TestAppContext, s: &Studio) -> bool {
        cx.update_window(s.window, |_, window, cx| window.has_active_dialog(cx))
            .unwrap()
    }

    /// Every file under `dir`, as text.
    fn texts(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(texts(&p));
            } else if let Ok(t) = std::fs::read_to_string(&p) {
                out.push(t);
            }
        }
        out
    }

    #[gpui_kit::test]
    fn paste_link_resolves_and_adds_collection(cx: &mut TestAppContext) {
        let s = studio(cx);
        let view = open_flow(cx, &s);
        assert!(labelled(cx, &s, "open-shared-title", "Open a shared link"));
        paste_and_open(cx, &s, &view, PATH_LINK);
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Review);
        assert!(labelled(
            cx,
            &s,
            "open-shared-title",
            "Acme shared Shipping API with you"
        ));
        assert!(labelled(
            cx,
            &s,
            "open-shared-summary",
            "12 requests · rates, labels, tracking · updated 3 days ago"
        ));
        assert!(labelled(cx, &s, "open-shared-from", PATH_LINK));
        click(cx, &s, "open-shared-continue");
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Credentials);
        // Required first; BASE_URL comes filled in from Acme.
        let names = view.read_with(cx, |v, _| {
            v.fields
                .iter()
                .map(|f| f.var.name.clone())
                .collect::<Vec<_>>()
        });
        assert_eq!(names, ["BASE_URL", "API_TOKEN", "ACCOUNT_ID", "CURRENCY"]);
        assert_eq!(
            view.read_with(cx, |v, cx| v.fields[0].input.read(cx).value().to_string()),
            "https://api.example.com/shipping"
        );
        // CURRENCY is locked: shown read only, set by Acme.
        let set_by = cx
            .update_window(s.window, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find(("open-shared-locked", 3usize))
                    .and_then(|e| e.label().map(String::from))
            })
            .unwrap();
        assert_eq!(set_by.as_deref(), Some("Set by Acme"));
        assert!(view.read_with(cx, |v, _| v.fields[3].locked()));
        assert!(
            !view.read_with(cx, |v, _| v.fields[0].locked()),
            "suggested"
        );
        // API_TOKEN is required: Add waits for it.
        assert!(!view.read_with(cx, |v, cx| v.ready(cx)));
        click(cx, &s, "open-shared-add");
        assert!(!s.shared_dir().exists());
        fill(cx, &s, &view, "API_TOKEN", TOKEN);
        assert!(view.read_with(cx, |v, cx| v.ready(cx)));
        click(cx, &s, "open-shared-add");

        let root = s.shared_dir().join("acme-shipping-api");
        let yaml = crate::model::backend::sync_files(&root);
        assert_eq!(yaml.len(), 12);
        let received = s.backend.received();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].team, "Acme");
        // Where the copy lives stays on this Mac: never in a call.
        let local = crate::model::received::load(Some(s.state.path()));
        assert_eq!(local.len(), 1);
        assert_eq!(
            (local[0].share_id.as_str(), &local[0].root),
            (received[0].link.token.as_str(), &root)
        );
        let state_dir = s.state.path().to_string_lossy().into_owned();
        for call in s.backend.dev().unwrap().recorded() {
            assert!(!call.contains(&state_dir), "a local path left: {call}");
            assert!(!call.contains("/shared/"), "a local path left: {call}");
        }
        s.ws.read_with(cx, |ws, cx| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, root);
            assert_eq!(ws.meta(cx).read_only_from.as_deref(), Some("Acme"));
            assert_eq!(ws.meta(cx).received_vars.len(), 4);
            assert_eq!(local[0].collection_id, ws.meta(cx).id);
            assert!(ws.open_shared_view().is_none());
        });
        assert!(!dialog_open(cx, &s));

        // Opened again: it's already here.
        let view = open_flow(cx, &s);
        paste_and_open(cx, &s, &view, DEEP_LINK);
        assert!(labelled(
            cx,
            &s,
            "open-shared-existing",
            "Already in your studio — open it?"
        ));
        click(cx, &s, "open-shared-existing-open");
        assert!(!s.shared_dir().join("acme-shipping-api-2").exists());
        assert_eq!(s.backend.received().len(), 1);
        assert!(!dialog_open(cx, &s));
    }

    #[gpui_kit::test]
    fn credentials_go_to_keychain_and_env(cx: &mut TestAppContext) {
        let s = studio(cx);
        let view = open_flow(cx, &s);
        paste_and_open(cx, &s, &view, DEEP_LINK);
        click(cx, &s, "open-shared-continue");
        // Masked, with a reveal.
        assert!(!view.read_with(cx, |v, _| v.fields[1].revealed), "masked");
        cx.update_window(s.window, |_, window, cx| {
            window.render_frame(cx);
            window.click(("open-shared-reveal", 1usize), cx);
        })
        .unwrap();
        assert!(view.read_with(cx, |v, _| v.fields[1].revealed));
        fill(cx, &s, &view, "API_TOKEN", TOKEN);
        fill(cx, &s, &view, "ACCOUNT_ID", "acct_4242");
        // A locked field takes nothing, even if something got typed in.
        fill(cx, &s, &view, "CURRENCY", "eur");
        click(cx, &s, "open-shared-add");

        let root = s.shared_dir().join("acme-shipping-api");
        let id = s.ws.read_with(cx, |ws, cx| ws.meta(cx).id.clone());
        let service = service_name("acme-shipping-api", "Default", &id);
        assert_eq!(s.secrets.get(&service, "API_TOKEN").as_deref(), Some(TOKEN));
        assert_eq!(s.secrets.get(&service, "BASE_URL"), None);
        let env = std::fs::read_to_string(root.join(".env")).unwrap();
        assert!(
            env.contains("BASE_URL=https://api.example.com/shipping"),
            "{env}"
        );
        assert!(env.contains("ACCOUNT_ID=acct_4242"), "{env}");
        assert!(!env.contains("CURRENCY"), "locked: the owner's: {env}");
        s.ws.read_with(cx, |ws, cx| {
            assert_eq!(ws.meta(cx).secret_names, ["API_TOKEN"]);
        });
        // The token is nowhere but the Keychain: not on disk (the
        // collection or the studio's state), not sent to the backend.
        for text in texts(s.state.path()) {
            assert!(!text.contains(TOKEN), "on disk: {text}");
        }
        for call in s.backend.dev().unwrap().recorded() {
            assert!(!call.contains(TOKEN), "sent: {call}");
            assert!(!call.contains("acct_4242"), "sent: {call}");
        }
    }

    #[gpui_kit::test]
    fn expired_link_creates_nothing(cx: &mut TestAppContext) {
        let s = studio(cx);
        let dev = s.backend.dev().unwrap();
        dev.expire_links(true);
        let view = open_flow(cx, &s);
        paste_and_open(cx, &s, &view, DEEP_LINK);
        let expired = "This link has expired. Ask whoever shared it for a new one.";
        assert_eq!(
            view.read_with(cx, |v, _| v.error().map(String::from))
                .as_deref(),
            Some(expired)
        );
        assert!(labelled(cx, &s, "open-shared-error", expired));
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Paste);

        dev.expire_links(false);
        dev.set_offline(true);
        click(cx, &s, "open-shared-open");
        assert!(labelled(
            cx,
            &s,
            "open-shared-error",
            "You're offline. Connect to open shared links."
        ));

        // It expires between Review and Add: still nothing.
        dev.set_offline(false);
        click(cx, &s, "open-shared-open");
        click(cx, &s, "open-shared-continue");
        fill(cx, &s, &view, "API_TOKEN", TOKEN);
        dev.expire_links(true);
        click(cx, &s, "open-shared-add");
        // By now the collection's org is known.
        assert!(labelled(
            cx,
            &s,
            "open-shared-error",
            "This link has expired. Ask Acme for a new one."
        ));
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Credentials);
        assert!(!s.shared_dir().exists());
        assert!(s.backend.received().is_empty());
        assert!(s.ws.read_with(cx, |ws, _| ws.state.collection.is_none()));
    }

    #[gpui_kit::test]
    fn restricted_link_asks_for_sign_in_then_continues(cx: &mut TestAppContext) {
        use crate::model::backend::{
            Access, CollectionKey, MOCK_SERVER, ShareLink, ShareUpdate, testing,
        };
        let s = studio(cx);
        // Acme's restricted share; this studio is signed in nowhere.
        testing::sign_in(&*s.backend);
        let key = CollectionKey {
            id: "a1b2c3d4".into(),
            name: "Payments API".into(),
            cloud: None,
        };
        let update = ShareUpdate {
            access: Some(Access::InvitedOnly),
            ..Default::default()
        };
        let url = futures::executor::block_on(s.backend.update_share(&key, update))
            .unwrap()
            .link
            .unwrap();
        testing::sign_out(&*s.backend);
        let link = ShareLink::parse(&url).unwrap();
        assert_eq!(link.server, MOCK_SERVER);

        let view = open_flow(cx, &s);
        paste_and_open(cx, &s, &view, &url);
        // Asked once, before the form: no review, no error.
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Paste);
        assert!(view.read_with(cx, |v, _| v.error().is_none()));
        assert!(labelled(
            cx,
            &s,
            "open-shared-sign-in-note",
            "Sign in to api.mock.curl-runner.test to open this collection."
        ));
        assert_eq!(s.backend.account_on(MOCK_SERVER), None);
        click(cx, &s, "open-shared-sign-in");

        // Signed in to the link's server (not the studio's), and the open
        // went on with the same link.
        assert!(s.backend.account_on(MOCK_SERVER).is_some());
        assert_eq!(s.backend.account(), None);
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Review);
        assert!(labelled(
            cx,
            &s,
            "open-shared-title",
            "Acme shared Payments API with you"
        ));
        assert!(s.ws.read_with(cx, |ws, _| ws.pending_link.is_none()));
        let recorded = s.backend.dev().unwrap().recorded();
        let resolves = recorded
            .iter()
            .filter(|c| c.starts_with("resolve_link("))
            .count();
        assert_eq!(resolves, 2, "{recorded:?}");
        assert!(
            recorded.iter().any(
                |c| c.starts_with(&format!("sign_in_url(SelfHosted {{ url: \"{MOCK_SERVER}\""))
            ),
            "{recorded:?}"
        );
    }

    /// A link's sign-in refused, or its dialog closed: the link and its
    /// server are forgotten, so a later sign-in to that server opens
    /// nothing.
    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn an_abandoned_link_sign_in_never_opens_its_link_later(cx: &mut TestAppContext) {
        use crate::model::backend::{BackendError, MOCK_SERVER};
        let s = studio(cx);
        let ws = s.ws.clone();
        let resolves = |b: &Arc<dyn Backend>| {
            b.dev()
                .unwrap()
                .recorded()
                .iter()
                .filter(|c| c.starts_with("resolve_link("))
                .count()
        };
        let wait = |ws: &mut Workspace| {
            ws.pending_link = Some(DEEP_LINK.into());
            ws.link_sign_in = Some(MOCK_SERVER.into());
        };
        let forgotten = |cx: &mut TestAppContext| {
            ws.read_with(cx, |ws, _| {
                ws.pending_link.is_none() && ws.link_sign_in.is_none()
            })
        };
        // Refused.
        let view = open_flow(cx, &s);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                wait(ws);
                ws.sign_in_completed(MOCK_SERVER, Err(BackendError::Unauthorized), window, cx);
            })
        })
        .unwrap();
        assert!(forgotten(cx));
        assert!(view.read_with(cx, |v, _| v.error().is_some()), "said so");
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.sign_in_completed(MOCK_SERVER, Ok(()), window, cx)
            })
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(resolves(&s.backend), 0, "nothing opened");
        // Abandoned: the dialog closed while the browser was out.
        cx.update_window(s.window, |_, _, cx| ws.update(cx, |ws, _| wait(ws)))
            .unwrap();
        click(cx, &s, "open-shared-cancel");
        assert!(!dialog_open(cx, &s));
        assert!(forgotten(cx));
        // Closed some other way (Esc): the callback finds no dialog.
        open_flow(cx, &s);
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, _| wait(ws));
            window.close_dialog(cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.sign_in_completed(MOCK_SERVER, Ok(()), window, cx)
            })
        })
        .unwrap();
        cx.run_until_parked();
        assert!(forgotten(cx));
        assert!(!dialog_open(cx, &s));
        assert_eq!(resolves(&s.backend), 0, "nothing opened");
    }

    /// Link B's browser sign-in is out; link C comes in (a deep link, or
    /// pasted by hand). When B's sign-in lands, C stays: B never opens
    /// over it.
    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn a_newer_link_wins_over_a_waiting_sign_in(cx: &mut TestAppContext) {
        use crate::model::backend::{Access, CollectionKey, MOCK_SERVER, ShareUpdate, testing};
        for by_hand in [false, true] {
            let s = studio(cx);
            testing::sign_in(&*s.backend);
            let key = CollectionKey {
                id: "a1b2c3d4".into(),
                name: "Payments API".into(),
                cloud: None,
            };
            let update = ShareUpdate {
                access: Some(Access::InvitedOnly),
                ..Default::default()
            };
            let b = futures::executor::block_on(s.backend.update_share(&key, update))
                .unwrap()
                .link
                .unwrap();
            testing::sign_out(&*s.backend);
            let view = open_flow(cx, &s);
            paste_and_open(cx, &s, &view, &b);
            // Sign in clicked: B waits for its server's callback.
            let waiting = view.read_with(cx, |v, _| v.sign_in_for.clone()).unwrap();
            let ws = s.ws.clone();
            ws.update(cx, |ws, _| {
                ws.pending_link = Some(waiting.url());
                ws.link_sign_in = Some(MOCK_SERVER.into());
            });
            // C, the fixture's public link.
            if by_hand {
                cx.update_window(s.window, |_, window, cx| {
                    view.update(cx, |v, cx| {
                        v.paste(DEEP_LINK, window, cx);
                        v.open_link(window, cx);
                    })
                })
                .unwrap();
                cx.run_until_parked();
            } else {
                cx.update_window(s.window, |_, window, cx| {
                    ws.update(cx, |ws, cx| {
                        ws.open_shared(Some(DEEP_LINK.into()), window, cx)
                    });
                })
                .unwrap();
                cx.run_until_parked();
            }
            let shipping = "Acme shared Shipping API with you";
            assert!(labelled(cx, &s, "open-shared-title", shipping));
            let resolves = |b: &Arc<dyn Backend>| {
                b.dev()
                    .unwrap()
                    .recorded()
                    .iter()
                    .filter(|c| c.starts_with("resolve_link("))
                    .count()
            };
            let before = resolves(&s.backend);
            // B's sign-in lands, the dialog still up.
            testing::sign_in(&*s.backend);
            cx.update_window(s.window, |_, window, cx| {
                ws.update(cx, |ws, cx| {
                    ws.sign_in_completed(MOCK_SERVER, Ok(()), window, cx)
                })
            })
            .unwrap();
            cx.run_until_parked();
            assert!(labelled(cx, &s, "open-shared-title", shipping), "C stays");
            assert_eq!(resolves(&s.backend), before, "B never opened");
            assert!(ws.read_with(cx, |ws, _| ws.pending_link.is_none()
                && ws.link_sign_in.is_none()));
        }
    }

    #[gpui_kit::test]
    fn garbage_link_shows_error(cx: &mut TestAppContext) {
        let s = studio(cx);
        let view = open_flow(cx, &s);
        paste_and_open(cx, &s, &view, "hello there");
        assert!(labelled(
            cx,
            &s,
            "open-shared-error",
            "That isn't a curl-runner share link."
        ));
        let resolved = |b: &Arc<dyn Backend>| {
            b.dev()
                .unwrap()
                .recorded()
                .iter()
                .filter(|c| c.starts_with("resolve_link("))
                .count()
        };
        assert_eq!(resolved(&s.backend), 0, "never sent");
        // Typing clears it; a well-formed link the server doesn't know.
        paste_and_open(
            cx,
            &s,
            &view,
            "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_nothing9",
        );
        assert_eq!(resolved(&s.backend), 1);
        assert!(labelled(
            cx,
            &s,
            "open-shared-error",
            "This link doesn't work."
        ));
        assert!(!s.shared_dir().exists());
        // Cancel closes it.
        click(cx, &s, "open-shared-cancel");
        assert!(!dialog_open(cx, &s));
        assert!(s.ws.read_with(cx, |ws, _| ws.open_shared_view().is_none()));
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn deep_link_event_opens_flow(cx: &mut TestAppContext) {
        let s = studio(cx);
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let mut handler = link_handler(tx);
        // Before anything takes them (a cold launch): they wait.
        handler(vec!["https://example.com/ignored".into(), DEEP_LINK.into()]);
        let (window, ws) = (s.window, s.ws.downgrade());
        cx.update(|cx| pump_links(rx, window, ws, cx));
        cx.run_until_parked();
        let view =
            s.ws.read_with(cx, |ws, _| ws.open_shared_view())
                .expect("the flow opened");
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Review);
        assert!(labelled(cx, &s, "open-shared-from", DEEP_LINK));
        assert!(dialog_open(cx, &s));
        // A second link while it is open replaces the first, in place.
        handler(vec![
            "curl-runner://open?server=https%3A%2F%2Fapi.mock.curl-runner.test&share=crsl_zz9"
                .into(),
        ]);
        cx.run_until_parked();
        let again = s.ws.read_with(cx, |ws, _| ws.open_shared_view()).unwrap();
        assert_eq!(again, view);
        assert!(labelled(
            cx,
            &s,
            "open-shared-error",
            "This link doesn't work."
        ));
    }

    fn entries(dir: &Path) -> Vec<std::ffi::OsString> {
        std::fs::read_dir(dir)
            .map(|d| d.flatten().map(|e| e.file_name()).collect())
            .unwrap_or_default()
    }

    /// Credentials filled and Add pressed, with `add_received` held.
    fn add_held(cx: &mut TestAppContext, s: &Studio) -> Entity<OpenSharedView> {
        s.backend.dev().unwrap().hold_updates(true);
        let view = open_flow(cx, s);
        paste_and_open(cx, s, &view, DEEP_LINK);
        click(cx, s, "open-shared-continue");
        fill(cx, s, &view, "API_TOKEN", TOKEN);
        click(cx, s, "open-shared-add");
        assert!(view.read_with(cx, |v, _| v.adding()), "in flight");
        view
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn closing_mid_add_still_finishes_and_opens(cx: &mut TestAppContext) {
        let s = studio(cx);
        let view = add_held(cx, &s);
        // Cancel waits for it; a link that comes in waits too.
        click(cx, &s, "open-shared-cancel");
        assert!(dialog_open(cx, &s));
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_urls(vec![DEEP_LINK.into()], window, cx);
            });
        })
        .unwrap();
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Credentials);
        assert!(
            s.ws.read_with(cx, |ws, _| ws.pending_link.is_some()),
            "queued"
        );
        // Closed anyway (the view goes with it), then the record lands.
        cx.update_window(s.window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        drop(view);
        cx.run_until_parked();
        s.backend.dev().unwrap().hold_updates(false);
        cx.run_until_parked();
        let root = s.shared_dir().join("acme-shipping-api");
        assert_eq!(s.backend.received().len(), 1);
        let local = crate::model::received::load(Some(s.state.path()));
        assert_eq!(local[0].root, root);
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, root);
        });
        // The waiting link opened next: already in the studio.
        assert!(labelled(
            cx,
            &s,
            "open-shared-existing",
            "Already in your studio — open it?"
        ));
    }

    /// A link from the website opens the flow on its review; Add closes it,
    /// even with the studio in the background (nothing focused).
    #[gpui_kit::test]
    fn a_deep_linked_add_closes_the_dialog(cx: &mut TestAppContext) {
        for background in [false, true] {
            let s = studio(cx);
            let (tx, rx) = futures::channel::mpsc::unbounded();
            let mut handler = link_handler(tx);
            handler(vec![DEEP_LINK.into()]);
            let (window, ws) = (s.window, s.ws.downgrade());
            cx.update(|cx| pump_links(rx, window, ws, cx));
            cx.run_until_parked();
            let view = s.ws.read_with(cx, |ws, _| ws.open_shared_view()).unwrap();
            click(cx, &s, "open-shared-continue");
            fill(cx, &s, &view, "API_TOKEN", TOKEN);
            if background {
                cx.update_window(s.window, |_, window, cx| window.blur(cx))
                    .unwrap();
            }
            click(cx, &s, "open-shared-add");
            cx.run_until_parked();
            assert!(!dialog_open(cx, &s), "closed (background: {background})");
            assert!(s.ws.read_with(cx, |ws, _| ws.open_shared_view()).is_none());
            assert_eq!(s.backend.received().len(), 1, "one copy");
        }
    }

    #[gpui_kit::test]
    fn a_dialog_opened_over_the_flow_stays_when_the_add_lands(cx: &mut TestAppContext) {
        use gpui_kit::TestSupportExt as _;
        use gpui_kit::prelude::*;
        let s = studio(cx);
        let _view = add_held(cx, &s);
        cx.update_window(s.window, |_, window, cx| {
            window.open_dialog(cx, |d, _, _| {
                d.child(
                    gpui_kit::div()
                        .id("another-dialog")
                        .role(gpui_kit::Role::Group)
                        .test_support()
                        .aria_label("Another dialog"),
                )
            });
            window.render_frame(cx);
        })
        .unwrap();
        s.backend.dev().unwrap().hold_updates(false);
        cx.run_until_parked();
        let root = s.shared_dir().join("acme-shipping-api");
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, root);
        });
        assert!(
            labelled(cx, &s, "another-dialog", "Another dialog"),
            "not the flow's to close"
        );
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn an_added_copy_is_listed_while_unsaved_changes_wait(cx: &mut TestAppContext) {
        let s = studio(cx);
        let mine = tempfile::tempdir().unwrap();
        std::fs::write(
            mine.path().join("a.yaml"),
            "request:\n  url: https://api.example.com\n",
        )
        .unwrap();
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(mine.path(), window, cx));
                ws.open_file(Path::new("a.yaml"), window, cx);
                let editor = ws.tabs[0].editor.clone();
                editor.update(cx, |e, cx| e.set_value("request: {}\n", window, cx));
                ws.state.open[0].set_buffer("request: {}\n".into());
            });
        })
        .unwrap();
        let _view = add_held(cx, &s);
        s.backend.dev().unwrap().hold_updates(false);
        cx.run_until_parked();
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, mine.path());
            let listed: Vec<_> = ws.received_all.iter().map(|r| r.root.clone()).collect();
            assert_eq!(listed, [s.shared_dir().join("acme-shipping-api")]);
        });
        assert!(dialog_open(cx, &s), "asking about the unsaved file");
    }

    #[gpui_kit::test]
    fn refused_record_rolls_everything_back(cx: &mut TestAppContext) {
        let s = studio(cx);
        let view = add_held(cx, &s);
        assert_eq!(entries(&s.shared_dir()).len(), 1, "written, not recorded");
        cx.update_window(s.window, |_, window, cx| window.close_dialog(cx))
            .unwrap();
        drop(view);
        let dev = s.backend.dev().unwrap();
        dev.set_offline(true);
        dev.hold_updates(false);
        cx.run_until_parked();
        assert!(s.backend.received().is_empty());
        assert!(crate::model::received::load(Some(s.state.path())).is_empty());
        for dir in ["shared", "meta", "sync"] {
            let left = entries(&s.state.path().join(dir));
            assert!(left.is_empty(), "{dir}: {left:?}");
        }
        assert_eq!(s.secrets.len(), 0, "no Keychain items");
        assert!(s.ws.read_with(cx, |ws, _| ws.state.collection.is_none()));
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn open_urls_routes_callbacks_to_sign_in(cx: &mut TestAppContext) {
        let s = studio(cx);
        let ws = s.ws.clone();
        let callback = "curl-runner://auth/callback?code=crh_abc123&server=https%3A%2F%2Fapi.mock.curl-runner.test";
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_urls(vec![callback.into()], window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        assert!(
            s.ws.read_with(cx, |ws, _| ws.open_shared_view().is_none()
                && ws.pending_link.is_none()),
            "a callback never reaches the share flow"
        );
        // No sign-in was started: it is refused, with a toast, and nobody
        // is signed in.
        s.ws.read_with(cx, |ws, _| {
            assert_eq!(
                ws.toasts.last().map(String::as_str),
                Some("That sign-in link didn't come from this studio.")
            );
        });
        assert!(s.backend.account().is_none());
        // A share link still does.
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_urls(vec![DEEP_LINK.into()], window, cx)
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(s.ws.read_with(cx, |ws, _| ws.open_shared_view().is_some()));
    }

    #[cfg(feature = "server")]
    #[gpui_kit::test]
    fn a_link_during_sign_in_waits_for_it(cx: &mut TestAppContext) {
        let s = studio(cx);
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                drop(ws.open_sign_in(None, window, cx));
                ws.open_urls(vec![DEEP_LINK.into()], window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        assert!(s.ws.read_with(cx, |ws, _| ws.open_shared_view().is_none()));
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.close_sign_in(window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        let view = s.ws.read_with(cx, |ws, _| ws.open_shared_view()).unwrap();
        assert_eq!(view.read_with(cx, |v, _| v.step()), Step::Review);
    }

    /// End to end: the copy runs with the token typed in, read back from
    /// the Keychain under the service the workspace resolves.
    #[gpui_kit::test]
    fn added_copy_runs_with_the_keychain_token(cx: &mut TestAppContext) {
        use gpui_kit::Action as _;
        use std::sync::Mutex;
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = seen.clone();
        let srv = testkit::TestServer::with_handler(move |req| {
            let auth = req
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            log.lock().unwrap().push(auth);
            testkit::Response::from_data(br#"{"carriers":["a"]}"#.to_vec())
                .with_header("Content-Type", "application/json")
        });
        let s = studio(cx);
        let view = open_flow(cx, &s);
        paste_and_open(cx, &s, &view, DEEP_LINK);
        click(cx, &s, "open-shared-continue");
        fill(cx, &s, &view, "BASE_URL", &srv.base_url);
        fill(cx, &s, &view, "API_TOKEN", TOKEN);
        click(cx, &s, "open-shared-add");
        let ws = s.ws.clone();
        cx.update_window(s.window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("rates/list-carriers.yaml"), window, cx)
            });
            window.render_frame(cx);
            window.dispatch_action(super::super::app::RunActive.boxed_clone(), cx);
        })
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(20));
            cx.run_until_parked();
            let done = !seen.lock().unwrap().is_empty()
                && s.ws.read_with(cx, |ws, _| ws.runs.active.is_none());
            if done {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the run did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(*seen.lock().unwrap(), [format!("Bearer {TOKEN}")]);
    }
}
