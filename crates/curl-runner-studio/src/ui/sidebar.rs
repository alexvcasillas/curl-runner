//! The collections column: header, filter and the collection tree.

use super::icons::{self, glyph};
use super::method_stack::{Stack, method_stack};
use super::runs::RunStatus;
use super::sync::{SyncLine, sync_dot};
use super::workspace::Workspace;
use super::{color, method_color};
use crate::model::collection::TreeNode;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, Position};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const SIDEBAR_WIDTH: f32 = 264.;
const ROW_HEIGHT: f32 = 28.;
const FILTER_HEIGHT: f32 = 30.;
const CHEVRON_SIZE: f32 = 12.;
/// Selected tree row (Main.png: `#1f1f1f`).
const SELECTED_BG: &str = "surface-300";
/// Indent per tree level; one level lines a request's badge up under its
/// folder's icon, as in the mockup.
const INDENT: f32 = 22.;

/// One visible row of the tree, after collapsing and filtering.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub depth: usize,
    pub kind: RowKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    Folder {
        path: PathBuf,
        name: String,
        open: bool,
        /// Requests anywhere below the folder.
        count: usize,
    },
    File {
        rel_path: PathBuf,
        /// The file name without `.yaml` / `.yml`.
        label: String,
        method: Option<String>,
        parse_ok: bool,
        /// Set for a file without a single `request:`: it expands into
        /// its requests.
        group: Option<Group>,
    },
    /// One request of an expanded multi-request file.
    Request {
        rel_path: PathBuf,
        /// Position in the file's request list.
        index: usize,
        method: String,
        name: String,
        line: Option<u32>,
    },
}

/// The expandable part of a multi-request file row.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub open: bool,
    pub count: usize,
    /// Every request's method, in request order.
    pub methods: Vec<String>,
}

/// The rows to draw. Folders in `collapsed` hide their children;
/// multi-request files show theirs only when in `expanded`. A non-empty
/// `filter` keeps files whose name contains it (any case), requests whose
/// name contains it (inside their file, shown open) and folders whose name
/// contains it (with all their contents), plus the folders leading to them,
/// shown open.
pub fn visible_rows(
    tree: &[TreeNode],
    collapsed: &HashSet<PathBuf>,
    expanded: &HashSet<PathBuf>,
    filter: &str,
) -> Vec<Row> {
    let needle = filter.trim().to_lowercase();
    let mut rows = Vec::new();
    let open = Open {
        collapsed,
        expanded,
        needle: &needle,
    };
    walk(tree, Path::new(""), 0, &open, false, &mut rows);
    rows
}

struct Open<'a> {
    collapsed: &'a HashSet<PathBuf>,
    expanded: &'a HashSet<PathBuf>,
    needle: &'a str,
}

/// Appends `nodes`' rows; returns whether any was appended.
fn walk(
    nodes: &[TreeNode],
    parent: &Path,
    depth: usize,
    open: &Open,
    shown_whole: bool,
    rows: &mut Vec<Row>,
) -> bool {
    let start = rows.len();
    let needle = open.needle;
    let filtering = !needle.is_empty();
    let matches = |name: &str| shown_whole || !filtering || name.to_lowercase().contains(needle);
    for node in nodes {
        match node {
            TreeNode::Folder { name, children } => {
                let path = parent.join(name);
                let is_open = filtering || !open.collapsed.contains(&path);
                let at = rows.len();
                rows.push(Row {
                    depth,
                    kind: RowKind::Folder {
                        path: path.clone(),
                        name: name.clone(),
                        open: is_open,
                        count: request_count(children),
                    },
                });
                let whole = filtering && matches(name);
                let any = is_open && walk(children, &path, depth + 1, open, whole, rows);
                if filtering && !whole && !any {
                    rows.truncate(at);
                }
            }
            TreeNode::File {
                name,
                rel_path,
                method,
                requests,
                parse_ok,
            } => {
                let label = request_label(name);
                let file_hit = matches(label);
                let multi = method.is_none() && *parse_ok;
                let hits: Vec<usize> = if multi && !file_hit {
                    requests
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.name.to_lowercase().contains(needle))
                        .map(|(i, _)| i)
                        .collect()
                } else {
                    Vec::new()
                };
                if !file_hit && hits.is_empty() {
                    continue;
                }
                let is_open = !hits.is_empty() || open.expanded.contains(rel_path);
                rows.push(Row {
                    depth,
                    kind: RowKind::File {
                        rel_path: rel_path.clone(),
                        label: label.to_string(),
                        method: method.clone(),
                        parse_ok: *parse_ok,
                        group: multi.then(|| Group {
                            open: is_open && !requests.is_empty(),
                            count: requests.len(),
                            methods: requests.iter().map(|r| r.method.clone()).collect(),
                        }),
                    },
                });
                if multi && is_open {
                    for (index, r) in requests.iter().enumerate() {
                        if file_hit || hits.contains(&index) {
                            rows.push(Row {
                                depth: depth + 1,
                                kind: RowKind::Request {
                                    rel_path: rel_path.clone(),
                                    index,
                                    method: r.method.clone(),
                                    name: r.name.clone(),
                                    line: r.line,
                                },
                            });
                        }
                    }
                }
            }
        }
    }
    rows.len() > start
}

/// Requests below `nodes`: a multi-request file counts each of its
/// requests, any other file one.
pub fn request_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|n| match n {
            TreeNode::Folder { children, .. } => request_count(children),
            TreeNode::File { requests, .. } => requests.len().max(1),
        })
        .sum()
}

/// `create-charge.yaml` → `create-charge`.
pub fn request_label(file_name: &str) -> &str {
    file_name
        .strip_suffix(".yaml")
        .or_else(|| file_name.strip_suffix(".yml"))
        .unwrap_or(file_name)
}

impl Workspace {
    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .pt(px(12.))
            .pr(px(12.))
            .pb(px(8.))
            .pl(px(16.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color("text-secondary"))
                    .child("Collections"),
            );
        let column = div()
            .id("sidebar")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .border_r_1()
            .border_color(color("border-100"));
        let Some(collection) = &self.state.collection else {
            return column
                .child(header)
                .child(self.render_no_collections(cx))
                .child(self.render_cloud_footer(cx));
        };

        let rows = visible_rows(
            &collection.tree,
            &self.collapsed,
            &self.expanded,
            &self.filter.read(cx).value(),
        );
        let active = self.state.active_document().map(|d| d.rel_path.clone());
        let read_only = collection.read_only.is_some();
        let tree = div()
            .id("collection-tree")
            .role(Role::Tree)
            .test_support()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap(px(1.))
            .px(px(8.))
            .pb(px(8.))
            .overflow_y_scroll()
            .children(
                rows.into_iter()
                    .enumerate()
                    .map(|(ix, row)| self.render_row(ix, row, active.as_deref(), read_only, cx)),
            )
            .children(self.render_other_collections(cx));

        if read_only {
            // The ReadOnly board: the shared name and a lock, no New or
            // filter; the foot says when the owner last changed it.
            let title = self.collection_title().unwrap_or_default();
            return column
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_between()
                        .h(px(48.))
                        .pt(px(12.))
                        .pr(px(12.))
                        .pb(px(8.))
                        .pl(px(16.))
                        .child(
                            div()
                                .id("sidebar-title")
                                .role(Role::Label)
                                .test_support()
                                .aria_label(title.clone())
                                .min_w_0()
                                .text_ellipsis()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color("text-secondary"))
                                .child(title),
                        )
                        .child(
                            div()
                                .id("sidebar-lock")
                                .flex_none()
                                .child(
                                    glyph(icons::LOCK)
                                        .size(px(13.))
                                        .text_color(color("text-gutter")),
                                )
                                .tooltip(|window, cx| {
                                    gpui_kit::component::tooltip::Tooltip::new("Read only")
                                        .build(window, cx)
                                }),
                        ),
                )
                .child(tree)
                .child(self.render_updates_footer(cx));
        }

        column
            .child(
                header.child(
                    Button::new("sidebar-new-request")
                        .ghost()
                        .small()
                        .icon(
                            glyph(icons::PLUS)
                                .size(px(14.))
                                .text_color(color("text-secondary")),
                        )
                        .tooltip("New request")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.new_request(None, window, cx);
                        })),
                ),
            )
            .child(
                div().px(px(12.)).pb(px(8.)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .h(px(FILTER_HEIGHT))
                        .px(px(8.))
                        .border_1()
                        .border_color(color("border-100"))
                        .rounded(px(6.))
                        .child(
                            glyph(icons::FILTER)
                                .size(px(13.))
                                .flex_none()
                                .text_color(color("text-secondary")),
                        )
                        .child(
                            Input::new(&self.filter)
                                .appearance(false)
                                .small()
                                .px_0()
                                .cleanable(true),
                        ),
                ),
            )
            .child(tree)
            .child(self.render_sync_footer(cx))
    }

    fn render_row(
        &self,
        ix: usize,
        row: Row,
        active: Option<&Path>,
        read_only: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let depth = row.depth;
        let base = div()
            .id(("tree-row", ix))
            .role(Role::TreeItem)
            .test_support()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(ROW_GAP))
            .h(px(ROW_HEIGHT))
            .pr(px(8.))
            .pl(px(method_x(depth)))
            .rounded(px(6.))
            .text_size(px(13.))
            .text_color(color("text-secondary"))
            .whitespace_nowrap()
            .overflow_hidden()
            .cursor_pointer();
        let selected_bg = color(SELECTED_BG);
        let ws = cx.entity().downgrade();
        match row.kind {
            RowKind::Folder {
                path,
                name,
                open,
                count,
            } => {
                let toggle = path.clone();
                let menu_path = path.clone();
                base.text_color(color("text-primary"))
                    .hover(hover)
                    .child(chevron(open))
                    .child(
                        glyph(icons::FOLDER)
                            .size(px(14.))
                            .flex_none()
                            .text_color(color("text-secondary")),
                    )
                    .child(name_cell(name).font_weight(FontWeight::MEDIUM))
                    .when(!open, |row| row.child(count_cell(count)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.collapsed.remove(&toggle) {
                            this.collapsed.insert(toggle.clone());
                        }
                        cx.notify();
                    }))
                    .map(|row| {
                        if read_only {
                            row.into_any_element()
                        } else {
                            row.context_menu(move |menu, _, _| {
                                tree_menu(menu, &ws, &menu_path, Some(&menu_path))
                            })
                            .into_any_element()
                        }
                    })
            }
            RowKind::File {
                rel_path,
                label,
                method,
                parse_ok,
                group,
            } => {
                let is_active = active == Some(rel_path.as_path());
                let child_selected = group.as_ref().is_some_and(|g| g.open)
                    && self
                        .selected_request
                        .as_ref()
                        .is_some_and(|(p, _)| p == &rel_path);
                let selected = is_active && !child_selected;
                let open_path = rel_path.clone();
                let folder = rel_path.parent().map(Path::to_path_buf);
                let group_name = SharedString::from(format!("tree-row-{ix}"));
                let row = select(base.group(group_name.clone()), selected, selected_bg).tooltip({
                    let full = rel_path.display().to_string();
                    move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(full.clone()).build(window, cx)
                    }
                });
                let row = match group {
                    Some(group) => {
                        let fill = if selected {
                            selected_bg
                        } else {
                            color("bg-100")
                        };
                        let toggle = rel_path.clone();
                        let chevron = if group.count == 0 {
                            div().into_any_element()
                        } else {
                            div()
                                .id(("tree-chevron", ix))
                                .child(chevron(group.open))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    if !this.expanded.remove(&toggle) {
                                        this.expanded.insert(toggle.clone());
                                    }
                                    cx.notify();
                                }))
                                .test_support()
                                .into_any_element()
                        };
                        let stack = Stack::new(group.methods.iter().map(String::as_str));
                        row.relative()
                            .child(
                                div()
                                    .absolute()
                                    .left(px(chevron_x(depth)))
                                    .top(px((ROW_HEIGHT - CHEVRON_SIZE) / 2.))
                                    .size(px(CHEVRON_SIZE))
                                    .child(chevron),
                            )
                            .child(method_stack(
                                &stack,
                                METHOD_BADGE_WIDTH,
                                fill,
                                (!selected).then(|| (group_name, color("surface-100"))),
                            ))
                            .child(name_cell(label))
                            .child(count_cell(group.count))
                            .child(status_dot(ix, parse_ok, self.runs.status(&rel_path, None)))
                    }
                    None => row
                        .child(method_badge(method.as_deref()))
                        .child(name_cell(label))
                        .child(status_dot(ix, parse_ok, self.runs.status(&rel_path, None))),
                };
                row.on_click(cx.listener(move |this, _, window, cx| {
                    this.selected_request = None;
                    this.expanded.insert(open_path.clone());
                    // The file is what the click asks for, not the
                    // conflicts over it (they stay pending).
                    this.hide_conflicts(window, cx);
                    this.open_file(&open_path, window, cx);
                }))
                .map(|row| {
                    if read_only {
                        row.into_any_element()
                    } else {
                        row.context_menu(move |menu, _, _| {
                            tree_menu(menu, &ws, &rel_path, folder.as_deref())
                        })
                        .into_any_element()
                    }
                })
            }
            RowKind::Request {
                rel_path,
                index,
                method,
                name,
                line,
            } => {
                let selected = active == Some(rel_path.as_path())
                    && self.selected_request.as_ref() == Some(&(rel_path.clone(), index));
                let open_path = rel_path.clone();
                let folder = rel_path.parent().map(Path::to_path_buf);
                select(base, selected, selected_bg)
                    .child(method_badge(Some(&method)))
                    .child(name_cell(name))
                    .child(status_dot(
                        ix,
                        true,
                        self.runs.status(&rel_path, Some(index)),
                    ))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.hide_conflicts(window, cx);
                        this.open_request(&open_path, index, line, window, cx);
                    }))
                    .map(|row| {
                        if read_only {
                            row.into_any_element()
                        } else {
                            row.context_menu(move |menu, _, _| {
                                tree_menu(menu, &ws, &rel_path, folder.as_deref())
                            })
                            .into_any_element()
                        }
                    })
            }
        }
    }

    /// Opens `rel` with request `index` selected and the cursor on its
    /// `line`.
    pub(super) fn open_request(
        &mut self,
        rel: &Path,
        index: usize,
        line: Option<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_request = Some((rel.to_path_buf(), index));
        self.open_file(rel, window, cx);
        let Some(ix) = self.state.active else { return };
        if let (Some(line), true) = (line, self.state.open[ix].rel_path == rel) {
            self.tabs[ix].editor.update(cx, |e, cx| {
                e.set_cursor_position(Position::new(line, 0), window, cx);
            });
        }
        cx.notify();
    }

    fn render_no_collections(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(10.))
            .p(px(24.))
            .text_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(40.))
                    .rounded(px(8.))
                    .border_1()
                    .border_dashed()
                    .border_color(color("border-200"))
                    .child(
                        Icon::new(IconName::Folder)
                            .size(px(16.))
                            .text_color(color("text-tertiary")),
                    ),
            )
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child("No collections yet"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(relative(1.5))
                    .text_color(color("text-secondary"))
                    .child(
                        "Collections group your request files into folders you can run and share.",
                    ),
            )
            .child(
                Button::new("sidebar-new-collection")
                    .outline()
                    .small()
                    .mt(px(4.))
                    .icon(IconName::Plus)
                    .label("New collection")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.new_collection(window, cx);
                    })),
            )
    }
}

impl Workspace {
    /// Welcome's sidebar foot: where collections live (`Connected to
    /// curl-runner Cloud`, `Self-hosted · {host}`, `Offline`, `Not signed
    /// in`) and `Change`, which opens the sign-in screen.
    fn render_cloud_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (dot, text) = self.connection_text();
        div()
            .id("sidebar-cloud")
            .role(Role::Group)
            .test_support()
            .aria_label(text.clone())
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
            .child(div().flex_none().size(px(6.)).rounded_full().bg(color(dot)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .line_height(relative(1.4))
                    .child(text),
            )
            .child(
                div()
                    .id("sidebar-cloud-change")
                    .role(Role::Button)
                    .test_support()
                    .aria_label("Change")
                    .flex_none()
                    .cursor_pointer()
                    .hover(|s| s.text_color(color("text-primary")))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_sign_in(None, window, cx);
                    }))
                    .child("Change"),
            )
    }
}

impl Workspace {
    /// The Main board's foot: `● Synced 2 min ago` and `Share`; offline,
    /// the SyncConflict board's amber `Offline · 3 changes waiting`; with
    /// conflicts, `1 conflict · Resolve`; signed out, `Local only`. Share
    /// is disabled signed out (`Sign in to share`) and offline (`Go online
    /// to share`).
    fn render_sync_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let line = self.sync_line();
        let text = line.text(chrono::Utc::now());
        let share = div()
            .id("sidebar-share")
            .role(Role::Button)
            .test_support()
            .aria_label("Share")
            .flex_none()
            .px(px(10.))
            .py(px(4.))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(6.))
            .line_height(relative(1.4))
            .child("Share");
        let blocked = self.share_blocked(cx);
        let share = if let Some(why) = blocked {
            share
                .text_color(color("text-disabled"))
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(why).build(window, cx)
                })
        } else {
            share
                .text_color(color("text-primary"))
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-100")))
                .on_click(cx.listener(|this, _, window, cx| this.open_share(window, cx)))
        };
        let resolve = matches!(line, SyncLine::Conflicts(_)).then(|| {
            div()
                .id("sidebar-resolve")
                .role(Role::Button)
                .test_support()
                .aria_label("Resolve")
                .flex_none()
                .text_color(color("text-primary"))
                .cursor_pointer()
                .hover(|s| s.underline())
                .on_click(cx.listener(|this, _, window, cx| this.open_conflicts(window, cx)))
                .child("Resolve")
        });
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
            .children(line.dot().map(|(token, ring)| sync_dot(token, ring)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .line_height(relative(1.4))
                    .child(
                        div()
                            .id("sidebar-sync-status")
                            .role(Role::Status)
                            .test_support()
                            .aria_label(text.clone())
                            .min_w_0()
                            .text_ellipsis()
                            .text_color(color(line.text_color()))
                            .child(text),
                    )
                    .when_some(resolve, |d, resolve| d.child("·").child(resolve)),
            )
            .child(share)
    }
}

impl Workspace {
    /// Below the tree: `Shared with me` (each received copy, `Read only`;
    /// the Main board), or, in a received copy, `My collections` (the
    /// recent folders that are not received copies; the ReadOnly board).
    fn render_other_collections(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let root = self.state.collection.as_ref()?.root.clone();
        let received = &self.received_all;
        let (heading, rows): (&'static str, Vec<(PathBuf, String, bool)>) =
            if self.read_only_org().is_some() {
                let rows = self
                    .recent
                    .iter()
                    .filter(|(r, exists)| {
                        *exists && r.path != root && !received.iter().any(|s| s.root == r.path)
                    })
                    .map(|(r, _)| {
                        let name = r.path.file_name().map_or_else(
                            || r.path.display().to_string(),
                            |n| n.to_string_lossy().into_owned(),
                        );
                        (r.path.clone(), name, false)
                    })
                    .collect();
                ("My collections", rows)
            } else {
                let rows = received
                    .iter()
                    .filter(|r| r.root != root)
                    .map(|r| (r.root.clone(), r.share.name.clone(), true))
                    .collect();
                ("Shared with me", rows)
            };
        if rows.is_empty() {
            return None;
        }
        let id = SharedString::from(heading.to_lowercase().replace(' ', "-"));
        Some(
            div()
                .id(id)
                .role(Role::Group)
                .test_support()
                .aria_label(heading)
                .flex()
                .flex_col()
                .flex_none()
                .gap(px(1.))
                .child(
                    div()
                        .h(px(1.))
                        .mx(px(8.))
                        .my(px(10.))
                        .bg(color("border-100")),
                )
                .child(
                    div()
                        .px(px(8.))
                        .pb(px(6.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color("text-secondary"))
                        .child(heading),
                )
                .children(
                    rows.into_iter()
                        .enumerate()
                        .map(|(ix, (path, name, pill))| {
                            div()
                                .id(SharedString::from(format!("other-collection-{ix}")))
                                .role(Role::Button)
                                .test_support()
                                .aria_label(name.clone())
                                .flex()
                                .flex_none()
                                .items_center()
                                .gap(px(8.))
                                .h(px(ROW_HEIGHT))
                                .px(px(8.))
                                .rounded(px(6.))
                                .text_size(px(13.))
                                .text_color(color(if pill {
                                    "text-primary"
                                } else {
                                    "text-secondary"
                                }))
                                .whitespace_nowrap()
                                .cursor_pointer()
                                .hover(hover)
                                .child(chevron(false))
                                .child(name_cell(name))
                                .when(pill, |row| {
                                    row.child(
                                        div()
                                            .flex_none()
                                            .px(px(6.))
                                            .border_1()
                                            .border_color(color("border-200"))
                                            .rounded(px(4.))
                                            .text_size(px(11.))
                                            .text_color(color("text-secondary"))
                                            .child("Read only"),
                                    )
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.switch_to(path.clone(), window, cx)
                                }))
                        }),
                ),
        )
    }

    /// The ReadOnly board's foot: `Updated 3 days ago` and `Check for
    /// updates`.
    fn render_updates_footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let text = self.received_updated(cx).map_or_else(
            || "Shared with you".to_string(),
            |ago| {
                let mut c = ago.chars();
                c.next()
                    .map(|f| format!("Updated {}{}", f.to_uppercase(), c.as_str()))
                    .unwrap_or_default()
            },
        );
        let checking = self.update_check.is_some();
        div()
            .id("sidebar-updates")
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
            .child(
                div()
                    .id("sidebar-updated")
                    .role(Role::Status)
                    .test_support()
                    .aria_label(text.clone())
                    .flex_1()
                    .min_w_0()
                    .line_height(relative(1.4))
                    .child(text),
            )
            .child(
                div()
                    .id("sidebar-check-updates")
                    .role(Role::Button)
                    .test_support()
                    .aria_label("Check for updates")
                    .flex_none()
                    .px(px(10.))
                    .py(px(4.))
                    .border_1()
                    .border_color(color("border-200"))
                    .rounded(px(6.))
                    .line_height(relative(1.4))
                    .map(|b| {
                        if checking {
                            b.text_color(color("text-disabled"))
                        } else {
                            b.text_color(color("text-primary"))
                                .cursor_pointer()
                                .hover(|s| s.bg(color("surface-100")))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.check_for_updates(window, cx)
                                }))
                        }
                    })
                    .child("Check for updates"),
            )
    }
}

fn hover(style: StyleRefinement) -> StyleRefinement {
    style.bg(color("surface-100"))
}

/// The selected background and text, or the hover background.
fn select<E: Styled + InteractiveElement>(row: E, selected: bool, bg: Hsla) -> E {
    if selected {
        row.bg(bg).text_color(color("text-primary"))
    } else {
        row.hover(hover)
    }
}

fn chevron(open: bool) -> Icon {
    glyph(if open {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    })
    .size(px(CHEVRON_SIZE))
    .flex_none()
    .text_color(color("text-secondary"))
}

fn name_cell(name: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .child(name.into())
}

fn count_cell(count: usize) -> Div {
    div()
        .flex_none()
        .text_size(px(11.))
        .text_color(color("text-tertiary"))
        .child(count.to_string())
}

/// The status column: the last run's result (green passed, red failed), a
/// hollow ring until it has run, a red dot for a file that does not parse.
fn status_dot(ix: usize, parse_ok: bool, status: RunStatus) -> AnyElement {
    let label = match (parse_ok, status) {
        (false, _) => "Can't parse",
        (true, RunStatus::NotRun) => "Not run",
        (true, RunStatus::Passed) => "Passed",
        (true, RunStatus::Failed) => "Failed",
    };
    let dot = div()
        .id(("status-dot", ix))
        .role(Role::Label)
        .aria_label(label)
        .test_support()
        .flex_none()
        .size(px(6.))
        .rounded_full();
    if parse_ok {
        match status {
            RunStatus::NotRun => dot.border_1().border_color(color("text-disabled")),
            RunStatus::Passed => dot.bg(color("success")),
            RunStatus::Failed => dot.bg(color("danger")),
        }
        .into_any_element()
    } else {
        dot.bg(color("danger"))
            .tooltip(|window, cx| {
                gpui_kit::component::tooltip::Tooltip::new("Can't parse this file")
                    .build(window, cx)
            })
            .into_any_element()
    }
}

const BADGE_TEXT_SIZE: f32 = 10.;
/// For methods longer than `DELETE` (`OPTIONS`, custom verbs), so they stay
/// clear of the name.
const BADGE_TEXT_SIZE_LONG: f32 = 8.5;

fn badge_text_size(method: &str) -> f32 {
    if method.len() > "DELETE".len() {
        BADGE_TEXT_SIZE_LONG
    } else {
        BADGE_TEXT_SIZE
    }
}
/// The mockup's 34px slot: `DELETE` runs 3px into the gap, longer methods
/// are drawn smaller. Rows without a method keep the slot so names line up.
const METHOD_BADGE_WIDTH: f32 = 34.;
/// Gap between the badge and the name.
const ROW_GAP: f32 = 8.;
/// What a multi-request row's chevron takes left of the method slot: the
/// chevron and a gap.
const MULTI_HANG: f32 = CHEVRON_SIZE + 2.;
/// Left padding of a depth-0 row: room for a multi-request file's chevron
/// left of the method column.
const LEAD: f32 = MULTI_HANG;

/// Where the method column (badge or dots) starts at `depth`, px.
pub fn method_x(depth: usize) -> f32 {
    LEAD + INDENT * depth as f32
}

/// Where the file or request name starts at `depth`, px: the same for
/// single- and multi-request rows.
#[cfg(test)]
pub fn name_x(depth: usize) -> f32 {
    method_x(depth) + METHOD_BADGE_WIDTH + ROW_GAP
}

/// Where a multi-request row's chevron starts at `depth`, px: left of the
/// method column.
pub fn chevron_x(depth: usize) -> f32 {
    method_x(depth) - MULTI_HANG
}

/// The method as coloured mono text in a fixed slot so names line up.
pub fn method_badge(method: Option<&str>) -> Div {
    let text = method.map(str::to_owned).unwrap_or_default();
    div()
        .flex_none()
        .w(px(METHOD_BADGE_WIDTH))
        .font_family(super::app::mono_font())
        .text_size(px(badge_text_size(&text)))
        .font_weight(FontWeight::MEDIUM)
        .text_color(method_color(method.unwrap_or("")))
        .child(text)
}

/// The tree's context menu for `target`; `folder` is where "New request
/// here" puts the file.
fn tree_menu(
    menu: gpui_kit::component::menu::PopupMenu,
    ws: &WeakEntity<Workspace>,
    target: &Path,
    folder: Option<&Path>,
) -> gpui_kit::component::menu::PopupMenu {
    let folder = folder.map(Path::to_path_buf);
    let (new_ws, rename_ws, delete_ws) = (ws.clone(), ws.clone(), ws.clone());
    let (rename_path, delete_path) = (target.to_path_buf(), target.to_path_buf());
    menu.item(
        PopupMenuItem::new("New request here").on_click(move |_, window, cx| {
            let folder = folder.clone();
            new_ws
                .update(cx, |this, cx| {
                    this.new_request(folder, window, cx);
                })
                .ok();
        }),
    )
    .item(PopupMenuItem::new("Rename").on_click(move |_, window, cx| {
        let path = rename_path.clone();
        rename_ws
            .update(cx, |this, cx| this.prompt_rename(path, window, cx))
            .ok();
    }))
    .separator()
    .item(PopupMenuItem::new("Delete").on_click(move |_, window, cx| {
        let path = delete_path.clone();
        delete_ws
            .update(cx, |this, cx| this.prompt_delete(path, window, cx))
            .ok();
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        BADGE_TEXT_SIZE, CHEVRON_SIZE, Group, INDENT, METHOD_BADGE_WIDTH, MULTI_HANG, ROW_GAP, Row,
        RowKind, TreeNode, badge_text_size, chevron_x, method_x, name_x, request_label,
        visible_rows,
    };
    use crate::model::collection::RequestEntry;

    /// The advance of Geist Mono, in em.
    const MONO_ADVANCE_EM: f32 = 0.61;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn multi_and_single_rows_share_name_and_method_x() {
        for depth in 0..4 {
            // Single rows: padding = method_x, then badge, gap, name.
            let single_name = method_x(depth) + METHOD_BADGE_WIDTH + ROW_GAP;
            assert_eq!(name_x(depth), single_name);
            // Multi rows: same padding; the chevron hangs left of it.
            assert!(chevron_x(depth) >= 0.);
            assert_eq!(chevron_x(depth) + MULTI_HANG, method_x(depth));
            assert!(chevron_x(depth) + CHEVRON_SIZE <= method_x(depth));
        }
        assert_eq!(name_x(1) - name_x(0), INDENT);
    }

    #[test]
    fn dots_start_at_the_badge_x_and_chip_stays_near_the_name() {
        // The stack is left-aligned at offset 0 of the method column.
        assert_eq!(super::Stack::dot_left(0), 0.);
        let max = super::Stack::new(["GET", "POST", "PUT", "PATCH"]);
        assert!(max.width() <= METHOD_BADGE_WIDTH + ROW_GAP);
    }

    #[test]
    fn method_badge_fits_every_method_with_room_before_the_name() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
            let text = method.len() as f32 * MONO_ADVANCE_EM * badge_text_size(method);
            assert!(
                // At least 4px of the gap stays clear before the name.
                text <= METHOD_BADGE_WIDTH + ROW_GAP - 4.,
                "{method} needs {text}px of {METHOD_BADGE_WIDTH}px"
            );
        }
        assert_eq!(badge_text_size("DELETE"), BADGE_TEXT_SIZE);
    }

    fn file(name: &str, rel: &str, method: Option<&str>) -> TreeNode {
        TreeNode::File {
            name: name.into(),
            rel_path: rel.into(),
            method: method.map(Into::into),
            requests: Vec::new(),
            parse_ok: true,
        }
    }

    fn multi(name: &str, rel: &str, requests: &[(&str, &str)]) -> TreeNode {
        TreeNode::File {
            name: name.into(),
            rel_path: rel.into(),
            method: None,
            requests: requests
                .iter()
                .enumerate()
                .map(|(i, &(method, name))| RequestEntry {
                    method: method.into(),
                    name: name.into(),
                    line: Some(i as u32 * 3 + 1),
                })
                .collect(),
            parse_ok: true,
        }
    }

    fn tree() -> Vec<TreeNode> {
        vec![
            TreeNode::Folder {
                name: "auth".into(),
                children: vec![file("login.yaml", "auth/login.yaml", Some("POST"))],
            },
            TreeNode::Folder {
                name: "charges".into(),
                children: vec![
                    file(
                        "create-charge.yaml",
                        "charges/create-charge.yaml",
                        Some("POST"),
                    ),
                    file("list-charges.yml", "charges/list-charges.yml", Some("GET")),
                ],
            },
            file("smoke.yaml", "smoke.yaml", Some("GET")),
        ]
    }

    fn none() -> HashSet<PathBuf> {
        HashSet::new()
    }

    fn labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match &r.kind {
                RowKind::Folder { name, .. } => format!("{}{name}/", "  ".repeat(r.depth)),
                RowKind::File { label, .. } => format!("{}{label}", "  ".repeat(r.depth)),
                RowKind::Request { name, .. } => format!("{}- {name}", "  ".repeat(r.depth)),
            })
            .collect()
    }

    #[test]
    fn rows_list_everything_open_by_default() {
        let rows = visible_rows(&tree(), &none(), &none(), "");
        assert_eq!(
            labels(&rows),
            [
                "auth/",
                "  login",
                "charges/",
                "  create-charge",
                "  list-charges",
                "smoke"
            ]
        );
    }

    #[test]
    fn collapsed_folder_hides_children_and_counts_them() {
        let collapsed = HashSet::from([PathBuf::from("charges")]);
        let rows = visible_rows(&tree(), &collapsed, &none(), "");
        assert_eq!(labels(&rows), ["auth/", "  login", "charges/", "smoke"]);
        assert!(matches!(
            rows[2].kind,
            RowKind::Folder {
                open: false,
                count: 2,
                ..
            }
        ));
    }

    #[test]
    fn filter_is_case_insensitive_and_keeps_parents_open() {
        let collapsed = HashSet::from([PathBuf::from("charges")]);
        let rows = visible_rows(&tree(), &collapsed, &none(), "  CHARGE ");
        assert_eq!(
            labels(&rows),
            ["charges/", "  create-charge", "  list-charges"]
        );
        let rows = visible_rows(&tree(), &none(), &none(), "list");
        assert_eq!(labels(&rows), ["charges/", "  list-charges"]);
    }

    #[test]
    fn filter_matching_a_folder_keeps_its_contents() {
        let rows = visible_rows(&tree(), &none(), &none(), "auth");
        assert_eq!(labels(&rows), ["auth/", "  login"]);
        assert!(visible_rows(&tree(), &none(), &none(), "nothing").is_empty());
    }

    #[test]
    fn request_label_drops_yaml_extension() {
        assert_eq!(request_label("create-charge.yaml"), "create-charge");
        assert_eq!(request_label("a.yml"), "a");
        assert_eq!(request_label("notes"), "notes");
    }

    fn flows() -> Vec<TreeNode> {
        vec![TreeNode::Folder {
            name: "users".into(),
            children: vec![
                multi(
                    "user-flow.yaml",
                    "users/user-flow.yaml",
                    &[
                        ("GET", "Get user"),
                        ("GET", "User posts"),
                        ("POST", "Create todo"),
                    ],
                ),
                file("get-user.yaml", "users/get-user.yaml", Some("GET")),
            ],
        }]
    }

    #[test]
    fn multi_request_file_is_collapsed_by_default_with_count_and_methods() {
        let rows = visible_rows(&flows(), &none(), &none(), "");
        assert_eq!(labels(&rows), ["users/", "  user-flow", "  get-user"]);
        let RowKind::File { group, .. } = &rows[1].kind else {
            panic!()
        };
        assert_eq!(
            group.as_ref(),
            Some(&Group {
                open: false,
                count: 3,
                methods: vec!["GET".into(), "GET".into(), "POST".into()],
            })
        );
        let RowKind::File { group, .. } = &rows[2].kind else {
            panic!()
        };
        assert_eq!(group, &None, "single-request files keep the badge");
        // Folder counts requests, not files.
        assert!(matches!(rows[0].kind, RowKind::Folder { count: 4, .. }));
    }

    #[test]
    fn expanded_file_lists_its_requests_one_level_deeper() {
        let expanded = HashSet::from([PathBuf::from("users/user-flow.yaml")]);
        let rows = visible_rows(&flows(), &none(), &expanded, "");
        assert_eq!(
            labels(&rows),
            [
                "users/",
                "  user-flow",
                "    - Get user",
                "    - User posts",
                "    - Create todo",
                "  get-user"
            ]
        );
        assert_eq!(
            rows[4].kind,
            RowKind::Request {
                rel_path: "users/user-flow.yaml".into(),
                index: 2,
                method: "POST".into(),
                name: "Create todo".into(),
                line: Some(7),
            }
        );
    }

    #[test]
    fn filter_on_a_request_name_opens_its_file_with_only_the_hits() {
        let rows = visible_rows(&flows(), &none(), &none(), "posts");
        assert_eq!(labels(&rows), ["users/", "  user-flow", "    - User posts"]);
        // A file-name hit keeps the file's own expansion state.
        let rows = visible_rows(&flows(), &none(), &none(), "flow");
        assert_eq!(labels(&rows), ["users/", "  user-flow"]);
    }

    #[test]
    fn parse_failed_file_is_not_expandable() {
        let tree = vec![TreeNode::File {
            name: "bad.yaml".into(),
            rel_path: "bad.yaml".into(),
            method: None,
            requests: Vec::new(),
            parse_ok: false,
        }];
        let rows = visible_rows(&tree, &none(), &none(), "");
        assert!(matches!(
            &rows[0].kind,
            RowKind::File {
                group: None,
                parse_ok: false,
                ..
            }
        ));
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, Role, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;

    const FLOW: &str = "requests:\n  - name: One\n    url: https://a.example.com\n  - name: Two\n    method: POST\n    url: https://b.example.com\n";

    fn open(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("users")).unwrap();
        std::fs::write(dir.path().join("users/flow.yaml"), FLOW).unwrap();
        let root = dir.path().to_path_buf();
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
                    Workspace::new(
                        Some(root),
                        StateStore::none(),
                        super::super::workspace::test_secrets(),
                        super::super::workspace::test_backend(),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        (dir, window, ws)
    }

    #[gpui_kit::test]
    fn chevron_expands_a_multi_request_file_without_opening_it(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            let rows = |window: &mut gpui_kit::Window| {
                window
                    .within("collection-tree")
                    .find_all_by_role(Role::TreeItem)
                    .len()
            };
            // users/, flow
            assert_eq!(rows(window), 2);
            window
                .within("collection-tree")
                .click(("tree-chevron", 1usize), cx);
            window.render_frame(cx);
            assert_eq!(rows(window), 4);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(ws.state.open.is_empty()));
    }

    #[gpui_kit::test]
    fn clicking_a_request_opens_its_file_at_the_request(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, _| {
                ws.expanded.insert("users/flow.yaml".into());
            });
            window.render_frame(cx);
            window
                .within("collection-tree")
                .click(("tree-row", 3usize), cx);
            window.render_frame(cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, cx| {
            assert_eq!(ws.state.open.len(), 1);
            assert_eq!(ws.state.open[0].rel_path, Path::new("users/flow.yaml"));
            assert_eq!(
                ws.selected_request,
                Some(("users/flow.yaml".into(), 1usize))
            );
            let cursor = ws.tabs[0].editor.read(cx).cursor_position();
            assert_eq!(cursor.line, 3, "the second request's item line");
        });
    }
}
