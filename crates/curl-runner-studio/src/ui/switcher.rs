//! The collection switcher (board Switcher): the sidebar header names the
//! open collection and opens a menu of the others, in two groups, My
//! collections (recent folders of one's own) and Shared with me (received
//! copies), with the ways to open more. Right-click a row: Reveal in
//! Finder, and Remove from list or Remove copy….

use super::color;
use super::icons::{self, glyph};
use super::workspace::{Workspace, notify_error};
use crate::model::collection_meta::CollectionMeta;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MENU_WIDTH: f32 = 300.;
/// A click on the header right after an outside click closed the menu
/// doesn't reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(200);

/// One row of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherRow {
    pub root: PathBuf,
    pub name: String,
    /// `Cloud` for a folder linked to a collection there; `by Acme` for a
    /// shared copy.
    pub detail: Option<String>,
    pub shared: bool,
}

#[derive(Default)]
pub(super) struct SwitcherState {
    open: bool,
    closed_at: Option<Instant>,
    mine: Vec<SwitcherRow>,
    shared: Vec<SwitcherRow>,
}

fn folder_name(root: &std::path::Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl Workspace {
    /// The header's button: the open collection's name, or `Collections`
    /// with none open.
    pub(super) fn render_switcher(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let title = self
            .state
            .collection
            .as_ref()
            .map(|_| self.collection_title().unwrap_or_default())
            .filter(|t| !t.is_empty());
        let label = title.clone().unwrap_or_else(|| "Collections".into());
        let menu = self.switcher.open.then(|| self.render_switcher_menu(cx));
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .id("sidebar-switcher")
                    .role(Role::Button)
                    .test_support()
                    .aria_label(format!("Collection: {label}"))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(30.))
                    .px(px(8.))
                    .border_1()
                    .border_color(color("border-200"))
                    .rounded(px(6.))
                    .text_size(px(13.))
                    .text_color(color("text-primary"))
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface-100")))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_switcher(cx)))
                    .child(
                        div()
                            .id("sidebar-title")
                            .role(Role::Label)
                            .test_support()
                            .aria_label(label.clone())
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .child(label),
                    )
                    .child(
                        glyph(icons::CHEVRON_DOWN)
                            .size(px(12.))
                            .flex_none()
                            .text_color(color("text-tertiary")),
                    ),
            )
            .children(menu.map(|m| deferred(m).with_priority(1)))
    }

    /// Opens or closes the menu, reading its rows afresh on open.
    pub(super) fn toggle_switcher(&mut self, cx: &mut Context<Self>) {
        let just_closed = self
            .switcher
            .closed_at
            .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
        self.switcher.closed_at = None;
        if self.switcher.open || just_closed {
            self.switcher.open = false;
        } else {
            self.refresh_recent();
            self.refresh_received_all();
            self.switcher.mine = self.my_collections();
            self.switcher.shared = self.shared_with_me();
            self.switcher.open = true;
        }
        cx.notify();
    }

    fn close_switcher(&mut self, cx: &mut Context<Self>) {
        self.switcher.open = false;
        cx.notify();
    }

    /// Recent folders that exist and aren't received copies.
    fn my_collections(&self) -> Vec<SwitcherRow> {
        let server = self
            .backend
            .server()
            .url()
            .trim_end_matches('/')
            .to_string();
        let base = self.store.base().map(std::path::Path::to_path_buf);
        let row = |root: &std::path::Path| {
            let linked = base.as_deref().is_some_and(|b| {
                CollectionMeta::load(b, root)
                    .cloud
                    .is_some_and(|l| l.server == server)
            });
            SwitcherRow {
                root: root.to_path_buf(),
                name: folder_name(root),
                detail: (crate::features::SERVER && linked).then(|| "Cloud".into()),
                shared: false,
            }
        };
        let mut rows: Vec<SwitcherRow> = self
            .recent
            .iter()
            .filter(|(r, exists)| *exists && !self.received_all.iter().any(|c| c.root == r.path))
            .map(|(r, _)| row(&r.path))
            .collect();
        // The open folder is listed even when Recent forgot it.
        if let Some(c) = &self.state.collection
            && self.shared_by().is_none()
            && !rows.iter().any(|r| r.root == c.root)
        {
            rows.insert(0, row(&c.root));
        }
        rows
    }

    fn shared_with_me(&self) -> Vec<SwitcherRow> {
        if !crate::features::SERVER {
            return Vec::new();
        }
        self.received_all
            .iter()
            .map(|c| SwitcherRow {
                root: c.root.clone(),
                name: c.share.name.clone(),
                detail: Some(format!("by {}", c.share.team)),
                shared: true,
            })
            .collect()
    }

    fn render_switcher_menu(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let current = self.state.collection.as_ref().map(|c| c.root.clone());
        let ws = cx.entity().downgrade();
        let group = |title: &'static str| {
            div()
                .px(px(10.))
                .pt(px(8.))
                .pb(px(4.))
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color("text-tertiary"))
                .child(title)
        };
        let row = |ix: usize, r: SwitcherRow, cx: &mut Context<Self>| {
            let on = current.as_ref() == Some(&r.root);
            let root = r.root.clone();
            let (menu_ws, menu_root, shared, name) =
                (ws.clone(), r.root.clone(), r.shared, r.name.clone());
            div()
                .id(SharedString::from(format!(
                    "switcher-{}-{ix}",
                    if r.shared { "shared" } else { "mine" }
                )))
                .role(Role::MenuItemRadio)
                .test_support()
                .aria_label(r.name.clone())
                .aria_selected(on)
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(6.))
                .text_size(px(13.))
                .text_color(color("text-primary"))
                .cursor_pointer()
                .when(on, |d| d.bg(color("surface-200")))
                .when(!on, |d| d.hover(|s| s.bg(color("surface-100"))))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.close_switcher(cx);
                    if !on {
                        this.switch_to(root.clone(), window, cx);
                    }
                }))
                .context_menu(move |menu, _, _| {
                    let (reveal, remove_ws, remove_root, remove_name) = (
                        menu_root.clone(),
                        menu_ws.clone(),
                        menu_root.clone(),
                        name.clone(),
                    );
                    let menu = menu.item(
                        PopupMenuItem::new("Reveal in Finder")
                            .on_click(move |_, _, cx| cx.reveal_path(&reveal)),
                    );
                    // The open one stays: open another first.
                    if on {
                        return menu;
                    }
                    let label = if shared {
                        "Remove copy…"
                    } else {
                        "Remove from list"
                    };
                    menu.separator().item(PopupMenuItem::new(label).on_click(
                        move |_, window, cx| {
                            let (root, name) = (remove_root.clone(), remove_name.clone());
                            remove_ws
                                .update(cx, |this, cx| {
                                    if shared {
                                        this.prompt_remove_copy(root, name, window, cx);
                                    } else {
                                        this.remove_from_list(&root, cx);
                                    }
                                })
                                .ok();
                        },
                    ))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis()
                        .child(r.name.clone()),
                )
                .children(r.detail.clone().map(|d| {
                    div()
                        .flex_none()
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(d)
                }))
                .when(on, |d| {
                    d.child(
                        glyph(icons::CHECK)
                            .size(px(12.))
                            .flex_none()
                            .text_color(color("text-secondary")),
                    )
                })
        };
        let action = |id: &'static str, label: &'static str, keys: Option<&'static str>| {
            div()
                .id(id)
                .role(Role::MenuItem)
                .test_support()
                .aria_label(label)
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(6.))
                .text_size(px(13.))
                .text_color(color("text-primary"))
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-100")))
                .child(div().flex_1().child(label))
                .children(keys.map(|k| {
                    div()
                        .text_size(px(11.))
                        .font_family(super::app::mono_font())
                        .text_color(color("text-tertiary"))
                        .child(k)
                }))
        };
        let mine = self.switcher.mine.clone();
        let shared = self.switcher.shared.clone();
        let server = crate::features::SERVER;
        div()
            .id("switcher-menu")
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.switcher.open = false;
                this.switcher.closed_at = Some(Instant::now());
                cx.notify();
            }))
            .role(Role::Menu)
            .test_support()
            .aria_label("Collections")
            .occlude()
            .absolute()
            .top(px(32.))
            .left_0()
            .w(px(MENU_WIDTH))
            .max_h(px(480.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p(px(6.))
            .bg(color("surface-raised"))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(10.))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.6),
                offset: point(px(0.), px(12.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }])
            .when(!mine.is_empty(), |m| {
                m.child(group("My collections"))
                    .children(mine.into_iter().enumerate().map(|(ix, r)| row(ix, r, cx)))
            })
            .when(!shared.is_empty(), |m| {
                m.child(group("Shared with me"))
                    .children(shared.into_iter().enumerate().map(|(ix, r)| row(ix, r, cx)))
            })
            .child(
                div()
                    .h(px(1.))
                    .mx(px(4.))
                    .my(px(6.))
                    .bg(color("border-100")),
            )
            .child(
                action("switcher-open-folder", "Open folder…", Some("⌘O")).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.close_switcher(cx);
                        this.open_folder(window, cx);
                    },
                )),
            )
            .when(server, |m| {
                m.child(
                    action("switcher-open-cloud", "Open from Cloud…", None).on_click(cx.listener(
                        |this, _, window, cx| {
                            this.close_switcher(cx);
                            this.open_from_cloud(None, window, cx);
                        },
                    )),
                )
                .child(
                    action("switcher-open-link", "Open a shared link…", Some("⌘L")).on_click(
                        cx.listener(|this, _, window, cx| {
                            this.close_switcher(cx);
                            this.open_shared(None, window, cx);
                        }),
                    ),
                )
            })
    }

    /// The open menu's rows, by name: (My collections, Shared with me).
    #[cfg(all(test, feature = "server"))]
    pub(super) fn switcher_rows(&self) -> (Vec<String>, Vec<String>) {
        let names = |rows: &[SwitcherRow]| rows.iter().map(|r| r.name.clone()).collect();
        (names(&self.switcher.mine), names(&self.switcher.shared))
    }

    /// `Remove from list`: forgets a recent folder; the folder stays.
    pub(super) fn remove_from_list(&mut self, root: &std::path::Path, cx: &mut Context<Self>) {
        self.store.remove_recent(root);
        self.refresh_recent();
        self.switcher.mine = self.my_collections();
        cx.notify();
    }

    /// `Remove copy…`: asks, then deletes the copy, its values and record.
    fn prompt_remove_copy(
        &mut self,
        root: PathBuf,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ws = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (ws, root) = (ws.clone(), root.clone());
            alert
                .title(format!("Remove your copy of {name}?"))
                .description(
                    "Its folder and the values you filled in are deleted. You can add it again from its link.",
                )
                .ok_text("Remove copy")
                .ok_variant(ButtonVariant::Danger)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    ws.update(cx, |this, cx| this.remove_copy(&root, window, cx))
                        .ok();
                    true
                })
        });
    }

    pub(super) fn remove_copy(
        &mut self,
        root: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Some(base), Some(copy)) = (
            self.store.base().map(std::path::Path::to_path_buf),
            self.received_all.iter().find(|c| c.root == root).cloned(),
        ) else {
            return;
        };
        if let Err(e) = crate::model::shared_copy::remove_copy(&base, &copy, self.secrets.as_ref())
        {
            notify_error(e, window, cx);
        }
        self.store.remove_recent(root);
        self.refresh_recent();
        self.refresh_received_all();
        self.switcher.shared = self.shared_with_me();
        cx.notify();
    }
}
