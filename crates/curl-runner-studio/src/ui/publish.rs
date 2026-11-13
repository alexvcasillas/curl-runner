//! Publish from the studio (boards PublishFirst and PublishStates): the
//! sidebar foot for a folder that publishes, the first-time dialog (team,
//! collection name, what goes up), the refusals and the toasts. The checks
//! are the CLI's (`curl_runner_cloud_client::publish::check`), run off the
//! UI thread; nothing is sent until they pass.

use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{Observed, label_text, primary_button, small_button, switch};
use super::sync::sync_dot;
use super::workspace::Workspace;
use crate::model::backend::{
    BackendError, CollectionSource, PublishResponse, RemoteCollection, TeamRef,
};
use crate::model::publish::{
    CONFIG_FILE, CloudLink, PublishLine, Summary, folder_git, published, save_cloud_block,
    suggest_slug,
};
use curl_runner_cloud_client::publish::{Checked, Problem, Target, check, diff};
use curl_runner_cloud_proto::{Plan, PublishStatus};
use curl_runner_core::cloud::{load_cloud_config, scan_bundle, valid_slug};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::notification::{Notification, NotificationType};
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
#[cfg(all(test, feature = "server"))]
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

const DIALOG_W: f32 = 560.;
const DIALOG_TOP: f32 = 60.;
/// A pause after the last edit before the folder is compared again.
const RECHECK_DELAY: Duration = Duration::from_millis(600);
/// A pause after the last keystroke before the name is looked up.
const LOOKUP_DELAY: Duration = Duration::from_millis(300);
const SLUG_HELP: &str = "Suggested from the folder name. Lowercase letters, numbers and dashes. It's in every link, so pick one you'll keep.";
const LEFT_OUT: &str = "Left out: .env*, dotfiles and anything in .gitignore. Values in your Keychain never leave this Mac.";
const YAML_HELP: &str =
    "So curl-runner publish in CI publishes to the same collection. Commit it with your requests.";
const SOURCE_NOTE: &str = "Source: this folder (git). Publish again from here or from CI.";
const CHECKS_NOTE: &str = "Same checks as curl-runner publish. Nothing was sent.";
const NO_TEAM_BODY: &str = "Collections belong to a team. Create one on curl-runner Cloud, then come back and publish. It takes a minute and Free is enough.";

/// The workspace's publish state for the open folder.
#[derive(Default)]
pub struct PublishUi {
    /// What the foot says; `None` until first worked out.
    pub(super) line: Option<PublishLine>,
    /// The current version, as last asked.
    pub(super) current: Option<u32>,
    refresh: Option<Task<()>>,
    job: Option<Task<()>>,
    /// The version being made, while a publish is under way.
    pub(super) publishing: Option<u32>,
    pub(super) view: Option<WeakEntity<PublishView>>,
    _events: Option<Subscription>,
    /// The collection's sharing as last read: access and people, for the
    /// title bar's chip.
    pub(super) share: Option<(crate::model::backend::LinkAccess, usize)>,
    pub(super) share_task: Option<Task<()>>,
    pub(super) share_view: Option<WeakEntity<super::share_cloud::ShareCloudView>>,
    pub(super) share_events: Option<Subscription>,
}

/// One problem as the dialog lists it: the file it opens, and the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemRow {
    pub file: Option<String>,
    pub text: String,
}

/// The rows for what the checks found that stops a publish. A bad slug is
/// the dialog's to say, next to the field.
pub fn problem_rows(problems: &[Problem]) -> Vec<ProblemRow> {
    let row = |file: Option<&str>, text: String| ProblemRow {
        file: file.map(str::to_string),
        text,
    };
    let mut out = Vec::new();
    for p in problems.iter().filter(|p| p.fails()) {
        match p {
            Problem::DottedConfig(name) => out.push(row(
                None,
                format!(
                    "Rename {name} to {}: files starting with a dot aren't published.",
                    name.trim_start_matches('.')
                ),
            )),
            Problem::Config(m) => out.push(row(Some(CONFIG_FILE), format!("{CONFIG_FILE}: {m}"))),
            Problem::NoCollection | Problem::InvalidSlug(_) => {}
            Problem::Scan(m) => out.push(row(None, m.clone())),
            Problem::TooBig(f) => out.push(row(Some(f), format!("{f} is larger than 1 MiB."))),
            Problem::Unreadable(f) => out.push(row(Some(f), format!("Can't read {f}."))),
            Problem::OutsideRoot(_) => {}
            Problem::NoFiles => out.push(row(None, "There are no YAML files to publish.".into())),
            Problem::Lint { path, issues, .. } => {
                for i in issues
                    .iter()
                    .filter(|i| i.severity == curl_runner_core::lint::Severity::Error)
                {
                    out.push(row(Some(path), format!("{path}: {}", i.message)));
                }
            }
            Problem::LintCrash { path, message } => {
                out.push(row(Some(path), format!("{path}: {message}")))
            }
            Problem::Secret { file, name } => out.push(row(
                Some(file),
                format!(
                    "{file} gives the secret {name} a value. Use ${{{name}}} and keep the value in your Keychain."
                ),
            )),
        }
    }
    out
}

/// `Fix 2 problems to publish`.
fn problems_title(n: usize) -> String {
    if n == 1 {
        "Fix 1 problem to publish".into()
    } else {
        format!("Fix {n} problems to publish")
    }
}

/// The success toast's line: `3 requests changed, 1 added.` (files, as
/// the server counts them).
pub fn changes_line(r: &PublishResponse) -> String {
    let c = &r.changed;
    let mut parts = Vec::new();
    let count = |n: usize, what: &str| {
        if n == 1 {
            format!("1 file {what}")
        } else {
            format!("{n} files {what}")
        }
    };
    if !c.changed.is_empty() {
        parts.push(count(c.changed.len(), "changed"));
    }
    if !c.added.is_empty() {
        parts.push(if parts.is_empty() {
            count(c.added.len(), "added")
        } else {
            format!("{} added", c.added.len())
        });
    }
    if !c.removed.is_empty() {
        parts.push(if parts.is_empty() {
            count(c.removed.len(), "removed")
        } else {
            format!("{} removed", c.removed.len())
        });
    }
    let what = if parts.is_empty() {
        String::new()
    } else {
        format!("{}. ", parts.join(", "))
    };
    format!("{what}People with the link get it as an update.")
}

/// The board's `.btn`: 32px, 13px text.
fn dialog_button(id: &'static str, label: impl Into<SharedString>) -> Button {
    Button::new(id)
        .h(px(32.))
        .px(px(12.))
        .text_size(px(13.))
        .label(label.into())
}

fn note(tone: &'static str, id: &'static str, text: String) -> Observed {
    let (border, bg, fg, icon) = match tone {
        "danger" => ("danger-border", "danger-subtle", "danger", icons::ERROR),
        _ => (
            "warning-border",
            "warning-subtle",
            "warning",
            icons::WARNING,
        ),
    };
    div()
        .id(id)
        .role(if tone == "danger" {
            Role::Alert
        } else {
            Role::Status
        })
        .test_support()
        .aria_label(text.clone())
        .flex()
        .gap(px(10.))
        .px(px(14.))
        .py(px(12.))
        .border_1()
        .border_color(color(border))
        .bg(color(bg))
        .rounded(px(8.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .child(
            glyph(icon)
                .size(px(14.))
                .flex_none()
                .mt(px(2.))
                .text_color(color(fg)),
        )
        .child(div().flex_1().min_w_0().child(text))
}

fn team_label(t: &TeamRef) -> String {
    let plan = match t.plan {
        Plan::Free => "Free",
        Plan::Team => "Team plan",
    };
    format!("{} · {plan}", t.name)
}

fn initial(name: &str) -> String {
    name.chars()
        .next()
        .map_or_else(|| "?".into(), |c| c.to_uppercase().collect())
}

/// What the first-time dialog was opened with.
pub struct PublishSetup {
    pub root: PathBuf,
    /// The collection's display name (`Payments API`).
    pub name: String,
    pub teams: Vec<TeamRef>,
    /// A slug the folder already names (`cloud.collection`), else one
    /// from its name.
    pub slug: String,
    pub backend: std::sync::Arc<dyn crate::model::backend::Backend>,
}

pub enum PublishEvent {
    Close,
    /// Publish `target` as `team/slug`; `save_yaml` writes the `cloud:`
    /// block first.
    Publish {
        team: String,
        slug: String,
        target: Box<Target>,
        save_yaml: bool,
        next: u32,
    },
    OpenFile(PathBuf),
}

/// The name's lookup on the server.
#[derive(Debug, Clone, PartialEq)]
enum Lookup {
    Checking,
    Free,
    Exists(RemoteCollection),
    Failed(String),
}

/// The first-time dialog (board PublishFirst).
pub struct PublishView {
    setup: PublishSetup,
    team: usize,
    suggested: String,
    slug: Entity<InputState>,
    lookup: Lookup,
    checked: Option<Checked>,
    save_yaml: bool,
    dashboard: Option<String>,
    /// Presses Publish once it can (`CURL_RUNNER_STUDIO_SUBMIT`, dev).
    pub(super) auto_submit: bool,
    _tasks: Vec<Task<()>>,
    lookup_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<PublishEvent> for PublishView {}

impl PublishView {
    pub fn new(setup: PublishSetup, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let suggested = setup.slug.clone();
        let slug = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("payments-api")
                .default_value(suggested.clone())
        });
        let sub = cx.subscribe(&slug, |this: &mut Self, _, e: &InputEvent, cx| {
            if matches!(e, InputEvent::Change) {
                this.look_up(cx);
            }
        });
        let root = setup.root.clone();
        let checks = cx.background_spawn(async move { check(&root, Some("x")) });
        let check_task = cx.spawn(async move |this, cx| {
            let checked = checks.await;
            this.update(cx, |this, cx| {
                this.checked = Some(checked);
                cx.notify();
            })
            .ok();
        });
        let dash = backend_call(cx, &setup.backend, |b| b.dashboard_url());
        let dash_task = cx.spawn(async move |this, cx| {
            if let Ok(url) = dash.await {
                this.update(cx, |this, cx| {
                    this.dashboard = Some(url);
                    cx.notify();
                })
                .ok();
            }
        });
        let mut view = PublishView {
            setup,
            team: 0,
            suggested,
            slug,
            lookup: Lookup::Checking,
            checked: None,
            save_yaml: true,
            dashboard: None,
            auto_submit: false,
            _tasks: vec![check_task, dash_task],
            lookup_task: None,
            _subs: vec![sub],
        };
        view.look_up_now(cx);
        view
    }

    fn slug(&self, cx: &App) -> String {
        self.slug.read(cx).value().trim().to_string()
    }

    fn team(&self) -> Option<&TeamRef> {
        self.setup.teams.get(self.team)
    }

    /// The name's lookup, after a pause in typing.
    fn look_up(&mut self, cx: &mut Context<Self>) {
        self.lookup = Lookup::Checking;
        self.lookup_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LOOKUP_DELAY).await;
            this.update(cx, |this, cx| this.look_up_now(cx)).ok();
        }));
        cx.notify();
    }

    fn look_up_now(&mut self, cx: &mut Context<Self>) {
        let slug = self.slug(cx);
        let Some(team) = self.team().map(|t| t.slug.clone()) else {
            return;
        };
        if !valid_slug(&slug) {
            self.lookup = Lookup::Free;
            cx.notify();
            return;
        }
        self.lookup = Lookup::Checking;
        let call = backend_call(cx, &self.setup.backend, move |b| {
            b.remote_collection(&team, &slug)
        });
        self.lookup_task = Some(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| {
                this.lookup = match r {
                    Ok(None) => Lookup::Free,
                    Ok(Some(c)) => Lookup::Exists(c),
                    Err(e) => Lookup::Failed(e.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn set_team(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.team = ix;
        self.look_up_now(cx);
    }

    /// The version this publish makes.
    fn next(&self) -> u32 {
        match &self.lookup {
            Lookup::Exists(c) => c.current.as_ref().map_or(1, |v| v.number + 1),
            _ => 1,
        }
    }

    fn problems(&self) -> Vec<ProblemRow> {
        self.checked
            .as_ref()
            .map(|c| problem_rows(&c.problems))
            .unwrap_or_default()
    }

    /// Why Publish is off, if it is.
    fn blocked(&self, cx: &App) -> Option<String> {
        let slug = self.slug(cx);
        if !valid_slug(&slug) {
            return Some(format!(
                "`{slug}` isn't a valid name. Use lowercase letters, numbers and dashes."
            ));
        }
        match &self.lookup {
            Lookup::Checking => return Some("Checking the name…".into()),
            Lookup::Exists(c) if c.source == CollectionSource::Cloud => {
                return Some("This collection syncs from the studio.".into());
            }
            Lookup::Failed(e) => return Some(e.clone()),
            _ => {}
        }
        match &self.checked {
            None => Some("Checking the folder…".into()),
            Some(c) if c.target.is_none() => Some("Fix the problems first.".into()),
            Some(_) => None,
        }
    }

    fn publish(&mut self, cx: &mut Context<Self>) {
        if self.blocked(cx).is_some() {
            return;
        }
        let (Some(team), Some(target)) = (
            self.team().map(|t| t.slug.clone()),
            self.checked.as_ref().and_then(|c| c.target.clone()),
        ) else {
            return;
        };
        cx.emit(PublishEvent::Publish {
            team,
            slug: self.slug(cx),
            target: Box::new(target),
            save_yaml: self.save_yaml,
            next: self.next(),
        });
    }

    fn host(&self) -> String {
        let url = self
            .dashboard
            .clone()
            .unwrap_or_else(|| "https://cloud.curl-runner.com".into());
        url.split_once("://")
            .map_or(url.clone(), |(_, h)| h.to_string())
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let title = format!("Publish {}", self.setup.name);
        div()
            .flex()
            .items_start()
            .gap(px(12.))
            .px(px(22.))
            .pt(px(18.))
            .pb(px(4.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1().min_w_0()
                    .gap(px(6.))
                    .child(
                        div()
                            .id("publish-title")
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(relative(1.5))
                            .text_color(color("text-secondary"))
                            .child(format!(
                                "Uploads this folder to curl-runner Cloud as version {}. Your team sees it in the dashboard, and anyone you share it with gets each new version as an update.",
                                self.next()
                            )),
                    ),
            )
            .child(
                div()
                    .id("publish-close")
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
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PublishEvent::Close)))
                    .child(glyph(icons::CROSS).size(px(14.))),
            )
    }

    fn render_team(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let weak = cx.entity().downgrade();
        let teams = self.setup.teams.clone();
        let current = self.team;
        let label = self.team().map(team_label).unwrap_or_default();
        let name = self.team().map(|t| t.name.clone()).unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(label_text("Team"))
            .child(
                Button::new("publish-team")
                    .outline()
                    .h(px(34.))
                    .pl(px(8.))
                    .pr(px(8.))
                    .gap(px(10.))
                    .text_size(px(13.))
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
                            .child(initial(&name)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(color("text-primary"))
                            .child(label),
                    )
                    .child(
                        glyph(icons::CHEVRON_DOWN)
                            .size(px(12.))
                            .text_color(color("text-secondary")),
                    )
                    .dropdown_menu(move |mut menu, _, _| {
                        for (ix, t) in teams.iter().enumerate() {
                            let weak = weak.clone();
                            menu = menu.item(
                                PopupMenuItem::new(team_label(t))
                                    .checked(ix == current)
                                    .on_click(move |_, _, cx| {
                                        weak.update(cx, |this, cx| this.set_team(ix, cx)).ok();
                                    }),
                            );
                        }
                        menu
                    }),
            )
    }

    fn render_slug(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let slug = self.slug(cx);
        let team = self.team().map(|t| t.slug.clone()).unwrap_or_default();
        let invalid = !slug.is_empty() && !valid_slug(&slug);
        let edited = slug != self.suggested;
        let note_el = match &self.lookup {
            _ if invalid => Some(note(
                "danger",
                "publish-slug-invalid",
                format!(
                    "`{slug}` isn't a valid name. Use lowercase letters, numbers and dashes, at most 63."
                ),
            )),
            Lookup::Exists(c) if c.source == CollectionSource::Cloud => Some(note(
                "danger",
                "publish-wrong-source",
                format!(
                    "{team}/{slug} syncs from the studio, so it can't be published from a folder. Pick another name, or switch its source to git in the dashboard."
                ),
            )),
            Lookup::Exists(RemoteCollection {
                current: Some(v), ..
            }) => Some(note(
                "warning",
                "publish-exists",
                format!(
                    "{team}/{slug} already exists (v{}, by {}). Publishing makes version {} from this folder.",
                    v.number,
                    crate::model::publish::publisher_label(&v.publisher, None),
                    v.number + 1
                ),
            )),
            Lookup::Failed(e) => Some(note("warning", "publish-lookup-failed", e.clone())),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(label_text("Collection"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .border_1()
                            .border_color(color(if invalid { "danger" } else { "border-200" }))
                            .rounded(px(6.))
                            .pl(px(10.))
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(color("text-tertiary"))
                                    .font_family(super::app::mono_font())
                                    .child(format!("{}/t/{team}/c/", self.host())),
                            )
                            .child(
                                div().flex_1().min_w_0().child(
                                    Input::new(&self.slug)
                                        .aria_label("Collection name in the cloud")
                                        .appearance(false)
                                        .min_h(px(32.))
                                        .text_size(px(12.)),
                                ),
                            ),
                    )
                    .when(edited, |d| {
                        d.child(
                            small_button("publish-slug-reset", "Reset")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    let s = this.suggested.clone();
                                    this.slug.update(cx, |i, cx| i.set_value(s, window, cx));
                                    this.look_up(cx);
                                }))
                                .child("Reset"),
                        )
                    }),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.5))
                    .text_color(color("text-tertiary"))
                    .child(SLUG_HELP),
            )
            .children(note_el)
    }

    fn render_checks(&self, cx: &mut Context<Self>) -> AnyElement {
        let card = div()
            .id("publish-checks")
            .test_support()
            .flex()
            .flex_col()
            .border_1()
            .border_color(color("border-100"))
            .rounded(px(8.))
            .bg(color("surface-raised"))
            .text_size(px(12.));
        let Some(checked) = &self.checked else {
            return card
                .aria_label("Checking the folder")
                .p(px(12.))
                .text_color(color("text-secondary"))
                .child("Checking the folder…")
                .into_any_element();
        };
        let rows = self.problems();
        if checked.target.is_none() {
            return card
                .aria_label(problems_title(rows.len()))
                .child(
                    div()
                        .px(px(12.))
                        .pt(px(10.))
                        .pb(px(4.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(problems_title(rows.len())),
                )
                .children(problem_list(rows, cx.entity().downgrade()))
                .into_any_element();
        }
        let summary = checked.target.as_ref().map(Summary::of);
        let line = |id: &'static str, text: String| {
            div()
                .id(id)
                .test_support()
                .aria_label(text.clone())
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    glyph(icons::CHECK)
                        .size(px(13.))
                        .text_color(color("success")),
                )
                .child(text)
        };
        card.aria_label("Ready to publish")
            .p(px(12.))
            .gap(px(8.))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child("Ready to publish"),
            )
            .children(
                summary.map(|s| line("publish-summary", format!("{} · {}", s.line(), s.size()))),
            )
            .child(line("publish-no-errors", "No YAML errors".into()))
            .child(line("publish-no-secrets", "No secrets in the files".into()))
            .child(
                div()
                    .pl(px(21.))
                    .line_height(relative(1.5))
                    .text_color(color("text-tertiary"))
                    .child(LEFT_OUT),
            )
            .into_any_element()
    }

    fn render_toggle(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let on = self.save_yaml;
        div()
            .id("publish-save-yaml")
            .role(Role::Switch)
            .test_support()
            .aria_label(format!("Save the name and description to {CONFIG_FILE}"))
            .flex()
            .items_start()
            .gap(px(12.))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.save_yaml = !this.save_yaml;
                cx.notify();
            }))
            .child(div().flex_none().mt(px(2.)).child(switch(on, true)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .text_size(px(13.))
                    .child(format!("Save the name and description to {CONFIG_FILE}"))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(relative(1.5))
                            .text_color(color("text-secondary"))
                            .child(YAML_HELP),
                    ),
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let blocked = self.blocked(cx);
        let label = format!("Publish version {}", self.next());
        let publish = dialog_button("publish-go", label)
            .primary()
            .disabled(blocked.is_some())
            .on_click(cx.listener(|this, _, _, cx| this.publish(cx)));
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .pl(px(22.))
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
                    .child(SOURCE_NOTE),
            )
            .child(
                dialog_button("publish-cancel", "Cancel")
                    .outline()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PublishEvent::Close))),
            )
            .child(publish)
    }
}

impl Render for PublishView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.auto_submit && self.blocked(cx).is_none() {
            self.auto_submit = false;
            cx.defer_in(window, |this, _, cx| this.publish(cx));
        }
        let room = window.viewport_size().height - px(DIALOG_TOP * 2. + 140.);
        let title = format!("Publish {}", self.setup.name);
        div()
            .id("publish-dialog")
            .role(Role::Dialog)
            .test_support()
            .aria_label(title)
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(
                div()
                    .id("publish-body")
                    .max_h(room.max(px(240.)))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .px(px(22.))
                    .pt(px(12.))
                    .pb(px(18.))
                    .child(self.render_team(cx))
                    .child(self.render_slug(cx))
                    .child(self.render_checks(cx))
                    .child(self.render_toggle(cx)),
            )
            .child(self.render_footer(cx))
    }
}

/// The problems, one row each, with `Open` for those in a file.
fn problem_list(rows: Vec<ProblemRow>, view: WeakEntity<PublishView>) -> Vec<AnyElement> {
    rows.into_iter()
        .enumerate()
        .map(|(ix, r)| {
            let open = r.file.clone().map(|f| {
                let view = view.clone();
                small_button(SharedString::from(format!("publish-open-{ix}")), "Open")
                    .on_click(move |_, _, cx| {
                        let f = PathBuf::from(&f);
                        view.update(cx, |_, cx| cx.emit(PublishEvent::OpenFile(f)))
                            .ok();
                    })
                    .child("Open")
            });
            problem_row(ix, &r.text, open).into_any_element()
        })
        .collect()
}

fn problem_row(ix: usize, text: &str, open: Option<impl IntoElement>) -> Observed {
    div()
        .id(("publish-problem", ix))
        .test_support()
        .aria_label(text.to_string())
        .flex()
        .items_start()
        .gap(px(10.))
        .px(px(12.))
        .py(px(10.))
        .border_t_1()
        .border_color(color("border-100"))
        .line_height(relative(1.5))
        .child(
            glyph(icons::ERROR)
                .size(px(14.))
                .flex_none()
                .mt(px(2.))
                .text_color(color("danger")),
        )
        .child(div().flex_1().min_w_0().child(text.to_string()))
        .children(open)
}

/// A small dialog: title, body, footer buttons (board PublishStates, row 2).
fn simple_dialog(
    id: &'static str,
    title: String,
    body: AnyElement,
    note_text: Option<&'static str>,
    buttons: Vec<AnyElement>,
) -> Observed {
    div()
        .id(id)
        .role(Role::Dialog)
        .test_support()
        .aria_label(title.clone())
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(22.))
                .pt(px(18.))
                .pb(px(4.))
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(div().px(px(22.)).pt(px(12.)).pb(px(18.)).child(body))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .pl(px(22.))
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
                        .children(note_text),
                )
                .children(buttons),
        )
}

fn open_dialog(window: &mut Window, cx: &mut App, content: impl Fn() -> AnyElement + 'static) {
    window.open_dialog(cx, move |d, _, _| {
        d.entrance(DialogEntrance::None)
            .w(px(DIALOG_W))
            .margin_top(px(DIALOG_TOP))
            .p_0()
            .close_button(false)
            .on_ok(|_, _, _| false)
            .child(content())
    });
}

/// `team/slug` and the version folder, for links into the dashboard.
fn collection_url(dashboard: &str, team: &str, slug: &str) -> String {
    format!("{dashboard}/t/{team}/c/{slug}")
}

/// What the background refresh works out.
enum Refreshed {
    Line(PublishLine),
    Remote(Option<u32>, PublishLine),
}

impl Workspace {
    /// Whether the open folder publishes (the real server, a folder that
    /// isn't a received copy).
    pub(super) fn publish_applies(&self) -> bool {
        crate::features::SERVER
            && self.backend.publishes()
            && self.state.collection.is_some()
            && !self.pull_only()
    }

    /// Where the open folder publishes on the current server: the notes'
    /// link, its slug overridden by `cloud.collection` in the folder.
    pub(super) fn publish_link(&self, cx: &App) -> Option<CloudLink> {
        let root = &self.state.collection.as_ref()?.root;
        let server = self
            .backend
            .server()
            .url()
            .trim_end_matches('/')
            .to_string();
        let mut link = self.notes.read(cx).cloud.clone()?;
        if link.server != server {
            return None;
        }
        if let Ok(Some(cfg)) = load_cloud_config(root) {
            link.slug = cfg.collection;
        }
        Some(link)
    }

    /// The foot's line now.
    pub(super) fn publish_line(&self, cx: &App) -> PublishLine {
        if self.connection() == Some(super::state::Connection::Offline) {
            return PublishLine::Offline;
        }
        if let Some(n) = self.publish.publishing {
            return PublishLine::Publishing(n);
        }
        if self.backend.account().is_none() {
            return PublishLine::SignIn;
        }
        match (&self.publish.line, self.publish_link(cx)) {
            (Some(line), _) => line.clone(),
            (None, None) => PublishLine::NotPublished,
            (None, Some(_)) => PublishLine::Unchecked("Checking…".into()),
        }
    }

    /// Works the foot out again: the folder against the current version,
    /// off the UI thread. `delay` waits first (edits in a row).
    pub(super) fn refresh_publish(
        &mut self,
        delay: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.publish_applies() || self.backend.account().is_none() {
            self.publish.refresh = None;
            cx.notify();
            return;
        }
        let Some(root) = self.state.collection.as_ref().map(|c| c.root.clone()) else {
            return;
        };
        let Some(link) = self.publish_link(cx) else {
            self.publish.line = Some(PublishLine::NotPublished);
            self.publish.current = None;
            cx.notify();
            return;
        };
        let me = self.backend.account().map(|a| a.email);
        let (team, slug) = (link.team.clone(), link.slug.clone());
        let remote = backend_call(cx, &self.backend, move |b| {
            b.remote_collection(&team, &slug)
        });
        let backend = self.backend.clone();
        self.publish.refresh = Some(cx.spawn_in(window, async move |this, cx| {
            if delay {
                cx.background_executor().timer(RECHECK_DELAY).await;
            }
            let result = match remote.await {
                Err(BackendError::SignedOut) => Refreshed::Line(PublishLine::SignIn),
                Err(e) => Refreshed::Line(PublishLine::Unchecked(e.to_string())),
                Ok(None) => Refreshed::Remote(None, PublishLine::NotPublished),
                Ok(Some(c)) if c.source == CollectionSource::Cloud => {
                    Refreshed::Line(PublishLine::Studio)
                }
                Ok(Some(RemoteCollection { current: None, .. })) => {
                    Refreshed::Remote(None, PublishLine::NotPublished)
                }
                Ok(Some(RemoteCollection {
                    current: Some(v), ..
                })) => {
                    let p = published(&v, me.as_deref());
                    let local = cx
                        .background_executor()
                        .spawn(async move { scan_bundle(&root).ok() })
                        .await;
                    match local {
                        Some(b) if b.hash == v.hash => {
                            Refreshed::Remote(Some(v.number), PublishLine::UpToDate(p))
                        }
                        Some(b) => {
                            let (team, slug, n) = (link.team.clone(), link.slug.clone(), v.number);
                            let files = cx
                                .background_executor()
                                .spawn(async move {
                                    futures::executor::block_on(
                                        backend.remote_files(&team, &slug, n),
                                    )
                                })
                                .await;
                            match files {
                                Ok(remote) => {
                                    let d = diff(&b.files, &remote);
                                    let n = d.added.len() + d.changed.len() + d.removed.len();
                                    if n == 0 {
                                        Refreshed::Remote(Some(v.number), PublishLine::UpToDate(p))
                                    } else {
                                        Refreshed::Remote(Some(v.number), PublishLine::Behind(n, p))
                                    }
                                }
                                Err(e) => Refreshed::Line(PublishLine::Unchecked(e.to_string())),
                            }
                        }
                        None => Refreshed::Line(PublishLine::Unchecked(
                            "Couldn't read the folder.".into(),
                        )),
                    }
                }
            };
            this.update_in(cx, |this, window, cx| {
                let was = this.cloud_synced();
                match result {
                    Refreshed::Line(l) => this.publish.line = Some(l),
                    Refreshed::Remote(n, l) => {
                        this.publish.current = n;
                        this.publish.line = Some(l);
                        // The chip, once per open (not on every edit).
                        if n.is_some() && !delay {
                            this.refresh_team_share(cx);
                        }
                    }
                }
                match (was, this.cloud_synced()) {
                    // Found to sync from the studio: two-way from now on.
                    (false, true) => {
                        this.refresh_team_share(cx);
                        this.begin_two_way(window, cx);
                        this.sync_changed(cx);
                    }
                    // Switched to publishing: the sync stops.
                    (true, false) => {
                        this.stop_two_way();
                        this.sync_changed(cx);
                    }
                    _ => {}
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// Publish (the foot, the palette). Signed out: sign in. No team: the
    /// board's `You're not in a team yet`. The first time here: the
    /// dialog. After that: the checks, then the upload, asking nothing.
    pub(super) fn start_publish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.publish_applies() || self.publish.publishing.is_some() {
            return;
        }
        let Some(account) = self.backend.account() else {
            self.open_sign_in(None, window, cx);
            return;
        };
        if account.teams.is_empty() {
            self.no_team_dialog(window, cx);
            return;
        }
        let Some(c) = self.state.collection.as_ref() else {
            return;
        };
        let root = c.root.clone();
        let Some(link) = self.publish_link(cx) else {
            let cfg = load_cloud_config(&root).ok().flatten();
            let name = cfg
                .as_ref()
                .and_then(|c| c.name.clone())
                .or_else(|| self.collection_title())
                .unwrap_or_else(|| c.name.clone());
            let slug = cfg
                .map(|cfg| cfg.collection)
                .unwrap_or_else(|| suggest_slug(&name));
            self.open_publish_dialog(
                PublishSetup {
                    root,
                    name,
                    teams: account.teams,
                    slug,
                    backend: self.backend.clone(),
                },
                window,
                cx,
            );
            return;
        };
        let next = self.publish.current.map_or(1, |n| n + 1);
        let slug = link.slug.clone();
        let checks = cx.background_spawn(async move { check(&root, Some(&slug)) });
        self.publish.job = Some(cx.spawn_in(window, async move |this, cx| {
            let checked = checks.await;
            this.update_in(cx, |this, window, cx| match checked.target {
                Some(target) => this.do_publish(link, target, None, next, window, cx),
                None => this.problems_dialog(problem_rows(&checked.problems), next, window, cx),
            })
            .ok();
        }));
    }

    fn open_publish_dialog(
        &mut self,
        setup: PublishSetup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = setup.name.clone();
        let view = cx.new(|cx| PublishView::new(setup, window, cx));
        self.publish._events = Some(cx.subscribe_in(
            &view,
            window,
            move |this, _, event: &PublishEvent, window, cx| match event {
                PublishEvent::Close => {
                    window.close_dialog(cx);
                    this.publish.view = None;
                }
                PublishEvent::OpenFile(rel) => {
                    window.close_dialog(cx);
                    this.publish.view = None;
                    this.open_file(rel, window, cx);
                }
                PublishEvent::Publish {
                    team,
                    slug,
                    target,
                    save_yaml,
                    next,
                } => {
                    window.close_dialog(cx);
                    this.publish.view = None;
                    let server = this
                        .backend
                        .server()
                        .url()
                        .trim_end_matches('/')
                        .to_string();
                    let link = CloudLink {
                        server,
                        team: team.clone(),
                        slug: slug.clone(),
                    };
                    let yaml = save_yaml.then(|| name.clone());
                    let (target, next) = ((**target).clone(), *next);
                    // After the dialog has gone: a quick refusal opens its
                    // own dialog, which the close must not take with it.
                    cx.defer_in(window, move |this, window, cx| {
                        this.do_publish(link, target, yaml, next, window, cx);
                    });
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
        self.publish.view = Some(view.downgrade());
    }

    /// Presses the open dialog's Publish once the checks allow (dev).
    pub(super) fn dev_submit_publish(&mut self, cx: &mut Context<Self>) {
        if let Some(v) = self.publish.view.as_ref().and_then(WeakEntity::upgrade) {
            v.update(cx, |v, cx| {
                v.auto_submit = true;
                cx.notify();
            });
        }
    }

    /// The open first-time dialog.
    #[cfg(all(test, feature = "server"))]
    pub(super) fn publish_view(&self) -> Option<Entity<PublishView>> {
        self.publish.view.as_ref().and_then(WeakEntity::upgrade)
    }

    /// Sends `target` as version `next` of `link`; `yaml` (the display
    /// name) writes the `cloud:` block first. The foot says `Publishing`.
    fn do_publish(
        &mut self,
        link: CloudLink,
        mut target: Target,
        yaml: Option<String>,
        next: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.state.collection.as_ref().map(|c| c.root.clone()) else {
            return;
        };
        if let Some(name) = &yaml {
            match save_cloud_block(&root, &link.slug, Some(name)) {
                Ok(()) => {
                    // The block is in the folder now: it goes up too.
                    if let Some(t) = check(&root, Some(&link.slug)).target {
                        target = t;
                    }
                }
                Err(e) => super::workspace::notify_error(
                    format!("Couldn't save {CONFIG_FILE}: {e}"),
                    window,
                    cx,
                ),
            }
        }
        if target.name.is_none() {
            target.name = self.collection_title();
        }
        self.publish.publishing = Some(next);
        let (team, slug) = (link.team.clone(), link.slug.clone());
        let backend = self.backend.clone();
        // The folder's git details go up as CI's do (off the UI thread:
        // it runs `git`).
        let call = cx.background_spawn(async move {
            let request = target.request(folder_git(&root));
            futures::executor::block_on(backend.publish(&team, &slug, request))
        });
        let dash = backend_call(cx, &self.backend, |b| b.dashboard_url());
        self.publish.job = Some(cx.spawn_in(window, async move |this, cx| {
            let result = call.await;
            let dashboard = dash.await.ok();
            this.update_in(cx, |this, window, cx| {
                this.publish.publishing = None;
                match result {
                    Ok(r) => {
                        this.notes.update(cx, |m, cx| {
                            m.cloud = Some(link.clone());
                            cx.notify();
                        });
                        this.published_toast(&r, &link, dashboard, window, cx);
                    }
                    Err(BackendError::SignedOut) => this.publish.line = Some(PublishLine::SignIn),
                    Err(e) => this.publish_failed(e, link, dashboard, window, cx),
                }
                this.refresh_publish(false, window, cx);
            })
            .ok();
        }));
        cx.notify();
    }

    fn published_toast(
        &mut self,
        r: &PublishResponse,
        link: &CloudLink,
        dashboard: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let note = match r.status {
            PublishStatus::Unchanged => {
                Notification::info(format!("The folder matches version {}.", r.version))
                    .title("Nothing to publish")
            }
            PublishStatus::Created => {
                let url = dashboard.map(|d| collection_url(&d, &link.team, &link.slug));
                let line = changes_line(r);
                let title = format!("Published version {}", r.version);
                match url {
                    // The line and View in Cloud, as the board: the
                    // content stands in for the message.
                    Some(url) => Notification::new()
                        .with_type(NotificationType::Success)
                        .title(title)
                        .content(move |_, _, _| {
                            let url = url.clone();
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .child(
                                    div()
                                        .text_color(color("text-secondary"))
                                        .child(line.clone()),
                                )
                                .child(
                                    div()
                                        .id("publish-view-in-cloud")
                                        .role(Role::Link)
                                        .test_support()
                                        .aria_label("View in Cloud")
                                        .mt(px(4.))
                                        .cursor_pointer()
                                        .underline()
                                        .on_click(move |_, _, cx| cx.open_url(&url))
                                        .child("View in Cloud"),
                                )
                                .into_any_element()
                        }),
                    None => Notification::success(line).title(title),
                }
            }
        };
        window.push_notification(note, cx);
    }

    /// `Couldn't publish` (board PublishStates, row 2): the server's
    /// words, with Upgrade for the plan limit and Try again otherwise.
    fn publish_failed(
        &mut self,
        err: BackendError,
        link: CloudLink,
        dashboard: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let plan = matches!(err, BackendError::PlanLimit(_));
        let text = match &err {
            BackendError::Offline => "You're offline. Connect and try again.".to_string(),
            e => e.to_string(),
        };
        let ws = cx.entity().downgrade();
        let upgrade = dashboard.map(|d| format!("{d}/t/{}/billing", link.team));
        open_dialog(window, cx, move || {
            let ws = ws.clone();
            let close = dialog_button("publish-failed-close", "Close")
                .outline()
                .on_click(|_, window, cx| window.close_dialog(cx));
            let action = if plan {
                let upgrade = upgrade.clone();
                dialog_button("publish-upgrade", "Upgrade")
                    .primary()
                    .on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        if let Some(u) = &upgrade {
                            cx.open_url(u);
                        }
                    })
            } else {
                dialog_button("publish-retry", "Try again")
                    .primary()
                    .on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        ws.update(cx, |ws, cx| ws.start_publish(window, cx)).ok();
                    })
            };
            simple_dialog(
                "publish-failed",
                "Couldn't publish".into(),
                note("warning", "publish-failed-reason", text.clone()).into_any_element(),
                None,
                vec![close.into_any_element(), action.into_any_element()],
            )
            .into_any_element()
        });
    }

    /// `Fix N problems to publish` (board PublishStates, row 2).
    fn problems_dialog(
        &mut self,
        rows: Vec<ProblemRow>,
        next: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ws = cx.entity().downgrade();
        open_dialog(window, cx, move || {
            let list = rows.iter().enumerate().map(|(ix, r)| {
                let open = r.file.clone().map(|f| {
                    let ws = ws.clone();
                    small_button(SharedString::from(format!("publish-open-{ix}")), "Open")
                        .on_click(move |_, window, cx| {
                            window.close_dialog(cx);
                            let f = PathBuf::from(&f);
                            ws.update(cx, |ws, cx| ws.open_file(&f, window, cx)).ok();
                        })
                        .child("Open")
                });
                problem_row(ix, &r.text, open)
            });
            let body = div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color("border-100"))
                .rounded(px(8.))
                .bg(color("surface-raised"))
                .text_size(px(12.))
                .overflow_hidden()
                .children(list)
                .into_any_element();
            simple_dialog(
                "publish-problems",
                problems_title(rows.len()),
                body,
                Some(CHECKS_NOTE),
                vec![
                    dialog_button("publish-problems-close", "Close")
                        .outline()
                        .on_click(|_, window, cx| window.close_dialog(cx))
                        .into_any_element(),
                    dialog_button("publish-problems-go", format!("Publish version {next}"))
                        .primary()
                        .disabled(true)
                        .into_any_element(),
                ],
            )
            .into_any_element()
        });
    }

    /// `You're not in a team yet` (board PublishStates, row 2).
    fn no_team_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let backend = self.backend.clone();
        open_dialog(window, cx, move || {
            let backend = backend.clone();
            simple_dialog(
                "publish-no-team",
                "You're not in a team yet".into(),
                div()
                    .text_size(px(13.))
                    .line_height(relative(1.6))
                    .text_color(color("text-secondary"))
                    .child(NO_TEAM_BODY)
                    .into_any_element(),
                None,
                vec![
                    dialog_button("publish-no-team-cancel", "Cancel")
                        .outline()
                        .on_click(|_, window, cx| window.close_dialog(cx))
                        .into_any_element(),
                    dialog_button("publish-create-team", "Create a team")
                        .primary()
                        .on_click(move |_, window, cx| {
                            window.close_dialog(cx);
                            let dash = backend_call(cx, &backend, |b| b.dashboard_url());
                            cx.spawn(async move |cx| {
                                if let Ok(d) = dash.await {
                                    cx.update(|cx| cx.open_url(&format!("{d}/new-team")));
                                }
                            })
                            .detach();
                        })
                        .into_any_element(),
                ],
            )
            .into_any_element()
        });
    }

    /// The sidebar foot (board PublishStates, row 1).
    pub(super) fn render_publish_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let now = chrono::Utc::now();
        let line = self.publish_line(cx);
        let text = line.text();
        let detail = line.detail(now);
        let dot = match line.dot() {
            Some(token) => sync_dot(token, false),
            None => sync_dot("text-tertiary", true),
        };
        let action = match &line {
            PublishLine::NotPublished | PublishLine::Behind(..) => Some(
                primary_button("sidebar-publish", "Publish")
                    .on_click(cx.listener(|this, _, window, cx| this.start_publish(window, cx)))
                    .child("Publish")
                    .into_any_element(),
            ),
            PublishLine::SignIn => Some(
                small_button("sidebar-publish-sign-in", "Sign in")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_sign_in(None, window, cx);
                    }))
                    .child("Sign in")
                    .into_any_element(),
            ),
            PublishLine::UpToDate(_) => Some(
                small_button("sidebar-share", "Share")
                    .on_click(cx.listener(|this, _, window, cx| this.open_team_share(window, cx)))
                    .child("Share")
                    .into_any_element(),
            ),
            PublishLine::Unchecked(_) => Some(
                small_button("sidebar-publish-retry", "Retry")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.refresh_publish(false, window, cx)),
                    )
                    .child("Retry")
                    .into_any_element(),
            ),
            _ => None,
        };
        div()
            .id("sidebar-sync")
            .role(Role::Group)
            .test_support()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .px(px(16.))
            .py(px(12.))
            .border_t_1()
            .border_color(color("border-100"))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .child(dot)
            .child(
                div()
                    .id("sidebar-sync-status")
                    .role(Role::Status)
                    .test_support()
                    .aria_label(match &detail {
                        Some(d) => format!("{text}. {d}"),
                        None => text.clone(),
                    })
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .line_height(relative(1.4))
                    .child(div().text_ellipsis().child(text))
                    .children(detail.map(|d| {
                        div()
                            .text_ellipsis()
                            .text_color(color("text-tertiary"))
                            .child(d)
                    })),
            )
            .children(action)
    }
}

/// The folder at `root`'s bundle hash (tests).
#[cfg(all(test, feature = "server"))]
pub fn local_hash(root: &Path) -> String {
    scan_bundle(root).unwrap().hash
}

#[cfg(all(test, feature = "server"))]
mod tests;
