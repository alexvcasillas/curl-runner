//! The Share dialog (board: Share), also the collection settings' Sharing
//! page: the share link with `Copy link`, who the link works for and when
//! it expires, invites, the people with access, what they get and, per
//! variable, whether its Default value is shared.
//!
//! It is a view onto the collection's [`ShareSync`], which makes the calls
//! (one at a time) and keeps them going after the dialog closes. A failed
//! call (offline, signed out) shows its error at the top.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{label_text, neutral_pill, switch};
use super::share_sync::{ShareSync, ShareSyncEvent};
use super::workspace::Workspace;
use crate::model::backend::{
    Access, Account, Expiry, Person, Role as PersonRole, ShareSettings, SharedVar,
};
use chrono::{Local, NaiveDate};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::time::Duration;

/// The board's dialog.
const DIALOG_W: f32 = 600.;
const DIALOG_TOP: f32 = 40.;
/// How long `Copy link` reads `Copied`.
const COPIED_FOR: Duration = Duration::from_millis(1500);
/// The board's copy, verbatim.
pub const SUBTITLE: &str = "People you share with can open and run every request with their own credentials. They can't change anything.";
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
    /// Close or Done (changes apply as they are made).
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareMode {
    /// The dialog: its header, close button and footer.
    Dialog,
    /// The Sharing settings page: the body only, under the page's header.
    Page,
}

/// The share as this frame draws it, read from the [`ShareSync`].
#[derive(Default)]
struct Shown {
    name: String,
    settings: Option<ShareSettings>,
    vars: Vec<SharedVar>,
    error: Option<String>,
    invite_error: Option<String>,
    account: Option<Account>,
    requests: usize,
    folders: usize,
    access: Access,
    expiry: Option<Expiry>,
    expiry_text: String,
    /// Names whose `Share default` switch is on (ticks in flight too).
    defaults: Vec<String>,
}

/// A view onto the collection's [`ShareSync`]: the dialog or the page.
/// Closing it leaves the share's calls running.
pub struct ShareView {
    pub(super) sync: Entity<ShareSync>,
    mode: ShareMode,
    shown: Shown,
    invite: Entity<InputState>,
    /// The person whose removal waits for a confirm, by email.
    pub(super) removing: Option<String>,
    /// Set while `Copy link` reads `Copied`.
    copied: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
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

impl ShareView {
    /// A view onto `sync`; opening it gives the share a link if it has none
    /// and sends the variables as the notes describe them.
    pub fn new(
        sync: Entity<ShareSync>,
        mode: ShareMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let invite = cx.new(|cx| InputState::new(window, cx).placeholder("name@example.com"));
        let invite_events = cx.subscribe_in(
            &invite,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    if this.sync.read(cx).invite_error.is_some() {
                        this.sync.update(cx, |s, cx| s.set_invite_error(None, cx));
                    }
                }
                InputEvent::PressEnter { .. } => this.invite(window, cx),
                _ => {}
            },
        );
        // An address that got in leaves the field (one that didn't stays).
        let invited = cx.subscribe_in(
            &sync,
            window,
            |this, _, event: &ShareSyncEvent, window, cx| {
                if let ShareSyncEvent::Invited(email) = event
                    && this.invite.read(cx).value().trim() == email
                {
                    this.invite.update(cx, |s, cx| s.set_value("", window, cx));
                }
            },
        );
        let redraw = cx.observe(&sync, |_, _, cx| cx.notify());
        sync.update(cx, |s, cx| s.open(cx));
        ShareView {
            sync,
            mode,
            shown: Shown::default(),
            invite,
            removing: None,
            copied: None,
            _subscriptions: vec![invite_events, invited, redraw],
        }
    }

    pub fn settings(&self, cx: &App) -> Option<ShareSettings> {
        self.sync.read(cx).settings.clone()
    }

    #[cfg(test)]
    pub fn error(&self, cx: &App) -> Option<String> {
        self.sync.read(cx).error.clone()
    }

    #[cfg(test)]
    pub fn invite_error(&self, cx: &App) -> Option<String> {
        self.sync.read(cx).invite_error.clone()
    }

    #[cfg(test)]
    pub fn people(&self, cx: &App) -> Option<usize> {
        self.sync.read(cx).people()
    }

    #[cfg(test)]
    pub fn offers_share_default(&self, name: &str, cx: &App) -> bool {
        self.sync.read(cx).offers_share_default(name, cx)
    }

    #[cfg(test)]
    pub fn shares_default(&self, name: &str, cx: &App) -> bool {
        self.sync.read(cx).shares_default(name, cx)
    }

    /// Reads the share for this frame.
    fn read_shown(&mut self, cx: &App) {
        let s = self.sync.read(cx);
        let today = Local::now().date_naive();
        let expiry = s.expiry();
        let expiry_text = match expiry.and_then(|e| EXPIRIES.iter().find(|(v, _)| *v == e)) {
            Some((_, label)) => (*label).to_string(),
            None => expiry_label(s.settings.as_ref().and_then(|s| s.expires), today),
        };
        self.shown = Shown {
            name: s.key().name.clone(),
            settings: s.settings.clone(),
            defaults: s
                .vars
                .iter()
                .filter(|v| s.shares_default(&v.name, cx))
                .map(|v| v.name.clone())
                .collect(),
            vars: s.vars.clone(),
            error: s.error.clone(),
            invite_error: s.invite_error.clone(),
            account: s.account(),
            requests: s.requests,
            folders: s.folders,
            access: s.access(),
            expiry,
            expiry_text,
        };
    }

    fn shares_shown(&self, name: &str) -> bool {
        self.shown.defaults.iter().any(|n| n == name)
    }

    pub fn set_access(&mut self, access: Access, cx: &mut Context<Self>) {
        self.sync.update(cx, |s, cx| s.set_access(access, cx));
    }

    pub fn set_expiry(&mut self, expiry: Expiry, cx: &mut Context<Self>) {
        self.sync.update(cx, |s, cx| s.set_expiry(expiry, cx));
    }

    /// `Reset link`: a new token; the old link stops working.
    pub fn reset_link(&mut self, cx: &mut Context<Self>) {
        self.sync.update(cx, |s, cx| s.reset_link(cx));
    }

    pub fn toggle_share_default(&mut self, name: &str, cx: &mut Context<Self>) {
        self.sync
            .update(cx, |s, cx| s.toggle_share_default(name, cx));
    }

    pub fn copy_link(&mut self, cx: &mut Context<Self>) {
        let Some(link) = self.settings(cx).and_then(|s| s.link) else {
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

    /// Invites the typed address, once it looks like one; the field clears
    /// when the server takes it.
    pub fn invite(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let email = self.invite.read(cx).value().trim().to_string();
        if !valid_email(&email) {
            self.sync.update(cx, |s, cx| {
                s.set_invite_error(
                    Some("Enter an email address, like name@example.com.".into()),
                    cx,
                )
            });
            return;
        }
        self.sync.update(cx, |s, cx| s.invite(email, cx));
    }

    /// Dev hook: invites `emails` in turn.
    pub fn dev_invite(&mut self, emails: Vec<String>, cx: &mut Context<Self>) {
        self.sync.update(cx, |s, cx| {
            for email in emails {
                s.invite(email, cx);
            }
        });
    }

    #[cfg(test)]
    pub fn type_invite(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.invite
            .update(cx, |s, cx| s.set_value(text, window, cx));
    }

    #[cfg(test)]
    pub fn invite_text(&self, cx: &App) -> String {
        self.invite.read(cx).value().to_string()
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
        if let Some(email) = self.removing.take() {
            self.sync.update(cx, |s, cx| s.remove(email, cx));
        }
        cx.notify();
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
                            .aria_label(format!("Share {}", self.shown.name))
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Share {}", self.shown.name)),
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
        let e = self.shown.error.clone()?;
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
        let link = self.shown.settings.as_ref().and_then(|s| s.link.clone());
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
                    .child(if self.shown.error.is_some() {
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
        let access = self.shown.access;
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
        let expiry_text = self.shown.expiry_text.clone();
        let current = self.shown.expiry;
        let expiry_menu = select_button("share-expiry", expiry_text, 26., 12.).dropdown_menu(
            move |mut menu, _, _| {
                for (value, label) in EXPIRIES {
                    let weak = weak.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label)
                            .checked(current == Some(value))
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
        let border = if self.shown.invite_error.is_some() {
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
            .children(self.shown.invite_error.clone().map(|e| {
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
            .shown
            .settings
            .as_ref()
            .map(|s| s.people.clone())
            .unwrap_or_default();
        let me = self.shown.account.clone();
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
        let chips = self.shown.vars.iter().map(|v| {
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
                    .child(requests_text(self.shown.requests, self.shown.folders)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(check())
                    .child(if self.shown.vars.is_empty() {
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
        let rows = self.shown.vars.iter().map(|v| {
            let offers = !v.secret && v.default.is_some();
            let on = offers && self.shares_shown(&v.name);
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
        let secret = self
            .shown
            .vars
            .iter()
            .find(|v| v.secret)
            .map(|v| v.name.clone());
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
            .when(self.shown.vars.is_empty(), |d| {
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
                dialog_button("share-done", "Done")
                    .primary()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareEvent::Close))),
            )
    }
}

impl Render for ShareView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.read_shown(cx);
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
                    .aria_label(format!("Share {}", self.shown.name))
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
    /// Opens the Share dialog (the footer's Share, the title bar's
    /// `Shared · N`), unless sharing is blocked.
    pub(super) fn open_share(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.share_blocked(cx).is_some() {
            return;
        }
        let Some(sync) = self.share_sync(window, cx) else {
            return;
        };
        let view = cx.new(|cx| ShareView::new(sync, ShareMode::Dialog, window, cx));
        self._share_events = Some(cx.subscribe_in(
            &view,
            window,
            |this, _, event: &ShareEvent, window, cx| match event {
                ShareEvent::Close => {
                    window.close_dialog(cx);
                    this.share_view = None;
                    this._share_events = None;
                }
            },
        ));
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
        // The dialog holds the view: however it closes (Done, ✕, Esc), the
        // view goes with it, and the share's calls carry on.
        self.share_view = Some(view.downgrade());
    }

    /// The open Share dialog's view.
    pub(super) fn share_view(&self) -> Option<Entity<ShareView>> {
        self.share_view.as_ref().and_then(WeakEntity::upgrade)
    }
}

#[cfg(test)]
mod tests {
    use super::{expiry_label, initials, requests_text, valid_email};
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
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::state::StateStore;
    use super::super::workspace::{Workspace, edit_notes, test_backend, test_secrets};
    use super::{Access, Expiry, ShareMode, ShareView};
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
        ws.update(cx, |ws, cx| {
            edit_notes(&ws.notes, cx, |meta| meta.set_secret("API_TOKEN", true));
            let name = ws.state.collection.as_ref().unwrap().name.clone();
            let service = service_name(&name, "Default", &ws.meta(cx).id);
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
        ws.read_with(cx, |ws, _| ws.share_view())
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
            .read_with(cx, |v, cx| v.settings(cx).and_then(|s| s.link.clone()))
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
        assert!(view.read_with(cx, |v, cx| v.invite_error(cx).is_some()));
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
        assert_eq!(view.read_with(cx, |v, cx| v.people(cx)), Some(3));
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
        assert_eq!(view.read_with(cx, |v, cx| v.people(cx)), Some(3));
        view.update(cx, |v, cx| v.ask_remove("maria@globex.example", cx));
        cx.update_window(window, |_, window, cx| window.click("share-remove-ok", cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |v, cx| v.people(cx)), Some(2));
        assert!(!labelled(
            cx,
            window,
            "share-dialog",
            "maria@globex.example"
        ));
        let key = ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
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
        view.read_with(cx, |v, cx| {
            assert!(v.offers_share_default("BASE_URL", cx));
            assert!(v.offers_share_default("ACCOUNT_ID", cx));
            assert!(!v.offers_share_default("API_TOKEN", cx), "marked secret");
            assert!(
                !v.offers_share_default("SECRET_SIGNING", cx),
                "SECRET_ prefix"
            );
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
        assert!(!view.read_with(cx, |v, cx| v.shares_default("API_TOKEN", cx)));
    }

    #[gpui_kit::test]
    fn share_payload_has_no_values_unless_share_default(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        // Even a secret ticked behind the dialog's back stays out.
        ws.update(cx, |ws, cx| {
            edit_notes(&ws.notes, cx, |meta| {
                for name in ["API_TOKEN", "SECRET_SIGNING"] {
                    meta.vars.entry(name.into()).or_default().share_default = true;
                }
            });
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
        assert!(view.read_with(cx, |v, cx| v.shares_default("BASE_URL", cx)));
        assert!(ws.read_with(cx, |ws, cx| ws.meta(cx).vars["BASE_URL"].share_default));
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
        let link = view.read_with(cx, |v, cx| v.settings(cx).unwrap());
        backend.dev().unwrap().set_offline(true);
        view.update(cx, |v, cx| {
            v.set_access(Access::InvitedOnly, cx);
        });
        cx.run_until_parked();
        let error = view.read_with(cx, |v, cx| v.error(cx)).expect("an error");
        assert!(error.starts_with("You're offline."), "{error}");
        assert!(labelled(cx, window, "share-error", &error));
        assert_eq!(
            view.read_with(cx, |v, cx| v.settings(cx).unwrap()),
            link,
            "nothing changed here"
        );
        // A Share default tick shows at once, then rolls back when it fails.
        view.update(cx, |v, cx| v.toggle_share_default("BASE_URL", cx));
        assert!(
            view.read_with(cx, |v, cx| v.shares_default("BASE_URL", cx)),
            "optimistic"
        );
        cx.run_until_parked();
        assert!(!view.read_with(cx, |v, cx| v.shares_default("BASE_URL", cx)));
        assert!(!ws.read_with(cx, |ws, cx| {
            ws.meta(cx)
                .vars
                .get("BASE_URL")
                .is_some_and(|v| v.share_default)
        }));
        // Back online, the next change clears it.
        backend.dev().unwrap().set_offline(false);
        view.update(cx, |v, cx| v.set_access(Access::InvitedOnly, cx));
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            assert_eq!(v.error(cx), None);
            assert_eq!(v.settings(cx).unwrap().access, Access::InvitedOnly);
        });
        // Done closes the dialog.
        cx.update_window(window, |_, window, cx| window.click("share-done", cx))
            .unwrap();
        cx.run_until_parked();
        assert!(ws.read_with(cx, |ws, _| ws.share_view().is_none()));
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
        let view = cx
            .update_window(window, |_, window, cx| {
                let sync = ws.update(cx, |ws, cx| ws.share_sync(window, cx).unwrap());
                cx.new(|cx| ShareView::new(sync, ShareMode::Dialog, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            assert!(
                v.error(cx)
                    .as_deref()
                    .unwrap()
                    .starts_with("You're offline.")
            );
            assert_eq!(v.settings(cx), None);
        });
        backend.dev().unwrap().set_offline(false);
        let key = ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        let s = futures::executor::block_on(backend.share_settings(&key)).unwrap();
        assert_eq!(s.link, None);
    }

    /// The share as the server has it now.
    fn on_server(
        cx: &mut TestAppContext,
        ws: &Entity<Workspace>,
        backend: &Arc<dyn Backend>,
    ) -> crate::model::backend::ShareSettings {
        let key = ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        futures::executor::block_on(backend.share_settings(&key)).unwrap()
    }

    fn server_default(s: &crate::model::backend::ShareSettings, name: &str) -> Option<String> {
        s.variables
            .iter()
            .find(|v| v.name == name)
            .and_then(|v| v.default.clone())
    }

    #[gpui_kit::test]
    fn two_quick_ticks_both_persist(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        view.update(cx, |v, cx| {
            v.toggle_share_default("BASE_URL", cx);
            v.toggle_share_default("ACCOUNT_ID", cx);
        });
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            assert!(v.shares_default("BASE_URL", cx) && v.shares_default("ACCOUNT_ID", cx));
        });
        ws.read_with(cx, |ws, cx| {
            assert!(ws.meta(cx).vars["BASE_URL"].share_default);
            assert!(ws.meta(cx).vars["ACCOUNT_ID"].share_default);
        });
        let s = on_server(cx, &ws, &backend);
        assert_eq!(
            server_default(&s, "BASE_URL").as_deref(),
            Some("https://api.example.com/shipping")
        );
        assert_eq!(
            server_default(&s, "ACCOUNT_ID").as_deref(),
            Some("acct-4242")
        );
    }

    #[gpui_kit::test]
    fn double_tick_ends_off(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        view.update(cx, |v, cx| {
            v.toggle_share_default("BASE_URL", cx);
            v.toggle_share_default("BASE_URL", cx);
        });
        assert!(!view.read_with(cx, |v, cx| v.shares_default("BASE_URL", cx)));
        cx.run_until_parked();
        assert!(!view.read_with(cx, |v, cx| v.shares_default("BASE_URL", cx)));
        assert!(!ws.read_with(cx, |ws, cx| {
            ws.meta(cx)
                .vars
                .get("BASE_URL")
                .is_some_and(|v| v.share_default)
        }));
        assert_eq!(
            server_default(&on_server(cx, &ws, &backend), "BASE_URL"),
            None
        );
    }

    #[gpui_kit::test]
    fn invite_while_an_update_is_in_flight_keeps_the_person(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let dev = backend.dev().unwrap();
        // The tick's answer is held: its call stays in flight.
        dev.hold_updates(true);
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.toggle_share_default("BASE_URL", cx);
                v.type_invite("maria@globex.example", window, cx);
                v.invite(window, cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            let s = v.settings(cx).unwrap();
            assert!(
                s.people.iter().all(|p| p.email != "maria@globex.example"),
                "queued"
            );
            assert_eq!(v.invite_text(cx), "maria@globex.example", "not cleared yet");
        });
        assert!(
            !dev.recorded().iter().any(|c| c.starts_with("invite(")),
            "one call at a time"
        );
        // The tick's answer (without Maria) lands first, then the invite's
        // re-read: she stays.
        dev.hold_updates(false);
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            let s = v.settings(cx).unwrap();
            assert!(s.people.iter().any(|p| p.email == "maria@globex.example"));
            assert!(v.shares_default("BASE_URL", cx));
            assert_eq!(v.invite_text(cx), "", "cleared once she got in");
        });
        assert_eq!(on_server(cx, &ws, &backend).people.len(), 2);
    }

    #[gpui_kit::test]
    fn refused_invite_keeps_the_address(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let owner = backend.account().unwrap().email;
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.type_invite(&owner, window, cx);
                v.invite(window, cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            assert!(v.invite_error(cx).unwrap().contains("already has access"));
            assert_eq!(v.invite_text(cx), owner);
        });
    }

    #[gpui_kit::test]
    fn access_there_and_back_ends_where_asked(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        backend.dev().unwrap().hold_updates(true);
        view.update(cx, |v, cx| {
            v.set_access(Access::InvitedOnly, cx);
            v.set_access(Access::AnyoneWithLink, cx);
        });
        cx.run_until_parked();
        backend.dev().unwrap().hold_updates(false);
        cx.run_until_parked();
        assert_eq!(on_server(cx, &ws, &backend).access, Access::AnyoneWithLink);
        let shown = view.read_with(cx, |v, cx| v.settings(cx).unwrap().access);
        assert_eq!(shown, Access::AnyoneWithLink);
    }

    #[gpui_kit::test]
    fn ticks_land_after_done(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        view.update(cx, |v, cx| {
            v.toggle_share_default("BASE_URL", cx);
            v.toggle_share_default("ACCOUNT_ID", cx);
        });
        // Done before either call answers.
        cx.update_window(window, |_, window, cx| window.click("share-done", cx))
            .unwrap();
        drop(view);
        cx.run_until_parked();
        assert!(ws.read_with(cx, |ws, _| ws.share_view().is_none()));
        let server = on_server(cx, &ws, &backend);
        for name in ["BASE_URL", "ACCOUNT_ID"] {
            assert!(
                server_default(&server, name).is_some(),
                "{name} on the server"
            );
            assert!(
                ws.read_with(cx, |ws, cx| ws.meta(cx).vars[name].share_default),
                "{name} in the notes"
            );
        }
    }

    #[gpui_kit::test]
    fn invite_after_a_failed_open_rereads_the_share(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        backend.dev().unwrap().set_offline(true);
        let view = cx
            .update_window(window, |_, window, cx| {
                let sync = ws.update(cx, |ws, cx| ws.share_sync(window, cx).unwrap());
                cx.new(|cx| ShareView::new(sync, ShareMode::Dialog, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        assert!(view.read_with(cx, |v, cx| v.settings(cx).is_none()));
        backend.dev().unwrap().set_offline(false);
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |v, cx| {
                v.type_invite("maria@globex.example", window, cx);
                v.invite(window, cx);
            })
        })
        .unwrap();
        cx.run_until_parked();
        view.read_with(cx, |v, cx| {
            assert_eq!(v.error(cx), None);
            let s = v.settings(cx).expect("re-read");
            assert!(s.link.is_some());
            assert_eq!(s.people.len(), 2);
        });
    }

    #[gpui_kit::test]
    fn same_expiry_sends_nothing_and_esc_clears_the_dialog(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let before = update_shares(&backend).len();
        view.update(cx, |v, cx| v.set_expiry(Expiry::Days(30), cx));
        cx.run_until_parked();
        assert_eq!(update_shares(&backend).len(), before, "already 30 days");
        view.update(cx, |v, cx| v.set_expiry(Expiry::Never, cx));
        cx.run_until_parked();
        assert_eq!(update_shares(&backend).len(), before + 1);
        assert!(!found(cx, window, "share-cancel"), "Done only");
        drop(view);
        cx.update_window(window, |_, window, cx| window.press("escape", cx))
            .unwrap();
        cx.run_until_parked();
        assert!(ws.read_with(cx, |ws, _| ws.share_view().is_none()));
    }

    #[gpui_kit::test]
    fn a_finishing_tick_never_reaches_the_next_collection(cx: &mut TestAppContext) {
        let a = collection();
        let b = tempfile::tempdir().unwrap();
        std::fs::write(
            b.path().join("get.yaml"),
            "request:\n  url: ${BASE_URL}/b\n",
        )
        .unwrap();
        std::fs::write(
            b.path().join(".env"),
            "BASE_URL=https://b.example.com/api\n",
        )
        .unwrap();
        let backend = signed_in();
        let (window, ws) = open(cx, a.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        let dev = backend.dev().unwrap();
        dev.hold_updates(true);
        view.update(cx, |v, cx| v.toggle_share_default("BASE_URL", cx));
        cx.run_until_parked();
        drop(view);
        // B opens while A's tick is in flight; A's dialog goes.
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                assert!(ws.open_collection(b.path(), window, cx))
            });
            assert!(!window.has_active_dialog(cx));
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_share(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        dev.hold_updates(false);
        cx.run_until_parked();
        let key_b = ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        assert!(ws.read_with(cx, |ws, cx| {
            ws.meta(cx).vars.values().all(|v| !v.share_default)
        }));
        let b_calls: Vec<String> = update_shares(&backend)
            .into_iter()
            .filter(|c| c.contains(&format!("{:?}", key_b.id)))
            .collect();
        assert!(!b_calls.is_empty(), "B's dialog opened its share");
        for call in &b_calls {
            assert!(!call.contains("https://b.example.com/api"), "{call}");
        }
        let b_view = ws.read_with(cx, |ws, _| ws.share_view()).unwrap();
        assert!(!b_view.read_with(cx, |v, cx| v.shares_default("BASE_URL", cx)));
    }

    #[gpui_kit::test]
    fn a_menu_choice_settles_once_its_call_is_back(cx: &mut TestAppContext) {
        let dir = collection();
        let backend = signed_in();
        let (window, ws) = open(cx, dir.path(), backend.clone());
        let view = open_share(cx, window, &ws);
        view.update(cx, |v, cx| v.set_access(Access::InvitedOnly, cx));
        cx.run_until_parked();
        // The server changes on its own; the next answer shows it.
        let key = ws.read_with(cx, |ws, cx| ws.collection_key(cx).unwrap());
        futures::executor::block_on(backend.update_share(
            &key,
            crate::model::backend::ShareUpdate {
                access: Some(Access::AnyoneWithLink),
                ..Default::default()
            },
        ))
        .unwrap();
        view.update(cx, |v, cx| v.reset_link(cx));
        cx.run_until_parked();
        let sync = view.read_with(cx, |v, _| v.sync.clone());
        assert_eq!(
            sync.read_with(cx, |s, _| s.access()),
            Access::AnyoneWithLink
        );
        // A choice with a newer one queued keeps the newer until it lands.
        backend.dev().unwrap().hold_updates(true);
        view.update(cx, |v, cx| {
            v.set_access(Access::InvitedOnly, cx);
            v.set_access(Access::AnyoneWithLink, cx);
            v.set_access(Access::InvitedOnly, cx);
        });
        cx.run_until_parked();
        assert_eq!(sync.read_with(cx, |s, _| s.access()), Access::InvitedOnly);
        backend.dev().unwrap().hold_updates(false);
        cx.run_until_parked();
        assert_eq!(sync.read_with(cx, |s, _| s.access()), Access::InvitedOnly);
        let on_server = futures::executor::block_on(backend.share_settings(&key)).unwrap();
        assert_eq!(on_server.access, Access::InvitedOnly);
    }
}
