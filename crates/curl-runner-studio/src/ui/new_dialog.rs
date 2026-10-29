//! The New dialog (`NewRequest.dc.html`): a request file from a blank
//! template or a curl command (⌘N, ⌘I), or a new collection (⇧⌘N).

use super::app::{CreateNew, mono_font};
use super::color;
use super::workspace::{Workspace, notify_error};
use crate::model::collection::{Collection, TreeNode};
use crate::model::templates::{file_name_for, request_from_curl, request_template};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::{DialogEntrance, DialogFooter};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::{IconName, IndexPath, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// The file a new collection starts with.
const STARTER_FILE: &str = "requests.yaml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Request,
    Collection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Blank,
    FromCurl,
}

pub struct NewDialog {
    ws: WeakEntity<Workspace>,
    kind: Kind,
    template: Template,
    name: Entity<InputState>,
    method: Entity<SelectState<Vec<SharedString>>>,
    url: Entity<InputState>,
    curl: Entity<TextareaState>,
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Create charge"));
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("https://api.example.com"));
        let curl = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(5)
                .placeholder("curl -X POST https://api.example.com/v1/charges -d amount=2500")
        });
        let methods = METHODS.map(SharedString::from).to_vec();
        let method = cx.new(|cx| SelectState::new(methods, Some(IndexPath::default()), window, cx));
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
            .items_center()
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
                    .icon(IconName::Close)
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
    }

    fn render_request(&self, cx: &mut Context<Self>) -> Div {
        let folder = match &self.folder {
            Some(select) => Select::new(select).into_any_element(),
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
                .gap(px(4.))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(8.))
                .border_1()
                .border_color(color(if on { "text-tertiary" } else { "border-100" }))
                .bg(color(if on { "surface-200" } else { "surface-raised" }))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.set_template(t, cx)))
                .child(div().child(title))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(color("text-tertiary"))
                        .child(detail),
                )
        };
        let name = self.name.read(cx).value();
        let saved_as =
            (!name.trim().is_empty()).then(|| hint(format!("Saved as {}", file_name_for(&name))));
        let request = match self.template {
            Template::Blank => field(
                "Request",
                div()
                    .flex()
                    .gap(px(8.))
                    .child(div().w(px(110.)).child(Select::new(&self.method)))
                    .child(
                        div()
                            .flex_1()
                            .font_family(mono_font())
                            .child(Input::new(&self.url)),
                    ),
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
                            .text_size(px(12.))
                            .child(Textarea::new(&self.curl)),
                    )
                    .children(self.render_conversion()),
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
                    )),
            )
            .child(field(
                "Name",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(Input::new(&self.name))
                    .children(saved_as),
            ))
            .child(request)
            .child(field("Folder", folder))
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
        let create = match self.kind {
            Kind::Request => "Create request",
            Kind::Collection => "Create collection",
        };
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
            .px(px(24.))
            .py(px(12.))
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
                    .ghost()
                    .label("Cancel")
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
            .child(
                Button::new("new-create")
                    .primary()
                    .label(create)
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

fn already_exists(file: &str) -> String {
    format!("A file named {file} already exists")
}

/// Every folder under `tree`, depth first, as paths relative to the root.
fn collect_folders(tree: &[TreeNode], parent: &Path, out: &mut Vec<PathBuf>) {
    for node in tree {
        if let TreeNode::Folder { name, children } = node {
            let path = parent.join(name);
            out.push(path.clone());
            collect_folders(children, &path, out);
        }
    }
}

/// `Payments API` for the root, `Payments API / charges / refunds` below it.
fn folder_label(collection: &str, folder: &Path) -> SharedString {
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
fn display_path(dir: &Path) -> String {
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
    pub(super) fn import(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<NewDialog> {
        self.open_new_dialog(Kind::Request, Template::FromCurl, None, window, cx)
    }

    fn open_new_dialog(
        &mut self,
        kind: Kind,
        template: Template,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<NewDialog> {
        let folder = folder.unwrap_or_else(|| {
            self.state
                .active_document()
                .and_then(|d| d.rel_path.parent().map(Path::to_path_buf))
                .unwrap_or_default()
        });
        let ws = cx.entity().downgrade();
        let collection = self.state.collection.as_ref().map(|c| (c, folder));
        let dialog = cx.new(|cx| NewDialog::new(ws, kind, template, collection, window, cx));
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
                    this.store.remember_folder(&dir);
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
    use super::{Kind, NewDialog};
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
                cx.new(|cx| Workspace::new(root, store, window, cx))
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
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
