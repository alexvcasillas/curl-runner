//! Who the studio is signed in as (the SignedIn boards): the title bar's
//! avatar and its menu, and the status bar's account line. Both open the
//! same menu.

use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_view::AppPage;
use super::state::Connection;
use super::workspace::{Workspace, host_of, notify_error};
use crate::model::backend::TeamRef;
use crate::model::backend::{Account, Backend, Server, method_label};
use curl_runner_cloud_proto::{Plan as TeamPlan, Role as TeamRole};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::time::{Duration, Instant};

/// Where the studio stands with its server, for display.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountState {
    /// Signed in to `server`.
    Connected { account: Account, server: Server },
    /// `server` refused the session (401): who it was, until they sign in
    /// again or out.
    Expired { account: Account, server: Server },
    /// A server is chosen but no one is signed in to it.
    SignedOut { server: Server },
    /// `Continue offline`: local files only.
    Offline,
}

impl AccountState {
    /// `None` before the sign-in screen was answered, and in builds without
    /// the server: there is nothing to show then.
    pub fn of(backend: &dyn Backend, connection: Option<Connection>) -> Option<AccountState> {
        if !crate::features::SERVER {
            return None;
        }
        Some(match connection? {
            Connection::Offline => AccountState::Offline,
            Connection::Online => {
                let server = backend.server();
                if let Some(account) = backend.account() {
                    AccountState::Connected { account, server }
                } else if let Some(account) = backend.lapsed() {
                    AccountState::Expired { account, server }
                } else {
                    AccountState::SignedOut { server }
                }
            }
        })
    }

    /// The dot's token.
    pub fn dot(&self) -> &'static str {
        match self {
            AccountState::Connected { .. } => "success",
            AccountState::Expired { .. } => "warning",
            AccountState::SignedOut { .. } | AccountState::Offline => "text-tertiary",
        }
    }

    /// The status bar's server item: `Cloud`, a self-hosted host, or
    /// `Offline`. Who is signed in is the avatar's job.
    pub fn status_text(&self) -> String {
        match self {
            AccountState::Connected { server, .. }
            | AccountState::Expired { server, .. }
            | AccountState::SignedOut { server } => short_name(server),
            AccountState::Offline => "Offline".into(),
        }
    }

    /// What a screen reader hears for the status bar's server item.
    pub fn status_label(&self) -> String {
        let name = match self {
            AccountState::Connected { server, .. }
            | AccountState::Expired { server, .. }
            | AccountState::SignedOut { server } => long_name(server),
            AccountState::Offline => "Offline".into(),
        };
        format!("{name}. Open Accounts & sync")
    }

    /// What a screen reader hears for the avatar.
    pub fn label(&self) -> String {
        match self {
            AccountState::Connected { account, server } => format!(
                "Signed in as {} on {}. Open account menu",
                account.email,
                long_name(server)
            ),
            AccountState::Expired { account, server } => format!(
                "Session expired for {} on {}. Open account menu",
                account.email,
                long_name(server)
            ),
            AccountState::SignedOut { server } => {
                format!("Not signed in to {}. Sign in", long_name(server))
            }
            AccountState::Offline => "Offline. Sign in to sync".into(),
        }
    }

    fn account(&self) -> Option<&Account> {
        match self {
            AccountState::Connected { account, .. } | AccountState::Expired { account, .. } => {
                Some(account)
            }
            _ => None,
        }
    }
}

/// Two letters from the name (`Jamie Rivera` → `JR`), one from a single
/// name, and the email's first letter when the server has no name (the
/// account's name is its email then).
pub fn initials(a: &Account) -> String {
    let named = !a.name.trim().is_empty() && a.name != a.email;
    let source = if named {
        a.name.as_str()
    } else {
        a.email.as_str()
    };
    let mut words = source.split_whitespace().filter_map(|w| w.chars().next());
    let first = words.next();
    let last = if named { words.next_back() } else { None };
    first
        .into_iter()
        .chain(last)
        .flat_map(char::to_uppercase)
        .collect()
}

/// `Cloud`, or the self-hosted server's host.
pub fn short_name(server: &Server) -> String {
    match server {
        Server::Cloud => "Cloud".into(),
        Server::SelfHosted { url } => host_of(url).into(),
    }
}

/// `curl-runner Cloud`, or the self-hosted server's host.
pub fn long_name(server: &Server) -> String {
    match server {
        Server::Cloud => "curl-runner Cloud".into(),
        Server::SelfHosted { url } => host_of(url).into(),
    }
}

/// `Signed in with GitHub`; the server doesn't always say how.
/// A team row's trailing text: `Owner · Team`, `Member · Free`.
fn team_detail(t: &TeamRef) -> String {
    let role = match t.role {
        TeamRole::Owner => "Owner",
        TeamRole::Member => "Member",
    };
    let plan = match t.plan {
        TeamPlan::Free => "Free",
        TeamPlan::Team => "Team",
    };
    format!("{role} · {plan}")
}

/// One row of the menu's Teams section: initial, name, role and plan.
fn team_row(t: &TeamRef) -> Div {
    let detail = team_detail(t);
    let initial: String = t
        .name
        .chars()
        .next()
        .map_or_else(|| "?".to_string(), |c| c.to_uppercase().collect());
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .h(px(36.))
        .px(px(14.))
        .text_size(px(12.))
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(20.))
                .rounded(px(5.))
                .bg(color("surface-200"))
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(initial),
        )
        .child(div().flex_1().min_w_0().truncate().child(t.name.clone()))
        .child(
            div()
                .flex_none()
                .text_color(color("text-tertiary"))
                .child(detail),
        )
}

fn provider_text(a: &Account) -> Option<String> {
    a.provider
        .map(|p| format!("Signed in with {}", method_label(p)))
}

/// A round avatar with `initials`, `size` px across.
pub fn avatar(initials: String, size: f32) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded_full()
        .bg(color("surface-200"))
        .text_color(color("text-primary"))
        .text_size(px((size * 0.4).round()))
        .font_weight(FontWeight::SEMIBOLD)
        .child(initials)
}

/// A 6px state dot.
pub fn dot(token: &'static str) -> Div {
    div()
        .flex_none()
        .size(px(6.))
        .rounded_full()
        .bg(color(token))
}

const MENU_WIDTH: f32 = 320.;
/// A press that closed the menu also lands on the button: this long, it
/// doesn't reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

impl Workspace {
    pub(super) fn account_state(&self) -> Option<AccountState> {
        AccountState::of(&*self.backend, self.connection())
    }

    /// The avatar, or `Offline` / `Sign in` (the title bar's far right).
    pub(super) fn render_account_chip(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let state = self.account_state()?;
        let open = self.account_menu;
        let chip = match &state {
            AccountState::Connected { account, .. } | AccountState::Expired { account, .. } => {
                let expired = matches!(state, AccountState::Expired { .. });
                let ring = if expired {
                    "warning"
                } else if open {
                    "text-primary"
                } else {
                    "border-200"
                };
                div()
                    .id("account-chip")
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(32.))
                    .rounded_full()
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-100")))
                    // The ring is a child, not this div's border: a border
                    // paints over the children, the dot included.
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_full()
                            .border_1()
                            .border_color(color(ring)),
                    )
                    .child(avatar(initials(account), 26.))
                    .child(
                        div()
                            .absolute()
                            .right(px(-1.))
                            .bottom(px(-1.))
                            .size(px(10.))
                            .rounded_full()
                            .border_2()
                            .border_color(color("bg-100"))
                            .bg(color(state.dot())),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_account_menu(cx)))
            }
            AccountState::Offline => chip_button("account-chip", "Offline", false)
                .child(glyph(icons::FOLDER).size(px(13.)).flex_none())
                .child("Offline")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_sign_in(None, window, cx);
                })),
            AccountState::SignedOut { .. } => chip_button("account-chip", "Sign in", true)
                .child("Sign in")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_sign_in(None, window, cx);
                })),
        };
        let chip = chip
            .role(Role::Button)
            .test_support()
            .aria_label(state.label());
        let menu =
            (open && state.account().is_some()).then(|| self.render_account_menu(&state, cx));
        Some(
            div()
                .relative()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(12.))
                .child(div().w(px(1.)).h(px(20.)).bg(color("border-100")))
                .child(chip)
                .children(menu.map(|m| deferred(m).with_priority(1))),
        )
    }

    pub(super) fn toggle_account_menu(&mut self, cx: &mut Context<Self>) {
        let just_closed = self
            .account_menu_closed_at
            .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
        self.account_menu = !self.account_menu && !just_closed;
        self.account_menu_closed_at = None;
        cx.notify();
    }

    pub(super) fn close_account_menu(&mut self, cx: &mut Context<Self>) {
        if self.account_menu {
            self.account_menu = false;
            cx.notify();
        }
    }

    fn render_account_menu(
        &self,
        state: &AccountState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let (account, server) = match state {
            AccountState::Connected { account, server }
            | AccountState::Expired { account, server } => (account.clone(), server.clone()),
            _ => unreachable!("the menu needs an account"),
        };
        let expired = matches!(state, AccountState::Expired { .. });
        // An ended session's teams may be stale: shown once signed in again.
        let teams = if expired {
            Vec::new()
        } else {
            account.teams.clone()
        };
        let current = server.url().to_string();
        let others: Vec<(String, Account)> = self
            .backend
            .accounts()
            .into_iter()
            .filter(|(url, _)| *url != current)
            .collect();
        let named = account.name != account.email;

        let who = if named {
            format!("{}, {}", account.name, account.email)
        } else {
            account.email.clone()
        };
        let header = div()
            .id("account-menu-who")
            .role(Role::Group)
            .test_support()
            .aria_label(who)
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(14.))
            .pt(px(14.))
            .pb(px(12.))
            .child(avatar(initials(&account), 36.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .min_w_0()
                    .when(named, |d| {
                        d.child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(account.name.clone()),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color(if named {
                                "text-secondary"
                            } else {
                                "text-primary"
                            }))
                            .text_ellipsis()
                            .child(account.email.clone()),
                    ),
            );

        let server_box = if expired {
            div()
                .id("account-menu-server")
                .role(Role::Status)
                .test_support()
                .aria_label(format!("Your {} session ended", long_name(&server)))
                .mx(px(10.))
                .mb(px(10.))
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .border_1()
                .border_color(color("warning-border"))
                .rounded(px(8.))
                .bg(color("warning-subtle"))
                .text_size(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(dot("warning"))
                        .child(format!("Your {} session ended", long_name(&server))),
                )
                .child(
                    div()
                        .pl(px(14.))
                        .line_height(relative(1.5))
                        .text_color(color("text-secondary"))
                        .child("Changes stay on this Mac and sync once you're back."),
                )
                .child(
                    div().pl(px(14.)).child(
                        chip_button("account-sign-in-again", "Sign in again", true)
                            .role(Role::Button)
                            .test_support()
                            .aria_label("Sign in again")
                            .child("Sign in again")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.account_menu = false;
                                this.open_sign_in(None, window, cx);
                            })),
                    ),
                )
        } else {
            let status = match provider_text(&account) {
                Some(p) => format!("{}, connected. {p}", long_name(&server)),
                None => format!("{}, connected", long_name(&server)),
            };
            div()
                .id("account-menu-server")
                .role(Role::Status)
                .test_support()
                .aria_label(status)
                .mx(px(10.))
                .mb(px(10.))
                .px(px(12.))
                .py(px(10.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .border_1()
                .border_color(color("border-100"))
                .rounded(px(8.))
                .bg(color("bg-100"))
                .text_size(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(dot("success"))
                        .child(div().flex_1().child(long_name(&server)))
                        .child(div().text_color(color("text-tertiary")).child("Connected")),
                )
                .children(provider_text(&account).map(|t| {
                    div()
                        .pl(px(14.))
                        .text_color(color("text-secondary"))
                        .child(t)
                }))
        };

        let other_rows = others.into_iter().map(|(url, a)| {
            let target = Server::for_url(&url);
            let id = SharedString::from(format!("account-switch-{}", host_of(&url)));
            div()
                .id(id)
                .role(Role::MenuItem)
                .test_support()
                .aria_label(format!("Switch to {} as {}", long_name(&target), a.email))
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(40.))
                .mx(px(6.))
                .px(px(8.))
                .rounded(px(6.))
                .text_color(color("text-secondary"))
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-300")))
                .child(
                    glyph(if target == Server::Cloud {
                        icons::CLOUD
                    } else {
                        icons::SERVER
                    })
                    .size(px(14.))
                    .flex_none(),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-primary"))
                                .text_ellipsis()
                                .child(long_name(&target)),
                        )
                        .child(div().text_size(px(11.)).text_ellipsis().child(a.email)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child("Switch"),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.switch_server(target.clone(), window, cx);
                }))
        });
        let has_others = self.backend.accounts().iter().any(|(u, _)| *u != current);

        let item = |id: &'static str, icon: &'static str, label: String, cx: &mut Context<Self>| {
            div()
                .id(id)
                .role(Role::MenuItem)
                .test_support()
                .aria_label(label.clone())
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(32.))
                .px(px(8.))
                .rounded(px(6.))
                .text_color(color("text-secondary"))
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-300")).text_color(color("text-primary")))
                .child(glyph(icon).size(px(14.)).flex_none())
                .child(div().flex_1().child(label))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .map(|d| {
                    d.on_click(cx.listener(move |this, _, window, cx| {
                        this.account_menu = false;
                        match id {
                            "account-settings" => {
                                this.open_app_settings(AppPage::Accounts, window, cx)
                            }
                            "account-change-server" => {
                                this.open_sign_in(None, window, cx);
                            }
                            _ => this.sign_out_current(window, cx),
                        }
                        cx.notify();
                    }))
                })
        };

        div()
            .id("account-menu")
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.account_menu = false;
                this.account_menu_closed_at = Some(Instant::now());
                cx.notify();
            }))
            .role(Role::Menu)
            .test_support()
            .aria_label("Account")
            .occlude()
            .absolute()
            .top(px(40.))
            .right_0()
            .w(px(MENU_WIDTH))
            .flex()
            .flex_col()
            .bg(color("surface-raised"))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(10.))
            .overflow_hidden()
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.6),
                offset: point(px(0.), px(12.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }])
            .text_size(px(13.))
            .text_color(color("text-primary"))
            .child(header)
            .child(server_box)
            .when(!teams.is_empty(), |m| {
                let rows = teams.iter().enumerate().map(|(i, t)| {
                    let slug = t.slug.clone();
                    // Opens the team's collections (Open from Cloud).
                    team_row(t)
                        .id(SharedString::from(format!("account-team-{i}")))
                        .test_support()
                        .aria_label(format!("{}, {}", t.name, team_detail(t)))
                        .cursor_pointer()
                        .hover(|s| s.bg(color("surface-100")))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.toggle_account_menu(cx);
                            this.open_from_cloud(Some(&slug), window, cx);
                        }))
                });
                m.child(div().h(px(1.)).bg(color("border-100")))
                    .child(
                        div()
                            .id("account-menu-teams")
                            .role(Role::Heading)
                            .test_support()
                            .aria_label("Teams")
                            .px(px(14.))
                            .pt(px(10.))
                            .pb(px(4.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(color("text-tertiary"))
                            .child("Teams"),
                    )
                    .children(rows)
                    .child(div().h(px(6.)))
            })
            .when(has_others, |m| {
                m.child(div().h(px(1.)).bg(color("border-100")))
                    .child(
                        div()
                            .id("account-menu-others")
                            .role(Role::Heading)
                            .test_support()
                            .aria_label("Also signed in to")
                            .px(px(14.))
                            .pt(px(10.))
                            .pb(px(4.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(color("text-tertiary"))
                            .child("Also signed in to"),
                    )
                    .children(other_rows)
                    .child(div().h(px(6.)))
            })
            .child(div().h(px(1.)).bg(color("border-100")))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p(px(6.))
                    .child(item(
                        "account-settings",
                        icons::USER,
                        "Accounts & sync…".into(),
                        cx,
                    ))
                    .child(item(
                        "account-change-server",
                        icons::SYNC,
                        "Change server…".into(),
                        cx,
                    ))
                    .when(!expired, |d| {
                        d.child(item(
                            "account-sign-out",
                            icons::LINK,
                            format!("Sign out of {}", long_name(&server)),
                            cx,
                        ))
                    }),
            )
    }

    /// Ends the session on the current server, then refreshes like a
    /// Settings sign-out.
    fn sign_out_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let server = self.backend.server();
        let call = backend_call(cx, &self.backend, move |b| b.sign_out(server.url()));
        self.account_task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(()) => this.account_changed(window, cx),
                Err(e) => notify_error(format!("Couldn't sign out: {e}"), window, cx),
            })
            .ok();
        }));
    }

    /// Makes `server` the one this studio publishes to; its session is
    /// already there (sessions are per server).
    fn switch_server(&mut self, server: Server, window: &mut Window, cx: &mut Context<Self>) {
        self.account_menu = false;
        let call = backend_call(cx, &self.backend, move |b| b.set_server(server));
        self.account_task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(()) => this.account_changed(window, cx),
                Err(e) => notify_error(format!("Couldn't switch servers: {e}"), window, cx),
            })
            .ok();
        }));
        cx.notify();
    }

    /// After a sign-out or a server switch: sync and chrome follow.
    pub(super) fn account_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.start_sync(window, cx);
        self.sync_chrome(window, cx);
    }
}

/// A 28px button in the title bar: outlined, or `primary`.
fn chip_button(id: &'static str, _label: &'static str, primary: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .h(px(28.))
        .px(px(10.))
        .rounded(px(6.))
        .text_size(px(12.))
        .cursor_pointer()
        .map(|d| {
            if primary {
                d.bg(color("primary"))
                    .text_color(color("on-primary"))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|s| s.bg(color("primary-hover")))
            } else {
                d.border_1()
                    .border_color(color("border-200"))
                    .bg(color("bg-100"))
                    .text_color(color("text-secondary"))
                    .hover(|s| s.bg(color("surface-100")))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{
        AccountState, TeamPlan, TeamRef, TeamRole, initials, long_name, provider_text, short_name,
        team_detail,
    };
    use crate::model::backend::{Account, Server, SignInMethod};

    fn acct(name: &str, email: &str) -> Account {
        Account {
            name: name.into(),
            email: email.into(),
            provider: None,
            teams: Vec::new(),
        }
    }

    #[test]
    fn initials_come_from_the_name_or_the_email() {
        assert_eq!(initials(&acct("Jamie Rivera", "jamie@acme.dev")), "JR");
        assert_eq!(initials(&acct("Ada Byron Lovelace", "a@x.test")), "AL");
        assert_eq!(initials(&acct("Maria", "maria@example.com")), "M");
        // No name on the server: the account's name is its email.
        assert_eq!(initials(&acct("maria@acme.dev", "maria@acme.dev")), "M");
        assert_eq!(initials(&acct("", "zoe@acme.dev")), "Z");
        assert_eq!(initials(&acct("élodie durand", "e@x.test")), "ÉD");
    }

    #[test]
    fn server_names() {
        let own = Server::SelfHosted {
            url: "https://studio.api.example.com".into(),
        };
        assert_eq!(short_name(&Server::Cloud), "Cloud");
        assert_eq!(long_name(&Server::Cloud), "curl-runner Cloud");
        assert_eq!(short_name(&own), "studio.api.example.com");
        assert_eq!(long_name(&own), "studio.api.example.com");
    }

    #[test]
    fn status_item_names_the_server() {
        let a = acct("Jamie Rivera", "jamie@acme.dev");
        let own = Server::SelfHosted {
            url: "https://curl.example.com".into(),
        };
        let connected = AccountState::Connected {
            account: a.clone(),
            server: Server::Cloud,
        };
        assert_eq!(connected.status_text(), "Cloud");
        assert_eq!(
            connected.status_label(),
            "curl-runner Cloud. Open Accounts & sync"
        );
        assert_eq!(connected.dot(), "success");
        let expired = AccountState::Expired {
            account: a,
            server: own.clone(),
        };
        assert_eq!(expired.status_text(), "curl.example.com");
        assert_eq!(expired.dot(), "warning");
        assert_eq!(
            AccountState::SignedOut { server: own }.status_text(),
            "curl.example.com"
        );
        assert_eq!(AccountState::Offline.status_text(), "Offline");
        assert_eq!(
            AccountState::Offline.status_label(),
            "Offline. Open Accounts & sync"
        );
    }

    #[cfg(not(feature = "server"))]
    #[test]
    fn nothing_shows_without_a_server() {
        use super::super::state::Connection;
        let b = crate::model::backend::NoServer;
        assert_eq!(AccountState::of(&b, Some(Connection::Online)), None);
        assert_eq!(AccountState::of(&b, Some(Connection::Offline)), None);
    }

    #[test]
    fn provider_shows_only_when_known() {
        let mut a = acct("Ada", "ada@example.test");
        assert_eq!(provider_text(&a), None);
        a.provider = Some(SignInMethod::Github);
        assert_eq!(provider_text(&a).as_deref(), Some("Signed in with GitHub"));
    }

    #[test]
    fn team_rows_name_role_and_plan() {
        let acme = TeamRef {
            slug: "acme".into(),
            name: "Acme".into(),
            role: TeamRole::Owner,
            plan: TeamPlan::Team,
        };
        assert_eq!(team_detail(&acme), "Owner · Team");
        let beta = TeamRef {
            slug: "beta".into(),
            name: "Beta".into(),
            role: TeamRole::Member,
            plan: TeamPlan::Free,
        };
        assert_eq!(team_detail(&beta), "Member · Free");
    }
}

/// The SignedIn boards in a window: the avatar, its menu and the status
/// line, by state.
#[cfg(all(test, feature = "server"))]
mod ui_tests {
    use super::super::state::{Connection, StateStore};
    use super::super::workspace::{Workspace, test_secrets};
    use crate::model::backend::{Backend, Server, test_backend, testing};
    use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::sync::Arc;

    const SIGNED_IN: &str = "Signed in as jamie@acme.dev on curl-runner Cloud. Open account menu";

    /// Welcome, with the sign-in screen answered as `connection`.
    fn open(
        cx: &mut TestAppContext,
        connection: Connection,
        backend: Arc<dyn Backend>,
    ) -> (AnyWindowHandle, Entity<Workspace>, tempfile::TempDir) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.set_connection(Some(connection));
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
                cx.new(|cx| {
                    let mut ws = Workspace::new(None, store, test_secrets(), backend, window, cx);
                    ws.first_launch(window, cx);
                    ws
                })
            })
            .unwrap()
        });
        (window, ws, state)
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
            window.try_find_by_label(label).is_some()
        })
        .unwrap()
    }

    /// `label` under the element `scope` (an id).
    fn shows_in(
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

    fn menu_open(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> bool {
        ws.read_with(cx, |ws, _| ws.account_menu)
    }

    #[gpui_kit::test]
    fn signed_in_shows_who_in_the_title_and_status_bars(cx: &mut TestAppContext) {
        let backend = test_backend();
        testing::sign_in(&*backend);
        let (window, ws, _state) = open(cx, Connection::Online, backend);
        assert!(shows_in(cx, window, "title-bar", SIGNED_IN));
        // The avatar alone says who: the status bar names the server.
        assert!(!shows_in(cx, window, "status-bar", SIGNED_IN));
        assert!(!menu_open(cx, &ws));
        click(cx, window, "account-chip");
        assert!(menu_open(cx, &ws));
        assert!(shows(cx, window, "Jamie Rivera, jamie@acme.dev"));
        assert!(shows(
            cx,
            window,
            "curl-runner Cloud, connected. Signed in with GitHub"
        ));
        assert!(shows(cx, window, "Teams"));
        assert!(shows(cx, window, "Acme, Owner · Team"));
        assert!(shows(cx, window, "Accounts & sync…"));
        assert!(shows(cx, window, "Sign out of curl-runner Cloud"));
        // Esc closes it.
        cx.simulate_keystrokes(window, "escape");
        assert!(!menu_open(cx, &ws));
    }

    #[gpui_kit::test]
    fn status_server_opens_accounts(cx: &mut TestAppContext) {
        use super::super::settings_view::AppPage;
        let backend = test_backend();
        testing::sign_in(&*backend);
        let (window, ws, _state) = open(cx, Connection::Online, backend);
        assert!(shows_in(
            cx,
            window,
            "status-bar",
            "curl-runner Cloud. Open Accounts & sync"
        ));
        click(cx, window, "status-server");
        assert!(!menu_open(cx, &ws));
        ws.read_with(cx, |ws, cx| {
            let view = ws.app_settings.clone().expect("Settings is open");
            assert_eq!(view.read(cx).page(), AppPage::Accounts);
        });
    }

    fn refreshes(backend: &Arc<dyn Backend>) -> usize {
        let recorded = backend.dev().expect("the mock").recorded();
        recorded
            .iter()
            .filter(|c| c.starts_with("refresh_account("))
            .count()
    }

    fn open_accounts(cx: &mut TestAppContext, window: AnyWindowHandle, ws: &Entity<Workspace>) {
        use super::super::settings_view::AppPage;
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_app_settings(AppPage::Accounts, window, cx)
            })
        })
        .unwrap();
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn opening_accounts_asks_the_server_who_you_are(cx: &mut TestAppContext) {
        let backend = test_backend();
        testing::sign_in(&*backend);
        let (window, ws, _state) = open(cx, Connection::Online, backend.clone());
        assert_eq!(refreshes(&backend), 0);
        open_accounts(cx, window, &ws);
        assert_eq!(refreshes(&backend), 1);
        // Still signed in, teams and all.
        assert_eq!(backend.account().unwrap().teams.len(), 1);
    }

    #[gpui_kit::test]
    fn offline_or_signed_out_asks_nothing(cx: &mut TestAppContext) {
        let backend = test_backend();
        let (window, ws, _state) = open(cx, Connection::Online, backend.clone());
        open_accounts(cx, window, &ws);
        let offline = test_backend();
        testing::sign_in(&*offline);
        let (window2, ws2, _state2) = open(cx, Connection::Offline, offline.clone());
        open_accounts(cx, window2, &ws2);
        assert_eq!((refreshes(&backend), refreshes(&offline)), (0, 0));
    }

    #[gpui_kit::test]
    fn sign_out_from_the_menu(cx: &mut TestAppContext) {
        let backend = test_backend();
        testing::sign_in(&*backend);
        let (window, ws, _state) = open(cx, Connection::Online, backend.clone());
        click(cx, window, "account-chip");
        click(cx, window, "account-sign-out");
        assert!(backend.account().is_none());
        assert!(!menu_open(cx, &ws));
        let signed_out = "Not signed in to curl-runner Cloud. Sign in";
        assert!(shows_in(cx, window, "title-bar", signed_out));
        assert!(shows_in(
            cx,
            window,
            "status-bar",
            "curl-runner Cloud. Open Accounts & sync"
        ));
        // `Sign in` opens the sign-in screen.
        click(cx, window, "account-chip");
        assert!(ws.read_with(cx, |ws, _| ws.sign_in_shown()));
    }

    #[gpui_kit::test]
    fn an_expired_session_asks_to_sign_in_again(cx: &mut TestAppContext) {
        let backend = test_backend();
        testing::sign_in(&*backend);
        let (window, ws, _state) = open(cx, Connection::Online, backend.clone());
        backend.dev().unwrap().expire_session();
        let expired = "Session expired for jamie@acme.dev on curl-runner Cloud. Open account menu";
        assert!(shows_in(cx, window, "title-bar", expired));
        click(cx, window, "account-chip");
        assert!(shows(cx, window, "Your curl-runner Cloud session ended"));
        assert!(!shows(cx, window, "Sign out of curl-runner Cloud"));
        click(cx, window, "account-sign-in-again");
        assert!(!menu_open(cx, &ws));
        assert!(ws.read_with(cx, |ws, _| ws.sign_in_shown()));
    }

    #[gpui_kit::test]
    fn offline_shows_a_pill_that_opens_sign_in(cx: &mut TestAppContext) {
        let (window, ws, _state) = open(cx, Connection::Offline, test_backend());
        assert!(shows_in(
            cx,
            window,
            "title-bar",
            "Offline. Sign in to sync"
        ));
        click(cx, window, "account-chip");
        assert!(!menu_open(cx, &ws));
        assert!(ws.read_with(cx, |ws, _| ws.sign_in_shown()));
    }

    #[gpui_kit::test]
    fn switch_to_another_server_signed_in(cx: &mut TestAppContext) {
        let backend = test_backend();
        let own = Server::SelfHosted {
            url: "https://curl-runner.example.com".into(),
        };
        futures::executor::block_on(backend.set_server(own.clone())).unwrap();
        testing::sign_in_by_email(&*backend);
        futures::executor::block_on(backend.set_server(Server::Cloud)).unwrap();
        testing::sign_in(&*backend);
        let (window, _ws, _state) = open(cx, Connection::Online, backend.clone());
        click(cx, window, "account-chip");
        assert!(shows(cx, window, "Also signed in to"));
        assert!(shows(
            cx,
            window,
            "Switch to curl-runner.example.com as maria@example.com"
        ));
        click(cx, window, "account-switch-curl-runner.example.com");
        assert_eq!(backend.server(), own);
        assert!(shows_in(
            cx,
            window,
            "title-bar",
            "Signed in as maria@example.com on curl-runner.example.com. Open account menu"
        ));
    }
}
