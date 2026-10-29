//! The collections column: header, filter and the collection tree.

use super::workspace::Workspace;
use super::{color, method_color};
use crate::model::collection::TreeNode;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const SIDEBAR_WIDTH: f32 = 264.;
const ROW_HEIGHT: f32 = 28.;
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
        /// Request files anywhere below the folder.
        count: usize,
    },
    File {
        rel_path: PathBuf,
        /// The file name without `.yaml` / `.yml`.
        label: String,
        method: Option<String>,
        parse_ok: bool,
    },
}

/// The rows to draw: folders in `collapsed` hide their children. A non-empty
/// `filter` keeps files whose name contains it (any case) and folders whose
/// name contains it (with all their contents), plus the folders leading to
/// them, shown open.
pub fn visible_rows(tree: &[TreeNode], collapsed: &HashSet<PathBuf>, filter: &str) -> Vec<Row> {
    let needle = filter.trim().to_lowercase();
    let mut rows = Vec::new();
    walk(tree, Path::new(""), 0, collapsed, &needle, false, &mut rows);
    rows
}

/// Appends `nodes`' rows; returns whether any was appended.
fn walk(
    nodes: &[TreeNode],
    parent: &Path,
    depth: usize,
    collapsed: &HashSet<PathBuf>,
    needle: &str,
    shown_whole: bool,
    rows: &mut Vec<Row>,
) -> bool {
    let start = rows.len();
    let matches =
        |name: &str| shown_whole || needle.is_empty() || name.to_lowercase().contains(needle);
    for node in nodes {
        match node {
            TreeNode::Folder { name, children } => {
                let path = parent.join(name);
                let filtering = !needle.is_empty();
                let open = filtering || !collapsed.contains(&path);
                let at = rows.len();
                rows.push(Row {
                    depth,
                    kind: RowKind::Folder {
                        path: path.clone(),
                        name: name.clone(),
                        open,
                        count: file_count(children),
                    },
                });
                let whole = filtering && matches(name);
                let any = open && walk(children, &path, depth + 1, collapsed, needle, whole, rows);
                if filtering && !whole && !any {
                    rows.truncate(at);
                }
            }
            TreeNode::File {
                name,
                rel_path,
                method,
                parse_ok,
            } => {
                let label = request_label(name);
                if matches(label) {
                    rows.push(Row {
                        depth,
                        kind: RowKind::File {
                            rel_path: rel_path.clone(),
                            label: label.to_string(),
                            method: method.clone(),
                            parse_ok: *parse_ok,
                        },
                    });
                }
            }
        }
    }
    rows.len() > start
}

fn file_count(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|n| match n {
            TreeNode::Folder { children, .. } => file_count(children),
            TreeNode::File { .. } => 1,
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
            return column.child(header).child(self.render_no_collections(cx));
        };

        let rows = visible_rows(
            &collection.tree,
            &self.collapsed,
            &self.filter.read(cx).value(),
        );
        let active = self.state.active_document().map(|d| d.rel_path.clone());
        let tree = div()
            .id("collection-tree")
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
                    .map(|(ix, row)| self.render_row(ix, row, active.as_deref(), cx)),
            );

        column
            .child(
                header.child(
                    Button::new("sidebar-new-request")
                        .ghost()
                        .small()
                        .icon(IconName::Plus)
                        .tooltip("New request")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.new_request(None, window, cx);
                        })),
                ),
            )
            .child(
                div().px(px(12.)).pb(px(8.)).child(
                    Input::new(&self.filter)
                        .small()
                        .prefix(
                            Icon::new(IconName::Search)
                                .size(px(13.))
                                .text_color(color("text-secondary")),
                        )
                        .cleanable(true),
                ),
            )
            .child(tree)
    }

    fn render_row(
        &self,
        ix: usize,
        row: Row,
        active: Option<&Path>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let base = div()
            .id(("tree-row", ix))
            .role(Role::TreeItem)
            .test_support()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(ROW_HEIGHT))
            .pr(px(8.))
            .pl(px(8. + INDENT * row.depth as f32))
            .rounded(px(6.))
            .text_size(px(13.))
            .text_color(color("text-secondary"))
            .whitespace_nowrap()
            .overflow_hidden()
            .cursor_pointer()
            .hover(|s| s.bg(color("surface-100")));
        let ws = cx.entity().downgrade();
        match row.kind {
            RowKind::Folder {
                path,
                name,
                open,
                count,
            } => {
                let chevron = if open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                };
                let toggle = path.clone();
                let menu_path = path.clone();
                base.text_color(color("text-primary"))
                    .child(
                        Icon::new(chevron)
                            .size(px(12.))
                            .text_color(color("text-secondary")),
                    )
                    .child(
                        Icon::new(IconName::Folder)
                            .size(px(14.))
                            .text_color(color("text-secondary")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_ellipsis()
                            .font_weight(FontWeight::MEDIUM)
                            .child(name),
                    )
                    .when(!open, |row| {
                        row.child(
                            div()
                                .text_size(px(11.))
                                .text_color(color("text-tertiary"))
                                .child(count.to_string()),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.collapsed.remove(&toggle) {
                            this.collapsed.insert(toggle.clone());
                        }
                        cx.notify();
                    }))
                    .context_menu(move |menu, _, _| {
                        tree_menu(menu, &ws, &menu_path, Some(&menu_path))
                    })
                    .into_any_element()
            }
            RowKind::File {
                rel_path,
                label,
                method,
                parse_ok,
            } => {
                let selected = active == Some(rel_path.as_path());
                let open_path = rel_path.clone();
                let folder = rel_path.parent().map(Path::to_path_buf);
                base.when(selected, |row| {
                    row.bg(color("surface-200"))
                        .text_color(color("text-primary"))
                })
                .tooltip({
                    let full = rel_path.display().to_string();
                    move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(full.clone()).build(window, cx)
                    }
                })
                .child(method_badge(method.as_deref()))
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(label),
                )
                .when(!parse_ok, |row| {
                    row.child(
                        div()
                            .id(("parse-error", ix))
                            .flex_none()
                            .size(px(6.))
                            .rounded_full()
                            .bg(color("danger"))
                            .tooltip(|window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new("Can't parse this file")
                                    .build(window, cx)
                            }),
                    )
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_file(&open_path, window, cx);
                }))
                .context_menu(move |menu, _, _| tree_menu(menu, &ws, &rel_path, folder.as_deref()))
                .into_any_element()
            }
        }
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

const BADGE_TEXT_SIZE: f32 = 10.;
/// The advance of Geist Mono and Menlo, in em.
const MONO_ADVANCE_EM: f32 = 0.61;
/// Fits `OPTIONS`, the longest method, with room before the name (about
/// 47px); rows without a method keep the slot so names line up.
const METHOD_BADGE_WIDTH: f32 = "OPTIONS".len() as f32 * MONO_ADVANCE_EM * BADGE_TEXT_SIZE + 4.;

/// The method as coloured mono text in a fixed slot so names line up.
pub fn method_badge(method: Option<&str>) -> Div {
    let text = method.map(str::to_owned).unwrap_or_default();
    div()
        .flex_none()
        .w(px(METHOD_BADGE_WIDTH))
        .font_family(super::app::mono_font())
        .text_size(px(BADGE_TEXT_SIZE))
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
        BADGE_TEXT_SIZE, METHOD_BADGE_WIDTH, MONO_ADVANCE_EM, Row, RowKind, TreeNode,
        request_label, visible_rows,
    };
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn method_badge_fits_every_method_with_room_before_the_name() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
            let text = method.len() as f32 * MONO_ADVANCE_EM * BADGE_TEXT_SIZE;
            assert!(
                text + 2. <= METHOD_BADGE_WIDTH,
                "{method} needs {text}px of {METHOD_BADGE_WIDTH}px"
            );
        }
    }

    fn file(name: &str, rel: &str, method: Option<&str>) -> TreeNode {
        TreeNode::File {
            name: name.into(),
            rel_path: rel.into(),
            method: method.map(Into::into),
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
            file("smoke.yaml", "smoke.yaml", None),
        ]
    }

    fn labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match &r.kind {
                RowKind::Folder { name, .. } => format!("{}{name}/", "  ".repeat(r.depth)),
                RowKind::File { label, .. } => format!("{}{label}", "  ".repeat(r.depth)),
            })
            .collect()
    }

    #[test]
    fn rows_list_everything_open_by_default() {
        let rows = visible_rows(&tree(), &HashSet::new(), "");
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
        let rows = visible_rows(&tree(), &collapsed, "");
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
        let rows = visible_rows(&tree(), &collapsed, "  CHARGE ");
        assert_eq!(
            labels(&rows),
            ["charges/", "  create-charge", "  list-charges"]
        );
        let rows = visible_rows(&tree(), &HashSet::new(), "list");
        assert_eq!(labels(&rows), ["charges/", "  list-charges"]);
    }

    #[test]
    fn filter_matching_a_folder_keeps_its_contents() {
        let rows = visible_rows(&tree(), &HashSet::new(), "auth");
        assert_eq!(labels(&rows), ["auth/", "  login"]);
        assert!(visible_rows(&tree(), &HashSet::new(), "nothing").is_empty());
    }

    #[test]
    fn request_label_drops_yaml_extension() {
        assert_eq!(request_label("create-charge.yaml"), "create-charge");
        assert_eq!(request_label("a.yml"), "a");
        assert_eq!(request_label("notes"), "notes");
    }
}
