//! Open from Cloud (board SyncCloud, row 3): a team's collections, and a
//! synced one opened into a folder on this Mac that syncs with it. Ones
//! published from files are listed but not opened here: their source is
//! the folder (often a repo).

use super::backend_call;
use super::color;
use super::icons::{self, glyph};
use super::settings_kit::{label_text, neutral_pill, primary_button, small_button};
use super::workspace::Workspace;
use crate::model::backend::{Backend, CollectionSource, TeamCollection, TeamRef};
use crate::model::history::relative_time_inline;
use crate::model::publish::{CloudLink, publisher_label};
use gpui_kit::component::button::Button;
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DIALOG_W: f32 = 560.;
const DIALOG_TOP: f32 = 60.;
const SUBTITLE: &str =
    "Your team's synced collections. Opening one makes a folder on this Mac that syncs with it.";

/// Where synced collections open: `~/curl-runner/{team}/{name}`.
pub fn default_parent(home: &Path, team: &str) -> PathBuf {
    home.join("curl-runner").join(safe_name(team))
}

/// `name` as a folder name: no path separators or leading dots.
pub fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') {
                '-'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim().trim_start_matches('.').to_string();
    if s.is_empty() { "Collection".into() } else { s }
}

/// `parent/name`, or `parent/name 2`, `name 3`… when taken by something
/// that isn't an empty folder.
pub fn free_folder(parent: &Path, name: &str) -> PathBuf {
    let base = safe_name(name);
    let taken = |p: &Path| {
        p.exists()
            && std::fs::read_dir(p)
                .map(|mut d| d.next().is_some())
                .unwrap_or(true)
    };
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            parent.join(&base)
        } else {
            parent.join(format!("{base} {n}"))
        };
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// A row's second line: `v27 · Sam edited 1 h ago`, or why it doesn't open.
pub fn row_detail(
    c: &TeamCollection,
    me: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let ago = |at: chrono::DateTime<chrono::Utc>| {
        relative_time_inline(
            at.with_timezone(&chrono::Local),
            now.with_timezone(&chrono::Local),
        )
    };
    match (c.source, c.version) {
        (CollectionSource::Git, Some(v)) => {
            format!("Published from files · v{v}. Get it from its repo.")
        }
        (CollectionSource::Git, None) => "Published from files. Get it from its repo.".into(),
        (CollectionSource::Cloud, None) => "Nothing synced yet".into(),
        (CollectionSource::Cloud, Some(v)) => match (&c.publisher, c.updated_at) {
            (Some(by), Some(at)) => {
                format!("v{v} · {} edited {}", publisher_label(by, me), ago(at))
            }
            _ => format!("v{v}"),
        },
    }
}

pub enum OpenCloudEvent {
    Close,
    /// Open `link`'s collection: in `existing` when this Mac has it, else in
    /// a new folder under `parent`.
    Open {
        link: CloudLink,
        name: String,
        existing: Option<PathBuf>,
        parent: PathBuf,
    },
}

pub struct OpenCloudView {
    backend: Arc<dyn Backend>,
    server: String,
    me: Option<String>,
    teams: Vec<TeamRef>,
    team: usize,
    /// The team's collections; `Err` says why they couldn't be listed.
    rows: Option<Result<Vec<TeamCollection>, String>>,
    /// Folders on this Mac already linked, by `team/slug`.
    here: HashMap<String, PathBuf>,
    home: PathBuf,
    /// A folder picked with `Change…` instead of `~/curl-runner/{team}`.
    parent: Option<PathBuf>,
    _task: Option<Task<()>>,
}

impl EventEmitter<OpenCloudEvent> for OpenCloudView {}

impl OpenCloudView {
    pub fn new(
        backend: Arc<dyn Backend>,
        teams: Vec<TeamRef>,
        here: HashMap<String, PathBuf>,
        home: PathBuf,
        cx: &mut Context<Self>,
    ) -> Self {
        let server = backend.server().url().trim_end_matches('/').to_string();
        let me = backend.account().map(|a| a.email);
        let mut view = OpenCloudView {
            backend,
            server,
            me,
            teams,
            team: 0,
            rows: None,
            here,
            home,
            parent: None,
            _task: None,
        };
        view.load(cx);
        view
    }

    fn team(&self) -> Option<&TeamRef> {
        self.teams.get(self.team)
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(team) = self.team().map(|t| t.slug.clone()) else {
            return;
        };
        self.rows = None;
        let call = backend_call(cx, &self.backend, move |b| b.team_collections(&team));
        self._task = Some(cx.spawn(async move |this, cx| {
            let r = call.await;
            this.update(cx, |this, cx| {
                this.rows = Some(r.map_err(|e| e.to_string()));
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn set_team(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.team = ix;
        self.parent = None;
        self.load(cx);
    }

    /// Whether `slug` (in the shown team) has a folder on this Mac.
    #[cfg(all(test, feature = "server"))]
    pub(super) fn is_here(&self, slug: &str) -> bool {
        self.here.contains_key(&self.key(slug))
    }

    fn parent(&self) -> PathBuf {
        self.parent.clone().unwrap_or_else(|| {
            let team = self.team().map(|t| t.name.clone()).unwrap_or_default();
            default_parent(&self.home, &team)
        })
    }

    fn key(&self, slug: &str) -> String {
        let team = self.team().map(|t| t.slug.as_str()).unwrap_or_default();
        format!("{team}/{slug}")
    }

    /// Opens `c`: its folder here, or a new one.
    pub(super) fn open(&mut self, slug: &str, cx: &mut Context<Self>) {
        let Some(Ok(rows)) = &self.rows else { return };
        let Some(c) = rows.iter().find(|c| c.slug == slug) else {
            return;
        };
        let Some(team) = self.team() else { return };
        cx.emit(OpenCloudEvent::Open {
            link: CloudLink {
                server: self.server.clone(),
                team: team.slug.clone(),
                slug: c.slug.clone(),
            },
            name: c.name.clone(),
            existing: self.here.get(&self.key(&c.slug)).cloned(),
            parent: self.parent(),
        });
    }

    fn change_parent(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await
                && let Some(p) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| {
                    this.parent = Some(p);
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn render_team(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let weak = cx.entity().downgrade();
        let teams = self.teams.clone();
        let current = self.team;
        let name = self.team().map(|t| t.name.clone()).unwrap_or_default();
        Button::new("open-cloud-team")
            .outline()
            .h(px(32.))
            .w(px(200.))
            .px(px(10.))
            .text_size(px(13.))
            .child(div().flex_1().min_w_0().text_ellipsis().child(name))
            .child(
                glyph(icons::CHEVRON_DOWN)
                    .size(px(12.))
                    .text_color(color("text-secondary")),
            )
            .dropdown_menu(move |mut menu, _, _| {
                for (ix, t) in teams.iter().enumerate() {
                    let weak = weak.clone();
                    menu = menu.item(
                        PopupMenuItem::new(t.name.clone())
                            .checked(ix == current)
                            .on_click(move |_, _, cx| {
                                weak.update(cx, |this, cx| this.set_team(ix, cx)).ok();
                            }),
                    );
                }
                menu
            })
    }

    fn render_rows(&self, cx: &mut Context<Self>) -> AnyElement {
        let message = |text: String| {
            div()
                .id("open-cloud-message")
                .test_support()
                .aria_label(text.clone())
                .px(px(14.))
                .py(px(14.))
                .text_size(px(13.))
                .text_color(color("text-secondary"))
                .child(text)
                .into_any_element()
        };
        let rows = match &self.rows {
            None => return message("Loading…".into()),
            Some(Err(e)) => return message(e.clone()),
            Some(Ok(rows)) if rows.is_empty() => {
                return message("This team has no collections yet.".into());
            }
            Some(Ok(rows)) => rows.clone(),
        };
        let now = chrono::Utc::now();
        div()
            .id("open-cloud-rows")
            .role(Role::List)
            .test_support()
            .flex()
            .flex_col()
            .border_1()
            .border_color(color("border-100"))
            .rounded(px(8.))
            .max_h(px(320.))
            .overflow_y_scroll()
            .children(rows.into_iter().enumerate().map(|(ix, c)| {
                let here = self.here.contains_key(&self.key(&c.slug));
                let detail = row_detail(&c, self.me.as_deref(), now);
                let slug = c.slug.clone();
                let action = match c.source {
                    CollectionSource::Git => neutral_pill().child("Publish").into_any_element(),
                    CollectionSource::Cloud => {
                        let id = SharedString::from(format!("open-cloud-open-{}", c.slug));
                        let button = if here {
                            small_button(id, "Open")
                        } else {
                            primary_button(id, "Open")
                        };
                        button
                            .on_click(cx.listener(move |this, _, _, cx| this.open(&slug, cx)))
                            .child("Open")
                            .into_any_element()
                    }
                };
                div()
                    .id(("open-cloud-row", ix))
                    .role(Role::ListItem)
                    .test_support()
                    .aria_label(c.name.clone())
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(14.))
                    .py(px(12.))
                    .when(ix > 0, |d| d.border_t_1().border_color(color("border-100")))
                    .text_size(px(13.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(div().text_ellipsis().child(c.name.clone()))
                            .child(
                                div()
                                    .id(("open-cloud-detail", ix))
                                    .test_support()
                                    .aria_label(detail.clone())
                                    .text_size(px(12.))
                                    .text_color(color("text-secondary"))
                                    .text_ellipsis()
                                    .child(detail),
                            ),
                    )
                    .when(here, |d| d.child(neutral_pill().child("On this Mac")))
                    .child(action)
            }))
            .into_any_element()
    }
}

impl Render for OpenCloudView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let home = self.home.clone();
        let parent = self.parent();
        let shown = parent
            .strip_prefix(&home)
            .map(|p| format!("~/{}", p.display()))
            .unwrap_or_else(|_| parent.display().to_string());
        div()
            .id("open-cloud-dialog")
            .role(Role::Dialog)
            .test_support()
            .aria_label("Open from Cloud")
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .px(px(22.))
                    .pt(px(18.))
                    .pb(px(4.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Open from Cloud"),
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
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .px(px(22.))
                    .pt(px(12.))
                    .pb(px(18.))
                    .child(label_text("Team"))
                    .child(self.render_team(cx))
                    .child(self.render_rows(cx))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child("Opens into")
                            .child(
                                div()
                                    .id("open-cloud-parent")
                                    .test_support()
                                    .aria_label(shown.clone())
                                    .font_family(super::app::mono_font())
                                    .child(shown),
                            )
                            .child(
                                div()
                                    .id("open-cloud-change")
                                    .role(Role::Link)
                                    .test_support()
                                    .aria_label("Change…")
                                    .text_color(color("text-primary"))
                                    .cursor_pointer()
                                    .hover(|s| s.underline())
                                    .on_click(cx.listener(|this, _, _, cx| this.change_parent(cx)))
                                    .child("Change…"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .pl(px(22.))
                    .pr(px(16.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(color("border-100"))
                    .bg(color("surface-raised"))
                    .child(
                        Button::new("open-cloud-close")
                            .outline()
                            .h(px(32.))
                            .px(px(12.))
                            .text_size(px(13.))
                            .label("Close")
                            .disabled(false)
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(OpenCloudEvent::Close))),
                    ),
            )
    }
}

impl Workspace {
    /// ⌘K › Open from Cloud…, the Welcome screen and the account menu's
    /// team rows. Signed out: sign in first; no team: the publish board's
    /// `You're not in a team yet`.
    pub(in crate::ui) fn open_from_cloud(
        &mut self,
        team: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !crate::features::SERVER {
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
        let mut here = self.linked_folders();
        // The open folder counts too, remembered or not.
        if let (Some(link), Some(c)) = (self.publish_link(cx), &self.state.collection) {
            here.insert(format!("{}/{}", link.team, link.slug), c.root.clone());
        }
        let home = self
            .cloud_home
            .clone()
            .or_else(std::env::home_dir)
            .unwrap_or_else(|| PathBuf::from("/"));
        let teams = account.teams.clone();
        let start = team.and_then(|t| teams.iter().position(|x| x.slug == t));
        let backend = self.backend.clone();
        let view = cx.new(|cx| {
            let mut v = OpenCloudView::new(backend, teams, here, home, cx);
            if let Some(ix) = start {
                v.set_team(ix, cx);
            }
            v
        });
        self._open_cloud_events = Some(cx.subscribe_in(
            &view,
            window,
            |this, _, event: &OpenCloudEvent, window, cx| match event {
                OpenCloudEvent::Close => {
                    window.close_dialog(cx);
                    this.open_cloud_view = None;
                }
                OpenCloudEvent::Open {
                    link,
                    name,
                    existing,
                    parent,
                } => {
                    window.close_dialog(cx);
                    this.open_cloud_view = None;
                    let (link, name, existing, parent) =
                        (link.clone(), name.clone(), existing.clone(), parent.clone());
                    cx.defer_in(window, move |this, window, cx| {
                        this.open_synced(link, name, existing, parent, window, cx);
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
        self.open_cloud_view = Some(view.downgrade());
    }

    /// Recent folders linked to a collection on the current server, by
    /// `team/slug`.
    fn linked_folders(&self) -> HashMap<String, PathBuf> {
        let Some(base) = self.store.base() else {
            return HashMap::new();
        };
        let server = self
            .backend
            .server()
            .url()
            .trim_end_matches('/')
            .to_string();
        self.recent
            .iter()
            .filter(|(_, exists)| *exists)
            .filter_map(|(r, _)| {
                let meta = crate::model::collection_meta::CollectionMeta::load(base, &r.path);
                let link = meta.cloud.filter(|l| l.server == server)?;
                Some((format!("{}/{}", link.team, link.slug), r.path.clone()))
            })
            .collect()
    }

    /// Opens a synced collection: its folder here, or a new empty one that
    /// the first sync fills. Unsaved edits are settled first.
    fn open_synced(
        &mut self,
        link: CloudLink,
        name: String,
        existing: Option<PathBuf>,
        parent: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = existing.unwrap_or_else(|| free_folder(&parent, &name));
        self.settle_unsaved(
            std::rc::Rc::new(move |this, window, cx| {
                if let Err(e) = std::fs::create_dir_all(&target) {
                    super::workspace::notify_error(
                        format!("Couldn't make {}: {e}", target.display()),
                        window,
                        cx,
                    );
                    return;
                }
                if !this.open_collection(&target, window, cx) {
                    return;
                }
                this.remember_folder(&target);
                let link = link.clone();
                this.notes.update(cx, |m, cx| {
                    m.cloud = Some(link);
                    cx.notify();
                });
                this.refresh_publish(false, window, cx);
            }),
            window,
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CollectionSource, TeamCollection, default_parent, free_folder, row_detail, safe_name,
    };
    use std::path::Path;

    #[test]
    fn folders_go_under_curl_runner_by_team_and_never_clobber() {
        assert_eq!(
            default_parent(Path::new("/Users/jamie"), "Acme"),
            Path::new("/Users/jamie/curl-runner/Acme")
        );
        assert_eq!(safe_name("a/b:c"), "a-b-c");
        assert_eq!(safe_name("..hidden"), "hidden");
        assert_eq!(safe_name("  "), "Collection");
        let d = tempfile::tempdir().unwrap();
        assert_eq!(free_folder(d.path(), "Pay"), d.path().join("Pay"));
        // An empty folder is fine to open into; one with files isn't.
        std::fs::create_dir(d.path().join("Pay")).unwrap();
        assert_eq!(free_folder(d.path(), "Pay"), d.path().join("Pay"));
        std::fs::write(d.path().join("Pay/a.yaml"), "x").unwrap();
        assert_eq!(free_folder(d.path(), "Pay"), d.path().join("Pay 2"));
    }

    #[test]
    fn rows_say_who_edited_or_where_to_get_it() {
        let now = chrono::Utc::now();
        let row = |source, version| TeamCollection {
            slug: "pay".into(),
            name: "Payments".into(),
            source,
            version,
            updated_at: Some(now - chrono::Duration::hours(1)),
            publisher: Some("sam@acme.dev".into()),
        };
        assert_eq!(
            row_detail(
                &row(CollectionSource::Cloud, Some(27)),
                Some("ada@acme.dev"),
                now
            ),
            "v27 · sam@acme.dev edited 1 h ago"
        );
        assert_eq!(
            row_detail(&row(CollectionSource::Git, Some(5)), None, now),
            "Published from files · v5. Get it from its repo."
        );
        assert_eq!(
            row_detail(&row(CollectionSource::Cloud, None), None, now),
            "Nothing synced yet"
        );
    }
}
