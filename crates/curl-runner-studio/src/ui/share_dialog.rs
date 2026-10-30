//! The Share dialog (board: Share), also the collection settings' Sharing
//! page: the share link with `Copy link`, who the link works for and when
//! it expires, invites, the people with access, what they get and, per
//! variable, whether its Default value is shared.
//!
//! Every change goes to the server at once; a failed call (offline, signed
//! out) shows the error at the top and changes nothing here. The variables
//! sent are [`share_payload`]'s: names, descriptions, the required flag and
//! only the ticked non-secret `.env` defaults. No Keychain value is read.

use super::app::mono_font;
use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{label_text, neutral_pill, switch};
use super::workspace::Workspace;
use crate::model::backend::{
    Access, Backend, BackendError, CollectionKey, Expiry, Person, Role as PersonRole,
    ShareSettings, ShareUpdate, SharedVar, share_candidates, share_payload,
};
use crate::model::collection::TreeNode;
use crate::model::collection_meta::CollectionMeta;
use chrono::{Local, NaiveDate};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The board's dialog.
const DIALOG_W: f32 = 600.;
const DIALOG_TOP: f32 = 40.;
/// How long `Copy link` reads `Copied`.
const COPIED_FOR: Duration = Duration::from_millis(1500);
/// The board's copy, verbatim.
const SUBTITLE: &str = "People you share with can open and run every request with their own credentials. They can't change anything.";
const NEVER_SHARED: &str =
    "Never shared: variable values, secrets from your keychain, your run history";
const FOOTER_NOTE: &str = "They'll get updates when you push changes.";
/// The Expires menu, in the board's order.
const EXPIRIES: [(Expiry, &str); 3] = [
    (Expiry::Days(30), "in 30 days"),
    (Expiry::Days(7), "in 7 days"),
    (Expiry::Never, "Never"),
];

/// What the view asks of whoever shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareEvent {
    /// The share changed on the server (link, people, variables) or the
    /// collection's notes did (a `Share default`): re-read both.
    Changed,
    /// Close, Cancel or Done.
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareMode {
    /// The dialog: its header, close button and footer.
    Dialog,
    /// The Sharing settings page: the body only, under the page's header.
    Page,
}

/// The collection being shared.
pub struct ShareSource {
    pub backend: Arc<dyn Backend>,
    pub key: CollectionKey,
    pub root: PathBuf,
    /// Where the notes are saved (`None`: kept in memory).
    pub base: Option<PathBuf>,
    pub meta: CollectionMeta,
    /// `18 requests in 4 folders`.
    pub requests: usize,
    pub folders: usize,
}

pub struct ShareView {
    src: ShareSource,
    mode: ShareMode,
    /// The server's view, once a call got through.
    pub(super) settings: Option<ShareSettings>,
    /// Every variable, with the default it would share where it has one
    /// (never a secret's): what the Variables list shows.
    vars: Vec<SharedVar>,
    /// The last call's failure, at the top.
    pub(super) error: Option<String>,
    invite: Entity<InputState>,
    pub(super) invite_error: Option<String>,
    /// The person whose removal waits for a confirm, by email.
    pub(super) removing: Option<String>,
    /// Set while `Copy link` reads `Copied`.
    copied: Option<Task<()>>,
    save: Option<Task<()>>,
    _invite_events: Subscription,
}

impl EventEmitter<ShareEvent> for ShareView {}

/// `true` for `x@y.z`: something, an `@`, a host with a dot inside it.
pub fn valid_email(text: &str) -> bool {
    let Some((user, host)) = text.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && !text.contains(char::is_whitespace)
        && !host.contains('@')
        && host
            .split_once('.')
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && !b.ends_with('.'))
}

/// What a failed call says at the top: offline and signed out in the
/// boards' words, and that nothing changed.
pub fn share_error(e: &BackendError) -> String {
    match e {
        BackendError::Offline => {
            "You're offline. Connect to change sharing; nothing changed.".into()
        }
        BackendError::SignedOut => "Sign in to share. Nothing changed.".into(),
        other => other.to_string(),
    }
}

/// The Expires menu's label for a link that expires on `date`.
pub fn expiry_label(date: Option<NaiveDate>, today: NaiveDate) -> String {
    let Some(date) = date else {
        return "Never".into();
    };
    match (date - today).num_days() {
        n if n < 0 => "Expired".into(),
        0 => "today".into(),
        1 => "tomorrow".into(),
        n => format!("in {n} days"),
    }
}

/// `18 requests in 4 folders, with every check you wrote`.
pub fn requests_text(requests: usize, folders: usize) -> String {
    let r = if requests == 1 { "request" } else { "requests" };
    match folders {
        0 => format!("{requests} {r}, with every check you wrote"),
        1 => format!("{requests} {r} in 1 folder, with every check you wrote"),
        f => format!("{requests} {r} in {f} folders, with every check you wrote"),
    }
}

/// `JR` for `Jon Rivera`, `AC` for `Acme`.
fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let letters: String = match words.as_slice() {
        [] => String::new(),
        [one] => one.chars().take(2).collect(),
        [a, b, ..] => a.chars().take(1).chain(b.chars().take(1)).collect(),
    };
    letters.to_uppercase()
}

/// The requests and top-level folders of a collection's tree.
pub fn tree_counts(tree: &[TreeNode]) -> (usize, usize) {
    fn requests(nodes: &[TreeNode]) -> usize {
        nodes
            .iter()
            .map(|n| match n {
                TreeNode::Folder { children, .. } => requests(children),
                TreeNode::File { requests, .. } => requests.len().max(1),
            })
            .sum()
    }
    let folders = tree
        .iter()
        .filter(|n| matches!(n, TreeNode::Folder { .. }))
        .count();
    (requests(tree), folders)
}

impl ShareView {
    /// Opens on the collection's share, giving it a link if it has none and
    /// sending the variables as the notes describe them.
    pub fn new(
        src: ShareSource,
        mode: ShareMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let invite = cx.new(|cx| InputState::new(window, cx).placeholder("name@example.com"));
        let invite_events = cx.subscribe_in(
            &invite,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change if this.invite_error.take().is_some() => cx.notify(),
                InputEvent::PressEnter { .. } => this.invite(window, cx),
                _ => {}
            },
        );
        let mut this = ShareView {
            src,
            mode,
            settings: None,
            vars: Vec::new(),
            error: None,
            invite,
            invite_error: None,
            removing: None,
            copied: None,
            save: None,
            _invite_events: invite_events,
        };
        this.send_variables(this.src.meta.clone(), true, cx);
        this
    }

    /// The notes as this view last saved them.
    pub fn meta(&self) -> &CollectionMeta {
        &self.src.meta
    }

    /// People with access, once known.
    pub fn people(&self) -> Option<usize> {
        self.settings.as_ref().map(|s| s.people.len())
    }

    /// `name` can share its Default value: not a secret, and `.env` has one.
    pub fn offers_share_default(&self, name: &str) -> bool {
        self.vars
            .iter()
            .any(|v| v.name == name && !v.secret && v.default.is_some())
    }

    pub fn shares_default(&self, name: &str) -> bool {
        self.src
            .meta
            .vars
            .get(name)
            .is_some_and(|v| v.share_default)
    }

    /// Sends the variables `meta` describes (scanned off the main thread);
    /// on success `meta` becomes the notes, saved. `list` also refreshes
    /// what the Variables list shows.
    fn send_variables(&mut self, meta: CollectionMeta, list: bool, cx: &mut Context<Self>) {
        let (root, scan_meta) = (self.src.root.clone(), meta.clone());
        let prep = cx.background_spawn(async move {
            let candidates = list.then(|| share_candidates(&root, &scan_meta));
            (candidates, share_payload(&root, &scan_meta))
        });
        cx.spawn(async move |this, cx| {
            let (candidates, payload) = prep.await;
            let Ok(call) = this.update(cx, |this, cx| {
                if let Some(vars) = candidates {
                    this.vars = vars;
                }
                let key = this.src.key.clone();
                let update = ShareUpdate {
                    variables: Some(payload),
                    ..ShareUpdate::default()
                };
                backend_call(cx, &this.src.backend, move |b| b.update_share(&key, update))
            }) else {
                return;
            };
            let result = call.await;
            this.update(cx, |this, cx| {
                if result.is_ok() && meta != this.src.meta {
                    this.src.meta = meta;
                    this.save_meta(cx);
                }
                this.apply(result, cx);
            })
            .ok();
        })
        .detach();
    }

    fn save_meta(&mut self, cx: &mut Context<Self>) {
        let Some(base) = self.src.base.clone() else {
            return;
        };
        let (root, meta) = (self.src.root.clone(), self.src.meta.clone());
        let write = cx.background_spawn(async move { meta.save(&base, &root) });
        self.save = Some(cx.spawn(async move |_, _| {
            if let Err(e) = write.await {
                log::warn!("Couldn't save the collection's notes: {e}");
            }
        }));
    }

    /// A settings answer: shown, or its error at the top (and nothing
    /// else changes).
    fn apply(&mut self, result: Result<ShareSettings, BackendError>, cx: &mut Context<Self>) {
        match result {
            Ok(settings) => {
                self.settings = Some(settings);
                self.error = None;
                cx.emit(ShareEvent::Changed);
            }
            Err(e) => self.error = Some(share_error(&e)),
        }
        cx.notify();
    }

    fn update(&mut self, update: ShareUpdate, cx: &mut Context<Self>) {
        let key = self.src.key.clone();
        let call = backend_call(cx, &self.src.backend, move |b| b.update_share(&key, update));
        cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| this.apply(result, cx)).ok();
        })
        .detach();
    }

    pub fn set_access(&mut self, access: Access, cx: &mut Context<Self>) {
        if self.settings.as_ref().is_some_and(|s| s.access == access) {
            return;
        }
        self.update(
            ShareUpdate {
                access: Some(access),
                ..ShareUpdate::default()
            },
            cx,
        );
    }

    pub fn set_expiry(&mut self, expiry: Expiry, cx: &mut Context<Self>) {
        self.update(
            ShareUpdate {
                expires: Some(expiry),
                ..ShareUpdate::default()
            },
            cx,
        );
    }

    /// `Reset link`: a new token; the old link stops working.
    pub fn reset_link(&mut self, cx: &mut Context<Self>) {
        self.update(
            ShareUpdate {
                reset_link: true,
                ..ShareUpdate::default()
            },
            cx,
        );
    }

    /// Ticks or unticks `Share default` for `name` (only where offered).
    pub fn toggle_share_default(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.offers_share_default(name) {
            return;
        }
        let mut meta = self.src.meta.clone();
        let notes = meta.vars.entry(name.to_string()).or_default();
        notes.share_default = !notes.share_default;
        self.send_variables(meta, false, cx);
    }

    pub fn copy_link(&mut self, cx: &mut Context<Self>) {
        let Some(link) = self.settings.as_ref().and_then(|s| s.link.clone()) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(link));
        self.copied = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            this.update(cx, |this, cx| {
                this.copied = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Invites the typed address, once it looks like one.
    pub fn invite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let email = self.invite.read(cx).value().trim().to_string();
        if !valid_email(&email) {
            self.invite_error = Some("Enter an email address, like name@example.com.".into());
            cx.notify();
            return;
        }
        let key = self.src.key.clone();
        let call = backend_call(cx, &self.src.backend, move |b| b.invite(&key, &email));
        cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(person) => {
                        if let Some(s) = &mut this.settings {
                            s.people.push(person);
                        }
                        this.error = None;
                        this.invite.update(cx, |s, cx| s.set_value("", window, cx));
                        cx.emit(ShareEvent::Changed);
                    }
                    Err(BackendError::Invalid(why)) => this.invite_error = Some(why),
                    Err(e) => this.error = Some(share_error(&e)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Dev hook: invites `emails` one by one, then re-reads the share.
    pub fn dev_invite(&mut self, emails: Vec<String>, cx: &mut Context<Self>) {
        let (backend, key) = (self.src.backend.clone(), self.src.key.clone());
        let call = cx.background_spawn(async move {
            for email in &emails {
                if let Err(e) = backend.invite(&key, email).await {
                    log::warn!("dev invite {email}: {e}");
                }
            }
            backend.share_settings(&key).await
        });
        cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| this.apply(result, cx)).ok();
        })
        .detach();
    }

    #[cfg(test)]
    pub fn type_invite(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.invite
            .update(cx, |s, cx| s.set_value(text, window, cx));
    }

    /// Remove asks first, on the person's row.
    pub fn ask_remove(&mut self, email: &str, cx: &mut Context<Self>) {
        self.removing = Some(email.to_string());
        cx.notify();
    }

    pub fn cancel_remove(&mut self, cx: &mut Context<Self>) {
        self.removing = None;
        cx.notify();
    }

    pub fn confirm_remove(&mut self, cx: &mut Context<Self>) {
        let Some(email) = self.removing.clone() else {
            return;
        };
        let key = self.src.key.clone();
        let sent = email.clone();
        let call = backend_call(cx, &self.src.backend, move |b| b.remove_person(&key, &sent));
        cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                this.removing = None;
                match result {
                    Ok(()) => {
                        if let Some(s) = &mut this.settings {
                            s.people.retain(|p| !p.email.eq_ignore_ascii_case(&email));
                        }
                        this.error = None;
                        cx.emit(ShareEvent::Changed);
                    }
                    Err(e) => this.error = Some(share_error(&e)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// The board's `.sel`: an outlined menu button with a 12px chevron.
fn select_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    height: f32,
    text: f32,
) -> Button {
    let label = label.into();
    Button::new(id)
        .outline()
        .h(px(height))
        .pl(px(10.))
        .pr(px(8.))
        .gap(px(10.))
        .text_size(px(text))
        .child(div().text_color(color("text-primary")).child(label))
        .child(
            glyph(icons::CHEVRON_DOWN)
                .size(px(12.))
                .text_color(color("text-secondary")),
        )
}

/// The board's `.btn`: 32px, 13px text.
fn dialog_button(id: &'static str, label: &'static str) -> Button {
    Button::new(id)
        .h(px(32.))
        .px(px(12.))
        .text_size(px(13.))
        .label(label)
}

fn section(gap: f32) -> Div {
    div().flex().flex_col().gap(px(gap))
}

impl ShareView {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .items_start()
            .gap(px(12.))
            .px(px(24.))
            .pt(px(20.))
            .pb(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(
                        div()
                            .id("share-title")
                            .test_support()
                            .aria_label(format!("Share {}", self.src.key.name))
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Share {}", self.src.key.name)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(relative(1.5))
                            .text_color(color("text-secondary"))
                            .child(SUBTITLE),
                    ),
            )
            .child(
                div()
                    .id("share-close")
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
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-200")).text_color(color("text-primary")))
                    .child(glyph(icons::CROSS).size(px(14.)))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareEvent::Close))),
            )
    }

    fn render_error(&self) -> Option<impl IntoElement + use<>> {
        let e = self.error.clone()?;
        Some(
            div()
                .id("share-error")
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

    fn render_link(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let link = self.settings.as_ref().and_then(|s| s.link.clone());
        let copied = self.copied.is_some();
        let copy_label = if copied { "Copied" } else { "Copy link" };
        let field = div()
            .id("share-link")
            .test_support()
            .aria_label(link.clone().unwrap_or_default())
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap(px(8.))
            .h(px(36.))
            .px(px(10.))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(6.))
            .bg(color("bg-100"))
            .child(
                glyph(icons::LOCK)
                    .size(px(13.))
                    .flex_none()
                    .text_color(color("text-secondary")),
            )
            .child(match &link {
                Some(link) => div()
                    .flex_1()
                    .min_w_0()
                    .text_ellipsis()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .child(link.clone()),
                None => div()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child(if self.error.is_some() {
                        "No link yet"
                    } else {
                        "Creating link…"
                    }),
            });
        let copy = div()
            .id("share-copy")
            .role(Role::Button)
            .test_support()
            .aria_label(copy_label)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(32.))
            .px(px(12.))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(6.))
            .bg(color("bg-100"))
            .text_size(px(13.))
            .when(link.is_none(), |d| d.opacity(0.5))
            .when(link.is_some(), |d| {
                d.cursor_pointer()
                    .hover(|s| s.bg(color("surface-100")))
                    .on_click(cx.listener(|this, _, _, cx| this.copy_link(cx)))
            })
            .child(
                glyph(if copied { icons::CHECK } else { icons::COPY })
                    .size(px(13.))
                    .text_color(color(if copied { "success" } else { "text-primary" })),
            )
            .child(copy_label);
        let today = Local::now().date_naive();
        let (access, expires) = self
            .settings
            .as_ref()
            .map_or((Access::default(), None), |s| (s.access, s.expires));
        let weak = cx.entity().downgrade();
        let access_menu = select_button(
            "share-access",
            match access {
                Access::AnyoneWithLink => "Can view and run",
                Access::InvitedOnly => "Invited people only",
            },
            26.,
            12.,
        )
        .dropdown_menu(move |menu, _, _| {
            let item = |label: &'static str, value: Access| {
                let weak = weak.clone();
                PopupMenuItem::new(label)
                    .checked(access == value)
                    .on_click(move |_, _, cx| {
                        weak.update(cx, |this, cx| this.set_access(value, cx)).ok();
                    })
            };
            menu.item(item("Can view and run", Access::AnyoneWithLink))
                .item(item("Invited people only", Access::InvitedOnly))
        });
        let weak = cx.entity().downgrade();
        let expiry_text = expiry_label(expires, today);
        let current = expiry_text.clone();
        let expiry_menu = select_button("share-expiry", expiry_text, 26., 12.).dropdown_menu(
            move |mut menu, _, _| {
                for (value, label) in EXPIRIES {
                    let weak = weak.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label)
                            .checked(current == label)
                            .on_click(move |_, _, cx| {
                                weak.update(cx, |this, cx| this.set_expiry(value, cx)).ok();
                            }),
                    );
                }
                menu
            },
        );
        let reset = div()
            .id("share-reset")
            .role(Role::Button)
            .test_support()
            .aria_label("Reset link")
            .flex()
            .items_center()
            .h(px(26.))
            .px(px(6.))
            .rounded(px(6.))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .cursor_pointer()
            .hover(|s| s.text_color(color("text-primary")))
            .child("Reset link")
            .on_click(cx.listener(|this, _, _, cx| this.reset_link(cx)));
        section(8.)
            .child(label_text("Share link"))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(field)
                    .child(copy),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .when(access == Access::AnyoneWithLink, |d| {
                        d.child("Anyone with the link")
                    })
                    .when(access == Access::InvitedOnly, |d| d.child("Link works for"))
                    .child(access_menu)
                    .child("· Expires")
                    .child(expiry_menu)
                    .child(div().flex_1())
                    .child(reset),
            )
    }

    fn render_invite(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let role = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .h(px(36.))
            .pl(px(10.))
            .pr(px(9.))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(6.))
            .bg(color("bg-100"))
            .text_size(px(13.))
            .child("Viewer · read only")
            .child(
                glyph(icons::CHEVRON_DOWN)
                    .size(px(12.))
                    .text_color(color("text-secondary")),
            );
        let invite = div()
            .id("share-invite")
            .role(Role::Button)
            .test_support()
            .aria_label("Invite")
            .flex()
            .flex_none()
            .items_center()
            .h(px(36.))
            .px(px(12.))
            .rounded(px(6.))
            .bg(color("primary"))
            .text_color(color("on-primary"))
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(|s| s.bg(color("primary-hover")))
            .child("Invite")
            .on_click(cx.listener(|this, _, window, cx| this.invite(window, cx)));
        let border = if self.invite_error.is_some() {
            color("danger")
        } else {
            color("border-200")
        };
        section(8.)
            .child(label_text("Invite people"))
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.invite)
                                .aria_label("Invite by email")
                                .min_h(px(36.))
                                .text_size(px(13.))
                                .border_color(border),
                        ),
                    )
                    .child(role)
                    .child(invite),
            )
            .children(self.invite_error.clone().map(|e| {
                div()
                    .id("share-invite-error")
                    .role(Role::Alert)
                    .test_support()
                    .aria_label(e.clone())
                    .text_size(px(12.))
                    .text_color(color("danger"))
                    .child(e)
            }))
    }

    fn render_people(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let people = self
            .settings
            .as_ref()
            .map(|s| s.people.clone())
            .unwrap_or_default();
        let me = self.src.backend.account();
        let rows = people.iter().enumerate().map(|(ix, p)| {
            let you = me
                .as_ref()
                .is_some_and(|a| a.email.eq_ignore_ascii_case(&p.email));
            self.render_person(ix, p, you, me.as_ref().and_then(|a| a.org.clone()), cx)
        });
        section(2.)
            .child(label_text(format!("People with access · {}", people.len())).pb(px(4.)))
            .children(rows)
    }

    fn render_person(
        &self,
        ix: usize,
        p: &Person,
        you: bool,
        org: Option<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let owner = p.role == PersonRole::Owner;
        let (name, detail) = if you {
            (
                "You".to_string(),
                org.clone().unwrap_or_else(|| p.email.clone()),
            )
        } else {
            (p.name.clone(), p.email.clone())
        };
        let tone = ["accent-blue", "accent-purple", "success", "warning"][ix % 4];
        let avatar = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(28.))
            .rounded_full()
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(color("on-primary"))
            .map(|d| {
                if you {
                    d.bg(linear_gradient(
                        135.,
                        linear_color_stop(color("text-primary"), 0.),
                        linear_color_stop(color("text-secondary").opacity(0.8), 1.),
                    ))
                } else {
                    d.bg(color(tone))
                }
            })
            .child(initials(if you {
                org.as_deref().unwrap_or(&p.name)
            } else {
                &p.name
            }));
        let confirming = self
            .removing
            .as_deref()
            .is_some_and(|e| e.eq_ignore_ascii_case(&p.email));
        let control = if owner {
            div()
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child("Owner")
                .into_any_element()
        } else if confirming {
            let first = p
                .name
                .split_whitespace()
                .next()
                .unwrap_or(&p.name)
                .to_string();
            div()
                .id("share-remove-confirm")
                .role(Role::Group)
                .test_support()
                .aria_label(format!("Remove {first}?"))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child(format!("Remove {first}?"))
                .child(
                    div()
                        .id("share-remove-cancel")
                        .role(Role::Button)
                        .test_support()
                        .aria_label("Cancel")
                        .px(px(6.))
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|s| s.text_color(color("text-primary")))
                        .child("Cancel")
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_remove(cx))),
                )
                .child(
                    div()
                        .id("share-remove-ok")
                        .role(Role::Button)
                        .test_support()
                        .aria_label("Remove")
                        .px(px(10.))
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(color("danger-border"))
                        .bg(color("danger-subtle"))
                        .text_color(color("danger"))
                        .cursor_pointer()
                        .child("Remove")
                        .on_click(cx.listener(|this, _, _, cx| this.confirm_remove(cx))),
                )
                .into_any_element()
        } else {
            let weak = cx.entity().downgrade();
            let email = p.email.clone();
            let role = match p.role {
                PersonRole::Editor => "Editor",
                _ => "Viewer",
            };
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .when(p.invited, |d| {
                    d.child(
                        div()
                            .px(px(8.))
                            .py(px(1.))
                            .rounded_full()
                            .border_1()
                            .border_color(color("warning-border"))
                            .text_size(px(11.))
                            .text_color(color("warning"))
                            .child("Invited · not opened"),
                    )
                })
                .child(
                    select_button(("share-role", ix), role, 28., 12.).dropdown_menu(
                        move |menu, _, _| {
                            let (weak, email) = (weak.clone(), email.clone());
                            menu.item(PopupMenuItem::new(role).checked(true))
                                .separator()
                                .item(PopupMenuItem::new("Remove").on_click(move |_, _, cx| {
                                    weak.update(cx, |this, cx| this.ask_remove(&email, cx)).ok();
                                }))
                        },
                    ),
                )
                .into_any_element()
        };
        div()
            .id(("share-person", ix))
            .role(Role::Row)
            .test_support()
            .aria_label(p.email.clone())
            .flex()
            .items_center()
            .gap(px(10.))
            .h(px(40.))
            .child(avatar)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_size(px(13.)).text_ellipsis().child(name))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .text_ellipsis()
                            .child(detail),
                    ),
            )
            .child(control)
    }

    fn render_what_they_get(&self) -> impl IntoElement + use<> {
        let check = || {
            glyph(icons::CHECK)
                .size(px(13.))
                .flex_none()
                .text_color(color("success"))
        };
        let chips = self.vars.iter().map(|v| {
            div()
                .px(px(6.))
                .py(px(1.))
                .border_1()
                .border_color(color("border-200"))
                .rounded(px(4.))
                .bg(color("surface-raised"))
                .font_family(mono_font())
                .text_size(px(11.5))
                .text_color(color("warning"))
                .child(v.name.clone())
        });
        div()
            .id("share-what-they-get")
            .role(Role::Group)
            .test_support()
            .flex()
            .flex_col()
            .gap(px(10.))
            .px(px(16.))
            .py(px(14.))
            .border_1()
            .border_color(color("border-100"))
            .rounded(px(8.))
            .bg(color("surface-raised"))
            .text_size(px(13.))
            .child(label_text("What they get").text_color(color("text-primary")))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(check())
                    .child(requests_text(self.src.requests, self.src.folders)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(check())
                    .child(if self.vars.is_empty() {
                        "No variables to fill in"
                    } else {
                        "Variables they fill in themselves:"
                    })
                    .children(chips),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_color(color("text-secondary"))
                    .child(
                        glyph(icons::CROSS)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("danger")),
                    )
                    .child(NEVER_SHARED),
            )
    }

    fn render_variables(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chip = |text: &'static str| {
            neutral_pill()
                .text_size(px(11.))
                .px(px(6.))
                .py_0()
                .child(text)
        };
        let rows = self.vars.iter().map(|v| {
            let offers = !v.secret && v.default.is_some();
            let on = offers && self.shares_default(&v.name);
            let name = v.name.clone();
            let value = v.default.clone().filter(|_| offers).map(|d| {
                div()
                    .max_w(px(220.))
                    .min_w_0()
                    .text_ellipsis()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .text_color(color(if on { "text-secondary" } else { "text-gutter" }))
                    .child(format!("= {d}"))
            });
            let toggle = offers.then(|| {
                div()
                    .id(SharedString::from(format!("share-default-{name}")))
                    .role(Role::Switch)
                    .test_support()
                    .aria_label(format!("Share default for {name}"))
                    .cursor_pointer()
                    .child(switch(on, true))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.toggle_share_default(&name, cx)),
                    )
            });
            div()
                .id(SharedString::from(format!("share-var-{}", v.name)))
                .role(Role::Row)
                .test_support()
                .aria_label(v.name.clone())
                .flex()
                .items_center()
                .gap(px(12.))
                .min_h(px(36.))
                .py(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .font_family(mono_font())
                                        .text_size(px(12.))
                                        .child(v.name.clone()),
                                )
                                .when(v.required, |d| d.child(chip("required")))
                                .when(v.secret, |d| d.child(chip("secret"))),
                        )
                        .when(!v.description.is_empty(), |d| {
                            d.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(color("text-secondary"))
                                    .text_ellipsis()
                                    .child(v.description.clone()),
                            )
                        }),
                )
                .children(value)
                .child(div().w(px(28.)).flex_none().children(toggle))
        });
        let secret = self.vars.iter().find(|v| v.secret).map(|v| v.name.clone());
        let note = match secret {
            Some(name) => format!("Secrets like {name} are never included, in any environment"),
            None => "Secrets are never included, in any environment".into(),
        };
        section(4.)
            .child(
                div()
                    .flex()
                    .items_center()
                    .pb(px(4.))
                    .child(label_text("Variables").flex_1())
                    .child(label_text("Share default")),
            )
            .children(rows)
            .when(self.vars.is_empty(), |d| {
                d.child(
                    div()
                        .h(px(36.))
                        .flex()
                        .items_center()
                        .text_size(px(13.))
                        .text_color(color("text-tertiary"))
                        .child("No variables in this collection yet."),
                )
            })
            .child(
                div()
                    .id("share-secrets-note")
                    .test_support()
                    .aria_label(note.clone())
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(36.))
                    .text_size(px(13.))
                    .text_color(color("text-tertiary"))
                    .child(glyph(icons::LOCK).size(px(13.)).flex_none())
                    .child(note),
            )
    }

    fn render_body(&self, cx: &mut Context<Self>) -> Div {
        section(18.)
            .children(self.render_error())
            .child(self.render_link(cx))
            .child(self.render_invite(cx))
            .child(self.render_people(cx))
            .child(self.render_what_they_get())
            .child(self.render_variables(cx))
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
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
            .child(
                div()
                    .flex_1()
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(FOOTER_NOTE),
            )
            .child(
                dialog_button("share-cancel", "Cancel")
                    .outline()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareEvent::Close))),
            )
            .child(
                dialog_button("share-done", "Done")
                    .primary()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareEvent::Close))),
            )
    }
}

impl Render for ShareView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.mode {
            ShareMode::Page => div()
                .id("share-page")
                .test_support()
                .child(self.render_body(cx))
                .into_any_element(),
            ShareMode::Dialog => {
                // The board's 820px fits a 900px window; a shorter one
                // scrolls the body.
                let room = window.viewport_size().height - px(DIALOG_TOP * 2. + 140.);
                div()
                    .id("share-dialog")
                    .role(Role::Dialog)
                    .test_support()
                    .aria_label(format!("Share {}", self.src.key.name))
                    .flex()
                    .flex_col()
                    .child(self.render_header(cx))
                    .child(
                        div()
                            .id("share-body")
                            .max_h(room.max(px(240.)))
                            .overflow_y_scroll()
                            .px(px(24.))
                            .pb(px(20.))
                            .child(self.render_body(cx)),
                    )
                    .child(self.render_footer(cx))
                    .into_any_element()
            }
        }
    }
}

impl Workspace {
    /// What the Share dialog and the Sharing page need of the open
    /// collection.
    pub(super) fn share_source(&self) -> Option<ShareSource> {
        let c = self.state.collection.as_ref()?;
        let (requests, folders) = tree_counts(&c.tree);
        Some(ShareSource {
            backend: self.backend.clone(),
            key: self.collection_key()?,
            root: c.root.clone(),
            base: self.store.base().map(std::path::Path::to_path_buf),
            meta: self.meta.clone(),
            requests,
            folders,
        })
    }

    /// Opens the Share dialog (the footer's Share, the title bar's
    /// `Shared · N`), unless sharing is blocked.
    pub(super) fn open_share(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.share_blocked().is_some() {
            return;
        }
        let Some(src) = self.share_source() else {
            return;
        };
        let view = cx.new(|cx| ShareView::new(src, ShareMode::Dialog, window, cx));
        self._share_events = Some(cx.subscribe_in(&view, window, Self::on_share_event));
        let content = view.clone();
        window.open_dialog(cx, move |d, _, _| {
            d.entrance(DialogEntrance::None)
                .w(px(DIALOG_W))
                .margin_top(px(DIALOG_TOP))
                .p_0()
                .close_button(false)
                .on_ok(|_, _, _| false)
                .child(content.clone())
        });
        self.share_view = Some(view);
    }

    fn on_share_event(
        &mut self,
        view: &Entity<ShareView>,
        event: &ShareEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ShareEvent::Changed => {
                self.meta = view.read(cx).meta().clone();
                self.refresh_share(window, cx);
            }
            ShareEvent::Close => {
                window.close_dialog(cx);
                self.share_view = None;
                self._share_events = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendError, expiry_label, initials, requests_text, share_error, valid_email};
    use chrono::NaiveDate;

    #[test]
    fn emails_need_the_x_at_y_dot_z_shape() {
        for ok in ["maria@globex.example", "a@b.co", "jon.r+x@initech.example"] {
            assert!(valid_email(ok), "{ok}");
        }
        for bad in [
            "",
            "maria",
            "maria@",
            "@globex.example",
            "maria@globex",
            "maria@.example",
            "maria@globex.",
            "ma ria@globex.example",
            "a@b@c.d",
        ] {
            assert!(!valid_email(bad), "{bad:?}");
        }
    }

    #[test]
    fn copy_reads_like_the_board() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let in_days = |n| today.checked_add_days(chrono::Days::new(n));
        assert_eq!(expiry_label(in_days(30), today), "in 30 days");
        assert_eq!(expiry_label(in_days(7), today), "in 7 days");
        assert_eq!(expiry_label(in_days(1), today), "tomorrow");
        assert_eq!(expiry_label(None, today), "Never");
        assert_eq!(
            expiry_label(today.checked_sub_days(chrono::Days::new(1)), today),
            "Expired"
        );
        assert_eq!(
            requests_text(18, 4),
            "18 requests in 4 folders, with every check you wrote"
        );
        assert_eq!(requests_text(1, 0), "1 request, with every check you wrote");
        assert_eq!(initials("Jon Rivera"), "JR");
        assert_eq!(initials("Acme"), "AC");
        assert_eq!(initials("lena kim"), "LK");
        assert_eq!(
            share_error(&BackendError::Offline),
            "You're offline. Connect to change sharing; nothing changed."
        );
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, test_backend, test_secrets};
    use super::{Access, ShareMode, ShareView};
    use crate::model::backend::{Backend, SignInMethod};
    use crate::model::secrets::service_name;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    const REQUEST: &str = "request:\n  url: ${BASE_URL}/rates?account=${ACCOUNT_ID}\n  headers:\n    Authorization: Bearer ${API_TOKEN}\n";
    const DOTENV: &str = "BASE_URL=https://api.example.com/shipping\nACCOUNT_ID=acct-4242\nAPI_TOKEN=sk-dotenv-secret\nSECRET_SIGNING=sk-signing\n";
    const KEYCHAIN_VALUE: &str = "sk-test-value";

    fn collection() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("rates")).unwrap();
        std::fs::write(dir.path().join("rates/get-rates.yaml"), REQUEST).unwrap();
        std::fs::write(
            dir.path().join("sign.yaml"),
            "request:\n  url: ${BASE_URL}/sign\n  headers:\n    X-Sig: ${SECRET_SIGNING}\n",
        )
        .unwrap();
        std::fs::write(dir.path().join(".env"), DOTENV).unwrap();
        dir
    }

    fn signed_in() -> Arc<dyn Backend> {
        let backend = test_backend();
        futures::executor::block_on(backend.sign_in(SignInMethod::GitHub)).unwrap();
        backend
    }

    /// The workspace on `root`, `API_TOKEN` marked secret and its Keychain
    /// item holding [`KEYCHAIN_VALUE`].
    fn open(
        cx: &mut TestAppContext,
        root: &Path,
        backend: Arc<dyn Backend>,
    ) -> (AnyWindowHandle, Entity<Workspace>) {
        let root = root.to_path_buf();
        let secrets = test_secrets();
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
            let secrets = secrets.clone();
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    Workspace::new(Some(root), StateStore::none(), secrets, backend, window, cx)
                })
            })
            .unwrap()
        });
        cx.run_until_parked();
        ws.update(cx, |ws, _| {
            ws.meta.set_secret("API_TOKEN", true);
            let name = ws.state.collection.as_ref().unwrap().name.clone();
            let service = service_name(&name, "Default", &ws.meta.id);
            secrets.set(&service, "API_TOKEN", KEYCHAIN_VALUE).unwrap();
        });
        (window, ws)
    }

    fn open_share(
        cx: &mut TestAppContext,
        window: AnyWindowHandle,
        ws: &Entity<Workspace>,
    ) -> Entity<ShareView> {
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_share(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| ws.share_view.clone())
            .expect("the dialog opened")
    }

    fn labelled(cx: &mut TestAppContext, window: AnyWindowHandle, id: &str, label: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            match window.try_find(id.to_string()) {
                Some(e) if e.label() == Some(label) => true,
                Some(_) => window
                    .within(id.to_string())
                    .try_find_by_label(label)
                    .is_some(),
                None => false,
            }
        })
        .unwrap()
    }

    fn found(cx: &mut TestAppContext, window: AnyWindowHandle, id: &str) -> bool {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.try_find(id.to_string()).is_some()
        })
        .unwrap()
    }

    fn update_shares(backend: &Arc<dyn Backend>) -> Vec<String> {
        backend
            .dev()
            .unwrap()
            .recorded()
            .into_iter()
            .filter(|c| c.starts_with("update_share("))
            .collect()
    }

    #[gpui_kit::test]
    fn share_creates_link_and_copy_puts_it_on_clipboard(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let link = view
            .read_with(cx, |v, _| v.settings.as_ref().and_then(|s| s.link.clone()))
            .expect("opening Share gives the collection a link");
        assert!(link.starts_with("https://studio.example.com/s/"), "{link}");
        assert!(labelled(cx, window, "share-link", &link));
        let name = ws.read_with(cx, |ws, _| {
            ws.state.collection.as_ref().unwrap().name.clone()
        });
        assert!(labelled(
            cx,
            window,
            "share-title",
            &format!("Share {name}")
        ));
        assert!(labelled(cx, window, "share-copy", "Copy link"));
        cx.update_window(window, |_, window, cx| window.click("share-copy", cx))
            .unwrap();
        assert_eq!(
            cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
            Some(link.as_str())
        );
        assert!(labelled(cx, window, "share-copy", "Copied"));
        cx.executor().advance_clock(Duration::from_millis(1600));
        cx.run_until_parked();
        assert!(labelled(cx, window, "share-copy", "Copy link"));
        // The title bar's pill: the owner alone.
        assert!(labelled(cx, window, "title-shared", "Shared · 1"));
    }

    #[gpui_kit::test]
    fn invite_and_remove_person(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let invites = |b: &Arc<dyn Backend>| {
            b.dev()
                .unwrap()
                .recorded()
                .iter()
                .filter(|c| c.starts_with("invite("))
                .count()
        };
        // Not an address: said under the field, nothing sent.
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.type_invite("maria@globex", window, cx);
                v.invite(window, cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
        assert!(view.read_with(cx, |v, _| v.invite_error.is_some()));
        assert!(found(cx, window, "share-invite-error"));
        assert_eq!(invites(&backend), 0);
        for email in ["maria@globex.example", "jon@initech.example"] {
            cx.update_window(window, |_, window, cx| {
                view.update(cx, |v, cx| {
                    v.type_invite(email, window, cx);
                    v.invite(window, cx);
                })
            })
            .unwrap();
            cx.run_until_parked();
        }
        assert_eq!(view.read_with(cx, |v, _| v.people()), Some(3));
        assert!(labelled(cx, window, "share-dialog", "maria@globex.example"));
        assert!(labelled(cx, window, "title-shared", "Shared · 3"));
        // Remove asks on the row first.
        view.update(cx, |v, cx| v.ask_remove("maria@globex.example", cx));
        assert!(labelled(
            cx,
            window,
            "share-remove-confirm",
            "Remove Maria?"
        ));
        view.update(cx, |v, cx| v.cancel_remove(cx));
        assert!(!found(cx, window, "share-remove-confirm"));
        assert_eq!(view.read_with(cx, |v, _| v.people()), Some(3));
        view.update(cx, |v, cx| v.ask_remove("maria@globex.example", cx));
        cx.update_window(window, |_, window, cx| window.click("share-remove-ok", cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |v, _| v.people()), Some(2));
        assert!(!labelled(
            cx,
            window,
            "share-dialog",
            "maria@globex.example"
        ));
        let key = ws.read_with(cx, |ws, _| ws.collection_key().unwrap());
        let on_server = futures::executor::block_on(backend.share_settings(&key)).unwrap();
        let emails: Vec<_> = on_server.people.iter().map(|p| p.email.as_str()).collect();
        assert_eq!(emails.len(), 2);
        assert!(emails.contains(&"jon@initech.example"));
        assert!(labelled(cx, window, "title-shared", "Shared · 2"));
    }

    #[gpui_kit::test]
    fn secret_vars_never_offer_share_default(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        view.read_with(cx, |v, _| {
            assert!(v.offers_share_default("BASE_URL"));
            assert!(v.offers_share_default("ACCOUNT_ID"));
            assert!(!v.offers_share_default("API_TOKEN"), "marked secret");
            assert!(!v.offers_share_default("SECRET_SIGNING"), "SECRET_ prefix");
        });
        assert!(found(cx, window, "share-default-BASE_URL"));
        assert!(found(cx, window, "share-var-API_TOKEN"), "listed");
        assert!(!found(cx, window, "share-default-API_TOKEN"));
        assert!(!found(cx, window, "share-default-SECRET_SIGNING"));
        assert!(labelled(
            cx,
            window,
            "share-dialog",
            "Secrets like API_TOKEN are never included, in any environment"
        ));
        // Asking anyway changes nothing.
        let before = update_shares(&backend).len();
        view.update(cx, |v, cx| v.toggle_share_default("API_TOKEN", cx));
        cx.run_until_parked();
        assert_eq!(update_shares(&backend).len(), before);
        assert!(!view.read_with(cx, |v, _| v.shares_default("API_TOKEN")));
    }

    #[gpui_kit::test]
    fn share_payload_has_no_values_unless_share_default(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        // Even a secret ticked behind the dialog's back stays out.
        ws.update(cx, |ws, _| {
            ws.meta
                .vars
                .entry("API_TOKEN".into())
                .or_default()
                .share_default = true;
            ws.meta
                .vars
                .entry("SECRET_SIGNING".into())
                .or_default()
                .share_default = true;
        });
        let view = open_share(cx, window, &ws);
        let calls = update_shares(&backend);
        assert_eq!(calls.len(), 1, "{calls:?}");
        for value in [
            "https://api.example.com/shipping",
            "acct-4242",
            "sk-dotenv-secret",
            "sk-signing",
            KEYCHAIN_VALUE,
        ] {
            assert!(!calls[0].contains(value), "{value} in {}", calls[0]);
        }
        assert!(calls[0].contains("BASE_URL") && calls[0].contains("API_TOKEN"));
        // Ticking BASE_URL sends its Default value, and only that.
        cx.update_window(window, |_, window, cx| {
            window.click("share-default-BASE_URL", cx)
        })
        .unwrap();
        cx.run_until_parked();
        assert!(view.read_with(cx, |v, _| v.shares_default("BASE_URL")));
        assert!(ws.read_with(cx, |ws, _| ws.meta.vars["BASE_URL"].share_default));
        let calls = update_shares(&backend);
        assert_eq!(calls.len(), 2);
        let last = calls.last().unwrap();
        assert!(last.contains("https://api.example.com/shipping"), "{last}");
        for value in [
            "acct-4242",
            "sk-dotenv-secret",
            "sk-signing",
            KEYCHAIN_VALUE,
        ] {
            assert!(!last.contains(value), "{value} in {last}");
        }
        // Nothing the backend ever got holds a secret.
        for call in backend.dev().unwrap().recorded() {
            for value in ["sk-dotenv-secret", "sk-signing", KEYCHAIN_VALUE] {
                assert!(!call.contains(value), "{value} in {call}");
            }
        }
        // Unticking takes it back out.
        view.update(cx, |v, cx| v.toggle_share_default("BASE_URL", cx));
        cx.run_until_parked();
        let calls = update_shares(&backend);
        assert!(
            !calls
                .last()
                .unwrap()
                .contains("https://api.example.com/shipping")
        );
    }

    #[gpui_kit::test]
    fn offline_share_shows_inline_error(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let link = view.read_with(cx, |v, _| v.settings.clone().unwrap());
        backend.dev().unwrap().set_offline(true);
        view.update(cx, |v, cx| {
            v.set_access(Access::InvitedOnly, cx);
        });
        cx.run_until_parked();
        let error = view
            .read_with(cx, |v, _| v.error.clone())
            .expect("an error");
        assert!(error.starts_with("You're offline."), "{error}");
        assert!(labelled(cx, window, "share-error", &error));
        assert_eq!(
            view.read_with(cx, |v, _| v.settings.clone().unwrap()),
            link,
            "nothing changed here"
        );
        // A Share default tick fails too and stays off.
        view.update(cx, |v, cx| v.toggle_share_default("BASE_URL", cx));
        cx.run_until_parked();
        assert!(!view.read_with(cx, |v, _| v.shares_default("BASE_URL")));
        assert!(!ws.read_with(cx, |ws, _| {
            ws.meta
                .vars
                .get("BASE_URL")
                .is_some_and(|v| v.share_default)
        }));
        // Back online, the next change clears it.
        backend.dev().unwrap().set_offline(false);
        view.update(cx, |v, cx| v.set_access(Access::InvitedOnly, cx));
        cx.run_until_parked();
        view.read_with(cx, |v, _| {
            assert_eq!(v.error, None);
            assert_eq!(v.settings.as_ref().unwrap().access, Access::InvitedOnly);
        });
        // Done closes the dialog.
        cx.update_window(window, |_, window, cx| window.click("share-done", cx))
            .unwrap();
        cx.run_until_parked();
        assert!(ws.read_with(cx, |ws, _| ws.share_view.is_none()));
        cx.update_window(window, |_, window, cx| {
            assert!(!window.has_active_dialog(cx));
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn share_opened_offline_says_so_and_makes_no_link(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        backend.dev().unwrap().set_offline(true);
        // Past `share_blocked`, as when Offline flips while it opens.
        let src = ws.read_with(cx, |ws, _| ws.share_source().unwrap());
        let view = cx
            .update_window(window, |_, window, cx| {
                cx.new(|cx| ShareView::new(src, ShareMode::Dialog, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        view.read_with(cx, |v, _| {
            assert!(v.error.as_deref().unwrap().starts_with("You're offline."));
            assert_eq!(v.settings, None);
        });
        backend.dev().unwrap().set_offline(false);
        let key = ws.read_with(cx, |ws, _| ws.collection_key().unwrap());
        let s = futures::executor::block_on(backend.share_settings(&key)).unwrap();
        assert_eq!(s.link, None);
    }
}
