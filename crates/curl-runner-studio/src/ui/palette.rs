//! The command palette (⌘K, `Palette.dc.html`): fuzzy search over the
//! collection's requests and the studio's actions.

use super::app::{NewCollection, NewRequest, OpenFolder, ToggleProblems, mono_font};
use super::color;
use super::sidebar::{method_badge, request_label};
use super::workspace::Workspace;
use crate::model::collection::TreeNode;
use gpui_kit::base::actions::Cancel;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Requests,
    /// Empty until there is more than one collection.
    Collections,
    Actions,
}

impl Group {
    const ALL: [Group; 3] = [Group::Requests, Group::Collections, Group::Actions];

    fn label(self) -> &'static str {
        match self {
            Group::Requests => "Requests",
            Group::Collections => "Collections",
            Group::Actions => "Actions",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteItem {
    pub group: Group,
    pub label: String,
    pub detail: String,
}

/// The items whose label contains `query` as a case-insensitive subsequence
/// (whitespace in the query is ignored), each with the matched character
/// indices of its label. Ranked by a contiguous match first, then the
/// earliest match, then the shorter label; ties keep input order. An empty
/// query keeps every item in input order.
pub fn palette_matches(query: &str, items: &[PaletteItem]) -> Vec<(usize, Vec<usize>)> {
    let query: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if query.is_empty() {
        return (0..items.len()).map(|ix| (ix, Vec::new())).collect();
    }
    let mut hits: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(ix, item)| {
            let label: Vec<char> = item.label.chars().map(fold).collect();
            let (contiguous, indices) = match_label(&query, &label)?;
            Some(((!contiguous, indices[0], label.len(), ix), indices))
        })
        .collect();
    hits.sort_by_key(|(key, _)| *key);
    hits.into_iter()
        .map(|((_, _, _, ix), indices)| (ix, indices))
        .collect()
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// `(contiguous, indices)` for a non-empty `query` in `label`, both folded.
fn match_label(query: &[char], label: &[char]) -> Option<(bool, Vec<usize>)> {
    if let Some(start) = label.windows(query.len()).position(|w| w == query) {
        return Some((true, (start..start + query.len()).collect()));
    }
    let mut indices = Vec::with_capacity(query.len());
    let mut rest = query.iter().peekable();
    for (i, c) in label.iter().enumerate() {
        if rest.next_if(|q| *q == c).is_some() {
            indices.push(i);
        }
    }
    (rest.peek().is_none()).then_some((false, indices))
}

/// What ↵ does on an item.
#[derive(Debug, Clone, PartialEq)]
enum Target {
    Request {
        rel_path: PathBuf,
        method: Option<String>,
    },
    Action(PaletteAction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PaletteAction {
    OpenFolder,
    NewRequest,
    NewCollection,
    ToggleProblems,
}

impl PaletteAction {
    const ALL: [PaletteAction; 4] = [
        PaletteAction::OpenFolder,
        PaletteAction::NewRequest,
        PaletteAction::NewCollection,
        PaletteAction::ToggleProblems,
    ];

    fn label(self) -> &'static str {
        match self {
            PaletteAction::OpenFolder => "Open folder…",
            PaletteAction::NewRequest => "New request…",
            PaletteAction::NewCollection => "New collection…",
            PaletteAction::ToggleProblems => "Toggle Problems",
        }
    }

    /// The studio action, for its shortcut.
    fn action(self) -> Box<dyn Action> {
        match self {
            PaletteAction::OpenFolder => Box::new(OpenFolder),
            PaletteAction::NewRequest => Box::new(NewRequest),
            PaletteAction::NewCollection => Box::new(NewCollection),
            PaletteAction::ToggleProblems => Box::new(ToggleProblems),
        }
    }
}

pub struct Palette {
    ws: WeakEntity<Workspace>,
    state: Entity<CommandState>,
    items: Vec<PaletteItem>,
    targets: Vec<Target>,
    query: SharedString,
}

impl Palette {
    /// A palette over `tree`'s request files and the studio actions.
    fn new(
        ws: WeakEntity<Workspace>,
        tree: &[TreeNode],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut entries = Vec::new();
        collect_requests(tree, &mut entries);
        entries.extend(PaletteAction::ALL.map(|a| {
            let item = PaletteItem {
                group: Group::Actions,
                label: a.label().into(),
                detail: String::new(),
            };
            (item, Target::Action(a))
        }));
        let (items, targets) = entries.into_iter().unzip();
        Palette {
            ws,
            state: cx.new(|cx| CommandState::new(window, cx)),
            items,
            targets,
            query: SharedString::default(),
        }
    }

    /// Opens or performs item `ix`, closing the palette first.
    fn perform(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.targets.get(ix).cloned() else {
            return;
        };
        window.close_dialog(cx);
        self.ws
            .update(cx, |ws, cx| match target {
                Target::Request { rel_path, .. } => ws.open_file(&rel_path, window, cx),
                Target::Action(PaletteAction::OpenFolder) => ws.open_folder(window, cx),
                Target::Action(PaletteAction::NewRequest) => {
                    ws.new_request(None, window, cx);
                }
                Target::Action(PaletteAction::NewCollection) => {
                    ws.new_collection(window, cx);
                }
                Target::Action(PaletteAction::ToggleProblems) => ws.toggle_problems(cx),
            })
            .ok();
    }

    fn render_row(&self, ix: usize, highlights: &[usize]) -> CommandItem {
        let item = &self.items[ix];
        let label = item.label.clone();
        let highlights = highlights.to_vec();
        let detail = SharedString::from(item.detail.clone());
        let target = self.targets[ix].clone();
        CommandItem::new()
            .label(item.label.clone())
            .child(move |window, _| {
                let lead = match &target {
                    Target::Request { method, .. } => method_badge(method.as_deref()),
                    Target::Action(_) => div().flex_none().w(px(34.)),
                };
                let kbd = match &target {
                    Target::Action(a) => Kbd::binding_for_action(a.action().as_ref(), None, window),
                    Target::Request { .. } => None,
                };
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(10.))
                    .h(px(28.))
                    .child(lead)
                    .child(
                        div()
                            .text_color(color("text-primary"))
                            .child(highlighted(&label, &highlights)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(color("text-tertiary"))
                            .child(detail.clone()),
                    )
                    .children(kbd.map(|k| k.ml_auto()))
            })
    }
}

/// `label` with the characters at `highlights` bold and underlined.
fn highlighted(label: &str, highlights: &[usize]) -> StyledText {
    let style = HighlightStyle {
        font_weight: Some(FontWeight::BOLD),
        underline: Some(UnderlineStyle {
            thickness: px(1.),
            ..Default::default()
        }),
        ..Default::default()
    };
    let ranges = label
        .char_indices()
        .enumerate()
        .filter(|(i, _)| highlights.contains(i))
        .map(|(_, (at, c))| (at..at + c.len_utf8(), style));
    StyledText::new(label.to_string()).with_highlights(ranges)
}

/// Every request file in `tree`, depth first.
fn collect_requests(tree: &[TreeNode], out: &mut Vec<(PaletteItem, Target)>) {
    for node in tree {
        match node {
            TreeNode::Folder { children, .. } => collect_requests(children, out),
            TreeNode::File {
                name,
                rel_path,
                method,
                ..
            } => {
                let detail = rel_path
                    .parent()
                    .filter(|p| *p != Path::new(""))
                    .map_or_else(String::new, |p| format!("{} /", p.display()));
                let item = PaletteItem {
                    group: Group::Requests,
                    label: request_label(name).to_string(),
                    detail,
                };
                let target = Target::Request {
                    rel_path: rel_path.clone(),
                    method: method.clone(),
                };
                out.push((item, target));
            }
        }
    }
}

impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let matches = palette_matches(&self.query, &self.items);
        let count = matches.len();
        // The item index behind each (section, row) of the command list.
        let mut sections: Vec<Vec<usize>> = Vec::new();
        let mut command = Command::new(&self.state)
            .filterable(false)
            .bordered(false)
            .placeholder("Search requests and actions")
            .max_h(px(420.));
        for group in Group::ALL {
            let rows: Vec<_> = matches
                .iter()
                .filter(|(ix, _)| self.items[*ix].group == group)
                .collect();
            if rows.is_empty() {
                continue;
            }
            sections.push(rows.iter().map(|(ix, _)| *ix).collect());
            command = command.group(
                CommandGroup::new()
                    .label(group.label())
                    .items(rows.iter().map(|(ix, hl)| self.render_row(*ix, hl))),
            );
        }
        let sections = Rc::new(sections);
        let (on_query, on_confirm) = (cx.entity().downgrade(), cx.entity().downgrade());
        let command = command
            .on_query(move |query, _, cx| {
                let query = SharedString::from(query.to_string());
                on_query
                    .update(cx, |this, cx| {
                        this.query = query;
                        cx.notify();
                    })
                    .ok();
            })
            .on_confirm(move |path, window, cx| {
                if let Some(&ix) = sections.get(path.section).and_then(|s| s.get(path.row)) {
                    on_confirm
                        .update(cx, |this, cx| this.perform(ix, window, cx))
                        .ok();
                }
            })
            .footer(move |_, _, _| legend(count));
        div()
            .id("palette")
            // Esc closes the palette, query or not.
            .capture_action(|_: &Cancel, window, cx| {
                window.close_dialog(cx);
                cx.stop_propagation();
            })
            .child(command)
    }
}

/// The keyboard legend and the result count.
fn legend(count: usize) -> impl IntoElement {
    let key = |k: &'static str| {
        div()
            .px(px(5.))
            .rounded(px(4.))
            .border_1()
            .border_color(color("border-200"))
            .font_family(mono_font())
            .text_size(px(11.))
            .line_height(px(18.))
            .text_color(color("text-secondary"))
            .child(k)
    };
    let hint = |keys: &[&'static str], text: &'static str| {
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .children(keys.iter().map(|k| key(k)))
            .child(text)
    };
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .h(px(36.))
        .px(px(16.))
        .border_t_1()
        .border_color(color("border-100"))
        .bg(color("bg-100"))
        .text_size(px(12.))
        .text_color(color("text-tertiary"))
        .child(hint(&["↑", "↓"], "Navigate"))
        .child(hint(&["↵"], "Open"))
        .child(hint(&["esc"], "Close"))
        .child(div().flex_1())
        .child(match count {
            1 => "1 result".to_string(),
            n => format!("{n} results"),
        })
}

impl Workspace {
    /// Opens the ⌘K palette.
    pub(super) fn open_palette(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Palette> {
        let tree = self
            .state
            .collection
            .as_ref()
            .map_or(&[][..], |c| &c.tree[..]);
        let ws = cx.entity().downgrade();
        let palette = cx.new(|cx| Palette::new(ws, tree, window, cx));
        let first_build = std::cell::Cell::new(true);
        let content = palette.clone();
        window.open_dialog(cx, move |dialog, window, cx| {
            if first_build.replace(false) {
                let state = content.read(cx).state.clone();
                state.update(cx, |s, cx| s.focus(window, cx));
            }
            dialog
                .entrance(DialogEntrance::None)
                .w(px(560.))
                .margin_top(px(88.))
                .p_0()
                .close_button(false)
                .child(content.clone())
        });
        palette
    }
}

#[cfg(test)]
mod tests {
    use super::{Group, PaletteItem, palette_matches};

    fn items(labels: &[&str]) -> Vec<PaletteItem> {
        labels
            .iter()
            .map(|l| PaletteItem {
                group: Group::Requests,
                label: l.to_string(),
                detail: String::new(),
            })
            .collect()
    }

    fn order(query: &str, items: &[PaletteItem]) -> Vec<String> {
        palette_matches(query, items)
            .into_iter()
            .map(|(ix, _)| items[ix].label.clone())
            .collect()
    }

    #[test]
    fn palette_ranks_contiguous_first() {
        let items = items(&[
            "c-h-a-r-g-e",
            "create-charge",
            "list-charges",
            "refund-charge",
            "get-charge",
            "login",
        ]);
        assert_eq!(
            order("charge", &items),
            [
                // contiguous, earliest match first, then shorter label,
                // then input order
                "get-charge",
                "list-charges",
                "create-charge",
                "refund-charge",
                // a scattered match comes last
                "c-h-a-r-g-e",
            ]
        );
        // Same start: the shorter label wins; equal labels keep input order.
        let items = self::items(&["charges", "charge", "charge"]);
        assert_eq!(
            palette_matches("charge", &items)
                .into_iter()
                .map(|(ix, _)| ix)
                .collect::<Vec<_>>(),
            [1, 2, 0]
        );
    }

    #[test]
    fn palette_returns_highlight_indices() {
        let items = items(&["Create-Charge", "Ünïcode"]);
        assert_eq!(
            palette_matches("CHARGE", &items),
            [(0, vec![7, 8, 9, 10, 11, 12])]
        );
        assert_eq!(palette_matches("c ch", &items), [(0, vec![0, 7, 8])]);
        // Indices count characters, not bytes.
        assert_eq!(palette_matches("ünc", &items), [(1, vec![0, 1, 3])]);
        assert_eq!(palette_matches("xyz", &items), []);
        assert_eq!(palette_matches("", &items), [(0, vec![]), (1, vec![])]);
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::workspace::Workspace;
    use gpui_kit::base::actions::Cancel;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::{
        Action as _, AppContext as _, Bounds, Point, TestAppContext, VisualTestContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;

    #[gpui_kit::test]
    fn palette_opens_a_request_and_esc_closes_it(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("charges")).unwrap();
        for f in ["charges/create-charge.yaml", "smoke.yaml"] {
            std::fs::write(
                dir.path().join(f),
                "request:\n  url: https://api.example.com\n",
            )
            .unwrap();
        }
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
                        super::super::state::StateStore::none(),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        let cx = &mut VisualTestContext::from_window(window, cx);

        let palette = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.open_palette(window, cx)));
        cx.run_until_parked();
        let labels = palette.read_with(cx, |p, _| {
            p.items.iter().map(|i| i.label.clone()).collect::<Vec<_>>()
        });
        assert_eq!(
            labels,
            [
                "create-charge",
                "smoke",
                "Open folder…",
                "New request…",
                "New collection…",
                "Toggle Problems"
            ]
        );
        cx.update(|window, cx| palette.update(cx, |p, cx| p.perform(0, window, cx)));
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("charges/create-charge.yaml"));
        });
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));

        // Esc closes even with a query typed.
        let palette = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.open_palette(window, cx)));
        cx.run_until_parked();
        let state = palette.read_with(cx, |p, _| p.state.clone());
        cx.update(|window, cx| {
            state.update(cx, |s, cx| s.set_query("smo", window, cx));
            window.dispatch_action(Cancel.boxed_clone(), cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }
}
