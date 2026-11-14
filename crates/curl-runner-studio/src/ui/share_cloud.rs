//! Share from the studio (board ShareCloud): a published collection's
//! sharing on the server. Who else can open it (Off, Public, Restricted),
//! its link, and the people on its list. The dashboard's Sharing page, in
//! a dialog: it changes the server's settings, never the folder.

use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{Observed, label_text, small_button};
use super::workspace::Workspace;
use crate::model::backend::{Backend, BackendError, LinkAccess, ShareChange, TeamShare};
use crate::model::publish::{CONFIG_FILE, CloudLink};
use chrono::Utc;
use curl_runner_cloud_proto::Plan;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const DIALOG_W: f32 = 600.;
const DIALOG_TOP: f32 = 40.;
const COPIED_FOR: Duration = Duration::from_millis(1500);
const FOOTER: &str = "They get each version you publish.";
const RESTRICT_WARNING: &str = "The link will need sign-in. Anyone already using it who isn't on the list stops getting updates. Add them before you switch.";
const ENTRY_HELP: &str = "Enter an email (jane@globex.com) or a domain (*@globex.com).";
const OFFLINE: &str = "You're offline. Changes to sharing need curl-runner Cloud; this shows the last settings it saw.";

/// The gallery description's limit, as the server's.
const MAX_DESCRIPTION: usize = 500;

/// The collection's page in the public gallery.
pub fn gallery_url(link: &CloudLink) -> String {
    format!(
        "https://www.curl-runner.com/collections/{}/{}",
        link.team, link.slug
    )
}

/// What the title bar's chip says: `Shared · 3`, `Public`, or nothing.
pub fn chip_text(access: LinkAccess, people: usize) -> Option<String> {
    match access {
        LinkAccess::Off => None,
        LinkAccess::Public => Some("Public".into()),
        LinkAccess::Restricted => Some(format!("Shared · {people}")),
    }
}

/// An email or `*@domain.tld`, lower-cased, as the server takes it.
pub fn valid_entry(raw: &str) -> Option<String> {
    let entry = raw.trim().to_lowercase();
    if entry.is_empty() || entry.chars().any(char::is_whitespace) {
        return None;
    }
    let (local, domain) = entry.split_once('@')?;
    if domain.is_empty() || domain.contains('@') {
        return None;
    }
    let ok = if local == "*" {
        domain.contains('.') && domain.split('.').all(|l| !l.is_empty())
    } else {
        !local.is_empty() && !local.contains('*')
    };
    ok.then_some(entry)
}

/// `Opened Nov 3` or `Invited · not opened`.
fn entry_status(opened: bool, at: chrono::DateTime<Utc>) -> String {
    if opened {
        format!("Opened {}", at.format("%b %-d"))
    } else {
        "Invited · not opened".into()
    }
}

/// Initials for an entry's disc: `JG` for jane.goodall@…, `@` for a domain.
fn entry_initials(entry: &str) -> String {
    if entry.starts_with("*@") {
        return "@".into();
    }
    let local = entry.split('@').next().unwrap_or(entry);
    let mut parts = local.split(['.', '_', '-', '+']).filter(|p| !p.is_empty());
    let a = parts.next().and_then(|p| p.chars().next());
    let b = parts.next().and_then(|p| p.chars().next());
    match (a, b) {
        (Some(a), Some(b)) => format!("{a}{b}").to_uppercase(),
        (Some(a), None) => {
            let mut c = local.chars();
            let first = c.next().unwrap_or(a);
            let second = c.next();
            format!("{first}{}", second.map(String::from).unwrap_or_default()).to_uppercase()
        }
        _ => "?".into(),
    }
}

/// `In 30 days`, `Never`.
fn expiry_text(expires_at: Option<chrono::DateTime<Utc>>) -> String {
    match expires_at {
        None => "Never".into(),
        Some(at) => {
            let days = (at - Utc::now()).num_days();
            match days {
                ..=0 => "Today".into(),
                1 => "In 1 day".into(),
                n => format!("In {n} days"),
            }
        }
    }
}

/// What the dialog was opened with.
pub struct ShareSetup {
    pub link: CloudLink,
    pub name: String,
    pub team_name: String,
    pub plan: Plan,
    /// Variable names from `curl-runner.yaml`'s `cloud.inputs`.
    pub inputs: Vec<String>,
    pub version: Option<u32>,
    pub offline: bool,
    /// The server has the public gallery (curl-runner Cloud).
    pub gallery: bool,
    pub backend: Arc<dyn Backend>,
}

pub enum ShareCloudEvent {
    Close,
    /// The settings changed: the title bar's chip follows.
    Changed(LinkAccess, usize),
    OpenConfig,
    Upgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirm {
    Restrict,
    Reset,
}

pub struct ShareCloudView {
    /// The gallery description's field, and whether `Edit` opened it.
    description: Entity<TextareaState>,
    editing_description: bool,
    setup: ShareSetup,
    share: Option<TeamShare>,
    error: Option<BackendError>,
    confirm: Option<Confirm>,
    entry: Entity<InputState>,
    entry_error: Option<String>,
    notify: bool,
    busy: bool,
    copied: bool,
    /// Empty the field on the next render (after an add).
    clear_entry: bool,
    tasks: Vec<Task<()>>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<ShareCloudEvent> for ShareCloudView {}

impl ShareCloudView {
    pub fn new(setup: ShareSetup, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let entry = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Email, or a domain like *@globex.com")
        });
        let sub = cx.subscribe(&entry, |this: &mut Self, _, e: &InputEvent, cx| match e {
            InputEvent::Change => {
                this.entry_error = None;
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.add_entry(cx),
            _ => {}
        });
        let description = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(3)
                .placeholder("What these requests are for, in a sentence or two")
        });
        let desc_sub = cx.observe(&description, |_, _, cx| cx.notify());
        let mut view = ShareCloudView {
            description,
            editing_description: false,
            setup,
            share: None,
            error: None,
            confirm: None,
            entry,
            entry_error: None,
            notify: true,
            busy: false,
            copied: false,
            clear_entry: false,
            tasks: Vec::new(),
            _subs: vec![sub, desc_sub],
        };
        view.load(cx);
        view
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let (team, slug) = (self.setup.link.team.clone(), self.setup.link.slug.clone());
        let call = backend_call(cx, &self.setup.backend, move |b| b.team_share(&team, &slug));
        self.tasks.push(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| this.took(r, cx)).ok();
        }));
    }

    fn took(&mut self, r: Result<TeamShare, BackendError>, cx: &mut Context<Self>) {
        self.busy = false;
        match r {
            Ok(s) => {
                self.editing_description = false;
                cx.emit(ShareCloudEvent::Changed(s.access, s.entries.len()));
                self.share = Some(s);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }

    fn locked(&self) -> bool {
        self.setup.offline
            || self.busy
            || matches!(self.error, Some(BackendError::Offline))
            || self.share.is_none()
    }

    fn change(&mut self, change: ShareChange, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        self.busy = true;
        self.confirm = None;
        let (team, slug) = (self.setup.link.team.clone(), self.setup.link.slug.clone());
        let call = backend_call(cx, &self.setup.backend, move |b| {
            b.change_team_share(&team, &slug, change)
        });
        self.tasks.push(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| this.took(r, cx)).ok();
        }));
        cx.notify();
    }

    /// Picking an access: Public → Restricted asks first; Restricted on a
    /// Free team is locked.
    fn pick(&mut self, access: LinkAccess, cx: &mut Context<Self>) {
        let Some(current) = self.share.as_ref().map(|s| s.access) else {
            return;
        };
        if access == current {
            return;
        }
        if access == LinkAccess::Restricted && self.setup.plan == Plan::Free {
            return;
        }
        if access == LinkAccess::Restricted && current == LinkAccess::Public {
            self.confirm = Some(Confirm::Restrict);
            cx.notify();
            return;
        }
        self.change(
            ShareChange {
                access: Some(access),
                ..Default::default()
            },
            cx,
        );
    }

    fn add_entry(&mut self, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let raw = self.entry.read(cx).value().to_string();
        let Some(entry) = valid_entry(&raw) else {
            self.entry_error = Some(ENTRY_HELP.into());
            cx.notify();
            return;
        };
        self.busy = true;
        let (team, slug, notify) = (
            self.setup.link.team.clone(),
            self.setup.link.slug.clone(),
            self.notify,
        );
        let call = backend_call(cx, &self.setup.backend, move |b| {
            b.add_share_entry(&team, &slug, &entry, notify)
        });
        self.tasks.push(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match r {
                    Ok(e) => {
                        if let Some(s) = this.share.as_mut() {
                            s.entries.retain(|x| x.entry != e.entry);
                            s.entries.push(e);
                            cx.emit(ShareCloudEvent::Changed(s.access, s.entries.len()));
                        }
                        this.entry_error = None;
                        this.clear_entry = true;
                    }
                    Err(e) => this.entry_error = Some(e.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn remove(&mut self, entry: String, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        self.busy = true;
        let (team, slug, e) = (
            self.setup.link.team.clone(),
            self.setup.link.slug.clone(),
            entry.clone(),
        );
        let call = backend_call(cx, &self.setup.backend, move |b| {
            b.remove_share_entry(&team, &slug, &e)
        });
        self.tasks.push(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match r {
                    Ok(()) => {
                        if let Some(s) = this.share.as_mut() {
                            s.entries.retain(|x| x.entry != entry);
                            cx.emit(ShareCloudEvent::Changed(s.access, s.entries.len()));
                        }
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        let Some(s) = &self.share else { return };
        cx.write_to_clipboard(ClipboardItem::new_string(s.link.clone()));
        self.copied = true;
        self.tasks.push(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            this.update(cx, |this, cx| {
                this.copied = false;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .items_start()
            .gap(px(12.))
            .px(px(24.))
            .pt(px(20.))
            .pb(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Share {}", self.setup.name)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(relative(1.5))
                            .text_color(color("text-secondary"))
                            .child(format!(
                                "Members of {} can always open it. This decides who else can.",
                                self.setup.team_name
                            )),
                    ),
            )
            .child(
                div()
                    .id("share-cloud-close")
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
                    .hover(|s| s.bg(color("surface-100")))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareCloudEvent::Close)))
                    .child(glyph(icons::CROSS).size(px(14.))),
            )
    }

    fn access_options(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let current = self.share.as_ref().map(|s| s.access);
        let free = self.setup.plan == Plan::Free;
        let team = self.setup.team_name.clone();
        let opt = |id: &'static str,
                   access: LinkAccess,
                   title: &'static str,
                   desc: String,
                   cx: &mut Context<Self>| {
            let on = current == Some(access);
            let locked = free && access == LinkAccess::Restricted;
            div()
                .id(id)
                .role(Role::RadioButton)
                .test_support()
                .aria_label(title)
                .flex()
                .items_start()
                .gap(px(12.))
                .px(px(14.))
                .py(px(12.))
                .border_t_1()
                .border_color(color("border-100"))
                .text_size(px(13.))
                .when(on, |d| d.bg(color("surface-100")))
                .when(!locked && !on, |d| {
                    d.cursor_pointer()
                        .hover(|s| s.bg(color("surface-100")))
                        .on_click(cx.listener(move |this, _, _, cx| this.pick(access, cx)))
                })
                .child(
                    div()
                        .flex_none()
                        .mt(px(3.))
                        .size(px(14.))
                        .rounded_full()
                        .border_color(color(if on { "text-primary" } else { "border-200" }))
                        .map(|d| if on { d.border_4() } else { d.border_1() })
                        .when(locked, |d| d.opacity(0.4)),
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
                                .text_color(color(if locked {
                                    "text-tertiary"
                                } else {
                                    "text-primary"
                                }))
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-secondary"))
                                .child(desc),
                        ),
                )
                .when(access == LinkAccess::Restricted, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .px(px(8.))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .border_1()
                            .border_color(color("border-200"))
                            .rounded_full()
                            .text_size(px(11.))
                            .text_color(color("text-secondary"))
                            .child("Team plan"),
                    )
                })
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(label_text("Who can open it"))
            .child(
                div()
                    .id("share-cloud-access")
                    .role(Role::RadioGroup)
                    .test_support()
                    .aria_label("Who can open it")
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(color("border-100"))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .child(opt(
                        "share-cloud-off",
                        LinkAccess::Off,
                        "Off",
                        format!("Only members of {team}."),
                        cx,
                    ))
                    .child(opt(
                        "share-cloud-public",
                        LinkAccess::Public,
                        "Public",
                        "Anyone with the link. No account needed.".into(),
                        cx,
                    ))
                    .child(opt(
                        "share-cloud-restricted",
                        LinkAccess::Restricted,
                        "Restricted",
                        "Only people on the list, after they sign in.".into(),
                        cx,
                    )),
            )
            .when(free, |d| {
                d.child(note(
                    "info",
                    "share-cloud-free",
                    format!(
                        "Restricted sharing needs the Team plan. Upgrade {} to share only with people you name.",
                        self.setup.team_name
                    ),
                    Some(
                        small_button("share-cloud-upgrade", "Upgrade")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareCloudEvent::Upgrade)))
                            .child("Upgrade")
                            .into_any_element(),
                    ),
                ))
            })
            .when(self.confirm == Some(Confirm::Restrict), |d| {
                d.child(confirm_note(
                    "share-cloud-confirm-restrict",
                    RESTRICT_WARNING,
                    ("share-cloud-keep-public", "Keep public"),
                    ("share-cloud-make-restricted", "Make restricted"),
                    cx.listener(|this, _, _, cx| {
                        this.confirm = None;
                        cx.notify();
                    }),
                    cx.listener(|this, _, _, cx| {
                        this.confirm = None;
                        this.change(
                            ShareChange {
                                access: Some(LinkAccess::Restricted),
                                ..Default::default()
                            },
                            cx,
                        );
                    }),
                ))
            })
    }

    fn link_section(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let share = self.share.clone();
        let off = share.as_ref().is_none_or(|s| s.access == LinkAccess::Off);
        let link = share.as_ref().map(|s| s.link.clone()).unwrap_or_default();
        let help = match share.as_ref().map(|s| s.access) {
            Some(LinkAccess::Public) => format!(
                "Anyone who has it can open {} and get every new version. Treat it like a password: it isn't indexed by search engines.",
                self.setup.name
            ),
            Some(LinkAccess::Restricted) => format!(
                "People on the list sign in once, then open {} and get every version you publish.",
                self.setup.name
            ),
            _ => "Turn on Public or Restricted to share it. The link is kept, so it works again when you do.".into(),
        };
        let weak = cx.entity().downgrade();
        let expiry = expiry_text(share.as_ref().and_then(|s| s.expires_at));
        let locked = self.locked();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(label_text("Share link"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("share-cloud-link")
                            .test_support()
                            .aria_label(link.clone())
                            .flex_1()
                            .min_w_0()
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .px(px(10.))
                            .border_1()
                            .border_color(color("border-200"))
                            .rounded(px(6.))
                            .text_size(px(12.))
                            .font_family(super::app::mono_font())
                            .text_color(color(if off { "text-disabled" } else { "text-primary" }))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(link),
                    )
                    .child(
                        Button::new("share-cloud-copy")
                            .outline()
                            .h(px(32.))
                            .px(px(12.))
                            .text_size(px(13.))
                            .label(if self.copied { "Copied" } else { "Copy link" })
                            .disabled(off)
                            .on_click(cx.listener(|this, _, _, cx| this.copy(cx))),
                    ),
            )
            .when(!off, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-tertiary"))
                                .child("Expires"),
                        )
                        .child(
                            Button::new("share-cloud-expiry")
                                .outline()
                                .h(px(28.))
                                .pl(px(10.))
                                .pr(px(8.))
                                .gap(px(8.))
                                .text_size(px(12.))
                                .disabled(locked)
                                .child(expiry)
                                .child(
                                    glyph(icons::CHEVRON_DOWN)
                                        .size(px(12.))
                                        .text_color(color("text-secondary")),
                                )
                                .dropdown_menu(move |mut menu, _, _| {
                                    for (label, days) in
                                        [("Never", None), ("In 7 days", Some(7)), ("In 30 days", Some(30))]
                                    {
                                        let weak = weak.clone();
                                        menu = menu.item(PopupMenuItem::new(label).on_click(
                                            move |_, _, cx| {
                                                weak.update(cx, |this, cx| {
                                                    this.change(
                                                        ShareChange {
                                                            expires_in_days: Some(days),
                                                            ..Default::default()
                                                        },
                                                        cx,
                                                    )
                                                })
                                                .ok();
                                            },
                                        ));
                                    }
                                    menu
                                }),
                        )
                        .child(div().flex_1())
                        .child(
                            small_button("share-cloud-reset", "Reset link")
                                .when(locked, |b| b.opacity(0.4))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if !this.locked() {
                                        this.confirm = Some(Confirm::Reset);
                                        cx.notify();
                                    }
                                }))
                                .child("Reset link"),
                        ),
                )
            })
            .when(self.confirm == Some(Confirm::Reset), |d| {
                d.child(confirm_note(
                    "share-cloud-confirm-reset",
                    &format!(
                        "The old link stops working right away, for everyone. People who already added {} need the new link to get updates.",
                        self.setup.name
                    ),
                    ("share-cloud-reset-cancel", "Cancel"),
                    ("share-cloud-reset-go", "Reset link"),
                    cx.listener(|this, _, _, cx| {
                        this.confirm = None;
                        cx.notify();
                    }),
                    cx.listener(|this, _, _, cx| {
                        this.change(
                            ShareChange {
                                reset_link: true,
                                ..Default::default()
                            },
                            cx,
                        );
                    }),
                ))
            })
            .child(help_text(help))
    }

    fn people(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let entries = self
            .share
            .as_ref()
            .map(|s| s.entries.clone())
            .unwrap_or_default();
        let invalid = self.entry_error.is_some();
        let rows = entries.into_iter().enumerate().map(|(ix, e)| {
            let detail = if e.kind == "domain" {
                format!("Anyone at {}", e.entry.trim_start_matches("*@"))
            } else {
                "Email".into()
            };
            let entry = e.entry.clone();
            div()
                .id(SharedString::from(format!("share-cloud-entry-{ix}")))
                .test_support()
                .aria_label(format!(
                    "{}, {}",
                    e.entry,
                    entry_status(e.opened, e.invited_at)
                ))
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(12.))
                .py(px(10.))
                .border_t_1()
                .border_color(color("border-100"))
                .text_size(px(13.))
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(px(26.))
                        .rounded_full()
                        .bg(color("surface-200"))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(entry_initials(&e.entry)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_ellipsis().child(e.entry.clone()))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-secondary"))
                                .child(detail),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .h(px(20.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .border_1()
                        .rounded_full()
                        .text_size(px(11.))
                        .map(|d| {
                            if e.opened {
                                d.border_color(color("success-border"))
                                    .text_color(color("success"))
                            } else {
                                d.border_color(color("border-200"))
                                    .text_color(color("text-secondary"))
                            }
                        })
                        .child(entry_status(e.opened, e.invited_at)),
                )
                .child(
                    small_button(
                        SharedString::from(format!("share-cloud-remove-{ix}")),
                        format!("Remove {}", e.entry),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.remove(entry.clone(), cx)))
                    .child("Remove"),
                )
        });
        let count = self.share.as_ref().map_or(0, |s| s.entries.len());
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(label_text(format!("People with access · {count}")))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.entry)
                                .aria_label("Email or domain")
                                .min_h(px(32.))
                                .text_size(px(13.))
                                .when(invalid, |i| i.border_color(color("danger"))),
                        ),
                    )
                    .child(
                        Button::new("share-cloud-add")
                            .outline()
                            .h(px(32.))
                            .px(px(12.))
                            .text_size(px(13.))
                            .label("Add")
                            .disabled(self.locked())
                            .on_click(cx.listener(|this, _, _, cx| this.add_entry(cx))),
                    ),
            )
            .children(self.entry_error.clone().map(|e| {
                div()
                    .id("share-cloud-entry-error")
                    .test_support()
                    .aria_label(e.clone())
                    .text_size(px(12.))
                    .text_color(color("danger"))
                    .child(e)
            }))
            .child(
                div()
                    .id("share-cloud-notify")
                    .role(Role::Switch)
                    .test_support()
                    .aria_label("Email them the link")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.notify = !this.notify;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(14.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(color("border-200"))
                            .when(self.notify, |d| {
                                d.bg(color("text-primary")).child(
                                    glyph(icons::CHECK)
                                        .size(px(11.))
                                        .text_color(color("bg-100")),
                                )
                            }),
                    )
                    .child("Email them the link. Domains never get an email."),
            )
            .when(count > 0, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .border_1()
                        .border_color(color("border-100"))
                        .rounded(px(8.))
                        .overflow_hidden()
                        .children(rows),
                )
            })
            .child(help_text(
                "Removing someone stops their next update. Their copy stays on their machine."
                    .into(),
            ))
    }

    /// `Edit`: the saved description back into the field.
    fn edit_description(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self
            .share
            .as_ref()
            .and_then(|s| s.description.clone())
            .unwrap_or_default();
        self.description
            .update(cx, |t, cx| t.set_value(current, window, cx));
        self.editing_description = true;
        cx.notify();
    }

    fn save_description(&mut self, cx: &mut Context<Self>) {
        let text = self.description.read(cx).value().trim().to_string();
        if text.is_empty() || text.chars().count() > MAX_DESCRIPTION {
            return;
        }
        self.change(
            ShareChange {
                description: Some(text),
                ..Default::default()
            },
            cx,
        );
    }

    /// The description field (board Switcher, Share): shown while there's
    /// none, and after `Edit`.
    fn description_field(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let text = self.description.read(cx).value().to_string();
        let count = text.trim().chars().count();
        let ok = count > 0 && count <= MAX_DESCRIPTION && !self.locked();
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(Textarea::new(&self.description))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("share-cloud-description-count")
                            .test_support()
                            .aria_label(format!("{count} / {MAX_DESCRIPTION}"))
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(color(if count > MAX_DESCRIPTION {
                                "danger"
                            } else {
                                "text-tertiary"
                            }))
                            .child(format!("{count} / {MAX_DESCRIPTION}")),
                    )
                    .when(self.editing_description, |d| {
                        d.child(
                            super::settings_kit::small_button(
                                "share-cloud-description-cancel",
                                "Cancel",
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.editing_description = false;
                                cx.notify();
                            }))
                            .child("Cancel"),
                        )
                    })
                    .child(
                        super::settings_kit::primary_button(
                            "share-cloud-description-save",
                            "Save description",
                        )
                        .when(!ok, |b| b.opacity(0.45).cursor_default())
                        .when(ok, |b| {
                            b.on_click(cx.listener(|this, _, _, cx| this.save_description(cx)))
                        })
                        .child("Save description"),
                    ),
            )
    }

    /// Public only, on Cloud: listing in the gallery (board GalleryListing).
    fn gallery(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let share = self.share.as_ref();
        let listed = share.is_some_and(|s| s.listed);
        let saved = share
            .and_then(|s| s.description.clone())
            .filter(|d| !d.trim().is_empty());
        let has_description = saved.is_some();
        let enabled = has_description && !self.locked();
        let url = gallery_url(&self.setup.link);
        let field =
            (!has_description || self.editing_description).then(|| self.description_field(cx));
        let shown = saved.filter(|_| !self.editing_description).map(|d| {
            div()
                .flex()
                .flex_wrap()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child(
                    div()
                        .id("share-cloud-description")
                        .test_support()
                        .aria_label(d.clone())
                        .child(d),
                )
                .child(
                    div()
                        .id("share-cloud-description-edit")
                        .role(Role::Link)
                        .test_support()
                        .aria_label("Edit")
                        .text_color(color("text-primary"))
                        .cursor_pointer()
                        .underline()
                        .on_click(cx.listener(|this, _, window, cx| {
                            // Not the row's listing toggle as well.
                            cx.stop_propagation();
                            this.edit_description(window, cx)
                        }))
                        .child("Edit"),
                )
        });
        let text = if !has_description {
            "Add a description first. It's the summary people read on curl-runner.com/collections."
                .to_string()
        } else {
            format!(
                "Anyone can find it on curl-runner.com: the name, description, {}, the requests and their YAML. Variable values never are.",
                self.setup.team_name
            )
        };
        div()
            .id("share-cloud-gallery")
            .role(Role::Switch)
            .test_support()
            .aria_label(if listed {
                "List in the gallery, on"
            } else {
                "List in the gallery, off"
            })
            .flex()
            .items_start()
            .gap(px(12.))
            // Not while the field is open: clicks there are typing.
            .when(enabled && !self.editing_description, |d| {
                d.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change(
                            ShareChange {
                                listed: Some(!listed),
                                ..Default::default()
                            },
                            cx,
                        )
                    }))
            })
            .child(
                div()
                    .flex_none()
                    .mt(px(2.))
                    .when(!enabled, |d| d.opacity(0.45))
                    .child(super::settings_kit::switch(listed, enabled)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(4.))
                    .text_size(px(13.))
                    .child(
                        div()
                            .text_color(color(if has_description {
                                "text-primary"
                            } else {
                                "text-tertiary"
                            }))
                            .child("List in the gallery"),
                    )
                    .children(shown)
                    .child(help_text(text))
                    .children(field)
                    .when(listed, |d| {
                        d.child(
                            div()
                                .id("share-cloud-gallery-link")
                                .role(Role::Link)
                                .test_support()
                                .aria_label("View it on curl-runner.com")
                                .text_size(px(12.))
                                .cursor_pointer()
                                .underline()
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    cx.open_url(&url)
                                })
                                .child("View it on curl-runner.com"),
                        )
                    }),
            )
    }

    fn what_they_get(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let version = self
            .setup
            .version
            .map_or_else(String::new, |v| format!("Version {v}. "));
        let vars = self.setup.inputs.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(label_text("What they get"))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(if vars.is_empty() {
                        format!("{version}No variables to fill in.")
                    } else {
                        format!("{version}Variables they fill in themselves:")
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .children(vars.into_iter().map(|v| {
                        div()
                            .h(px(22.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .rounded(px(4.))
                            .bg(color("surface-100"))
                            .text_size(px(11.5))
                            .font_family(super::app::mono_font())
                            .child(v)
                    }))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("share-cloud-edit-config")
                            .role(Role::Link)
                            .test_support()
                            .aria_label(format!("Edit in {CONFIG_FILE}"))
                            .text_size(px(12.))
                            .cursor_pointer()
                            .underline()
                            .on_click(
                                cx.listener(|_, _, _, cx| cx.emit(ShareCloudEvent::OpenConfig)),
                            )
                            .child(format!("Edit in {CONFIG_FILE}")),
                    ),
            )
            .child(help_text(
                "Never shared: values, secrets from your Keychain, your run history.".into(),
            ))
    }
}

fn help_text(text: String) -> Div {
    div()
        .text_size(px(12.))
        .line_height(relative(1.5))
        .text_color(color("text-tertiary"))
        .child(text)
}

fn note(
    tone: &'static str,
    id: &'static str,
    text: String,
    action: Option<AnyElement>,
) -> Observed {
    let (border, bg) = match tone {
        "info" => ("info-border", "info-subtle"),
        "danger" => ("danger-border", "danger-subtle"),
        _ => ("warning-border", "warning-subtle"),
    };
    div()
        .id(id)
        .role(Role::Status)
        .test_support()
        .aria_label(text.clone())
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(14.))
        .py(px(12.))
        .border_1()
        .border_color(color(border))
        .bg(color(bg))
        .rounded(px(8.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .child(div().flex_1().min_w_0().child(text))
        .children(action)
}

fn confirm_note(
    id: &'static str,
    text: &str,
    cancel: (&'static str, &'static str),
    go: (&'static str, &'static str),
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_go: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .role(Role::Alert)
        .test_support()
        .aria_label(text.to_string())
        .flex()
        .flex_col()
        .gap(px(10.))
        .px(px(14.))
        .py(px(12.))
        .border_1()
        .border_color(color("warning-border"))
        .bg(color("warning-subtle"))
        .rounded(px(8.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .child(text.to_string())
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(
                    small_button(cancel.0, cancel.1)
                        .on_click(on_cancel)
                        .child(cancel.1),
                )
                .child(
                    super::settings_kit::primary_button(go.0, go.1)
                        .on_click(on_go)
                        .child(go.1),
                ),
        )
}

impl Render for ShareCloudView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.clear_entry) {
            self.entry.update(cx, |i, cx| i.set_value("", window, cx));
        }
        let room = window.viewport_size().height - px(DIALOG_TOP * 2. + 140.);
        let restricted = self.share.as_ref().map(|s| s.access) == Some(LinkAccess::Restricted);
        let public = self.share.as_ref().map(|s| s.access) == Some(LinkAccess::Public);
        let gallery = self.setup.gallery && public;
        let status = if self.setup.offline || matches!(self.error, Some(BackendError::Offline)) {
            Some(note("warning", "share-cloud-offline", OFFLINE.into(), None))
        } else {
            match &self.error {
                Some(BackendError::PlanLimit(m)) => Some(note(
                    "info",
                    "share-cloud-plan",
                    m.clone(),
                    Some(
                        small_button("share-cloud-plan-upgrade", "Upgrade")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareCloudEvent::Upgrade)))
                            .child("Upgrade")
                            .into_any_element(),
                    ),
                )),
                Some(e) => Some(note("danger", "share-cloud-error", e.to_string(), None)),
                None if self.share.is_none() => Some(note(
                    "info",
                    "share-cloud-loading",
                    "Loading sharing…".into(),
                    None,
                )),
                None => None,
            }
        };
        div()
            .id("share-cloud-dialog")
            .role(Role::Dialog)
            .test_support()
            .aria_label(format!("Share {}", self.setup.name))
            .flex()
            .flex_col()
            .child(self.header(cx))
            .child(
                div()
                    .id("share-cloud-body")
                    .max_h(room.max(px(240.)))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .px(px(24.))
                    .pb(px(20.))
                    .children(status)
                    .child(self.access_options(cx))
                    .child(self.link_section(cx))
                    .when(gallery, |d| d.child(self.gallery(cx)))
                    .when(restricted, |d| d.child(self.people(cx)))
                    .child(self.what_they_get(cx)),
            )
            .child(
                div()
                    .flex()
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
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(FOOTER),
                    )
                    .child(
                        Button::new("share-cloud-done")
                            .primary()
                            .h(px(32.))
                            .px(px(12.))
                            .text_size(px(13.))
                            .label("Done")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(ShareCloudEvent::Close))),
                    ),
            )
    }
}

impl Workspace {
    /// Share for a published folder (board ShareCloud), from the foot, the
    /// palette and the title bar's chip.
    pub(super) fn open_team_share(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.publish_applies() {
            return;
        }
        let (Some(link), Some(account)) = (self.publish_link(cx), self.backend.account()) else {
            return;
        };
        let Some(root) = self.state.collection.as_ref().map(|c| c.root.clone()) else {
            return;
        };
        let team = account.teams.iter().find(|t| t.slug == link.team).cloned();
        let cfg = curl_runner_core::cloud::load_cloud_config(&root)
            .ok()
            .flatten();
        let name = cfg
            .as_ref()
            .and_then(|c| c.name.clone())
            .or_else(|| self.collection_title())
            .unwrap_or_else(|| link.slug.clone());
        let setup = ShareSetup {
            team_name: team
                .as_ref()
                .map_or_else(|| link.team.clone(), |t| t.name.clone()),
            plan: team.as_ref().map_or(Plan::Free, |t| t.plan),
            inputs: cfg
                .map(|c| c.inputs.into_iter().map(|i| i.name).collect())
                .unwrap_or_default(),
            version: self.publish.current,
            offline: self.connection() == Some(super::state::Connection::Offline),
            gallery: self.backend.server() == crate::model::backend::Server::Cloud,
            backend: self.backend.clone(),
            link,
            name,
        };
        let team_slug = setup.link.team.clone();
        let view = cx.new(|cx| ShareCloudView::new(setup, window, cx));
        self.publish.share_events = Some(cx.subscribe_in(
            &view,
            window,
            move |this, _, event: &ShareCloudEvent, window, cx| match event {
                ShareCloudEvent::Close => {
                    window.close_dialog(cx);
                    this.publish.share_view = None;
                }
                ShareCloudEvent::Changed(access, n) => {
                    this.publish.share = Some((*access, *n));
                    cx.notify();
                }
                ShareCloudEvent::OpenConfig => {
                    window.close_dialog(cx);
                    this.publish.share_view = None;
                    this.open_file(&PathBuf::from(CONFIG_FILE), window, cx);
                }
                ShareCloudEvent::Upgrade => {
                    let dash = backend_call(cx, &this.backend, |b| b.dashboard_url());
                    let team = team_slug.clone();
                    cx.spawn(async move |_, cx| {
                        if let Ok(d) = dash.await {
                            cx.update(|cx| cx.open_url(&format!("{d}/t/{team}/billing")));
                        }
                    })
                    .detach();
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
        self.publish.share_view = Some(view.downgrade());
    }

    /// Share from the palette: the server's for a published folder, else
    /// the sync-era dialog.
    pub(super) fn share_from_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.publish_applies() {
            if self.publish.current.is_some() {
                self.open_team_share(window, cx);
            }
        } else {
            self.open_share(window, cx);
        }
    }

    /// The open Share dialog (tests).
    #[cfg(all(test, feature = "server"))]
    pub(super) fn team_share_view(&self) -> Option<Entity<ShareCloudView>> {
        self.publish
            .share_view
            .as_ref()
            .and_then(WeakEntity::upgrade)
    }

    /// Reads the collection's sharing for the title bar's chip.
    pub(super) fn refresh_team_share(&mut self, cx: &mut Context<Self>) {
        let Some(link) = self.publish_link(cx) else {
            self.publish.share = None;
            return;
        };
        let (team, slug) = (link.team, link.slug);
        let call = backend_call(cx, &self.backend, move |b| b.team_share(&team, &slug));
        self.publish.share_task = Some(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| {
                if let Ok(s) = r {
                    this.publish.share = Some((s.access, s.entries.len()));
                    cx.notify();
                }
            })
            .ok();
        }));
    }
}

#[cfg(all(test, feature = "server"))]
mod tests;

#[cfg(test)]
mod unit_tests {
    use super::{chip_text, entry_initials, entry_status, expiry_text, valid_entry};
    use crate::model::backend::LinkAccess;
    use chrono::{TimeZone, Utc};

    #[test]
    fn entries_follow_the_servers_rule() {
        assert_eq!(
            valid_entry(" Jane@Globex.com ").as_deref(),
            Some("jane@globex.com")
        );
        assert_eq!(valid_entry("*@globex.com").as_deref(), Some("*@globex.com"));
        for bad in [
            "jane@globex@x",
            "jane",
            "*@globex",
            "@globex.com",
            "a b@c.d",
            "*a@b.c",
            "",
        ] {
            assert_eq!(valid_entry(bad), None, "{bad}");
        }
    }

    #[test]
    fn chip_by_access() {
        assert_eq!(chip_text(LinkAccess::Off, 3), None);
        assert_eq!(chip_text(LinkAccess::Public, 0).as_deref(), Some("Public"));
        assert_eq!(
            chip_text(LinkAccess::Restricted, 3).as_deref(),
            Some("Shared · 3")
        );
    }

    #[test]
    fn entry_rows() {
        assert_eq!(entry_initials("jane.goodall@globex.com"), "JG");
        assert_eq!(entry_initials("sam@umbrella.dev"), "SA");
        assert_eq!(entry_initials("*@initech.com"), "@");
        let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
        assert_eq!(entry_status(true, at), "Opened Nov 3");
        assert_eq!(entry_status(false, at), "Invited · not opened");
        assert_eq!(expiry_text(None), "Never");
        assert_eq!(
            expiry_text(Some(Utc::now() + chrono::Duration::hours(24 * 30 + 1))),
            "In 30 days"
        );
    }
}
