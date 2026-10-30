//! The empty state shown when no collection is open (`Welcome.dc.html`):
//! the start actions, the folders opened before and a note on secrets.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::new_dialog::display_path;
use super::settings_kit::later_badge;
use super::state::RecentEntry;
use super::workspace::Workspace;
use crate::model::history::relative_time;
use chrono::Local;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::Path;

/// One of the four start actions.
struct Card {
    id: &'static str,
    icon: &'static str,
    keys: &'static str,
    title: &'static str,
    detail: &'static str,
    /// `None`: shown disabled with the `Later build` badge.
    action: Option<fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>,
}

const CARDS: [Card; 4] = [
    Card {
        id: "welcome-new-collection",
        icon: icons::PLUS,
        keys: "⌘N",
        title: "New collection",
        detail: "Pick an empty folder and start with a first request.",
        action: Some(|ws, window, cx| {
            ws.new_collection(window, cx);
        }),
    },
    Card {
        id: "welcome-open-folder",
        icon: icons::FOLDER,
        keys: "⌘O",
        title: "Open folder",
        detail: "Work on YAML files already on disk, for example a repo checkout.",
        action: Some(|ws, window, cx| ws.open_folder(window, cx)),
    },
    Card {
        id: "welcome-open-link",
        icon: icons::LINK,
        keys: "⌘L",
        title: "Open a shared link",
        detail: "Paste a curl-runner link someone sent you. Opens read-only.",
        action: None,
    },
    Card {
        id: "welcome-import",
        icon: icons::DOWNLOAD,
        keys: "⌘I",
        title: "Import",
        detail: "From curl commands or a folder of YAML files.",
        action: Some(|ws, window, cx| {
            ws.import(window, cx);
        }),
    },
];

impl Workspace {
    pub(super) fn render_welcome(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let intro = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(40.))
                    .rounded(px(10.))
                    .bg(color("primary"))
                    .text_color(color("on-primary"))
                    .font_family(mono_font())
                    .font_weight(FontWeight::MEDIUM)
                    .text_size(px(15.))
                    .child(">_"),
            )
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(28.))
                    .line_height(relative(1.2))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Welcome to curl-runner studio"),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.6))
                    .text_color(color("text-secondary"))
                    .child(
                        "Write requests as YAML, run them as tests, and share collections \
                         read-only with the people who call your API.",
                    ),
            );
        div()
            .id("welcome")
            .flex()
            .flex_1()
            .min_w_0()
            .justify_center()
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(32.))
                    .w(px(720.))
                    .pt(px(72.))
                    .pb(px(40.))
                    .child(intro)
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap(px(12.))
                            .children(CARDS.iter().map(|card| render_card(card, cx))),
                    )
                    .children(self.render_recent(cx))
                    .child(render_secrets_note()),
            )
    }

    /// Recent, or nothing when no folder was opened yet.
    fn render_recent(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.recent.is_empty() {
            return None;
        }
        let now = Local::now();
        let rows = self
            .recent
            .iter()
            .enumerate()
            .map(|(ix, (entry, exists))| recent_row(ix, entry, *exists, now, cx))
            .collect::<Vec<_>>();
        Some(
            div()
                .id("welcome-recent")
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(8.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color("text-secondary"))
                                .child("Recent"),
                        )
                        .child(
                            div()
                                .id("welcome-recent-clear")
                                .role(Role::Button)
                                .test_support()
                                .text_color(color("text-tertiary"))
                                .hover(|s| s.text_color(color("text-primary")))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.clear_recent();
                                    this.refresh_recent();
                                    cx.notify();
                                }))
                                .child("Clear"),
                        ),
                )
                .child(div().flex().flex_col().children(rows)),
        )
    }

    /// Opens a recent folder as the collection, once unsaved changes are
    /// settled (there are none on Welcome, but a recent may be opened from
    /// elsewhere later).
    fn open_recent(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if !path.is_dir() {
            self.refresh_recent();
            cx.notify();
            return;
        }
        if self.open_collection(path, window, cx) {
            self.remember_folder(path);
        }
    }
}

fn recent_row(
    ix: usize,
    entry: &RecentEntry,
    exists: bool,
    now: chrono::DateTime<Local>,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let name = entry.path.file_name().map_or_else(
        || entry.path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let fg = |token| color(if exists { token } else { "text-disabled" });
    let path = entry.path.clone();
    let row = div()
        .id(("recent-row", ix))
        .role(Role::Button)
        .test_support()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(40.))
        .px(px(8.))
        .rounded(px(6.))
        .text_size(px(13.))
        .text_color(fg("text-primary"))
        .child(
            glyph(icons::FOLDER)
                .size(px(16.))
                .flex_none()
                .text_color(fg("text-secondary")),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .line_height(relative(1.3))
                .child(div().truncate().child(name))
                .child(
                    div()
                        .truncate()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(fg("text-tertiary"))
                        .child(display_path(&entry.path)),
                ),
        );
    if exists {
        row.child(
            div()
                .flex_none()
                .px(px(5.))
                .rounded(px(4.))
                .border_1()
                .border_color(color("border-200"))
                .text_size(px(11.))
                .text_color(color("text-secondary"))
                .child("Local"),
        )
        .child(
            div()
                .flex_none()
                .w(px(72.))
                .text_right()
                .text_size(px(12.))
                .text_color(color("text-tertiary"))
                .child(relative_time(entry.opened_at, now)),
        )
        .cursor_pointer()
        .hover(|s| s.bg(color("surface-100")))
        .on_click(cx.listener(move |this, _, window, cx| this.open_recent(&path, window, cx)))
    } else {
        row.child(
            div()
                .flex_none()
                .text_size(px(12.))
                .text_color(color("text-disabled"))
                .child("Folder not found"),
        )
        .child(
            div()
                .id(("recent-remove", ix))
                .role(Role::Button)
                .test_support()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(24.))
                .rounded(px(6.))
                .text_color(color("text-secondary"))
                .hover(|s| s.bg(color("surface-100")).text_color(color("text-primary")))
                .cursor_pointer()
                .tooltip(|window, cx| Tooltip::new("Remove from Recent").build(window, cx))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.remove_recent(&path);
                    this.refresh_recent();
                    cx.notify();
                }))
                .child(glyph(icons::CROSS).size(px(12.))),
        )
    }
}

/// The board's footnote: secrets stay in the Keychain.
fn render_secrets_note() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(14.))
        .py(px(12.))
        .rounded(px(8.))
        .border_1()
        .border_color(color("border-100"))
        .text_size(px(12.))
        .text_color(color("text-secondary"))
        .child(glyph(icons::LOCK).size(px(14.)).flex_none())
        .child(
            div()
                .flex_1()
                .child("Secrets like tokens stay in your OS keychain. Collections only ever store variable names."),
        )
}

fn render_card(card: &Card, cx: &mut Context<Workspace>) -> impl IntoElement + use<> {
    let enabled = card.action.is_some();
    let fg = |token| color(if enabled { token } else { "text-disabled" });
    let top = div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(32.))
                .rounded(px(6.))
                .border_1()
                .border_color(color("border-200"))
                .child(
                    glyph(card.icon)
                        .size(px(16.))
                        .text_color(fg("text-primary")),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .when(!enabled, |d| d.child(later_badge()))
                .child(
                    div()
                        .px(px(5.))
                        .rounded(px(4.))
                        .border_1()
                        .border_color(color("border-200"))
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .line_height(px(18.))
                        .text_color(fg("text-secondary"))
                        .child(card.keys),
                ),
        );
    div()
        .id(card.id)
        .flex()
        .flex_col()
        .items_start()
        .gap(px(10.))
        .p(px(16.))
        .rounded(px(8.))
        .border_1()
        .border_color(color("border-100"))
        .child(top)
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg("text-primary"))
                .child(card.title),
        )
        .child(
            div()
                .text_size(px(12.))
                .line_height(relative(1.5))
                .text_color(fg("text-secondary"))
                .child(card.detail),
        )
        .map(|el| match card.action {
            Some(action) => el
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-100")))
                .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx))),
            None => el.cursor_not_allowed(),
        })
}

#[cfg(test)]
mod tests {
    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };

    fn open(cx: &mut TestAppContext, store: StateStore) -> (AnyWindowHandle, Entity<Workspace>) {
        cx.update(|cx| {
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
                        None,
                        store,
                        super::super::workspace::test_secrets(),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        })
    }

    #[gpui_kit::test]
    fn recent_opens_a_folder_and_skips_missing_ones(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let gone = tempfile::tempdir().unwrap();
        let gone_path = gone.path().to_path_buf();
        store.remember_folder(&gone_path);
        drop(gone);
        let there = tempfile::tempdir().unwrap();
        std::fs::write(there.path().join("a.yaml"), "request:\n  url: https://a\n").unwrap();
        store.remember_folder(there.path());

        let (window, ws) = open(cx, store.clone());
        ws.read_with(cx, |ws, _| {
            let flags: Vec<bool> = ws.recent.iter().map(|(_, e)| *e).collect();
            assert_eq!(flags, [true, false]);
            assert!(ws.state.collection.is_none());
        });
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            // A missing folder's row does nothing; Remove forgets it.
            window.click(("recent-row", 1usize), cx);
            window.render_frame(cx);
            window.click(("recent-remove", 1usize), cx);
            window.render_frame(cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            assert!(ws.state.collection.is_none());
            assert_eq!(ws.recent.len(), 1);
        });
        assert_eq!(store.recent().len(), 1);
        cx.update_window(window, |_, window, cx| {
            window.click(("recent-row", 0usize), cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, there.path());
        });
    }

    #[gpui_kit::test]
    fn clear_empties_recent(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        store.remember_folder(std::path::Path::new("/nonexistent/acme"));
        let (window, ws) = open(cx, store.clone());
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click("welcome-recent-clear", cx);
        })
        .unwrap();
        ws.read_with(cx, |ws, _| assert!(ws.recent.is_empty()));
        assert!(store.recent().is_empty());
    }
}
