//! The command palette (⌘K, `Palette.dc.html`): fuzzy search over the
//! collection's requests and folders and the studio's actions.
//!
//! Drawn by the studio rather than the Kit's `Command`, whose fixed row
//! frame, query field and list scrollbar could not match the mockup (its
//! scrollbar covered the shortcut chips).

use super::app::{
    Import, NewCollection, NewRequest, OpenEnvironments, OpenFolder, OpenHistory, OpenSettings,
    ToggleProblems, mono_font,
};
use super::icons::{self, glyph};
use super::settings_view::AppPage;
use super::sidebar::{request_count, request_label};
use super::workspace::Workspace;
use super::{color, method_color};
use crate::model::collection::TreeNode;
use gpui_kit::base::actions::{Cancel, Confirm, SelectDown, SelectUp};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Path, PathBuf};

/// Key context of the palette, for its ↑ ↓ ↵ esc bindings.
pub const CONTEXT: &str = "Palette";

/// Palette.dc.html measurements, px.
const WIDTH: f32 = 640.;
const TOP: f32 = 88.;
const HEADER_H: f32 = 52.;
const FOOTER_H: f32 = 36.;
const ROW_H: f32 = 36.;
const ROW_PAD_X: f32 = 10.;
const LIST_PAD_X: f32 = 8.;
const LEAD_W: f32 = 40.;
/// Tallest list, from the mockup's 900px window.
const LIST_MAX_H: f32 = 520.;
/// Width of the Kit's overlay scrollbar (`gpui-base` `scrollbar.rs`); the
/// trailing chips must stay clear of it.
const SCROLLBAR_W: f32 = 16.;
const _: () = assert!(
    LIST_PAD_X + ROW_PAD_X >= SCROLLBAR_W,
    "chips under the scrollbar"
);
/// Alpha of the query-match tint (`.hl`).
const MATCH_ALPHA: f32 = 0.18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Requests,
    Folders,
    Actions,
}

impl Group {
    const ALL: [Group; 3] = [Group::Requests, Group::Folders, Group::Actions];

    fn label(self) -> &'static str {
        match self {
            Group::Requests => "Requests",
            Group::Folders => "Collections & folders",
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

/// Matches in display order: grouped by [`Group::ALL`], ranked within.
fn display_order(matches: &[(usize, Vec<usize>)], items: &[PaletteItem]) -> Vec<usize> {
    Group::ALL
        .iter()
        .flat_map(|g| {
            matches
                .iter()
                .enumerate()
                .filter(move |(_, (ix, _))| items[*ix].group == *g)
                .map(|(m, _)| m)
        })
        .collect()
}

/// What ↵ does on an item.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Request {
        rel_path: PathBuf,
        method: Option<String>,
        /// Position and line of a request inside a multi-request file.
        request: Option<(usize, Option<u32>)>,
        parse_ok: bool,
    },
    /// Reveals the folder in the tree.
    Folder {
        path: PathBuf,
    },
    Action(PaletteAction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaletteAction {
    OpenFolder,
    NewRequest,
    NewCollection,
    Import,
    ToggleProblems,
    OpenHistory,
    Environments,
    Settings,
}

impl PaletteAction {
    pub const ALL: [PaletteAction; 8] = [
        PaletteAction::OpenFolder,
        PaletteAction::NewRequest,
        PaletteAction::NewCollection,
        PaletteAction::Import,
        PaletteAction::ToggleProblems,
        PaletteAction::OpenHistory,
        PaletteAction::Environments,
        PaletteAction::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            PaletteAction::OpenFolder => "Open folder…",
            PaletteAction::NewRequest => "New request…",
            PaletteAction::NewCollection => "New collection…",
            PaletteAction::Import => "Import curl…",
            PaletteAction::ToggleProblems => "Toggle Problems",
            PaletteAction::OpenHistory => "Open history",
            PaletteAction::Environments => "Environments…",
            PaletteAction::Settings => "Settings…",
        }
    }

    /// The row's leading glyph.
    pub fn icon(self) -> &'static str {
        match self {
            PaletteAction::OpenFolder => icons::FOLDER_OPEN,
            PaletteAction::NewRequest => icons::FILE_PLUS,
            PaletteAction::NewCollection => icons::FOLDER_PLUS,
            PaletteAction::Import => icons::IMPORT,
            PaletteAction::ToggleProblems => icons::PANEL_BOTTOM,
            PaletteAction::OpenHistory => icons::HISTORY,
            PaletteAction::Environments => icons::KEY,
            PaletteAction::Settings => icons::GENERAL,
        }
    }

    /// The studio action, for its shortcut.
    fn action(self) -> Box<dyn Action> {
        match self {
            PaletteAction::OpenFolder => Box::new(OpenFolder),
            PaletteAction::NewRequest => Box::new(NewRequest),
            PaletteAction::NewCollection => Box::new(NewCollection),
            PaletteAction::Import => Box::new(Import),
            PaletteAction::ToggleProblems => Box::new(ToggleProblems),
            PaletteAction::OpenHistory => Box::new(OpenHistory),
            PaletteAction::Environments => Box::new(OpenEnvironments),
            PaletteAction::Settings => Box::new(OpenSettings),
        }
    }
}

/// Every palette entry for a collection named `name` with `tree`: its
/// requests (each request of a multi-request file on its own row), its
/// folders, then the actions.
pub fn palette_entries(name: Option<&str>, tree: &[TreeNode]) -> Vec<(PaletteItem, Target)> {
    let mut out = Vec::new();
    collect_requests(tree, &mut out);
    if let Some(name) = name {
        collect_folders(name, tree, Path::new(""), &mut out);
    }
    out.extend(PaletteAction::ALL.map(|a| {
        let item = PaletteItem {
            group: Group::Actions,
            label: a.label().into(),
            detail: String::new(),
        };
        (item, Target::Action(a))
    }));
    out
}

/// `charges/create-charge.yaml` → `charges /`.
fn folder_detail(rel_path: &Path) -> String {
    rel_path
        .parent()
        .filter(|p| *p != Path::new(""))
        .map_or_else(String::new, |p| {
            let parts: Vec<_> = p.iter().map(|c| c.to_string_lossy()).collect();
            format!("{} /", parts.join(" / "))
        })
}

fn collect_requests(tree: &[TreeNode], out: &mut Vec<(PaletteItem, Target)>) {
    for node in tree {
        match node {
            TreeNode::Folder { children, .. } => collect_requests(children, out),
            TreeNode::File {
                name,
                rel_path,
                method,
                requests,
                parse_ok,
            } => {
                let label = request_label(name);
                if method.is_none() && !requests.is_empty() {
                    let detail = format!("{} {label}", folder_detail(rel_path))
                        .trim_start()
                        .to_string();
                    for (index, r) in requests.iter().enumerate() {
                        let item = PaletteItem {
                            group: Group::Requests,
                            label: r.name.clone(),
                            detail: detail.clone(),
                        };
                        let target = Target::Request {
                            rel_path: rel_path.clone(),
                            method: Some(r.method.clone()),
                            request: Some((index, r.line)),
                            parse_ok: *parse_ok,
                        };
                        out.push((item, target));
                    }
                    continue;
                }
                let item = PaletteItem {
                    group: Group::Requests,
                    label: label.to_string(),
                    detail: folder_detail(rel_path),
                };
                let target = Target::Request {
                    rel_path: rel_path.clone(),
                    method: method.clone(),
                    request: None,
                    parse_ok: *parse_ok,
                };
                out.push((item, target));
            }
        }
    }
}

fn collect_folders(
    collection: &str,
    tree: &[TreeNode],
    parent: &Path,
    out: &mut Vec<(PaletteItem, Target)>,
) {
    for node in tree {
        if let TreeNode::Folder { name, children } = node {
            let path = parent.join(name);
            let count = request_count(children);
            let noun = if count == 1 { "request" } else { "requests" };
            let mut place = vec![collection.to_string()];
            place.extend(parent.iter().map(|c| c.to_string_lossy().into_owned()));
            let item = PaletteItem {
                group: Group::Folders,
                label: name.clone(),
                detail: format!("{} · {count} {noun}", place.join(" / ")),
            };
            out.push((item, Target::Folder { path: path.clone() }));
            collect_folders(collection, children, &path, out);
        }
    }
}

pub struct Palette {
    ws: WeakEntity<Workspace>,
    input: Entity<InputState>,
    scroll: ScrollHandle,
    items: Vec<PaletteItem>,
    targets: Vec<Target>,
    /// The open collection's name, for the scope chip.
    scope: Option<SharedString>,
    query: SharedString,
    /// Position of the highlighted row in display order.
    selected: usize,
    /// Scroll the highlighted row into view on the next render.
    reveal: bool,
    /// Last pointer position seen over a row.
    pointer: Option<Point<Pixels>>,
    _subscription: Subscription,
}

impl Palette {
    fn new(
        ws: WeakEntity<Workspace>,
        scope: Option<&str>,
        tree: &[TreeNode],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (items, targets) = palette_entries(scope, tree).into_iter().unzip();
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search requests and actions"));
        let _subscription = cx.subscribe_in(&input, window, |this, input, event, _, cx| {
            if let InputEvent::Change = event {
                this.query = input.read(cx).value();
                this.selected = 0;
                this.reveal = true;
                cx.notify();
            }
        });
        Palette {
            ws,
            input,
            scroll: ScrollHandle::new(),
            items,
            targets,
            scope: scope.map(|s| SharedString::from(s.to_string())),
            query: SharedString::default(),
            selected: 0,
            reveal: false,
            pointer: None,
            _subscription,
        }
    }

    /// Replaces the query, as if typed.
    pub fn set_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |s, cx| s.set_value(query.to_string(), window, cx));
        self.query = query.to_string().into();
        self.selected = 0;
        self.reveal = true;
        cx.notify();
    }

    /// Whether the pointer really moved to `at`. Only movement picks a
    /// row: the first event (the pointer resting where the palette opens)
    /// and repeats at the same spot (sent as rows move under a still
    /// pointer) leave the keyboard's row highlighted.
    fn pointer_moved(&mut self, at: Point<Pixels>) -> bool {
        self.pointer.replace(at).is_some_and(|last| last != at)
    }

    /// Highlights row `pos` in display order and scrolls it into view
    /// (development aid, `CURL_RUNNER_STUDIO_SELECT`).
    pub fn select(&mut self, pos: usize, cx: &mut Context<Self>) {
        self.selected = pos;
        self.reveal = true;
        cx.notify();
    }

    fn order(&self) -> (Vec<(usize, Vec<usize>)>, Vec<usize>) {
        let matches = palette_matches(&self.query, &self.items);
        let order = display_order(&matches, &self.items);
        (matches, order)
    }

    fn select_by(&mut self, step: isize, cx: &mut Context<Self>) {
        let len = self.order().1.len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected as isize + step).rem_euclid(len as isize) as usize;
        self.reveal = true;
        cx.notify();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (matches, order) = self.order();
        if let Some(&m) = order.get(self.selected) {
            self.perform(matches[m].0, window, cx);
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
                Target::Request {
                    rel_path,
                    request: Some((index, line)),
                    ..
                } => {
                    ws.expanded.insert(rel_path.clone());
                    ws.open_request(&rel_path, index, line, window, cx);
                }
                Target::Request { rel_path, .. } => {
                    ws.selected_request = None;
                    ws.open_file(&rel_path, window, cx);
                }
                Target::Folder { path } => ws.reveal_folder(&path, cx),
                Target::Action(PaletteAction::OpenFolder) => ws.open_folder(window, cx),
                Target::Action(PaletteAction::NewRequest) => drop(ws.new_request(None, window, cx)),
                Target::Action(PaletteAction::NewCollection) => {
                    drop(ws.new_collection(window, cx));
                }
                Target::Action(PaletteAction::Import) => drop(ws.import(window, cx)),
                Target::Action(PaletteAction::ToggleProblems) => ws.toggle_problems(cx),
                Target::Action(PaletteAction::OpenHistory) => ws.open_history(window, cx),
                Target::Action(PaletteAction::Environments) => ws.open_settings(window, cx),
                Target::Action(PaletteAction::Settings) => {
                    ws.open_app_settings(AppPage::General, window, cx)
                }
            })
            .ok();
    }

    fn render_header(&self) -> Div {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .h(px(HEADER_H))
            .px(px(16.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(
                glyph(icons::SEARCH)
                    .size(px(16.))
                    .flex_none()
                    .text_color(color("text-tertiary")),
            )
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.input)
                        .appearance(false)
                        .p_0()
                        .text_size(px(15.))
                        .text_color(color("text-primary")),
                ),
            )
            .children(self.scope.clone().map(|scope| {
                div()
                    .flex_none()
                    .px(px(8.))
                    .py(px(1.))
                    .rounded_full()
                    .border_1()
                    .border_color(color("border-200"))
                    .text_size(px(11.))
                    .text_color(color("text-secondary"))
                    .child(scope)
            }))
            .child(key_chip("esc"))
    }

    fn render_row(
        &self,
        pos: usize,
        ix: usize,
        highlights: &[usize],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let item = &self.items[ix];
        let target = &self.targets[ix];
        let selected = pos == self.selected;
        let lead = match target {
            Target::Request { method, .. } => {
                let method = method.clone().unwrap_or_default();
                div()
                    .flex_none()
                    .w(px(LEAD_W))
                    .font_family(mono_font())
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(method_color(&method))
                    .child(method)
            }
            Target::Folder { .. } => lead_icon(icons::FOLDER, 16.),
            Target::Action(a) => lead_icon(a.icon(), 14.),
        };
        let trailing = match target {
            Target::Action(a) => shortcut(a.action().as_ref(), window).map(key_chip),
            _ => None,
        };
        let broken = matches!(
            target,
            Target::Request {
                parse_ok: false,
                ..
            }
        );
        div()
            .id(("palette-row", pos))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .h(px(ROW_H))
            .px(px(ROW_PAD_X))
            .rounded(px(6.))
            .text_size(px(13.))
            .whitespace_nowrap()
            .overflow_hidden()
            .cursor_pointer()
            .text_color(color(if selected {
                "text-primary"
            } else {
                "text-secondary"
            }))
            .when(selected, |row| row.bg(color("surface-300")))
            .child(lead)
            .child(
                div()
                    .flex_none()
                    // Keeps the match tint glyph-high, like the mockup's.
                    .line_height(px(17.))
                    .child(highlighted(&item.label, highlights)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(px(12.))
                    .text_color(color(if selected {
                        "text-tertiary"
                    } else {
                        "text-gutter"
                    }))
                    .child(item.detail.clone()),
            )
            .when(broken, |row| {
                row.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(color("danger"))
                        .child(div().size(px(6.)).rounded_full().bg(color("danger")))
                        .child("Doesn't parse"),
                )
            })
            .when(selected && !matches!(target, Target::Action(_)), |row| {
                row.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(12.))
                        .text_color(color("text-tertiary"))
                        .child("Open")
                        .child(key_chip("↵")),
                )
            })
            .children(trailing)
            .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, _, cx| {
                if this.pointer_moved(e.position) && this.selected != pos {
                    this.selected = pos;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| this.perform(ix, window, cx)))
    }
}

fn lead_icon(svg: &'static str, size: f32) -> Div {
    div().flex().flex_none().items_center().w(px(LEAD_W)).child(
        glyph(svg)
            .size(px(size))
            .text_color(color("text-secondary")),
    )
}

/// The first shortcut bound to `action`, as macOS symbols (`⇧⌘N`).
fn shortcut(action: &dyn Action, window: &Window) -> Option<SharedString> {
    let binding = window.highest_precedence_binding_for_action(action)?;
    let key = binding.keystrokes().first()?;
    Some(Kbd::format(key.as_keystroke()).into())
}

/// The mockup's `.kb` chip.
fn key_chip(keys: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .px(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(color("border-200"))
        .font_family(mono_font())
        .text_size(px(11.))
        .line_height(px(18.))
        .text_color(color("text-secondary"))
        .child(keys.into())
}

/// `label` with the characters at `highlights` in the primary color on a
/// blue tint.
fn highlighted(label: &str, highlights: &[usize]) -> StyledText {
    let style = HighlightStyle {
        color: Some(color("text-primary")),
        background_color: Some(color("accent-blue").opacity(MATCH_ALPHA)),
        ..Default::default()
    };
    let ranges = label
        .char_indices()
        .enumerate()
        .filter(|(i, _)| highlights.contains(i))
        .map(|(_, (at, c))| (at..at + c.len_utf8(), style));
    StyledText::new(label.to_string()).with_highlights(ranges)
}

impl Render for Palette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (matches, order) = self.order();
        self.selected = self.selected.min(order.len().saturating_sub(1));
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .pt(px(6.))
            .px(px(LIST_PAD_X))
            .pb(px(8.))
            .max_h(px(list_max_height(window.viewport_size().height.into())))
            .overflow_y_scroll()
            .track_scroll(&self.scroll);
        let mut child = 0;
        let mut selected_child = None;
        let mut group = None;
        for (pos, &m) in order.iter().enumerate() {
            let (ix, highlights) = &matches[m];
            let g = self.items[*ix].group;
            if group != Some(g) {
                group = Some(g);
                list = list.child(
                    div()
                        .flex_none()
                        .pt(px(10.))
                        .px(px(10.))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color("text-tertiary"))
                        .child(g.label()),
                );
                child += 1;
            }
            if pos == self.selected {
                selected_child = Some(child);
            }
            list = list.child(self.render_row(pos, *ix, highlights, window, cx));
            child += 1;
        }
        if order.is_empty() {
            list = list.child(
                div()
                    .py(px(24.))
                    .text_center()
                    .text_size(px(13.))
                    .text_color(color("text-tertiary"))
                    .child("No matches"),
            );
        }
        if std::mem::take(&mut self.reveal)
            && let Some(child) = selected_child
        {
            // Keep the group header in view above the first row.
            let child = if self.selected == 0 { 0 } else { child };
            self.scroll.scroll_to_item(child);
        }
        div()
            .id("palette")
            .key_context(CONTEXT)
            .flex()
            .flex_col()
            .w_full()
            .text_color(color("text-primary"))
            .on_action(cx.listener(|this, _: &SelectUp, _, cx| this.select_by(-1, cx)))
            .on_action(cx.listener(|this, _: &SelectDown, _, cx| this.select_by(1, cx)))
            .on_action(cx.listener(|this, _: &Confirm, window, cx| this.confirm(window, cx)))
            // Esc closes the palette, query or not.
            .capture_action(|_: &Cancel, window, cx| {
                window.close_dialog(cx);
                cx.stop_propagation();
            })
            .child(self.render_header())
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .child(list)
                    .child(Scrollbar::vertical(&self.scroll)),
            )
            .child(legend(order.len()))
    }
}

/// The list's height limit in a `viewport`-tall window: the mockup's, or
/// what fits under the palette's top offset with a margin below.
fn list_max_height(viewport: f32) -> f32 {
    (viewport - TOP - HEADER_H - FOOTER_H - 2. - 24.).clamp(ROW_H * 3., LIST_MAX_H)
}

/// The keyboard legend and the result count.
fn legend(count: usize) -> impl IntoElement {
    let hint = |keys: &[&'static str], text: &'static str| {
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .children(keys.iter().map(|k| key_chip(*k)))
            .child(text)
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(16.))
        .h(px(FOOTER_H))
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
        let (scope, tree) = match &self.state.collection {
            Some(c) => (Some(c.name.as_str()), &c.tree[..]),
            None => (None, &[][..]),
        };
        let ws = cx.entity().downgrade();
        let palette = cx.new(|cx| Palette::new(ws, scope, tree, window, cx));
        let first_build = std::cell::Cell::new(true);
        let content = palette.clone();
        window.open_dialog(cx, move |dialog, window, cx| {
            if first_build.replace(false) {
                let input = content.read(cx).input.clone();
                input.update(cx, |s, cx| s.focus(window, cx));
            }
            dialog
                .entrance(DialogEntrance::None)
                .w(px(WIDTH))
                .margin_top(px(TOP))
                .p_0()
                .gap_0()
                .rounded(px(12.))
                .bg(color("surface-raised"))
                .border_color(color("border-200"))
                .shadow(vec![BoxShadow {
                    color: hsla(0., 0., 0., 0.7),
                    offset: point(px(0.), px(24.)),
                    blur_radius: px(64.),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .overflow_hidden()
                .close_button(false)
                .child(content.clone())
        });
        palette
    }

    /// Shows `folder` in the tree: it and its parents open.
    pub(super) fn reveal_folder(&mut self, folder: &Path, cx: &mut Context<Self>) {
        for dir in folder.ancestors() {
            self.collapsed.remove(dir);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Group, PaletteAction, PaletteItem, Target, display_order, list_max_height, palette_entries,
        palette_matches,
    };
    use crate::model::collection::{RequestEntry, TreeNode};
    use std::path::PathBuf;

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

    fn tree() -> Vec<TreeNode> {
        let file = |name: &str, rel: &str, method: Option<&str>| TreeNode::File {
            name: name.into(),
            rel_path: rel.into(),
            method: method.map(Into::into),
            requests: Vec::new(),
            parse_ok: true,
        };
        vec![
            TreeNode::Folder {
                name: "users".into(),
                children: vec![
                    file("get-user.yaml", "users/get-user.yaml", Some("GET")),
                    TreeNode::File {
                        name: "user-flow.yaml".into(),
                        rel_path: "users/user-flow.yaml".into(),
                        method: None,
                        requests: vec![
                            RequestEntry {
                                method: "GET".into(),
                                name: "Get user".into(),
                                line: Some(2),
                            },
                            RequestEntry {
                                method: "POST".into(),
                                name: "Create todo".into(),
                                line: Some(9),
                            },
                        ],
                        parse_ok: true,
                    },
                    TreeNode::Folder {
                        name: "admin".into(),
                        children: vec![file("ban.yaml", "users/admin/ban.yaml", Some("POST"))],
                    },
                ],
            },
            file("smoke.yaml", "smoke.yaml", Some("GET")),
        ]
    }

    #[test]
    fn entries_list_requests_folders_then_actions() {
        let entries = palette_entries(Some("Examples"), &tree());
        let rows: Vec<_> = entries
            .iter()
            .map(|(i, _)| (i.group, i.label.as_str(), i.detail.as_str()))
            .collect();
        assert_eq!(
            rows[..7],
            [
                (Group::Requests, "get-user", "users /"),
                // Each request of a multi-request file, with the file.
                (Group::Requests, "Get user", "users / user-flow"),
                (Group::Requests, "Create todo", "users / user-flow"),
                (Group::Requests, "ban", "users / admin /"),
                (Group::Requests, "smoke", ""),
                (Group::Folders, "users", "Examples · 4 requests"),
                (Group::Folders, "admin", "Examples / users · 1 request"),
            ]
        );
        assert_eq!(
            entries[2].1,
            Target::Request {
                rel_path: PathBuf::from("users/user-flow.yaml"),
                method: Some("POST".into()),
                request: Some((1, Some(9))),
                parse_ok: true,
            }
        );
        assert_eq!(
            entries[6].1,
            Target::Folder {
                path: "users/admin".into()
            }
        );
        let actions: Vec<_> = entries[7..].iter().map(|(_, t)| t.clone()).collect();
        assert_eq!(actions, PaletteAction::ALL.map(Target::Action));
        // No collection: no folders.
        assert!(
            palette_entries(None, &tree())
                .iter()
                .all(|(i, _)| i.group != Group::Folders)
        );
    }

    #[test]
    fn display_order_groups_ranked_matches() {
        let mut items = items(&["charges-action", "charge"]);
        items[0].group = Group::Actions;
        items.push(PaletteItem {
            group: Group::Folders,
            label: "charges".into(),
            detail: String::new(),
        });
        let matches = palette_matches("charge", &items);
        let shown: Vec<_> = display_order(&matches, &items)
            .into_iter()
            .map(|m| items[matches[m].0].label.as_str())
            .collect();
        assert_eq!(shown, ["charge", "charges", "charges-action"]);
    }

    #[test]
    fn every_action_has_its_own_icon() {
        let icons: Vec<_> = PaletteAction::ALL.iter().map(|a| a.icon()).collect();
        for (i, icon) in icons.iter().enumerate() {
            assert!(icon.starts_with("<svg ") && icon.ends_with("</svg>"));
            assert!(
                !icons[i + 1..].contains(icon),
                "{:?}",
                PaletteAction::ALL[i]
            );
        }
    }

    #[test]
    fn history_is_an_action() {
        let entries = palette_entries(None, &[]);
        let items: Vec<_> = entries.iter().map(|(i, _)| i.clone()).collect();
        let top = palette_matches("history", &items)[0].0;
        assert_eq!(items[top].label, "Open history");
        assert_eq!(entries[top].1, Target::Action(PaletteAction::OpenHistory));
    }

    #[test]
    fn list_fits_the_window() {
        assert_eq!(list_max_height(900.), 520.);
        assert!(list_max_height(600.) <= 600. - 88. - 52. - 36.);
    }
}

#[cfg(test)]
mod gpui_tests {
    use super::super::workspace::Workspace;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::{
        AppContext as _, Bounds, Entity, Point, TestAppContext, VisualTestContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;

    /// A workspace on a temporary collection with `extra` more requests.
    fn open(
        cx: &mut TestAppContext,
        extra: usize,
    ) -> (tempfile::TempDir, Entity<Workspace>, VisualTestContext) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("charges")).unwrap();
        let extra: Vec<_> = (0..extra).map(|i| format!("charges/r{i}.yaml")).collect();
        let files = ["charges/create-charge.yaml", "smoke.yaml"]
            .into_iter()
            .chain(extra.iter().map(String::as_str));
        for f in files {
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
                        super::super::workspace::test_secrets(),
                        super::super::workspace::test_backend(),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        let cx = VisualTestContext::from_window(window, cx);
        (dir, ws, cx)
    }

    #[gpui_kit::test]
    fn palette_opens_a_request_and_esc_closes_it(cx: &mut TestAppContext) {
        let (_dir, ws, mut vcx) = open(cx, 0);
        let cx = &mut vcx;

        let palette = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.open_palette(window, cx)));
        cx.run_until_parked();
        let labels = palette.read_with(cx, |p, _| {
            p.items.iter().map(|i| i.label.clone()).collect::<Vec<_>>()
        });
        assert_eq!(labels[..3], ["create-charge", "smoke", "charges"]);
        cx.update(|window, cx| palette.update(cx, |p, cx| p.perform(0, window, cx)));
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("charges/create-charge.yaml"));
        });
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));

        // Esc closes even with a query typed.
        cx.update(|window, cx| ws.update(cx, |ws, cx| ws.open_palette(window, cx)));
        cx.run_until_parked();
        cx.simulate_input("smo");
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn arrows_and_enter_drive_the_palette(cx: &mut TestAppContext) {
        let (_dir, ws, mut vcx) = open(cx, 20);
        let cx = &mut vcx;
        let palette = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.open_palette(window, cx)));
        cx.run_until_parked();
        // Typing filters and resets the highlight to the first row.
        cx.simulate_input("smoke");
        cx.run_until_parked();
        assert_eq!(palette.read_with(cx, |p, _| p.query.to_string()), "smoke");
        cx.simulate_keystrokes("down up up");
        cx.run_until_parked();
        // One match: the highlight wraps onto it.
        assert_eq!(palette.read_with(cx, |p, _| p.selected), 0);
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("smoke.yaml"));
        });
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));

        // ↓ moves to the second row; ↵ opens it.
        let palette = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.open_palette(window, cx)));
        cx.run_until_parked();
        cx.simulate_keystrokes("down");
        cx.run_until_parked();
        assert_eq!(palette.read_with(cx, |p, _| p.selected), 1);
        cx.simulate_keystrokes("up up");
        cx.run_until_parked();
        let last = palette.read_with(cx, |p, _| p.items.len() - 1);
        assert_eq!(palette.read_with(cx, |p, _| p.selected), last);
        // The last row is scrolled into view.
        let offset = palette.read_with(cx, |p, _| p.scroll.offset().y);
        assert!(offset < px(0.), "{offset:?}");
    }
}
