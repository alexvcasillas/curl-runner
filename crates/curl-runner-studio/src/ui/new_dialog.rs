//! The New dialog (`NewRequest.dc.html`): a request file from a blank
//! template, a curl command or a copy of another request (⌘N), or a new
//! collection (⇧⌘N). Import (⌘I) has its own dialog (`import_dialog.rs`).

use super::app::{CreateNew, mono_font};
use super::workspace::{Workspace, notify_error};
use super::{color, method_color};
use crate::model::collection::{Collection, TreeNode};
use crate::model::templates::{
    duplicate_request, file_name_for, request_from_curl, request_template, variable_spans,
};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::{DialogEntrance, DialogFooter};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::select::{Select, SelectItem, SelectState};
use gpui_kit::component::{IconName, IndexPath, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// The file a new collection starts with.
const STARTER_FILE: &str = "requests.yaml";

/// Field height and text sizes from the mockup.
const FIELD_H: f32 = 36.;
const FIELD_TEXT: f32 = 13.;
const MONO_TEXT: f32 = 12.;

/// A method in the method select, drawn in its method color.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MethodItem(SharedString);

impl MethodItem {
    fn label(&self) -> Div {
        div()
            .font_family(mono_font())
            .text_size(px(MONO_TEXT))
            .text_color(method_color(&self.0))
            .child(self.0.clone())
    }
}

impl SelectItem for MethodItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.0.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(self.label().into_any_element())
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.label()
    }

    fn value(&self) -> &SharedString {
        &self.0
    }
}

/// A request file the Duplicate template can copy.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceItem {
    rel: PathBuf,
    label: SharedString,
    /// The single request's method, else the first request's.
    method: Option<SharedString>,
    /// Requests in a multi-request file; 0 for a single request.
    count: usize,
}

impl SourceItem {
    fn row(&self) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .min_w_0()
            .child(
                div()
                    .flex_none()
                    .w(px(44.))
                    .font_family(mono_font())
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(method_color(self.method.as_deref().unwrap_or("")))
                    .child(self.method.clone().unwrap_or_default()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(self.label.clone()),
            )
            .when(self.count > 0, |row| {
                row.child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(color("text-tertiary"))
                        .child(format!("{} requests", self.count)),
                )
            })
    }
}

impl SelectItem for SourceItem {
    type Value = PathBuf;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(self.row().into_any_element())
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.row()
    }

    fn value(&self) -> &PathBuf {
        &self.rel
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Request,
    Collection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Blank,
    FromCurl,
    Duplicate,
}

pub struct NewDialog {
    ws: WeakEntity<Workspace>,
    kind: Kind,
    template: Template,
    name: Entity<InputState>,
    method: Entity<SelectState<Vec<MethodItem>>>,
    url: Entity<InputState>,
    curl: Entity<TextareaState>,
    /// The open collection's root; the Duplicate template reads from it.
    root: Option<PathBuf>,
    /// Request files the Duplicate template can copy; `None` without a
    /// collection.
    source: Option<Entity<SelectState<Vec<SourceItem>>>>,
    /// Folders of the open collection, relative to its root (`""` first),
    /// as the folder select lists them. Empty without a collection.
    folders: Vec<PathBuf>,
    folder: Option<Entity<SelectState<Vec<SharedString>>>>,
    /// The folder picked on disk: the new collection, or where a request
    /// goes when no collection is open.
    picked: Option<PathBuf>,
    /// The From curl conversion: its warnings, or why it fails.
    converted: Option<Result<Vec<String>, String>>,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl NewDialog {
    fn new(
        ws: WeakEntity<Workspace>,
        kind: Kind,
        template: Template,
        collection: Option<(&Collection, PathBuf)>,
        active: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("create-charge"));
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("${BASE_URL}/v1/charges"));
        let curl = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(5)
                .placeholder("curl -X POST https://api.example.com/v1/charges -d amount=2500")
        });
        let methods = METHODS.map(|m| MethodItem(SharedString::from(m))).to_vec();
        let method = cx.new(|cx| SelectState::new(methods, Some(IndexPath::default()), window, cx));
        let root = collection.as_ref().map(|(c, _)| c.root.clone());
        let source = collection.as_ref().map(|(c, _)| {
            let mut sources = Vec::new();
            collect_sources(&c.tree, &mut sources);
            let at = active
                .and_then(|a| sources.iter().position(|s| s.rel == a))
                .unwrap_or(0);
            let selected = (!sources.is_empty()).then_some(IndexPath {
                row: at,
                ..IndexPath::default()
            });
            cx.new(|cx| SelectState::new(sources, selected, window, cx))
        });
        let (folders, folder) = match collection {
            Some((collection, preselect)) => {
                let mut folders = vec![PathBuf::new()];
                collect_folders(&collection.tree, Path::new(""), &mut folders);
                let labels = folders
                    .iter()
                    .map(|f| folder_label(&collection.name, f))
                    .collect();
                let at = folders.iter().position(|f| *f == preselect).unwrap_or(0);
                let select = cx.new(|cx| {
                    SelectState::new(
                        labels,
                        Some(IndexPath {
                            row: at,
                            ..IndexPath::default()
                        }),
                        window,
                        cx,
                    )
                });
                (folders, Some(select))
            }
            None => (Vec::new(), None),
        };
        let clear_error =
            |this: &mut Self, _: Entity<InputState>, e: &InputEvent, cx: &mut Context<Self>| {
                if matches!(e, InputEvent::Change) && this.error.take().is_some() {
                    cx.notify();
                }
            };
        let subscriptions = vec![
            cx.subscribe(&name, move |this, input, e: &InputEvent, cx| {
                clear_error(this, input, e, cx);
                // The From curl preview names the request.
                if matches!(e, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&url, clear_error),
            cx.subscribe(&curl, |this, _, e: &InputEvent, cx| {
                if matches!(e, InputEvent::Change) {
                    this.error = None;
                    this.convert(cx);
                }
            }),
        ];
        NewDialog {
            ws,
            kind,
            template,
            name,
            method,
            url,
            curl,
            root,
            source,
            folders,
            folder,
            picked: None,
            converted: None,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn set_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        self.kind = kind;
        self.error = None;
        cx.notify();
    }

    fn set_template(&mut self, template: Template, cx: &mut Context<Self>) {
        self.template = template;
        self.error = None;
        cx.notify();
    }

    /// Development aid (`CURL_RUNNER_STUDIO_FILL`, `name|METHOD|url`):
    /// fills the request fields, focuses the URL, and with `duplicate`
    /// switches to the Duplicate template.
    pub(super) fn dev_fill(
        &mut self,
        spec: &str,
        duplicate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut parts = spec.splitn(3, '|');
        let (name, method, url) = (parts.next(), parts.next(), parts.next());
        if let Some(name) = name.filter(|n| !n.is_empty()) {
            let name = name.to_string();
            self.name.update(cx, |s, cx| s.set_value(name, window, cx));
        }
        if let Some(method) = method.filter(|m| !m.is_empty()) {
            let method = SharedString::from(method.to_ascii_uppercase());
            self.method
                .update(cx, |s, cx| s.set_selected_value(&method, window, cx));
        }
        if let Some(url) = url {
            let url = url.to_string();
            self.url.update(cx, |s, cx| s.set_value(url, window, cx));
            // After the dialog's first build, which focuses the name.
            let focus = self.url.read(cx).focus_handle(cx);
            cx.spawn_in(window, async move |_, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(300))
                    .await;
                cx.update(|window, cx| focus.focus(window, cx)).ok();
            })
            .detach();
        }
        if duplicate {
            self.set_template(Template::Duplicate, cx);
        }
    }

    fn convert(&mut self, cx: &mut Context<Self>) {
        let curl = self.curl.read(cx).value();
        self.converted =
            (!curl.trim().is_empty()).then(|| request_from_curl("", &curl).map(|c| c.warnings));
        cx.notify();
    }

    /// Writes the file (or the collection) and opens it, or shows why not.
    pub(super) fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let created = match self.kind {
            Kind::Request => self.create_request(window, cx),
            Kind::Collection => self.create_collection(window, cx),
        };
        self.error = created.err();
        cx.notify();
    }

    fn create_request(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let name = self.name.read(cx).value().trim().to_string();
        if name.is_empty() {
            return Err("Give the request a name.".into());
        }
        let text = match self.template {
            Template::Blank => {
                let url = self.url.read(cx).value().trim().to_string();
                if url.is_empty() {
                    return Err("Enter the URL to call.".into());
                }
                let method = self.method.read(cx).selected_value().cloned();
                request_template(&name, method.as_deref().unwrap_or("GET"), &url)
            }
            Template::FromCurl => request_from_curl(&name, &self.curl.read(cx).value())?.yaml,
            Template::Duplicate => {
                let (Some(root), Some(source)) = (&self.root, &self.source) else {
                    return Err("Open a collection to copy one of its requests.".into());
                };
                let rel = source
                    .read(cx)
                    .selected_value()
                    .cloned()
                    .ok_or("Choose the request to copy.")?;
                let text = std::fs::read_to_string(root.join(&rel))
                    .map_err(|e| format!("Couldn't read {}: {e}", rel.display()))?;
                duplicate_request(&text, &name)
            }
        };
        let file = file_name_for(&name);
        let Some(folder) = &self.folder else {
            // No collection open: the request starts one in the picked folder.
            let dir = self
                .picked
                .clone()
                .ok_or("Choose a folder for the request.")?;
            if dir.join(&file).exists() {
                return Err(already_exists(&file));
            }
            window.close_dialog(cx);
            self.ws
                .update(cx, |ws, cx| {
                    ws.create_collection(dir, file.into(), text, window, cx)
                })
                .ok();
            return Ok(());
        };
        let row = folder.read(cx).selected_index(cx).map_or(0, |ix| ix.row);
        let rel = self
            .folders
            .get(row)
            .cloned()
            .unwrap_or_default()
            .join(&file);
        let written = self
            .ws
            .update(cx, |ws, _| ws.create_file(&rel, &text))
            .map_err(|_| "The window is gone.".to_string())?;
        match written {
            Ok(()) => {
                window.close_dialog(cx);
                self.ws
                    .update(cx, |ws, cx| ws.open_file(&rel, window, cx))
                    .ok();
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(already_exists(&file)),
            Err(e) => Err(format!("Couldn't create {}: {e}", rel.display())),
        }
    }

    fn create_collection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let dir = self.picked.clone().ok_or("Choose an empty folder.")?;
        match is_empty_dir(&dir) {
            Ok(true) => {}
            Ok(false) => {
                return Err(format!(
                    "{} isn't empty. Choose an empty folder, or open it with Open folder.",
                    display_path(&dir)
                ));
            }
            Err(e) => return Err(format!("Couldn't read {}: {e}", display_path(&dir))),
        }
        let text = request_template("First request", "GET", "https://api.example.com");
        window.close_dialog(cx);
        self.ws
            .update(cx, |ws, cx| {
                ws.create_collection(dir, STARTER_FILE.into(), text, window, cx)
            })
            .ok();
        Ok(())
    }

    fn pick_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            this.update(cx, |this, cx| {
                this.picked = paths.into_iter().next();
                this.error = None;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Where focus goes when the dialog opens.
    fn initial_focus(&self, cx: &App) -> FocusHandle {
        match (self.kind, self.template) {
            (Kind::Request, Template::FromCurl) => self.curl.read(cx).focus_handle(cx),
            _ => self.name.read(cx).focus_handle(cx),
        }
    }

    // Rendering.

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let segment = |id: &'static str, label: &'static str, kind: Kind| {
            let on = self.kind == kind;
            div()
                .id(id)
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .h(px(28.))
                .rounded(px(6.))
                .text_color(color(if on { "text-primary" } else { "text-secondary" }))
                .when(on, |el| el.bg(color("surface-300")))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.set_kind(kind, cx)))
                .child(label)
        };
        div()
            .flex()
            .items_start()
            .gap(px(12.))
            .px(px(24.))
            .pt(px(20.))
            .pb(px(12.))
            .child(
                div()
                    .flex_1()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("New"),
            )
            .child(
                div()
                    .flex()
                    .w(px(220.))
                    .p(px(2.))
                    .gap(px(2.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(color("border-100"))
                    .bg(color("surface-raised"))
                    .child(segment("new-kind-request", "Request", Kind::Request))
                    .child(segment(
                        "new-kind-collection",
                        "Collection",
                        Kind::Collection,
                    )),
            )
            .child(
                Button::new("new-close")
                    .ghost()
                    .small()
                    // Centred on the 34px segmented control.
                    .mt(px(3.))
                    .icon(IconName::Close)
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
    }

    fn render_request(&self, cx: &mut Context<Self>) -> Div {
        let folder = match &self.folder {
            Some(select) => Select::new(select)
                .h(px(FIELD_H))
                .text_size(px(FIELD_TEXT))
                .into_any_element(),
            None => self
                .render_picker("Choose where the request goes.", cx)
                .into_any_element(),
        };
        let card = |id: &'static str, title: &'static str, detail: &'static str, t: Template| {
            let on = self.template == t;
            div()
                .id(id)
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(4.))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(8.))
                .border_1()
                .border_color(color(if on { "text-tertiary" } else { "border-100" }))
                .bg(color(if on { "surface-200" } else { "surface-raised" }))
                .when(!on, |el| {
                    el.hover(|el| el.border_color(color("border-200")))
                })
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.set_template(t, cx)))
                .child(
                    div()
                        .text_size(px(FIELD_TEXT))
                        .line_height(px(18.))
                        .text_color(color("text-primary"))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(color("text-tertiary"))
                        .child(detail),
                )
        };
        let name = self.name.read(cx).value();
        let file = file_name_for(&name);
        // The suffix shows the file name already; say so only when the
        // name is slugged into another one.
        let saved_as = (!name.trim().is_empty() && file != format!("{name}.yaml"))
            .then(|| hint(format!("Saved as {file}")));
        let request = match self.template {
            Template::Blank => field(
                "Request",
                div()
                    .flex()
                    .gap(px(8.))
                    .child(
                        div().flex_none().w(px(96.)).child(
                            Select::new(&self.method)
                                .h(px(FIELD_H))
                                .menu_width(px(120.)),
                        ),
                    )
                    .child(self.render_url()),
            ),
            Template::FromCurl => field(
                "curl command",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .font_family(mono_font())
                            .text_size(px(MONO_TEXT))
                            .child(Textarea::new(&self.curl)),
                    )
                    .children(self.render_conversion()),
            ),
            Template::Duplicate => field(
                "Copy from",
                match &self.source {
                    Some(source) => Select::new(source)
                        .h(px(FIELD_H))
                        .text_size(px(FIELD_TEXT))
                        .placeholder("No request files yet")
                        .into_any_element(),
                    None => hint("Open a collection to copy one of its requests.".into())
                        .into_any_element(),
                },
            ),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(card(
                        "new-template-blank",
                        "Blank",
                        "Empty request file",
                        Template::Blank,
                    ))
                    .child(card(
                        "new-template-curl",
                        "From curl",
                        "Paste a command",
                        Template::FromCurl,
                    ))
                    .child(card(
                        "new-template-duplicate",
                        "Duplicate",
                        "Copy another request",
                        Template::Duplicate,
                    )),
            )
            .child(field(
                "Name",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        Input::new(&self.name)
                            .min_h(px(FIELD_H))
                            .text_size(px(FIELD_TEXT))
                            .suffix(
                                div()
                                    .font_family(mono_font())
                                    .text_size(px(MONO_TEXT))
                                    .text_color(color("text-gutter"))
                                    .child(".yaml"),
                            ),
                    )
                    .children(saved_as),
            ))
            .child(request)
            .child(field("Folder", folder))
    }

    /// The URL field: the Kit input draws the caret and selection with its
    /// text transparent, and a canvas painted after it redraws the text with
    /// `${VAR}` references in the variable color (the Kit's single-line
    /// input has no highlighter or decorations).
    fn render_url(&self) -> Div {
        let url = self.url.clone();
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .font_family(mono_font())
            .text_color(transparent_black())
            .child(
                Input::new(&self.url)
                    .min_h(px(FIELD_H))
                    .text_size(px(MONO_TEXT)),
            )
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, cx| paint_url(&url, window, cx),
                )
                .absolute()
                .size_full(),
            )
    }

    /// The From curl result under the command: warnings or the failure.
    fn render_conversion(&self) -> Option<Div> {
        match self.converted.as_ref()? {
            Ok(warnings) if warnings.is_empty() => None,
            Ok(warnings) => Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .text_size(px(12.))
                    .text_color(color("warning"))
                    .child("Not converted, left as comments in the file:")
                    .children(warnings.iter().map(|w| div().child(format!("· {w}")))),
            ),
            Err(e) => Some(
                div()
                    .text_size(px(12.))
                    .text_color(color("danger"))
                    .child(e.clone()),
            ),
        }
    }

    fn render_collection(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .text_color(color("text-secondary"))
                    .line_height(relative(1.5))
                    .child(format!(
                        "A collection is a folder of request files. Pick an empty folder; \
                         it starts with a first request in {STARTER_FILE}."
                    )),
            )
            .child(field("Folder", self.render_picker("No folder chosen", cx)))
    }

    fn render_picker(&self, empty: &'static str, cx: &mut Context<Self>) -> Div {
        let path = match &self.picked {
            Some(dir) => div()
                .font_family(mono_font())
                .text_size(px(12.))
                .child(display_path(dir)),
            None => div().text_color(color("text-tertiary")).child(empty),
        };
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
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(color("border-200"))
                    .overflow_hidden()
                    .child(path),
            )
            .child(
                Button::new("new-pick-folder")
                    .outline()
                    .label("Choose…")
                    .on_click(cx.listener(|this, _, window, cx| this.pick_folder(window, cx))),
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> DialogFooter {
        let keys = |k: &'static str, what: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(kbd(k))
                .child(what)
        };
        DialogFooter::new()
            .w_full()
            .pl(px(24.))
            .pr(px(16.))
            .py(px(12.))
            .gap(px(8.))
            .border_t_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .gap(px(12.))
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child(keys("⌘N", "request"))
                    .child(keys("⇧⌘N", "collection")),
            )
            .child(
                Button::new("new-cancel")
                    .outline()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(FIELD_TEXT))
                    .label("Cancel")
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
            .child(
                Button::new("new-create")
                    .primary()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(FIELD_TEXT))
                    .label("Create")
                    .child(
                        div()
                            .font_family(mono_font())
                            .text_size(px(11.))
                            .opacity(0.7)
                            .child("⌘↵"),
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
            )
    }
}

impl Render for NewDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.kind {
            Kind::Request => self.render_request(cx),
            Kind::Collection => self.render_collection(cx),
        };
        div()
            .id("new-dialog")
            .key_context("NewDialog")
            .on_action(cx.listener(|this, _: &CreateNew, window, cx| this.create(window, cx)))
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .px(px(24.))
                    .pb(px(20.))
                    .child(body)
                    .children(self.error.clone().map(|e| {
                        div()
                            .id("new-error")
                            .text_size(px(12.))
                            .text_color(color("danger"))
                            .child(e)
                    })),
            )
            .child(self.render_footer(cx))
    }
}

fn field(label: &'static str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color("text-secondary"))
                .child(label),
        )
        .child(control)
}

fn hint(text: String) -> Div {
    div()
        .text_size(px(12.))
        .text_color(color("text-tertiary"))
        .child(text)
}

fn kbd(keys: &'static str) -> Div {
    div()
        .px(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(color("border-200"))
        .font_family(mono_font())
        .text_size(px(11.))
        .line_height(px(18.))
        .text_color(color("text-secondary"))
        .child(keys)
}

/// `text` split into `(byte length, is a ${VAR} reference)` runs.
fn url_segments(text: &str) -> Vec<(usize, bool)> {
    let mut segments = Vec::new();
    let mut at = 0;
    for span in variable_spans(text) {
        if span.start > at {
            segments.push((span.start - at, false));
        }
        segments.push((span.len(), true));
        at = span.end;
    }
    if at < text.len() {
        segments.push((text.len() - at, false));
    }
    segments
}

/// Paints the URL field's text over the Kit input (see `render_url`), from
/// the layout the input painted this frame.
fn paint_url(url: &Entity<InputState>, window: &mut Window, cx: &mut App) {
    let state = url.read(cx);
    let text = state.value();
    if text.is_empty() {
        return;
    }
    let (Some(bounds), Some(clip)) = (state.range_to_bounds(&(0..text.len())), state.text_bounds())
    else {
        return;
    };
    let mono = font(mono_font());
    let runs: Vec<TextRun> = url_segments(&text)
        .into_iter()
        .map(|(len, var)| TextRun {
            len,
            font: mono.clone(),
            color: color(if var {
                "syntax-variable"
            } else {
                "text-gutter"
            }),
            background_color: None,
            underline: None,
            strikethrough: None,
        })
        .collect();
    let line = window
        .text_system()
        .shape_line(text, px(MONO_TEXT), &runs, None);
    window.with_content_mask(Some(ContentMask { bounds: clip }), |window| {
        line.paint(
            bounds.origin,
            bounds.size.height,
            TextAlign::Left,
            None,
            window,
            cx,
        )
        .ok();
    });
}

/// Every request file under `tree`, depth first, for the Duplicate template.
fn collect_sources(tree: &[TreeNode], out: &mut Vec<SourceItem>) {
    for node in tree {
        match node {
            TreeNode::Folder { children, .. } => collect_sources(children, out),
            TreeNode::File {
                rel_path,
                method,
                requests,
                ..
            } => out.push(SourceItem {
                rel: rel_path.clone(),
                label: rel_path.display().to_string().into(),
                method: method
                    .clone()
                    .or_else(|| requests.first().map(|r| r.method.clone()))
                    .map(SharedString::from),
                count: requests.len(),
            }),
        }
    }
}

fn already_exists(file: &str) -> String {
    format!("A file named {file} already exists")
}

/// Every folder under `tree`, depth first, as paths relative to the root.
pub(super) fn collect_folders(tree: &[TreeNode], parent: &Path, out: &mut Vec<PathBuf>) {
    for node in tree {
        if let TreeNode::Folder { name, children } = node {
            let path = parent.join(name);
            out.push(path.clone());
            collect_folders(children, &path, out);
        }
    }
}

/// `Payments API` for the root, `Payments API / charges / refunds` below it.
pub(super) fn folder_label(collection: &str, folder: &Path) -> SharedString {
    folder
        .components()
        .fold(collection.to_string(), |label, c| {
            format!("{label} / {}", c.as_os_str().to_string_lossy())
        })
        .into()
}

/// An empty folder, ignoring the `.DS_Store` Finder leaves behind.
fn is_empty_dir(dir: &Path) -> io::Result<bool> {
    for entry in std::fs::read_dir(dir)? {
        if entry?.file_name() != ".DS_Store" {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `dir` with the home folder shown as `~`.
pub(super) fn display_path(dir: &Path) -> String {
    match std::env::home_dir().and_then(|home| dir.strip_prefix(home).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => dir.display().to_string(),
    }
}

impl Workspace {
    /// The New dialog on the request side; `folder` (relative to the root)
    /// is preselected, else the active file's folder.
    pub(super) fn new_request(
        &mut self,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<NewDialog> {
        self.open_new_dialog(Kind::Request, Template::Blank, folder, window, cx)
    }

    pub(super) fn new_collection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<NewDialog> {
        self.open_new_dialog(Kind::Collection, Template::Blank, None, window, cx)
    }

    /// The New dialog on the From curl template.
    #[cfg(test)]
    fn new_from_curl(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Entity<NewDialog> {
        self.open_new_dialog(Kind::Request, Template::FromCurl, None, window, cx)
    }

    /// Where a new file goes by default: the active file's folder, else
    /// the root.
    pub(super) fn default_folder(&self) -> PathBuf {
        self.state
            .active_document()
            .and_then(|d| d.rel_path.parent().map(Path::to_path_buf))
            .unwrap_or_default()
    }

    fn open_new_dialog(
        &mut self,
        kind: Kind,
        template: Template,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<NewDialog> {
        let folder = folder.unwrap_or_else(|| self.default_folder());
        let ws = cx.entity().downgrade();
        let active = self.state.active_document().map(|d| d.rel_path.clone());
        let collection = self.state.collection.as_ref().map(|c| (c, folder));
        let dialog =
            cx.new(|cx| NewDialog::new(ws, kind, template, collection, active, window, cx));
        let content = dialog.clone();
        let first_build = std::cell::Cell::new(true);
        window.open_dialog(cx, move |d, window, cx| {
            if first_build.replace(false) {
                content.read(cx).initial_focus(cx).focus(window, cx);
            }
            let on_ok = content.clone();
            // No entrance motion: a form opened from a shortcut should be
            // ready to type into at once.
            d.entrance(DialogEntrance::None)
                .w(px(520.))
                .margin_top(px(120.))
                .p_0()
                .close_button(false)
                // ↵ in a single-line field creates; Create closes on success.
                .on_ok(move |_, window, cx| {
                    on_ok.update(cx, |this, cx| this.create(window, cx));
                    false
                })
                .child(content.clone())
        });
        dialog
    }

    /// Writes a new file into the open collection; never overwrites.
    fn create_file(&mut self, rel: &Path, text: &str) -> io::Result<()> {
        match &mut self.state.collection {
            Some(collection) => collection.create_file(rel, text),
            None => Err(io::Error::other("no collection is open")),
        }
    }

    /// Once unsaved changes are settled, writes `file` into `dir` and opens
    /// `dir` as the collection with `file` in a tab.
    fn create_collection(
        &mut self,
        dir: PathBuf,
        file: PathBuf,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settle_unsaved(
            Rc::new(move |this, window, cx| {
                let written = Collection::open(&dir).and_then(|mut c| c.create_file(&file, &text));
                if let Err(e) = written {
                    let path = dir.join(&file);
                    notify_error(
                        format!("Couldn't create {}: {e}", path.display()),
                        window,
                        cx,
                    );
                    return;
                }
                if this.open_collection(&dir, window, cx) {
                    this.remember_folder(&dir);
                    this.open_file(&file, window, cx);
                }
            }),
            window,
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use super::{Kind, NewDialog, Template, url_segments};
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, VisualTestContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};

    /// A window on a collection with `charges/create-charge.yaml`.
    fn open(
        cx: &mut TestAppContext,
        root: Option<PathBuf>,
        store: StateStore,
    ) -> (AnyWindowHandle, Entity<Workspace>) {
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
                        root,
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

    fn collection() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("charges")).unwrap();
        std::fs::write(
            dir.path().join("charges/create-charge.yaml"),
            "request:\n  url: https://api.example.com\n",
        )
        .unwrap();
        dir
    }

    fn set(
        input: &Entity<gpui_kit::component::input::InputState>,
        text: &str,
        cx: &mut VisualTestContext,
    ) {
        let text = text.to_string();
        cx.update(|window, cx| input.update(cx, |s, cx| s.set_value(text, window, cx)));
    }

    #[gpui_kit::test]
    fn new_request_writes_the_file_and_opens_it(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| {
            ws.update(cx, |ws, cx| {
                ws.new_request(Some("charges".into()), window, cx)
            })
        });
        let (name, url) = dialog.read_with(cx, |d, _| (d.name.clone(), d.url.clone()));
        set(&name, "Capture charge", cx);
        set(&url, "https://api.example.com/v1/charges/1/capture", cx);
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.create(window, cx)));
        cx.run_until_parked();
        let text = std::fs::read_to_string(dir.path().join("charges/capture-charge.yaml")).unwrap();
        assert_eq!(
            text,
            "request:\n  name: Capture charge\n  url: https://api.example.com/v1/charges/1/capture\n  method: GET\n"
        );
        ws.read_with(cx, |ws, _| {
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("charges/capture-charge.yaml"));
        });
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[test]
    fn url_segments_mark_variables() {
        assert_eq!(
            url_segments("${BASE_URL}/v1/${id}"),
            [(11, true), (4, false), (5, true)]
        );
        assert_eq!(url_segments("https://x/${BA"), [(10, false), (4, true)]);
        assert_eq!(url_segments("plain"), [(5, false)]);
        assert!(url_segments("").is_empty());
    }

    #[gpui_kit::test]
    fn the_method_select_sets_the_method(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog =
            cx.update(|window, cx| ws.update(cx, |ws, cx| ws.new_request(None, window, cx)));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(
                    "Capture charge|post|${BASE_URL}/v1/charges",
                    false,
                    window,
                    cx,
                );
                d.create(window, cx);
            })
        });
        cx.run_until_parked();
        let text = std::fs::read_to_string(dir.path().join("capture-charge.yaml")).unwrap();
        assert_eq!(
            text,
            "request:\n  name: Capture charge\n  url: \"${BASE_URL}/v1/charges\"\n  method: POST\n"
        );
    }

    #[gpui_kit::test]
    fn duplicate_copies_the_active_request_under_the_new_name(cx: &mut TestAppContext) {
        let dir = collection();
        std::fs::write(
            dir.path().join("charges/get-charge.yaml"),
            "# Read it back\nrequest:\n  name: Get charge\n  url: ${BASE_URL}/v1/charges/1\n",
        )
        .unwrap();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(std::path::Path::new("charges/get-charge.yaml"), window, cx);
                ws.new_request(None, window, cx)
            })
        });
        let name = dialog.read_with(cx, |d, _| d.name.clone());
        dialog.update(cx, |d, cx| d.set_template(Template::Duplicate, cx));

        // The source's own name: refused, nothing overwritten.
        set(&name, "Get charge", cx);
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.create(window, cx)));
        dialog.read_with(cx, |d, _| {
            assert_eq!(
                d.error.as_deref(),
                Some("A file named get-charge.yaml already exists")
            )
        });
        let original = std::fs::read_to_string(dir.path().join("charges/get-charge.yaml")).unwrap();
        assert!(original.contains("name: Get charge"), "{original}");

        set(&name, "Get charge again", cx);
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.create(window, cx)));
        cx.run_until_parked();
        let copy =
            std::fs::read_to_string(dir.path().join("charges/get-charge-again.yaml")).unwrap();
        assert_eq!(
            copy,
            "# Read it back\nrequest:\n  name: Get charge again\n  url: ${BASE_URL}/v1/charges/1\n"
        );
        ws.read_with(cx, |ws, _| {
            let doc = ws.state.active_document().unwrap();
            assert_eq!(doc.rel_path, Path::new("charges/get-charge-again.yaml"));
        });
    }

    #[gpui_kit::test]
    fn duplicate_lists_every_request_file(cx: &mut TestAppContext) {
        let dir = collection();
        std::fs::write(
            dir.path().join("flow.yaml"),
            "requests:\n  - url: https://a\n    method: POST\n  - url: https://b\n",
        )
        .unwrap();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog =
            cx.update(|window, cx| ws.update(cx, |ws, cx| ws.new_request(None, window, cx)));
        let sources = dialog.read_with(cx, |d, cx| {
            let mut out = Vec::new();
            let root = ws.read(cx).state.collection.as_ref().unwrap();
            super::collect_sources(&root.tree, &mut out);
            assert!(d.source.is_some());
            out
        });
        let summary: Vec<(String, Option<&str>, usize)> = sources
            .iter()
            .map(|s| (s.rel.display().to_string(), s.method.as_deref(), s.count))
            .collect();
        assert_eq!(
            summary,
            [
                ("charges/create-charge.yaml".to_string(), Some("GET"), 0),
                ("flow.yaml".to_string(), Some("POST"), 2),
            ]
        );
    }

    #[gpui_kit::test]
    fn an_existing_file_is_an_inline_error(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| {
            ws.update(cx, |ws, cx| {
                ws.new_request(Some("charges".into()), window, cx)
            })
        });
        let (name, url) = dialog.read_with(cx, |d, _| (d.name.clone(), d.url.clone()));
        set(&name, "Create charge", cx);
        set(&url, "https://api.example.com/other", cx);
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.create(window, cx)));
        dialog.read_with(cx, |d, _| {
            assert_eq!(
                d.error.as_deref(),
                Some("A file named create-charge.yaml already exists")
            );
        });
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        let text = std::fs::read_to_string(dir.path().join("charges/create-charge.yaml")).unwrap();
        assert_eq!(text, "request:\n  url: https://api.example.com\n");
    }

    #[gpui_kit::test]
    fn import_converts_a_curl_command(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.new_from_curl(window, cx)));
        let (name, curl) = dialog.read_with(cx, |d, _| (d.name.clone(), d.curl.clone()));
        set(&name, "Refund", cx);
        cx.update(|window, cx| {
            curl.update(cx, |s, cx| {
                s.set_value(
                    "curl -X POST https://api.example.com/v1/refunds",
                    window,
                    cx,
                )
            });
            dialog.update(cx, |d, cx| d.create(window, cx));
        });
        cx.run_until_parked();
        let text = std::fs::read_to_string(dir.path().join("refund.yaml")).unwrap();
        assert!(text.contains("name: Refund"), "{text}");
        assert!(text.contains("method: POST"), "{text}");
        assert_eq!(crate::model::diagnostics::diagnose(&text), []);
    }

    #[gpui_kit::test]
    fn new_collection_needs_an_empty_folder(cx: &mut TestAppContext) {
        let dir = collection();
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), store.clone());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog: Entity<NewDialog> =
            cx.update(|window, cx| ws.update(cx, |ws, cx| ws.new_collection(window, cx)));
        dialog.read_with(cx, |d, _| assert_eq!(d.kind, Kind::Collection));

        // Not empty: refused, nothing written.
        dialog.update(cx, |d, _| d.picked = Some(dir.path().to_path_buf()));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.create(window, cx)));
        dialog.read_with(cx, |d, _| {
            assert!(
                d.error.as_deref().unwrap().contains("isn't empty"),
                "{:?}",
                d.error
            )
        });
        assert!(!dir.path().join("requests.yaml").exists());

        let empty = tempfile::tempdir().unwrap();
        dialog.update(cx, |d, _| d.picked = Some(empty.path().to_path_buf()));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.create(window, cx)));
        cx.run_until_parked();
        let text = std::fs::read_to_string(empty.path().join("requests.yaml")).unwrap();
        assert_eq!(
            text,
            "request:\n  name: First request\n  url: https://api.example.com\n  method: GET\n"
        );
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, empty.path());
            assert_eq!(
                ws.state.active_document().unwrap().rel_path,
                Path::new("requests.yaml")
            );
        });
        // Remembered in the injected store, not the user's.
        assert_eq!(store.last_folder().as_deref(), Some(empty.path()));
    }

    #[gpui_kit::test]
    fn enter_in_the_curl_field_is_a_newline_and_cmd_enter_creates(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.new_from_curl(window, cx)));
        let name = dialog.read_with(cx, |d, _| d.name.clone());
        set(&name, "Refund", cx);
        cx.run_until_parked();
        // The curl field has focus; paste-like typing with a continuation.
        cx.simulate_input("curl -X POST https://api.example.com/v1/refunds \\");
        cx.simulate_keystrokes("enter");
        cx.simulate_input("  -d amount=100");
        cx.run_until_parked();
        assert!(
            !dir.path().join("refund.yaml").exists(),
            "enter must not create"
        );
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        let curl = dialog.read_with(cx, |d, cx| d.curl.read(cx).value().to_string());
        assert_eq!(
            curl,
            "curl -X POST https://api.example.com/v1/refunds \\\n  -d amount=100"
        );
        cx.simulate_keystrokes("cmd-enter");
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| assert_eq!(d.error, None));
        let text = std::fs::read_to_string(dir.path().join("refund.yaml")).unwrap();
        assert!(text.contains("method: POST"), "{text}");
        assert!(text.contains("amount"), "{text}");
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }
}
