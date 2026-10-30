//! The Import dialog (`Import.dc.html`, ⌘I): request files from pasted curl
//! commands, or the YAML files of another folder copied into the
//! collection. The Postman and OpenAPI tabs come in a later build.

use super::app::{CreateNew, mono_font};
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::new_dialog::{collect_folders, display_path, folder_label};
use super::settings_kit::label_text;
use super::workspace::{Workspace, notify_error};
use super::{color, method_color};
use crate::model::collection::Collection;
use crate::model::collection_meta::CollectionMeta;
use crate::model::environments::{DEFAULT_ENVIRONMENT, EnvCtx, resolve_vars};
use crate::model::import::{
    CopyPlan, CurlImport, Extracted, ImportEnv, convert_commands, copy_planned, curl_targets,
    plan_yaml_copy, reads_as_yaml, replaced_note, write_new,
};
use crate::model::secrets::SecretStore;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::dialog::DialogEntrance;
use gpui_kit::component::input::{Editor, EditorState, InputEvent, Textarea, TextareaState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{Disableable as _, IndexPath, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Sizes from the board.
const DIALOG_W: f32 = 1080.;
const PANE_H: f32 = 300.;
/// The preview's rows: 12px Geist Mono on 18px lines, 8px above and below.
const PREVIEW_TEXT: f32 = 12.;
const PREVIEW_LINE: f32 = 18.;
const PREVIEW_PAD: f32 = 8.;
/// A tab's body: 16 + a 16px label + 10 + the pane + 10 + an 18px hint + 16.
const BODY_H: f32 = PANE_H + 86.;
/// A folded command's header row in the preview list.
const SECTION_H: f32 = 32.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportTab {
    Curl,
    Postman,
    OpenApi,
    Folder,
}

const TABS: [(ImportTab, &str, &str); 4] = [
    (ImportTab::Curl, "import-tab-curl", "curl"),
    (
        ImportTab::Postman,
        "import-tab-postman",
        "Postman collection",
    ),
    (ImportTab::OpenApi, "import-tab-openapi", "OpenAPI"),
    (ImportTab::Folder, "import-tab-folder", "Folder of YAML"),
];

pub struct ImportDialog {
    ws: WeakEntity<Workspace>,
    pub(super) tab: ImportTab,
    pub(super) curl: Entity<TextareaState>,
    /// Each pasted command, converted.
    imports: Vec<CurlImport>,
    /// Where each convertible command's file goes, relative to the target
    /// root (`None`: it failed to convert and is skipped).
    targets: Vec<Option<PathBuf>>,
    /// A read-only YAML preview per command, by index.
    previews: Vec<Option<Entity<EditorState>>>,
    /// Commands whose preview is folded away.
    collapsed: HashSet<usize>,
    /// The open collection's root; `None`: the import starts a collection
    /// in `picked_target`.
    root: Option<PathBuf>,
    folders: Vec<PathBuf>,
    folder: Option<Entity<SelectState<Vec<SharedString>>>>,
    /// The folder chosen on disk when no collection is open.
    pub(super) picked_target: Option<PathBuf>,
    /// Folder of YAML: the folder to copy from, and what would be copied.
    pub(super) source: Option<PathBuf>,
    plan: Result<Vec<CopyPlan>, String>,
    pub(super) error: Option<String>,
    /// The environment the import resolves `BASE_URL` and variables in.
    env: ImportContext,
    /// Names whose Keychain item holds another value: the commands skip
    /// them (`API_TOKEN` → `API_TOKEN_2`).
    held: HashMap<String, String>,
    /// Save the extracted secret values to the Keychain on Import.
    pub(super) save_secrets: bool,
    /// Tests: writing fails from this file on.
    #[cfg(test)]
    pub(super) fail_after: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

/// A variable's current value, by name.
pub(super) type VarLookup = Rc<dyn Fn(&str) -> Option<String>>;
/// The import context for a folder picked on Welcome.
pub(super) type ContextFor = Rc<dyn Fn(&Path) -> ImportContext>;

/// What the workspace hands the dialog about the target's environment.
pub(super) struct ImportContext {
    /// `Default` or a `.env.{name}` name.
    pub environment: String,
    pub base_url: Option<String>,
    /// A variable's current value (Keychain, process env, `.env`),
    /// remembered per name while the dialog is open.
    pub known: VarLookup,
    /// The Keychain's value right now, uncached: the last check before an
    /// import names a variable.
    pub keychain: VarLookup,
    /// With no collection open: the context of the folder picked as the
    /// target.
    pub for_folder: Option<ContextFor>,
}

/// `lookup`, asked at most once per name.
pub(super) fn remembered(lookup: impl Fn(&str) -> Option<String> + 'static) -> VarLookup {
    let seen = std::cell::RefCell::new(HashMap::<String, Option<String>>::new());
    Rc::new(move |name: &str| {
        if let Some(v) = seen.borrow().get(name) {
            return v.clone();
        }
        let v = lookup(name);
        seen.borrow_mut().insert(name.to_string(), v.clone());
        v
    })
}

/// What [`ImportDialog::write`] got on disk.
struct Written {
    root: PathBuf,
    first: Option<PathBuf>,
    /// The secrets of the files written.
    secrets: Vec<Extracted>,
    /// Why the rest wasn't written.
    error: Option<String>,
    /// The commands written (by index), when only some were.
    done: Vec<usize>,
}

impl ImportDialog {
    fn new(
        ws: WeakEntity<Workspace>,
        collection: Option<(&Collection, PathBuf)>,
        env: ImportContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let curl = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder(
                "curl -X POST https://api.example.com/v1/customers \\\n  -H \"Content-Type: application/json\" \\\n  -d '{\"name\":\"Acme\"}'",
            )
        });
        let mut subscriptions =
            vec![
                cx.subscribe_in(&curl, window, |this, _, e: &InputEvent, window, cx| {
                    if matches!(e, InputEvent::Change) {
                        this.error = None;
                        this.convert(window, cx);
                    }
                }),
            ];
        let root = collection.as_ref().map(|(c, _)| c.root.clone());
        let (folders, folder) = match collection {
            Some((collection, preselect)) => {
                let mut folders = vec![PathBuf::new()];
                collect_folders(&collection.tree, Path::new(""), &mut folders);
                let labels = folders
                    .iter()
                    .map(|f| folder_label(&collection.name, f))
                    .collect();
                let row = folders.iter().position(|f| *f == preselect).unwrap_or(0);
                let select = cx.new(|cx| {
                    SelectState::new(
                        labels,
                        Some(IndexPath {
                            row,
                            ..IndexPath::default()
                        }),
                        window,
                        cx,
                    )
                });
                subscriptions.push(cx.subscribe(
                    &select,
                    |this, _, _: &SelectEvent<Vec<SharedString>>, cx| {
                        this.retarget(cx);
                        this.replan(cx);
                    },
                ));
                (folders, Some(select))
            }
            None => (Vec::new(), None),
        };
        ImportDialog {
            ws,
            tab: ImportTab::Curl,
            curl,
            imports: Vec::new(),
            targets: Vec::new(),
            previews: Vec::new(),
            collapsed: HashSet::new(),
            root,
            folders,
            folder,
            picked_target: None,
            source: None,
            plan: Ok(Vec::new()),
            error: None,
            env,
            held: HashMap::new(),
            save_secrets: true,
            #[cfg(test)]
            fail_after: None,
            _subscriptions: subscriptions,
        }
    }

    pub(super) fn set_tab(&mut self, tab: ImportTab, cx: &mut Context<Self>) {
        self.tab = tab;
        self.error = None;
        self.replan(cx);
        cx.notify();
    }

    /// Where files go: the collection root, or the folder picked on disk.
    fn target_root(&self) -> Option<&Path> {
        self.root.as_deref().or(self.picked_target.as_deref())
    }

    /// The folder picked under the target root (`""`: the root itself).
    fn target_folder(&self, cx: &App) -> PathBuf {
        let Some(select) = &self.folder else {
            return PathBuf::new();
        };
        let row = select.read(cx).selected_index(cx).map_or(0, |ix| ix.row);
        self.folders.get(row).cloned().unwrap_or_default()
    }

    /// With no collection open, `dir` becomes the target: its own
    /// environment decides `BASE_URL` and which names are taken.
    pub(super) fn set_target(
        &mut self,
        dir: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picked_target = dir;
        if let (Some(dir), Some(for_folder)) = (&self.picked_target, &self.env.for_folder) {
            let mut env = for_folder(dir);
            env.for_folder = self.env.for_folder.clone();
            self.env = env;
            self.held.clear();
        }
        self.convert(window, cx);
        self.replan(cx);
    }

    /// Re-converts the pasted text and refreshes the previews.
    fn convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.curl.read(cx).value();
        let (lookup, held) = (self.env.known.clone(), &self.held);
        let known = |name: &str| held.get(name).cloned().or_else(|| lookup(name));
        let imports = convert_commands(
            &text,
            &ImportEnv {
                base_url: self.env.base_url.as_deref(),
                known: &known,
            },
        );
        // Several commands: the first opens, the others fold, so every
        // command's row shows in the list.
        if imports.len() != self.imports.len() {
            self.collapsed = (1..imports.len()).collect();
        }
        let mut previews = std::mem::take(&mut self.previews);
        previews.resize(imports.len(), None);
        for (slot, import) in previews.iter_mut().zip(&imports) {
            let Ok(converted) = &import.converted else {
                *slot = None;
                continue;
            };
            let yaml = converted.yaml.clone();
            match slot {
                Some(editor) => {
                    if editor.read(cx).value().as_ref() != yaml {
                        editor.update(cx, |e, cx| e.set_value(yaml, window, cx));
                    }
                }
                None => {
                    *slot = Some(cx.new(|cx| {
                        EditorState::new(window, cx)
                            .language("yaml")
                            .line_number(true)
                            .folding(true)
                            .soft_wrap(false)
                            .default_value(yaml)
                    }));
                }
            }
        }
        self.previews = previews;
        self.imports = imports;
        self.retarget(cx);
    }

    /// Recomputes where each command's file goes.
    fn retarget(&mut self, cx: &mut Context<Self>) {
        let folder = self.target_folder(cx);
        let root = self.target_root().map(Path::to_path_buf);
        self.targets = curl_targets(root.as_deref(), &folder, &self.imports);
        cx.notify();
    }

    /// Recomputes what Folder of YAML would copy; only on that tab, when
    /// its source or target changes.
    fn replan(&mut self, cx: &mut Context<Self>) {
        if self.tab != ImportTab::Folder {
            return;
        }
        let folder = self.target_folder(cx);
        let root = self.target_root().map(Path::to_path_buf);
        self.plan = match &self.source {
            Some(source) => {
                plan_yaml_copy(source, root.as_deref(), &folder).map_err(|e| e.to_string())
            }
            None => Ok(Vec::new()),
        };
        cx.notify();
    }

    /// How many files Import would write on the current tab.
    pub(super) fn count(&self) -> usize {
        match self.tab {
            ImportTab::Curl => self.targets.iter().flatten().count(),
            ImportTab::Folder => self.plan.as_ref().map_or(0, Vec::len),
            ImportTab::Postman | ImportTab::OpenApi => 0,
        }
    }

    fn import_label(&self) -> String {
        let n = self.count();
        let noun = match self.tab {
            ImportTab::Folder => "file",
            _ => "request",
        };
        match n {
            0 => "Import".to_string(),
            1 => format!("Import 1 {noun}"),
            n => format!("Import {n} {noun}s"),
        }
    }

    /// Writes the files and opens what was imported, or shows why not.
    /// When only some were written, those are followed up on (tree,
    /// secrets) and the dialog stays open on the error.
    pub(super) fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let written = match self.write(window, cx) {
            Ok(w) => w,
            Err(e) => {
                self.error = Some(e);
                cx.notify();
                return;
            }
        };
        let opened = self.root.is_none();
        let wrote_any = written.first.is_some();
        match &written.error {
            None => window.close_dialog(cx),
            Some(e) => {
                self.error = Some(e.clone());
                if opened && wrote_any {
                    // The folder is the collection now.
                    self.root = Some(written.root.clone());
                }
                cx.notify();
            }
        }
        if written.error.is_some() && !written.done.is_empty() {
            self.drop_written(&written, window, cx);
        }
        if wrote_any || written.error.is_none() {
            let imported = Imported {
                root: written.root,
                opened: opened && wrote_any,
                first: written.first,
                secrets: written.secrets,
                save_secrets: self.save_secrets,
            };
            self.ws
                .update(cx, |ws, cx| ws.imported(imported, window, cx))
                .ok();
        }
        if written.error.is_some() {
            self.retarget(cx);
            self.replan(cx);
        }
    }

    /// After a partial import: the commands already written leave the
    /// pasted text, so a second Import can't write them again as `-2`
    /// copies, and their variables stay taken by their values.
    fn drop_written(&mut self, written: &Written, window: &mut Window, cx: &mut Context<Self>) {
        let rest: Vec<&str> = self
            .imports
            .iter()
            .enumerate()
            .filter(|(ix, _)| !written.done.contains(ix))
            .map(|(_, i)| i.command.as_str())
            .collect();
        let text = rest.join("\n\n");
        for e in &written.secrets {
            self.held.insert(e.var.clone(), e.value.clone());
        }
        self.curl.update(cx, |s, cx| s.set_value(text, window, cx));
        self.convert(window, cx);
    }

    /// Every value taken out of the commands that will be written, once
    /// per variable.
    fn extracted(&self) -> Vec<Extracted> {
        let mut out: Vec<Extracted> = Vec::new();
        for (import, target) in self.imports.iter().zip(&self.targets) {
            if target.is_none() {
                continue;
            }
            for e in &import.extracted {
                if !out.iter().any(|o| o.var == e.var) {
                    out.push(e.clone());
                }
            }
        }
        out
    }

    /// A variable an extracted value would take whose Keychain item holds
    /// something else now; such names are held back and the commands
    /// converted again, so a stored value is never overwritten.
    fn hold_taken_names(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        for _ in 0..8 {
            let clashes: Vec<(String, String)> = self
                .extracted()
                .into_iter()
                .filter_map(|e| {
                    let held = (self.env.keychain)(&e.var)?;
                    (held != e.value).then_some((e.var, held))
                })
                .collect();
            if clashes.is_empty() {
                return Ok(());
            }
            self.held.extend(clashes);
            self.convert(window, cx);
        }
        match self
            .extracted()
            .into_iter()
            .find(|e| (self.env.keychain)(&e.var).is_some_and(|held| held != e.value))
        {
            Some(e) => Err(format!(
                "Couldn't find a free variable name for {}; nothing was imported.",
                e.var
            )),
            None => Ok(()),
        }
    }

    /// Writes what the tab imports. Nothing is written when a file wouldn't
    /// read back; a failure part way returns what got written and why the
    /// rest didn't.
    fn write(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<Written, String> {
        if self.count() == 0 {
            return Err(match self.tab {
                ImportTab::Curl => "Paste a curl command to import.".into(),
                ImportTab::Folder => "Choose a folder with YAML files.".into(),
                _ => "This import comes in a later build.".into(),
            });
        }
        let root = self
            .target_root()
            .map(Path::to_path_buf)
            .ok_or("Choose a folder for the requests.")?;
        match self.tab {
            ImportTab::Curl => {
                self.hold_taken_names(window, cx)?;
                // Names may have been taken since the preview was drawn.
                self.retarget(cx);
                let files: Vec<(usize, &CurlImport, &PathBuf, &str)> = self
                    .imports
                    .iter()
                    .zip(&self.targets)
                    .enumerate()
                    .filter_map(|(ix, (i, t))| {
                        Some((ix, i, t.as_ref()?, i.converted.as_ref().ok()?.yaml.as_str()))
                    })
                    .collect();
                if let Some((_, _, rel, _)) =
                    files.iter().find(|(_, _, _, yaml)| !reads_as_yaml(yaml))
                {
                    return Err(format!(
                        "{} wouldn't read back as YAML; nothing was imported.",
                        rel.display()
                    ));
                }
                let mut written = Written {
                    root: root.clone(),
                    first: None,
                    secrets: Vec::new(),
                    error: None,
                    done: Vec::new(),
                };
                for (n, (ix, import, rel, yaml)) in files.into_iter().enumerate() {
                    #[cfg(test)]
                    let failed = self.fail_after.is_some_and(|k| n >= k);
                    #[cfg(not(test))]
                    let failed = false;
                    let result = if failed {
                        Err(io::Error::other("test failure"))
                    } else {
                        write_new(&root.join(rel), yaml.as_bytes())
                    };
                    if let Err(e) = result {
                        written.error = Some(format!(
                            "Couldn't create {}: {e}. {n} imported before it.",
                            rel.display()
                        ));
                        break;
                    }
                    written.first.get_or_insert_with(|| rel.clone());
                    written.done.push(ix);
                    for e in &import.extracted {
                        if !written.secrets.iter().any(|w| w.var == e.var) {
                            written.secrets.push(e.clone());
                        }
                    }
                }
                Ok(written)
            }
            ImportTab::Folder => {
                self.replan(cx);
                let plan = self.plan.clone()?;
                let (copied, error) = copy_planned(&root, &plan);
                Ok(Written {
                    root,
                    first: (copied > 0).then(|| plan[0].to.clone()),
                    secrets: Vec::new(),
                    error: error.map(|e| format!("Couldn't copy {e}")),
                    done: Vec::new(),
                })
            }
            ImportTab::Postman | ImportTab::OpenApi => unreachable!("count is 0"),
        }
    }

    fn pick(&mut self, source: bool, window: &mut Window, cx: &mut Context<Self>) {
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
            this.update_in(cx, |this, window, cx| {
                let dir = paths.into_iter().next();
                this.error = None;
                if source {
                    this.source = dir;
                    this.replan(cx);
                } else {
                    this.set_target(dir, window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Development aid (`CURL_RUNNER_STUDIO_FILL`): pastes `text`, or with
    /// `folder` set picks it as the Folder of YAML source.
    pub(super) fn dev_fill(
        &mut self,
        text: &str,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(folder) = folder {
            self.source = Some(folder);
            self.set_tab(ImportTab::Folder, cx);
        } else {
            let text = text.replace("\\n", "\n");
            self.curl.update(cx, |s, cx| s.set_value(text, window, cx));
            self.convert(window, cx);
        }
    }

    // Rendering.

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let tabs = TABS.iter().map(|&(tab, id, label)| {
            let on = self.tab == tab;
            div()
                .id(id)
                .flex()
                .items_center()
                .h(px(40.))
                .px(px(2.))
                .mr(px(18.))
                .border_b_1()
                .border_color(if on {
                    color("text-primary")
                } else {
                    transparent_black()
                })
                .text_color(color(if on { "text-primary" } else { "text-secondary" }))
                .when(!on, |el| el.hover(|s| s.text_color(color("text-primary"))))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.set_tab(tab, cx)))
                .child(label)
        });
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .px(px(24.))
                    .pt(px(20.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Import requests"),
                            )
                            .child(
                                div()
                                    .mt(px(6.))
                                    .text_size(px(13.))
                                    .line_height(relative(1.5))
                                    .text_color(color("text-secondary"))
                                    .child(
                                        "Bring in existing calls. You'll see the YAML before \
                                         anything is added.",
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("import-close")
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(28.))
                            .rounded(px(6.))
                            .text_color(color("text-secondary"))
                            .hover(|s| s.bg(color("surface-100")))
                            .cursor_pointer()
                            .on_click(|_, window, cx| window.close_dialog(cx))
                            .child(glyph(icons::CROSS).size(px(14.))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .px(px(24.))
                    .pt(px(8.))
                    .border_b_1()
                    .border_color(color("border-100"))
                    .text_size(px(13.))
                    .children(tabs),
            )
    }

    fn render_curl(&self, cx: &mut Context<Self>) -> Div {
        let found = self.imports.len();
        let hint = match found {
            0 => "Paste a command that starts with curl".to_string(),
            1 => "1 command found · Separate several with a blank line".to_string(),
            n => format!("{n} commands found · Separate several with a blank line"),
        };
        let left = column(true)
            .child(label_text("Paste one or more curl commands"))
            .child(
                div()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .line_height(px(20.))
                    .child(
                        Textarea::new(&self.curl)
                            .h(px(PANE_H))
                            .rounded(px(8.))
                            .border_color(color("border-100"))
                            .bg(color("surface-raised")),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child(hint),
            );
        left_right(left, self.render_previews(cx))
    }

    /// One preview for a single command, a foldable list for several.
    fn render_previews(&self, cx: &mut Context<Self>) -> Div {
        let valid: Vec<usize> = (0..self.imports.len())
            .filter(|&i| self.targets.get(i).is_some_and(Option::is_some))
            .collect();
        let single = self.imports.len() == 1 && valid.len() == 1;
        let file_name = |i: usize| -> SharedString {
            self.targets[i]
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned().into())
                .unwrap_or_default()
        };
        let aside: SharedString = match valid.len() {
            _ if single => file_name(valid[0]),
            _ if self.imports.is_empty() => "".into(),
            1 => "1 file".into(),
            n => format!("{n} files").into(),
        };
        let header = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(label_text("Preview").flex_1())
            .child(
                div()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(aside),
            );
        let card = div()
            .id("import-preview")
            .flex()
            .flex_col()
            .h(px(PANE_H))
            .rounded(px(8.))
            .border_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .overflow_y_scroll();
        let card = if single {
            let editor = self.previews[valid[0]].as_ref().expect("a valid preview");
            card.child(preview_editor(editor, None))
        } else if self.imports.is_empty() {
            card.child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child("The YAML for each command shows here."),
            )
        } else {
            // The open previews share what the rows leave, at least 120px.
            let n = self.imports.len();
            let open = valid.iter().filter(|i| !self.collapsed.contains(i)).count();
            let max_h = ((PANE_H - 2. - n as f32 * SECTION_H) / open.max(1) as f32).max(120.);
            card.children((0..n).map(|i| self.render_section(i, max_h, cx)))
        };
        let notes = self.render_notes(single);
        column(false)
            .child(header)
            .child(card)
            .children(notes)
            .children(self.render_replaced(cx))
    }

    /// Command `i` in the preview list: its file and folded YAML (at most
    /// `max_h` tall), or why it is skipped.
    fn render_section(&self, i: usize, max_h: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let import = &self.imports[i];
        let row = div()
            .id(("import-section", i))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(SECTION_H))
            .px(px(12.))
            .when(i > 0, |d| d.border_t_1().border_color(color("border-100")));
        let section = div()
            .id(("import-command", i))
            .flex()
            .flex_col()
            .flex_none();
        let (Ok(_), Some(rel), Some(editor)) =
            (&import.converted, &self.targets[i], &self.previews[i])
        else {
            let why = import.converted.as_ref().err().cloned().unwrap_or_default();
            return section.child(
                row.child(
                    glyph(icons::ERROR)
                        .size(px(13.))
                        .text_color(color("danger")),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(color("text-secondary"))
                        .child(format!("Command {}", i + 1)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(color("danger"))
                        .child(format!("{why} Skipped.")),
                ),
            );
        };
        let open = !self.collapsed.contains(&i);
        let method = method_of(editor.read(cx).value().as_ref());
        let header = row
            .cursor_pointer()
            .hover(|s| s.bg(color("surface-100")))
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.collapsed.remove(&i) {
                    this.collapsed.insert(i);
                }
                cx.notify();
            }))
            .child(
                glyph(if open {
                    icons::CHEVRON_DOWN
                } else {
                    icons::CHEVRON_RIGHT
                })
                .size(px(12.))
                .text_color(color("text-secondary")),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(40.))
                    .font_family(mono_font())
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(method_color(&method))
                    .child(method.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_family(mono_font())
                    .text_size(px(12.))
                    .child(rel.display().to_string()),
            );
        let lines = editor.read(cx).value().lines().count();
        let fit = lines as f32 * PREVIEW_LINE + 2. * PREVIEW_PAD;
        section.child(header).when(open, |s| {
            s.child(preview_editor(editor, Some(fit.min(max_h))))
        })
    }

    /// What was moved out of the files, and whether the values go to the
    /// Keychain.
    fn render_replaced(&self, cx: &mut Context<Self>) -> Option<Div> {
        let extracted = self.extracted();
        let base_url = self
            .imports
            .iter()
            .zip(&self.targets)
            .any(|(i, t)| t.is_some() && i.base_url);
        let vars: Vec<&str> = extracted.iter().map(|e| e.var.as_str()).collect();
        let text = replaced_note(&vars, base_url)?;
        let env = env_label(&self.env.environment);
        let save = self.save_secrets;
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(note(icons::INFO, "accent-blue", text))
                .when(!extracted.is_empty(), |d| {
                    d.child(
                        div().pl(px(21.)).text_size(px(12.)).child(
                            Checkbox::new("import-save-secrets")
                                .checked(save)
                                .label(format!(
                                    "Save the extracted values to the Keychain for {env}"
                                ))
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.save_secrets = *checked;
                                    cx.notify();
                                })),
                        ),
                    )
                }),
        )
    }

    /// The conversion's warnings under the preview; with several commands
    /// each names its file.
    fn render_notes(&self, single: bool) -> Option<Div> {
        let notes: Vec<Div> = self
            .imports
            .iter()
            .zip(&self.targets)
            .filter_map(|(import, target)| Some((import.converted.as_ref().ok()?, target)))
            .flat_map(|(converted, target)| {
                let file = target
                    .as_ref()
                    .and_then(|t| t.file_name())
                    .map(|n| n.to_string_lossy().into_owned());
                converted.warnings.iter().map(move |w| {
                    let text = match (&file, single) {
                        (Some(file), false) => format!("{file}: {w}"),
                        _ => w.clone(),
                    };
                    note(icons::WARNING, "warning", text)
                })
            })
            .collect();
        (!notes.is_empty()).then(|| div().flex().flex_col().gap(px(8.)).children(notes))
    }

    fn render_folder(&self, cx: &mut Context<Self>) -> Div {
        let left = column(true)
            .child(label_text("Folder to copy from"))
            .child(picker(
                "import-pick-source",
                self.source.as_deref(),
                "No folder chosen",
                cx.listener(|this, _, window, cx| this.pick(true, window, cx)),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(note(
                        icons::INFO,
                        "accent-blue",
                        "Every .yaml and .yml file inside is copied, keeping its subfolders. \
                         Hidden files and a top-level curl-runner.yaml are left out."
                            .into(),
                    ))
                    .child(note(
                        icons::INFO,
                        "accent-blue",
                        "Nothing is overwritten: a name that is taken gets -2, -3, … .".into(),
                    )),
            );
        let folder = self.target_folder(cx);
        let (aside, list): (SharedString, AnyElement) = match &self.plan {
            Err(e) => (
                "".into(),
                empty_state(format!("Couldn't read the folder: {e}")).into_any_element(),
            ),
            Ok(plan) if self.source.is_none() => (
                "".into(),
                empty_state("Choose a folder to see its YAML files.".into()).into_any_element(),
            ),
            Ok(plan) if plan.is_empty() => (
                "0 files".into(),
                empty_state("No YAML files in this folder.".into()).into_any_element(),
            ),
            Ok(plan) => (
                match plan.len() {
                    1 => "1 file".into(),
                    n => format!("{n} files").into(),
                },
                div()
                    .flex()
                    .flex_col()
                    .py(px(4.))
                    .children(plan.iter().map(|p| copy_row(p, &folder)))
                    .into_any_element(),
            ),
        };
        let right = column(false)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(label_text("Files to copy").flex_1())
                    .child(
                        div()
                            .font_family(mono_font())
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(aside),
                    ),
            )
            .child(
                div()
                    .id("import-plan")
                    .flex()
                    .flex_col()
                    .h(px(PANE_H))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(color("border-100"))
                    .bg(color("surface-raised"))
                    .overflow_y_scroll()
                    .child(list),
            );
        left_right(left, right)
    }

    fn render_coming_soon(&self) -> Div {
        let what = match self.tab {
            ImportTab::Postman => "Postman collections",
            _ => "OpenAPI files",
        };
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.))
            .h(px(BODY_H))
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
                        glyph(icons::IMPORT)
                            .size(px(16.))
                            .text_color(color("text-tertiary")),
                    ),
            )
            .child(div().font_weight(FontWeight::MEDIUM).child("Coming soon"))
            .child(
                div()
                    .w(px(360.))
                    .text_size(px(12.))
                    .line_height(relative(1.5))
                    .text_color(color("text-secondary"))
                    .child(format!(
                        "Importing {what} comes in a later build. For now, paste curl \
                         commands or copy a folder of YAML."
                    )),
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> Div {
        let target = match &self.folder {
            Some(select) => div()
                .w(px(240.))
                .child(Select::new(select).h(px(32.)).text_size(px(13.)))
                .into_any_element(),
            None => div()
                .w(px(360.))
                .child(picker(
                    "import-pick-target",
                    self.picked_target.as_deref(),
                    "Choose where the files go",
                    cx.listener(|this, _, window, cx| this.pick(false, window, cx)),
                ))
                .into_any_element(),
        };
        let enabled = self.count() > 0;
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .pl(px(24.))
            .pr(px(16.))
            .py(px(12.))
            .border_t_1()
            .border_color(color("border-100"))
            .bg(color("surface-raised"))
            .child(label_text("Add to"))
            .child(target)
            .child(div().flex_1())
            .child(
                Button::new("import-cancel")
                    .outline()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(13.))
                    .label("Cancel")
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
            .child(
                Button::new("import-go")
                    .primary()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(13.))
                    .label(self.import_label())
                    .disabled(!enabled)
                    .on_click(cx.listener(|this, _, window, cx| this.import(window, cx))),
            )
    }
}

impl Render for ImportDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.tab {
            ImportTab::Curl => self.render_curl(cx),
            ImportTab::Folder => self.render_folder(cx),
            ImportTab::Postman | ImportTab::OpenApi => self.render_coming_soon(),
        };
        div()
            .id("import-dialog")
            .key_context("NewDialog")
            .on_action(cx.listener(|this, _: &CreateNew, window, cx| this.import(window, cx)))
            .flex()
            .flex_col()
            .text_size(px(13.))
            .child(self.render_header(cx))
            .child(body)
            .children(self.error.clone().map(|e| {
                div()
                    .id("import-error")
                    .px(px(24.))
                    .pb(px(12.))
                    .text_size(px(12.))
                    .text_color(color("danger"))
                    .child(e)
            }))
            .child(self.render_footer(cx))
    }
}

/// The two halves of a tab's body, split by a hairline.
/// Every tab's body is as tall as the curl tab's, so switching tabs
/// doesn't move the footer.
fn left_right(left: Div, right: Div) -> Div {
    div().flex().min_h(px(BODY_H)).child(left).child(right)
}

/// One half: 16px above and below, 24px on the dialog's edge, 20px on the
/// split's.
fn column(left: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .flex_basis(px(0.))
        .min_w_0()
        .gap(px(10.))
        .py(px(16.))
        .map(|d| {
            if left {
                d.pl(px(24.))
                    .pr(px(20.))
                    .border_r_1()
                    .border_color(color("border-100"))
            } else {
                d.pl(px(20.)).pr(px(24.))
            }
        })
}

/// A read-only YAML preview `height` px tall inside a list (it scrolls
/// past that); `None` fills its card.
fn preview_editor(editor: &Entity<EditorState>, height: Option<f32>) -> Div {
    let el = Editor::new(editor)
        .readonly(true)
        .bordered(false)
        .appearance(false)
        .text_size(px(PREVIEW_TEXT))
        .line_height(px(PREVIEW_LINE));
    // In a list the section's header row gives the space above.
    let wrap = div()
        .when(height.is_none(), |d| d.pt(px(PREVIEW_PAD)))
        .font_family(mono_font());
    match height {
        Some(h) => wrap.flex_none().h(px(h)).child(el.h_full()),
        None => wrap.flex_1().min_h_0().child(el.h_full()),
    }
}

/// A 12px note with its 13px icon in `tone`.
fn note(icon: &'static str, tone: &'static str, text: String) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(8.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .text_color(color(if tone == "warning" {
            "text-primary"
        } else {
            "text-secondary"
        }))
        .child(
            div()
                .flex_none()
                .mt(px(2.))
                .child(glyph(icon).size(px(13.)).text_color(color(tone))),
        )
        .child(div().flex_1().min_w_0().child(text))
}

fn empty_state(text: String) -> Div {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .h(px(PANE_H - 2.))
        .px(px(24.))
        .text_center()
        .text_size(px(12.))
        .text_color(color("text-tertiary"))
        .child(text)
}

/// One file of the Folder of YAML plan: its path in the source, and the
/// name it gets when that one is taken.
fn copy_row(plan: &CopyPlan, folder: &Path) -> Div {
    let renamed = plan.renamed(folder).then(|| {
        div()
            .flex_none()
            .text_size(px(11.))
            .text_color(color("warning"))
            .child(format!(
                "as {}",
                plan.to
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ))
    });
    let parent = plan
        .rel
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| format!("{}/", p.display()));
    let name = plan
        .rel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(12.))
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .font_family(mono_font())
                .text_size(px(12.))
                .children(parent.map(|p| div().text_color(color("text-tertiary")).child(p)))
                .child(name),
        )
        .children(renamed)
}

/// A chosen folder's path (`~` for home) and its Choose… button.
fn picker(
    id: &'static str,
    dir: Option<&Path>,
    empty: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let path = match dir {
        Some(dir) => div()
            .truncate()
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
            Button::new(id)
                .outline()
                .h(px(32.))
                .px(px(12.))
                .text_size(px(13.))
                .label("Choose…")
                .on_click(on_click),
        )
}

/// `request.method` of a converted file (GET when absent).
fn method_of(yaml: &str) -> SharedString {
    yaml.lines()
        .find_map(|l| l.strip_prefix("  method: "))
        .map_or("GET", str::trim)
        .to_string()
        .into()
}

impl Workspace {
    /// The Import dialog (⌘I), adding to the active file's folder.
    pub(super) fn import(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ImportDialog> {
        let folder = self.default_folder();
        let ws = cx.entity().downgrade();
        let env = self.import_context();
        let collection = self.state.collection.as_ref().map(|c| (c, folder));
        let dialog = cx.new(|cx| ImportDialog::new(ws, collection, env, window, cx));
        let content = dialog.clone();
        let first_build = std::cell::Cell::new(true);
        window.open_dialog(cx, move |d, window, cx| {
            if first_build.replace(false) {
                let focus = content.read(cx).curl.read(cx).focus_handle(cx);
                focus.focus(window, cx);
            }
            d.entrance(DialogEntrance::None)
                .w(px(DIALOG_W))
                .margin_top(px(140.))
                .p_0()
                .close_button(false)
                .on_ok(|_, _, _| false)
                .child(content.clone())
        });
        dialog
    }

    /// The picked environment's `BASE_URL` and lookups of the values its
    /// variables hold, for the Import dialog. With no collection open, the
    /// context comes once a target folder is picked.
    fn import_context(&self) -> ImportContext {
        let (cached, raw) = (self.secret_cache.clone(), self.secrets.clone());
        match &self.state.collection {
            Some(c) => context_for(self.env_ctx(c), cached, raw, None),
            None => {
                let (store, for_cached, for_raw) =
                    (self.store.clone(), cached.clone(), raw.clone());
                let for_folder: ContextFor = Rc::new(move |dir: &Path| {
                    // As `open_collection` will see it.
                    let meta = CollectionMeta::load_or_init(store.base(), dir);
                    let ctx = EnvCtx {
                        root: dir.to_path_buf(),
                        collection: dir.file_name().map_or_else(
                            || dir.display().to_string(),
                            |n| n.to_string_lossy().into_owned(),
                        ),
                        environment: store
                            .environment(dir)
                            .unwrap_or_else(|| DEFAULT_ENVIRONMENT.to_string()),
                        collection_id: meta.id,
                        marked_secret: meta.secret_names,
                    };
                    context_for(ctx, for_cached.clone(), for_raw.clone(), None)
                });
                ImportContext {
                    environment: DEFAULT_ENVIRONMENT.to_string(),
                    base_url: None,
                    known: Rc::new(|_| None),
                    keychain: Rc::new(|_| None),
                    for_folder: Some(for_folder),
                }
            }
        }
    }

    /// After an import: the tree shows the new files and the first opens;
    /// a new collection (`opened`) is opened and remembered first. The
    /// extracted secrets are marked secret in the collection and, when
    /// asked, saved to the Keychain for the picked environment; their
    /// values go nowhere else.
    fn imported(&mut self, imported: Imported, window: &mut Window, cx: &mut Context<Self>) {
        let Imported {
            root,
            opened,
            first,
            secrets,
            save_secrets,
        } = imported;
        if opened {
            if !self.open_collection(&root, window, cx) {
                return;
            }
            self.remember_folder(&root);
        } else if let Some(c) = &mut self.state.collection {
            c.refresh();
        }
        if !secrets.is_empty()
            && let Some(c) = &self.state.collection
        {
            let service = self.env_ctx(c).service();
            for e in &secrets {
                self.meta.set_secret(&e.var, true);
            }
            self.save_meta(window, cx);
            if save_secrets {
                for e in &secrets {
                    // Never over another value: the dialog already moved
                    // to a free name, this is the last line.
                    if self
                        .secrets
                        .get(&service, &e.var)
                        .is_some_and(|held| held != e.value)
                    {
                        notify_error(
                            format!(
                                "{} already holds another value in the Keychain; left unchanged.",
                                e.var
                            ),
                            window,
                            cx,
                        );
                        continue;
                    }
                    if let Err(err) = self.secret_cache.set(&service, &e.var, &e.value) {
                        notify_error(
                            format!("Couldn't save {} to the Keychain: {err}", e.var),
                            window,
                            cx,
                        );
                    }
                }
            }
            self.invalidate_secrets();
        }
        match first {
            Some(first) => self.open_file(&first, window, cx),
            None => notify_error("Nothing was imported.".into(), window, cx),
        }
        self.refresh_vars(cx);
        cx.notify();
    }
}

/// The import context of `ctx`'s environment: `BASE_URL`, and each
/// name's value, remembered for the dialog's life.
fn context_for(
    ctx: EnvCtx,
    cached: std::sync::Arc<dyn SecretStore>,
    raw: std::sync::Arc<dyn SecretStore>,
    for_folder: Option<ContextFor>,
) -> ImportContext {
    let service = ctx.service();
    let environment = ctx.environment.clone();
    let known = remembered(move |name: &str| {
        resolve_vars(
            &[name.to_string()],
            &Default::default(),
            &*cached,
            &ctx,
            &[],
        )
        .into_iter()
        .next()
        .and_then(|v| v.value)
    });
    ImportContext {
        environment,
        base_url: known("BASE_URL"),
        known,
        keychain: Rc::new(move |name: &str| raw.get(&service, name)),
        for_folder,
    }
}

/// What an Import wrote, for the workspace to follow up on.
struct Imported {
    root: PathBuf,
    /// A new collection: open it.
    opened: bool,
    first: Option<PathBuf>,
    /// The secret values taken out of the files.
    secrets: Vec<Extracted>,
    save_secrets: bool,
}

#[cfg(test)]
mod tests {
    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use super::ImportTab;
    use gpui_kit::component::WindowExt as _;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, VisualTestContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::{Path, PathBuf};

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
                        super::super::workspace::test_backend(),
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
        std::fs::create_dir_all(dir.path().join("customers")).unwrap();
        std::fs::write(
            dir.path().join("customers/get-customer.yaml"),
            "request:\n  url: https://api.example.com\n",
        )
        .unwrap();
        dir
    }

    const TWO: &str = "curl -X POST https://api.example.com/v1/customers \\\n  -H 'Content-Type: application/json' \\\n  -d '{\"name\":\"Acme\"}'\n\ncurl https://api.example.com/v1/customers/cus_8f2k1Abc\n\nnot a command";

    #[gpui_kit::test]
    fn import_two_curls_creates_two_files(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| {
            ws.update(cx, |ws, cx| {
                // The active file's folder is the target.
                ws.open_file(Path::new("customers/get-customer.yaml"), window, cx);
                ws.import(window, cx)
            })
        });
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.dev_fill(TWO, None, window, cx)));
        cx.run_until_parked();
        // Two convert; the third is shown with its error and skipped.
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.imports.len(), 3);
            assert_eq!(d.count(), 2);
            assert_eq!(d.import_label(), "Import 2 requests");
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        let created =
            std::fs::read_to_string(dir.path().join("customers/create-customer.yaml")).unwrap();
        assert!(created.starts_with("# Imported from curl\n"), "{created}");
        assert!(created.contains("name: create-customer"), "{created}");
        assert!(created.contains("method: POST"), "{created}");
        assert_eq!(crate::model::diagnostics::diagnose(&created), []);
        // `get-customer.yaml` was taken: never overwritten.
        let kept = std::fs::read_to_string(dir.path().join("customers/get-customer.yaml")).unwrap();
        assert_eq!(kept, "request:\n  url: https://api.example.com\n");
        let got =
            std::fs::read_to_string(dir.path().join("customers/get-customer-2.yaml")).unwrap();
        assert!(got.contains("name: get-customer"), "{got}");
        ws.read_with(cx, |ws, _| {
            assert_eq!(
                ws.state.active_document().unwrap().rel_path,
                Path::new("customers/create-customer.yaml")
            );
        });
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn folder_of_yaml_copies_into_the_target(cx: &mut TestAppContext) {
        let dir = collection();
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("nested")).unwrap();
        std::fs::write(src.path().join("a.yaml"), "request:\n  url: https://a\n").unwrap();
        std::fs::write(
            src.path().join("nested/b.yml"),
            "request:\n  url: https://b\n",
        )
        .unwrap();
        std::fs::write(src.path().join("readme.md"), "x").unwrap();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill("", Some(src.path().to_path_buf()), window, cx);
            })
        });
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.tab, ImportTab::Folder);
            assert_eq!(d.import_label(), "Import 2 files");
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        assert!(dir.path().join("a.yaml").is_file());
        assert!(dir.path().join("nested/b.yml").is_file());
        assert!(!dir.path().join("readme.md").exists());
        ws.read_with(cx, |ws, _| {
            let c = ws.state.collection.as_ref().unwrap();
            assert!(c.tree.len() >= 3, "{:?}", c.tree);
        });
    }

    #[gpui_kit::test]
    fn coming_soon_tabs_import_nothing(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.dev_fill(TWO, None, window, cx)));
        dialog.update(cx, |d, cx| d.set_tab(ImportTab::Postman, cx));
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.count(), 0);
            assert!(d.error.is_some());
        });
        assert!(!dir.path().join("customers/create-customer.yaml").exists());
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn without_a_collection_the_import_starts_one(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, None, store.clone());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        let target = tempfile::tempdir().unwrap();
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.picked_target = Some(target.path().to_path_buf());
                d.dev_fill("curl https://api.example.com/v1/charges", None, window, cx);
                d.import(window, cx);
            })
        });
        cx.run_until_parked();
        assert!(target.path().join("get-charges.yaml").is_file());
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, target.path());
            assert_eq!(ws.recent.len(), 1);
        });
        assert_eq!(store.recent()[0].path, target.path());
    }

    const KEYED: &str = concat!(
        "curl -X POST https://api.example.com/v1/customers -H 'Authorization: Bearer sk_test",
        "_51HxAbCdEfGhIjKlMnOpQrSt'"
    );

    /// Imports `KEYED` with the Keychain box `save`; returns the file and
    /// what the Keychain holds for API_TOKEN.
    fn import_keyed(cx: &mut TestAppContext, save: bool) -> (String, Option<String>, Vec<String>) {
        let dir = collection();
        std::fs::write(
            dir.path().join(".env"),
            "BASE_URL=https://api.example.com\n",
        )
        .unwrap();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(KEYED, None, window, cx);
                d.save_secrets = save;
            })
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        let text = std::fs::read_to_string(dir.path().join("create-customer.yaml")).unwrap();
        let (stored, marked) = ws.read_with(cx, |ws, _| {
            let c = ws.state.collection.as_ref().unwrap();
            let service = ws.env_ctx(c).service();
            (
                ws.secrets.get(&service, "API_TOKEN"),
                ws.meta.secret_names.clone(),
            )
        });
        (text, stored, marked)
    }

    #[gpui_kit::test]
    fn extracted_secrets_go_to_the_keychain_not_the_file(cx: &mut TestAppContext) {
        let (text, stored, marked) = import_keyed(cx, true);
        assert!(!text.contains("sk_test_51Hx"), "{text}");
        assert!(text.contains("url: ${BASE_URL}/v1/customers"), "{text}");
        assert!(
            text.contains("Authorization: Bearer ${API_TOKEN}"),
            "{text}"
        );
        assert_eq!(
            stored.as_deref(),
            Some(concat!("sk_test", "_51HxAbCdEfGhIjKlMnOpQrSt"))
        );
        assert_eq!(marked, ["API_TOKEN"]);
    }

    #[gpui_kit::test]
    fn unchecked_the_keychain_is_left_alone(cx: &mut TestAppContext) {
        let (text, stored, marked) = import_keyed(cx, false);
        assert!(!text.contains("sk_test_51Hx"), "{text}");
        assert_eq!(stored, None);
        // Still a secret: it shows as Not set, never as a plain value.
        assert_eq!(marked, ["API_TOKEN"]);
    }

    #[gpui_kit::test]
    fn welcome_import_checks_the_target_folders_keychain(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let (window, ws) = open(cx, None, StateStore::at(state.path()));
        let cx = &mut VisualTestContext::from_window(window, cx);
        let target = tempfile::tempdir().unwrap();
        let service = crate::model::secrets::service_for(state.path(), target.path(), "Default");
        let secrets = ws.read_with(cx, |ws, _| ws.secrets.clone());
        secrets
            .set(&service, "API_TOKEN", "tok_already_there")
            .unwrap();
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(KEYED, None, window, cx);
                d.set_target(Some(target.path().to_path_buf()), window, cx);
                d.import(window, cx);
            })
        });
        cx.run_until_parked();
        let text = std::fs::read_to_string(target.path().join("create-customer.yaml")).unwrap();
        assert!(text.contains("Bearer ${API_TOKEN_2}"), "{text}");
        assert_eq!(
            secrets.get(&service, "API_TOKEN").as_deref(),
            Some("tok_already_there")
        );
        assert_eq!(
            secrets.get(&service, "API_TOKEN_2").as_deref(),
            Some(concat!("sk_test", "_51HxAbCdEfGhIjKlMnOpQrSt"))
        );
    }

    #[gpui_kit::test]
    fn a_keychain_value_set_meanwhile_is_never_overwritten(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.dev_fill(KEYED, None, window, cx)));
        // Stored after the preview was drawn (the dialog's lookup is stale).
        let (secrets, service) = ws.read_with(cx, |ws, _| {
            let c = ws.state.collection.as_ref().unwrap();
            (ws.secrets.clone(), ws.env_ctx(c).service())
        });
        secrets
            .set(&service, "API_TOKEN", "tok_set_meanwhile")
            .unwrap();
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        let text = std::fs::read_to_string(dir.path().join("create-customer.yaml")).unwrap();
        assert!(text.contains("Bearer ${API_TOKEN_2}"), "{text}");
        assert_eq!(
            secrets.get(&service, "API_TOKEN").as_deref(),
            Some("tok_set_meanwhile")
        );
    }

    #[gpui_kit::test]
    fn a_partial_import_follows_up_on_what_was_written(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        let two = "curl https://api.example.com/v1/charges -H 'X-Auth-Token: tok_first_1111'\n\n\
                   curl https://api.example.com/v1/refunds -H 'X-Api-Key: key_second_2222'";
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(two, None, window, cx);
                d.fail_after = Some(1);
                d.import(window, cx);
            })
        });
        cx.run_until_parked();
        assert!(dir.path().join("get-charges.yaml").is_file());
        assert!(!dir.path().join("get-refunds.yaml").exists());
        dialog.read_with(cx, |d, _| {
            let e = d.error.as_deref().unwrap();
            assert!(
                e.contains("get-refunds.yaml") && e.contains("1 imported"),
                "{e}"
            );
        });
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.meta.secret_names, ["AUTH_TOKEN"]);
            let c = ws.state.collection.as_ref().unwrap();
            let service = ws.env_ctx(c).service();
            assert_eq!(
                ws.secrets.get(&service, "AUTH_TOKEN").as_deref(),
                Some("tok_first_1111")
            );
            assert_eq!(ws.secrets.get(&service, "API_KEY"), None);
            let names: Vec<String> = c
                .tree
                .iter()
                .map(|n| format!("{n:?}"))
                .filter(|n| n.contains("get-charges"))
                .collect();
            assert_eq!(names.len(), 1);
        });
    }

    #[gpui_kit::test]
    fn after_a_partial_import_a_retry_writes_no_duplicates(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        let two =
            "curl https://api.example.com/v1/charges\n\ncurl https://api.example.com/v1/refunds";
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(two, None, window, cx);
                d.fail_after = Some(1);
                d.import(window, cx);
            })
        });
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.imports.len(), 1);
            assert_eq!(d.imports[0].name, "get-refunds");
        });
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.fail_after = None;
                d.import(window, cx);
            })
        });
        cx.run_until_parked();
        assert!(dir.path().join("get-charges.yaml").is_file());
        assert!(dir.path().join("get-refunds.yaml").is_file());
        assert!(!dir.path().join("get-charges-2.yaml").exists());
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn no_free_name_fails_before_writing(cx: &mut TestAppContext) {
        let dir = collection();
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx)));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(KEYED, None, window, cx);
                // Every name already holds something else.
                d.env.keychain = std::rc::Rc::new(|_| Some("held elsewhere".into()));
                d.import(window, cx);
            })
        });
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| {
            let e = d.error.as_deref().unwrap();
            assert!(e.contains("free variable name"), "{e}");
        });
        assert!(!dir.path().join("create-customer.yaml").exists());
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
    }
}
