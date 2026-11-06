//! The Import dialog (`Import.dc.html`, ⌘I): request files from pasted curl
//! commands, a Postman collection, an OpenAPI or Swagger spec, or the YAML
//! files of another folder copied into the collection.

use super::app::{CreateNew, mono_font};
use super::icons::{self, glyph};
use super::inspector::env_label;
use super::new_dialog::{collect_folders, display_path, folder_label};
use super::settings_kit::label_text;
use super::workspace::{Workspace, edit_notes, notify_error};
use super::{color, method_color};
use crate::model::collection::Collection;
use crate::model::collection_meta::CollectionMeta;
use crate::model::environments::{DEFAULT_ENVIRONMENT, EnvCtx, resolve_vars};
use crate::model::import::{
    CopyPlan, CurlImport, Extracted, ImportEnv, convert_commands, copy_planned, curl_targets,
    plan_yaml_copy, reads_as_yaml, replaced_note, write_new,
};
use crate::model::secrets::SecretStore;
use curl_runner_core::convert::ConvertError;
use curl_runner_core::convert::import::{Attention, Import, ImportedVar, write_import_noting};
use curl_runner_core::convert::openapi::convert_openapi;
use curl_runner_core::convert::postman::convert_postman;
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
/// A file tab's attention pane, so it ends with the preview beside
/// it: the pane, less the picker (32), its status line (18), the pane's
/// label (about 19.5 on the dialog's default line height) and three 10px
/// gaps.
const ATTENTION_H: f32 = PANE_H - 99.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportTab {
    Curl,
    Postman,
    OpenApi,
    Folder,
}

/// What sets the Postman and OpenAPI tabs apart: their source and copy.
struct FileSource {
    convert: fn(&str) -> Result<Import, ConvertError>,
    /// Extensions the tab takes; `None`: any file.
    extensions: Option<&'static [&'static str]>,
    /// The picker's label.
    label: &'static str,
    /// What the file is, in errors: `Couldn't import the {noun}: …`.
    noun: &'static str,
    /// The status line before a pick.
    empty: &'static str,
    /// Import's error before a pick.
    choose: &'static str,
    /// The note under the preview.
    hint: &'static str,
    /// The summary's note on `.env`.
    env_note: &'static str,
    /// Where a secret value came from.
    place: &'static str,
    pick_id: &'static str,
    files_id: &'static str,
}

const POSTMAN: FileSource = FileSource {
    convert: convert_postman,
    extensions: None,
    label: "Postman collection",
    noun: "collection",
    empty: "Pick a collection exported from Postman, or drop it here.",
    choose: "Choose a Postman collection to import.",
    hint: "Each request gets its own file, in a folder per Postman folder. Variables go to .env. \
           Nothing is overwritten: a name that is taken gets -2, -3, … .",
    env_note: "The collection's variables are in .env. Secrets are listed there by name only.",
    place: "Postman variable",
    pick_id: "import-pick-postman",
    files_id: "import-postman-files",
};

const OPENAPI: FileSource = FileSource {
    convert: convert_openapi,
    extensions: Some(&["json", "yaml", "yml"]),
    label: "OpenAPI spec",
    noun: "spec",
    empty: "Pick an OpenAPI or Swagger file, or drop it here.",
    choose: "Choose an OpenAPI spec to import.",
    hint: "One file per operation, in a folder per tag. The base URL and secrets go to .env.",
    env_note: "The base URL and secrets are in .env. Secrets are listed there by name only.",
    place: "OpenAPI variable",
    pick_id: "import-pick-openapi",
    files_id: "import-openapi-files",
};

impl FileSource {
    /// Whether the tab takes a file with `path`'s extension.
    fn accepts(&self, path: &Path) -> bool {
        self.extensions.is_none_or(|exts| has_extension(path, exts))
    }
}

/// `path` ends in one of `exts`, in any case.
fn has_extension(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Why a file tab's pick doesn't import.
#[derive(Debug)]
pub(super) enum PickError {
    /// The file couldn't be read or converted.
    Failed(String),
    /// Not a file the tab takes (its extension).
    Kind,
}

impl PickError {
    /// The status line under the picker.
    fn status(&self, path: &Path) -> String {
        match self {
            PickError::Failed(e) => format!("Couldn't read {}: {e}", file_name(path)),
            PickError::Kind => kind_error(path),
        }
    }

    /// Import's error.
    fn import_error(&self, src: &FileSource, path: &Path) -> String {
        match self {
            PickError::Failed(e) => format!("Couldn't import the {}: {e}", src.noun),
            PickError::Kind => kind_error(path),
        }
    }
}

fn kind_error(path: &Path) -> String {
    format!("{} isn't a .json, .yaml or .yml file.", file_name(path))
}

impl ImportTab {
    /// The source of a tab that imports one file.
    fn file_source(self) -> Option<&'static FileSource> {
        match self {
            ImportTab::Postman => Some(&POSTMAN),
            ImportTab::OpenApi => Some(&OPENAPI),
            ImportTab::Curl | ImportTab::Folder => None,
        }
    }
}

/// A file tab's pick: the file, and what it converts to.
pub(super) type FilePick = Option<(PathBuf, Result<Import, PickError>)>;

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
    /// Postman: the collection file picked, and what it converts to.
    pub(super) postman: FilePick,
    /// Postman: what the import wrote and left to do, shown in its place.
    pub(super) postman_done: Option<FileImportDone>,
    /// OpenAPI: the spec picked, and what it converts to.
    pub(super) openapi: FilePick,
    /// OpenAPI: what the import wrote and left to do.
    pub(super) openapi_done: Option<FileImportDone>,
    /// A file tab's file being read and converted off the UI thread.
    reading: Option<(PathBuf, Task<()>)>,
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
    /// More names to mark secret, whose values aren't in `secrets`.
    secret_names: Vec<String>,
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
            postman: None,
            postman_done: None,
            openapi: None,
            openapi_done: None,
            reading: None,
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
        // The studio's own folder (where received copies live) is never a
        // target: the pick is refused, and an earlier one goes with it, so
        // the dialog asks for a folder again.
        let refused = dir.as_ref().is_some_and(|d| {
            self.ws
                .update(cx, |ws, cx| ws.refuse_received(d, window, cx))
                .unwrap_or(true)
        });
        self.picked_target = if refused { None } else { dir };
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
            ImportTab::Postman | ImportTab::OpenApi => match self.picked(self.tab) {
                Some((_, Ok(import))) if self.reading.is_none() => import.files.len(),
                _ => 0,
            },
        }
    }

    /// A file tab's pick and summary.
    fn file_slots(&mut self, tab: ImportTab) -> (&mut FilePick, &mut Option<FileImportDone>) {
        match tab {
            ImportTab::Postman => (&mut self.postman, &mut self.postman_done),
            ImportTab::OpenApi => (&mut self.openapi, &mut self.openapi_done),
            ImportTab::Curl | ImportTab::Folder => unreachable!("{tab:?} imports no file"),
        }
    }

    fn picked(&self, tab: ImportTab) -> Option<&(PathBuf, Result<Import, PickError>)> {
        match tab {
            ImportTab::Postman => self.postman.as_ref(),
            ImportTab::OpenApi => self.openapi.as_ref(),
            ImportTab::Curl | ImportTab::Folder => None,
        }
    }

    /// The current tab's summary, after a file import.
    fn file_done(&self) -> Option<&FileImportDone> {
        match self.tab {
            ImportTab::Postman => self.postman_done.as_ref(),
            ImportTab::OpenApi => self.openapi_done.as_ref(),
            ImportTab::Curl | ImportTab::Folder => None,
        }
    }

    /// Reads and converts the file at `path` for a file tab, on the UI
    /// thread, so a test sees it at once.
    #[cfg(test)]
    pub(super) fn set_file_import(
        &mut self,
        tab: ImportTab,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let (tab, import) = read_pick(&path, tab, false);
        self.apply_pick(tab, path, import, cx);
    }

    /// Reads, converts and (`route`) routes `path` off the UI thread, for
    /// the picker and drops; the status line says so meanwhile. A newer
    /// pick drops this one.
    fn load_file(&mut self, tab: ImportTab, path: PathBuf, route: bool, cx: &mut Context<Self>) {
        let read = path.clone();
        let work = cx.background_spawn(async move { read_pick(&read, tab, route) });
        let picked = path.clone();
        let task = cx.spawn(async move |this, cx| {
            let (tab, import) = work.await;
            this.update(cx, |this, cx| {
                // Detached, not dropped: this is the task running.
                if let Some((_, task)) = this.reading.take() {
                    task.detach();
                }
                this.apply_pick(tab, picked, import, cx);
                // Not if the user moved to the curl or Folder tab meanwhile.
                if route && this.tab.file_source().is_some() {
                    this.set_tab(tab, cx);
                }
            })
            .ok();
        });
        self.reading = Some((path, task));
        self.error = None;
        cx.notify();
    }

    fn apply_pick(
        &mut self,
        tab: ImportTab,
        path: PathBuf,
        import: Result<Import, PickError>,
        cx: &mut Context<Self>,
    ) {
        let (picked, done) = self.file_slots(tab);
        *picked = Some((path, import));
        *done = None;
        self.error = None;
        cx.notify();
    }

    #[cfg(test)]
    pub(super) fn set_postman(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.set_file_import(ImportTab::Postman, path, cx);
    }

    /// Opens the file a summary item points at (a Pick a file item's
    /// request), closing the dialog so it shows.
    pub(super) fn open_attention(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rel) = self
            .file_done()
            .and_then(|d| d.attention.get(ix))
            .and_then(|(_, opens)| opens.clone())
        else {
            return;
        };
        window.close_dialog(cx);
        self.ws
            .update(cx, |ws, cx| ws.open_file(&rel, window, cx))
            .ok();
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
        // On a file import's summary, ⌘↵ is Done.
        if self.file_done().is_some() {
            window.close_dialog(cx);
            return;
        }
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
            // A file import's summary stays up in the dialog.
            None if self.tab.file_source().is_some() => cx.notify(),
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
                secret_names: written.secret_names,
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
        let before = self.held.clone();
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
            Some(e) => {
                let why = format!(
                    "Couldn't find a free variable name for {}; nothing was imported.",
                    e.var
                );
                // Nothing was imported: the preview goes back to the names
                // it showed.
                self.held = before;
                self.convert(window, cx);
                Err(why)
            }
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
                tab @ (ImportTab::Postman | ImportTab::OpenApi) => {
                    let src = tab.file_source().expect("a file tab");
                    match self.picked(tab) {
                        _ if self.reading.is_some() => {
                            "Wait until the file is read, then import it.".into()
                        }
                        Some((path, Err(e))) => e.import_error(src, path),
                        _ => src.choose.into(),
                    }
                }
            });
        }
        let root = self
            .target_root()
            .map(Path::to_path_buf)
            .ok_or("Choose a folder for the requests.")?;
        // Again right before writing: the pick may be in the studio's own
        // folder by now.
        let refused = self
            .ws
            .update(cx, |ws, cx| ws.refuse_write_into(&root, window, cx))
            .unwrap_or_else(|_| Some("The window is gone.".into()));
        if let Some(why) = refused {
            return Err(why);
        }
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
                    secret_names: Vec::new(),
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
                    secret_names: Vec::new(),
                    error: error.map(|e| format!("Couldn't copy {e}")),
                    done: Vec::new(),
                })
            }
            tab @ (ImportTab::Postman | ImportTab::OpenApi) => {
                let src = tab.file_source().expect("a file tab");
                let folder = self.target_folder(cx);
                let Some((_, Ok(import))) = self.picked(tab) else {
                    unreachable!("count is 0");
                };
                if let Some(f) = import.files.iter().find(|f| !reads_as_yaml(&f.yaml)) {
                    return Err(format!(
                        "{} wouldn't read back as YAML; nothing was imported.",
                        f.rel_path.display()
                    ));
                }
                // A value never goes over another one in the Keychain: that
                // secret is left for the user to set.
                let mut secrets = Vec::new();
                let mut to_set = Vec::new();
                for v in import
                    .variables
                    .iter()
                    .filter(|v| v.secret && !v.value.is_empty())
                {
                    let taken = (self.env.keychain)(&v.name).is_some_and(|held| held != v.value);
                    if taken || !self.save_secrets {
                        to_set.push(Attention::SetSecret {
                            var: v.name.clone(),
                        });
                    }
                    if !taken {
                        secrets.push(Extracted {
                            var: v.name.clone(),
                            value: v.value.clone(),
                            place: src.place.into(),
                        });
                    }
                }
                // Only a value saved now is in the Keychain; the rest the
                // user sets.
                let saved_now: HashSet<&str> = if self.save_secrets {
                    secrets.iter().map(|e| e.var.as_str()).collect()
                } else {
                    HashSet::new()
                };
                let note = |v: &ImportedVar| {
                    if saved_now.contains(v.name.as_str()) {
                        "the studio keeps it in your Keychain".to_string()
                    } else {
                        "set it in the studio's Environments".to_string()
                    }
                };
                let outcome = match write_import_noting(&root, &folder, import, &note) {
                    Ok(o) => o,
                    Err(e) => {
                        return Ok(Written {
                            root,
                            first: None,
                            secrets: Vec::new(),
                            secret_names: Vec::new(),
                            error: Some(format!("Couldn't import the {}: {e}", src.noun)),
                            done: Vec::new(),
                        });
                    }
                };
                let kept = outcome
                    .kept_env
                    .iter()
                    .map(|var| Attention::KeptEnv { var: var.clone() })
                    .chain(
                        outcome
                            .cleaned_env
                            .iter()
                            .map(|var| Attention::CleanedEnv { var: var.clone() }),
                    );
                let mut attention: Vec<(Attention, Option<PathBuf>)> = import
                    .report
                    .attention
                    .iter()
                    .cloned()
                    .chain(to_set)
                    .chain(kept)
                    .map(|a| {
                        let opens = match &a {
                            Attention::PickFile { request, .. } => import
                                .files
                                .iter()
                                .position(|f| f.request == *request)
                                .and_then(|i| outcome.written.get(i).cloned()),
                            _ => None,
                        };
                        (a, opens)
                    })
                    .collect();
                attention.sort_by_key(|(a, _)| match a {
                    Attention::PickFile { .. } => 0,
                    Attention::SetSecret { .. } => 1,
                    _ => 2,
                });
                // Every secret, set or not, so the studio masks it.
                let secret_names = import
                    .variables
                    .iter()
                    .filter(|v| v.secret)
                    .map(|v| v.name.clone())
                    .collect();
                let saved = if self.save_secrets { secrets.len() } else { 0 };
                let first = outcome.written.first().cloned();
                let (picked, done) = self.file_slots(tab);
                *done = Some(FileImportDone {
                    written: outcome.written.len(),
                    folder,
                    saved,
                    attention,
                });
                *picked = None;
                Ok(Written {
                    root,
                    first,
                    secrets,
                    secret_names,
                    error: None,
                    done: Vec::new(),
                })
            }
        }
    }

    /// Picks the tab's source (`source`: a folder, or a file tab's file)
    /// or the target folder.
    fn pick(&mut self, source: bool, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tab;
        let file = source && tab.file_source().is_some();
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: file,
            directories: !file,
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
                if file {
                    if let Some(path) = dir {
                        this.load_file(tab, path, false, cx);
                    }
                } else if source {
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

    /// A file dropped on a file tab: read, routed by content and shown on
    /// its tab.
    fn drop_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let tab = self.tab;
        self.load_file(tab, path, true, cx);
    }

    /// Development aid (`CURL_RUNNER_STUDIO_FILL`): pastes `text`, with
    /// `folder` set picks it as the Folder of YAML source, and a path to a
    /// `.json`, `.yaml` or `.yml` file fills the Postman or OpenAPI tab, as
    /// a drop would.
    pub(super) fn dev_fill(
        &mut self,
        text: &str,
        folder: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let file = Path::new(text);
        if let Some(folder) = folder {
            self.source = Some(folder);
            self.set_tab(ImportTab::Folder, cx);
        } else if has_extension(file, &["json", "yaml", "yml"]) && file.is_file() {
            // On the UI thread, so a screenshot or test sees it at once.
            let (tab, import) = read_pick(file, ImportTab::Postman, true);
            self.apply_pick(tab, file.to_path_buf(), import, cx);
            self.set_tab(tab, cx);
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

    /// A file tab (Postman, OpenAPI): the file, what it converts to and
    /// what needs attention; after the import, the summary in its place.
    fn render_file_import(&self, src: &'static FileSource, cx: &mut Context<Self>) -> Div {
        if let Some(done) = self.file_done() {
            return self.render_file_done(src, done, cx);
        }
        let picked = self.picked(self.tab);
        let (path, import) = match (&self.reading, picked) {
            (Some((path, _)), _) => (Some(path), None),
            (None, Some((path, Ok(import)))) => (Some(path), Some(import)),
            (None, Some((path, Err(_)))) => (Some(path), None),
            (None, None) => (None, None),
        };
        let line = |tone: &'static str| {
            div()
                .h(px(18.))
                .text_size(px(12.))
                .line_height(px(18.))
                .text_color(color(tone))
        };
        let status = match (&self.reading, picked) {
            (Some((path, _)), _) => {
                line("text-secondary").child(format!("Reading {}…", file_name(path)))
            }
            (None, None) => line("text-tertiary").child(src.empty),
            (None, Some((path, Err(e)))) => note(icons::ERROR, "danger", e.status(path)),
            (None, Some((path, Ok(import)))) => line("text-secondary").child(format!(
                "{}: {} in {}",
                file_name(path),
                plural(import.report.requests, "request"),
                plural(import.report.folders, "folder"),
            )),
        };
        let attention: Vec<&Attention> = import
            .map(|i| {
                let mut items: Vec<&Attention> = i.report.attention.iter().collect();
                items.sort_by_key(|a| !matches!(a, Attention::PickFile { .. }));
                items
            })
            .unwrap_or_default();
        let list = if import.is_none() {
            empty_state_in(ATTENTION_H, "What needs your attention shows here.")
        } else if attention.is_empty() {
            empty_state_in(ATTENTION_H, "Nothing needs your attention.")
        } else {
            div().flex().flex_col().py(px(4.)).children(
                attention
                    .iter()
                    .enumerate()
                    .map(|(ix, a)| attention_row(ix, a)),
            )
        };
        let left = column(true)
            .child(label_text(src.label))
            .child(picker(
                src.pick_id,
                path.map(PathBuf::as_path),
                "No file chosen",
                cx.listener(|this, _, window, cx| this.pick(true, window, cx)),
            ))
            .child(status)
            .child(pane_header(
                "Needs your attention",
                count_label(attention.len(), import.is_some()),
            ))
            .child(pane("import-attention", ATTENTION_H).child(list));
        let folder = self.target_folder(cx);
        let files = match import {
            None => empty_state("The request files show here.".into()),
            Some(i) => div().flex().flex_col().py(px(4.)).children(
                i.files
                    .iter()
                    .map(|f| file_row(&folder.join(&f.rel_path), &f.yaml)),
            ),
        };
        let aside = import.map_or_else(String::new, |i| plural(i.files.len(), "file"));
        let secrets = import.map_or(0, |i| {
            i.variables
                .iter()
                .filter(|v| v.secret && !v.value.is_empty())
                .count()
        });
        let env = env_label(&self.env.environment);
        let save = self.save_secrets;
        let right = column(false)
            .child(pane_header("Preview", aside))
            .child(pane(src.files_id, PANE_H).child(files))
            .child(note(icons::INFO, "accent-blue", src.hint.into()))
            .when(secrets > 0, |d| {
                d.child(
                    div().pl(px(21.)).text_size(px(12.)).child(
                        Checkbox::new("import-save-secrets")
                            .checked(save)
                            .label(format!("Save the secret values to the Keychain for {env}"))
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.save_secrets = *checked;
                                cx.notify();
                            })),
                    ),
                )
            });
        left_right(left, right).on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
            if let Some(path) = paths.paths().iter().find(|p| p.is_file()) {
                this.drop_file(path.clone(), cx);
            }
        }))
    }

    /// After a file import: what was written, and the items left, each
    /// Pick a file one opening its request.
    fn render_file_done(
        &self,
        src: &'static FileSource,
        done: &FileImportDone,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut what = format!("Imported {}", plural(done.written, "request"));
        if !done.folder.as_os_str().is_empty() {
            what.push_str(&format!(" into {}/", done.folder.display()));
        }
        what.push('.');
        let env = env_label(&self.env.environment);
        let left = column(true)
            .child(label_text("Imported"))
            .child(note(icons::CHECK, "success", what))
            .child(note(icons::INFO, "accent-blue", src.env_note.into()))
            .when(done.saved > 0, |d| {
                d.child(note(
                    icons::KEY,
                    "accent-blue",
                    format!(
                        "Saved {} to the Keychain for {env}.",
                        plural(done.saved, "secret value")
                    ),
                ))
            });
        let list = if done.attention.is_empty() {
            empty_state("Nothing needs your attention.".into())
        } else {
            div()
                .flex()
                .flex_col()
                .py(px(4.))
                .children(done.attention.iter().enumerate().map(|(ix, (a, opens))| {
                    let row = attention_row(ix, a);
                    if opens.is_none() {
                        return row;
                    }
                    row.cursor_pointer()
                        .hover(|s| s.bg(color("surface-100")))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_attention(ix, window, cx)
                        }))
                        .child(
                            div()
                                .flex_none()
                                .text_color(color("text-secondary"))
                                .child("Open"),
                        )
                }))
        };
        let right = column(false)
            .child(pane_header(
                "Needs your attention",
                count_label(done.attention.len(), true),
            ))
            .child(pane("import-attention", PANE_H).child(list));
        left_right(left, right)
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
        let done = self.file_done().is_some();
        let buttons: Vec<AnyElement> = if done {
            vec![
                Button::new("import-done")
                    .primary()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(13.))
                    .label("Done")
                    .on_click(|_, window, cx| window.close_dialog(cx))
                    .into_any_element(),
            ]
        } else {
            vec![
                Button::new("import-cancel")
                    .outline()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(13.))
                    .label("Cancel")
                    .on_click(|_, window, cx| window.close_dialog(cx))
                    .into_any_element(),
                Button::new("import-go")
                    .primary()
                    .h(px(32.))
                    .px(px(12.))
                    .text_size(px(13.))
                    .label(self.import_label())
                    .disabled(!enabled)
                    .on_click(cx.listener(|this, _, window, cx| this.import(window, cx)))
                    .into_any_element(),
            ]
        };
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
            .children(buttons)
    }
}

impl Render for ImportDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.tab {
            ImportTab::Curl => self.render_curl(cx),
            ImportTab::Folder => self.render_folder(cx),
            ImportTab::Postman => self.render_file_import(&POSTMAN, cx),
            ImportTab::OpenApi => self.render_file_import(&OPENAPI, cx),
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

/// [`empty_state`] in a pane `h` px tall.
fn empty_state_in(h: f32, text: &'static str) -> Div {
    empty_state(text.into()).h(px(h - 2.))
}

/// A pane's label, with its count or file name on the right.
fn pane_header(label: &'static str, aside: String) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(label_text(label).flex_1())
        .child(
            div()
                .font_family(mono_font())
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .child(aside),
        )
}

/// A bordered pane `h` px tall that scrolls its list.
fn pane(id: &'static str, h: f32) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .flex_none()
        .h(px(h))
        .rounded(px(8.))
        .border_1()
        .border_color(color("border-100"))
        .bg(color("surface-raised"))
        .overflow_y_scroll()
}

/// `n` items, once there is something to count.
fn count_label(n: usize, shown: bool) -> String {
    if shown { n.to_string() } else { String::new() }
}

/// `1 request`, `5 requests`.
fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One item a file import couldn't finish, with its icon: a warning for
/// what the user must do or check, info for what was left out or changed.
fn attention_row(ix: usize, a: &Attention) -> Stateful<Div> {
    let (icon, tone) = match a {
        Attention::PickFile { .. }
        | Attention::ExtraFiles { .. }
        | Attention::SetSecret { .. }
        | Attention::SetVariable { .. }
        | Attention::UnsupportedAuth { .. }
        | Attention::UnresolvedRef { .. }
        | Attention::SkippedBody { .. }
        | Attention::UnsupportedScheme { .. }
        | Attention::ServerVariable { .. }
        | Attention::AccessToken { .. }
        | Attention::BadOperation { .. }
        | Attention::DocumentVars { .. }
        | Attention::ServerCredentials
        | Attention::AuthHeaderParam { .. }
        | Attention::CleanedEnv { .. }
        | Attention::ParamStyle { .. }
        | Attention::UndeclaredPathVars { .. } => (icons::WARNING, "warning"),
        Attention::Renamed { .. }
        | Attention::SkippedVariable { .. }
        | Attention::Script { .. }
        | Attention::UnsupportedBody { .. }
        | Attention::DynamicVar { .. }
        | Attention::Examples { .. }
        | Attention::KeptEnv { .. }
        | Attention::AuthChoice { .. }
        | Attention::OwnServer { .. }
        | Attention::CookieParams { .. }
        | Attention::BodyShapes { .. }
        | Attention::NotConverted { .. }
        | Attention::BadParams { .. } => (icons::INFO, "accent-blue"),
    };
    div()
        .id(("import-attention-item", ix))
        .flex()
        .items_start()
        .gap(px(8.))
        .px(px(12.))
        .py(px(5.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .child(
            div()
                .flex_none()
                .mt(px(2.))
                .child(glyph(icon).size(px(13.)).text_color(color(tone))),
        )
        .child(div().flex_1().min_w_0().child(a.to_string()))
}

/// A request file in a file tab's preview: its method and path.
fn file_row(rel: &Path, yaml: &str) -> Div {
    let method = method_of(yaml);
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(12.))
        .child(
            div()
                .flex_none()
                .w(px(48.))
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
        )
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

/// A file tab's file, read once: the tab it goes to (by its content when
/// `route`, else `tab`) and what it converts to. An unreadable file stays
/// on `tab`.
fn read_pick(path: &Path, tab: ImportTab, route: bool) -> (ImportTab, Result<Import, PickError>) {
    let kind = |tab: ImportTab| tab.file_source().expect("a file tab");
    if !route && !kind(tab).accepts(path) {
        return (tab, Err(PickError::Kind));
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => return (tab, Err(PickError::Failed(e.to_string()))),
    };
    let tab = if route { route_text(&text) } else { tab };
    let src = kind(tab);
    if !src.accepts(path) {
        return (tab, Err(PickError::Kind));
    }
    let import = (src.convert)(&text).map_err(|e| PickError::Failed(e.0));
    (tab, import)
}

/// The tab a dropped file's text goes to: OpenAPI when its top level has
/// `openapi` or `swagger`, Postman otherwise. JSON is read for its top
/// level keys only; YAML isn't parsed, its lines are scanned for the key at
/// column 0.
pub(super) fn route_text(text: &str) -> ImportTab {
    #[derive(serde::Deserialize)]
    struct Top {
        openapi: Option<serde::de::IgnoredAny>,
        swagger: Option<serde::de::IgnoredAny>,
    }
    let text = text.trim_start_matches('\u{feff}');
    let spec = if text.trim_start().starts_with('{') {
        match serde_json::from_str::<Top>(text) {
            Ok(t) => t.openapi.is_some() || t.swagger.is_some(),
            // Not JSON: YAML flow style, `{openapi: 3.0.0, ...}`.
            Err(_) => flow_spec_key(text),
        }
    } else {
        text.lines().any(spec_key)
    };
    if spec {
        ImportTab::OpenApi
    } else {
        ImportTab::Postman
    }
}

/// Whether the first 4 KB, a YAML flow map, has an `openapi` or `swagger`
/// key straight after the `{` or a `,`. Cheap, not a parser: a nested key
/// after a comma would match too.
fn flow_spec_key(text: &str) -> bool {
    let mut end = text.len().min(4096);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let head = &text[..end];
    ["openapi", "swagger"].iter().any(|key| {
        head.match_indices(key).any(|(i, _)| {
            let before = head[..i].trim_end_matches(['"', '\'']).trim_end();
            let after = head[i + key.len()..]
                .trim_start_matches(['"', '\''])
                .trim_start();
            before.ends_with(['{', ',']) && after.starts_with(':')
        })
    })
}

/// A YAML line at column 0 that is the `openapi` or `swagger` key, bare
/// or quoted.
fn spec_key(line: &str) -> bool {
    let (quote, body) = match line.chars().next() {
        Some(q @ ('"' | '\'')) => (Some(q), &line[1..]),
        _ => (None, line),
    };
    ["openapi", "swagger"].iter().any(|key| {
        let Some(rest) = body.strip_prefix(key) else {
            return false;
        };
        let rest = match quote {
            Some(q) => match rest.strip_prefix(q) {
                Some(rest) => rest,
                None => return false,
            },
            None => rest,
        };
        rest.trim_start_matches([' ', '\t'])
            .strip_prefix(':')
            .is_some_and(|v| v.is_empty() || v.starts_with([' ', '\t']))
    })
}

impl Workspace {
    /// The Import dialog (⌘I), adding to the active file's folder.
    pub(super) fn import(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Entity<ImportDialog>> {
        let folder = self.default_folder();
        let ws = cx.entity().downgrade();
        let env = self.import_context(cx);
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
        Some(dialog)
    }

    /// The picked environment's `BASE_URL` and lookups of the values its
    /// variables hold, for the Import dialog. With no collection open, the
    /// context comes once a target folder is picked.
    fn import_context(&self, cx: &App) -> ImportContext {
        let (cached, raw) = (self.secret_cache.clone(), self.secrets.clone());
        match &self.state.collection {
            Some(c) => context_for(self.env_ctx(c, cx), cached, raw, None),
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
                        shared: Default::default(),
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
    /// extracted secrets and `secret_names` are marked secret in the
    /// collection and the extracted ones, when
    /// asked, saved to the Keychain for the picked environment; their
    /// values go nowhere else.
    fn imported(&mut self, imported: Imported, window: &mut Window, cx: &mut Context<Self>) {
        let Imported {
            root,
            opened,
            first,
            secrets,
            secret_names,
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
        if !(secrets.is_empty() && secret_names.is_empty())
            && let Some(c) = &self.state.collection
        {
            let service = self.env_ctx(c, cx).service();
            // The shared notes; the workspace saves them once.
            edit_notes(&self.notes, cx, |meta| {
                for name in secrets.iter().map(|e| &e.var).chain(&secret_names) {
                    meta.set_secret(name, true);
                }
            });
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

/// What a file import (Postman, OpenAPI) wrote and left for the user.
pub(super) struct FileImportDone {
    /// How many request files were written.
    pub written: usize,
    /// The folder they went into, under the target root.
    pub folder: PathBuf,
    /// How many secret values go to the Keychain.
    pub saved: usize,
    /// What is left, Pick a file first, with the file each item opens.
    pub attention: Vec<(Attention, Option<PathBuf>)>,
}

/// What an Import wrote, for the workspace to follow up on.
struct Imported {
    root: PathBuf,
    /// A new collection: open it.
    opened: bool,
    first: Option<PathBuf>,
    /// The secret values taken out of the files.
    secrets: Vec<Extracted>,
    /// More names to mark secret, whose values aren't in `secrets`.
    secret_names: Vec<String>,
    save_secrets: bool,
}

#[cfg(test)]
mod tests {
    use super::super::state::StateStore;
    use super::super::workspace::Workspace;
    use super::ImportTab;
    use curl_runner_core::convert::postman::Attention;
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
                ws.import(window, cx).unwrap()
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
    fn without_a_collection_the_import_starts_one(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (window, ws) = open(cx, None, store.clone());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
        let (stored, marked) = ws.read_with(cx, |ws, cx| {
            let c = ws.state.collection.as_ref().unwrap();
            let service = ws.env_ctx(c, cx).service();
            (
                ws.secrets.get(&service, "API_TOKEN"),
                ws.meta(cx).secret_names.clone(),
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.dev_fill(KEYED, None, window, cx)));
        // Stored after the preview was drawn (the dialog's lookup is stale).
        let (secrets, service) = ws.read_with(cx, |ws, cx| {
            let c = ws.state.collection.as_ref().unwrap();
            (ws.secrets.clone(), ws.env_ctx(c, cx).service())
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
        ws.read_with(cx, |ws, cx| {
            assert_eq!(ws.meta(cx).secret_names, ["AUTH_TOKEN"]);
            let c = ws.state.collection.as_ref().unwrap();
            let service = ws.env_ctx(c, cx).service();
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
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
        // The names held back for the attempt are let go: the preview
        // reads as before.
        dialog.read_with(cx, |d, _| {
            assert!(d.held.is_empty(), "{:?}", d.held.keys());
            assert!(
                d.imports[0]
                    .extracted
                    .iter()
                    .all(|e| !e.var.ends_with("_9"))
            );
        });
    }

    const SAMPLE: &str = include_str!(
        "../../../curl-runner-core/tests/fixtures/postman/sample.postman_collection.json"
    );

    /// `json` in a temp file; the dir keeps it alive.
    fn postman_file(json: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.postman_collection.json");
        std::fs::write(&path, json).unwrap();
        (dir, path)
    }

    /// Every file under `dir`, read as text.
    fn all_text(dir: &Path) -> Vec<(PathBuf, String)> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.extend(all_text(&path));
            } else {
                out.push((
                    path.clone(),
                    std::fs::read_to_string(&path).unwrap_or_default(),
                ));
            }
        }
        out
    }

    #[gpui_kit::test]
    fn postman_import_writes_files_and_env(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = postman_file(SAMPLE);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        dialog.update(cx, |d, cx| {
            d.set_postman(file, cx);
            d.set_tab(ImportTab::Postman, cx);
        });
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.count(), 5);
            assert_eq!(d.import_label(), "Import 5 requests");
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        for rel in [
            "customers/create-customer.yaml",
            "customers/list-customers.yaml",
            "customers/delete-customer.yaml",
            "documents/convert-document.yaml",
            "documents/health.yaml",
        ] {
            assert!(dir.path().join(rel).is_file(), "{rel}");
        }
        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(env.contains("baseUrl="), "{env}");
        for (path, text) in all_text(dir.path()) {
            assert!(!text.contains("test-key-123"), "{}", path.display());
        }
        // The summary stays up with what is left, Pick a file first.
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        dialog.read_with(cx, |d, _| {
            assert!(d.error.is_none(), "{:?}", d.error);
            assert_eq!(d.count(), 0);
            let done = d.postman_done.as_ref().unwrap();
            assert_eq!(done.written, 5);
            let (first, opens) = &done.attention[0];
            assert!(matches!(first, Attention::PickFile { .. }), "{first}");
            assert_eq!(
                opens.as_deref(),
                Some(Path::new("documents/convert-document.yaml"))
            );
        });
        cx.update(|window, cx| window.render_frame(cx));
        // A Pick a file row opens its request.
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.open_attention(0, window, cx)));
        cx.run_until_parked();
        ws.read_with(cx, |ws, _| {
            assert_eq!(
                ws.state.active_document().unwrap().rel_path,
                Path::new("documents/convert-document.yaml")
            );
        });
    }

    /// Imports the sample with the Keychain box `save`, after `before` ran
    /// on the Keychain; returns the dialog's summary lines, what the
    /// Keychain holds for apiKey and PASSWORD, the names marked secret and
    /// the `.env` written.
    fn import_postman(
        cx: &mut TestAppContext,
        save: bool,
        before: impl FnOnce(&dyn crate::model::secrets::SecretStore, &str),
    ) -> (
        Vec<String>,
        Option<String>,
        Option<String>,
        Vec<String>,
        String,
    ) {
        let dir = collection();
        let (_src, file) = postman_file(SAMPLE);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let (secrets, service) = ws.read_with(cx, |ws, cx| {
            let c = ws.state.collection.as_ref().unwrap();
            (ws.secrets.clone(), ws.env_ctx(c, cx).service())
        });
        before(&*secrets, &service);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                // The dev hook: a .json path fills the Postman tab.
                d.dev_fill(&file.to_string_lossy(), None, window, cx);
                d.save_secrets = save;
            })
        });
        dialog.read_with(cx, |d, _| assert_eq!(d.tab, ImportTab::Postman));
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        let lines = dialog.read_with(cx, |d, _| {
            d.postman_done
                .as_ref()
                .unwrap()
                .attention
                .iter()
                .map(|(a, _)| a.to_string())
                .collect()
        });
        let marked = ws.read_with(cx, |ws, cx| ws.meta(cx).secret_names.clone());
        (
            lines,
            secrets.get(&service, "apiKey"),
            secrets.get(&service, "PASSWORD"),
            marked,
            std::fs::read_to_string(dir.path().join(".env")).unwrap(),
        )
    }

    #[gpui_kit::test]
    fn postman_secrets_go_to_the_keychain(cx: &mut TestAppContext) {
        let (lines, api_key, password, marked, env) = import_postman(cx, true, |_, _| {});
        assert_eq!(api_key.as_deref(), Some("test-key-123"));
        assert_eq!(password.as_deref(), Some("secret"));
        assert!(marked.iter().any(|n| n == "apiKey"), "{marked:?}");
        assert!(marked.iter().any(|n| n == "PASSWORD"), "{marked:?}");
        // Saved: nothing left to set.
        assert!(
            !lines.iter().any(|l| l.starts_with("Set the secret")),
            "{lines:?}"
        );
        assert!(
            env.contains("# apiKey is a secret: the studio keeps it in your Keychain"),
            "{env}"
        );
    }

    #[gpui_kit::test]
    fn unchecked_postman_secrets_leave_the_keychain_alone(cx: &mut TestAppContext) {
        let (lines, api_key, password, marked, env) = import_postman(cx, false, |_, _| {});
        assert_eq!(api_key, None);
        assert_eq!(password, None);
        assert!(marked.iter().any(|n| n == "apiKey"), "{marked:?}");
        for var in ["apiKey", "PASSWORD"] {
            let want = format!("Set the secret {var}");
            assert!(lines.contains(&want), "{lines:?}");
        }
        // Not in the Keychain: the note says where to set it.
        assert!(
            env.contains("# apiKey is a secret: set it in the studio's Environments"),
            "{env}"
        );
        assert!(!env.contains("Keychain"), "{env}");
    }

    #[gpui_kit::test]
    fn a_postman_secret_never_overwrites_the_keychain(cx: &mut TestAppContext) {
        let (lines, api_key, password, marked, _) = import_postman(cx, true, |secrets, service| {
            secrets.set(service, "apiKey", "key_already_there").unwrap();
        });
        assert_eq!(api_key.as_deref(), Some("key_already_there"));
        assert_eq!(password.as_deref(), Some("secret"));
        // Left for the user to set, and still marked secret.
        assert!(marked.iter().any(|n| n == "apiKey"), "{marked:?}");
        assert!(
            lines.contains(&"Set the secret apiKey".to_string()),
            "{lines:?}"
        );
        assert!(
            !lines.contains(&"Set the secret PASSWORD".to_string()),
            "{lines:?}"
        );
    }

    #[gpui_kit::test]
    fn a_bad_postman_file_shows_an_error_and_writes_nothing(cx: &mut TestAppContext) {
        let dir = collection();
        let before = all_text(dir.path());
        let (_src, file) = postman_file("{ not a collection");
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        dialog.update(cx, |d, cx| {
            d.set_postman(file, cx);
            d.set_tab(ImportTab::Postman, cx);
        });
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.count(), 0);
            assert!(matches!(&d.postman, Some((_, Err(_)))));
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        dialog.read_with(cx, |d, _| assert!(d.error.is_some()));
        assert_eq!(all_text(dir.path()), before);
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
    }

    const EMPTY_SECRET: &str = r#"{"info":{"name":"Empty secret","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
      "variable":[{"key":"apiToken","value":"","secret":true}],
      "item":[{"name":"Ping","request":{"method":"GET","url":"https://api.example.com/ping",
        "header":[{"key":"X-Token","value":"{{apiToken}}"}]}}]}"#;

    #[gpui_kit::test]
    fn an_empty_postman_secret_is_still_marked_secret(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = postman_file(EMPTY_SECRET);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(&file.to_string_lossy(), None, window, cx);
                d.save_secrets = true;
            })
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        assert!(dir.path().join("ping.yaml").is_file());
        ws.read_with(cx, |ws, cx| {
            let c = ws.state.collection.as_ref().unwrap();
            let service = ws.env_ctx(c, cx).service();
            let marked = &ws.meta(cx).secret_names;
            assert!(marked.iter().any(|n| n == "apiToken"), "{marked:?}");
            assert_eq!(ws.secrets.get(&service, "apiToken"), None);
        });
    }

    #[gpui_kit::test]
    fn import_again_on_the_postman_summary_closes_the_dialog(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = postman_file(SAMPLE);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.dev_fill(&file.to_string_lossy(), None, window, cx)
            })
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        // ⌘↵ on the summary is Done.
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| assert!(d.error.is_none(), "{:?}", d.error));
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn postman_without_a_collection_starts_one(cx: &mut TestAppContext) {
        let state = tempfile::tempdir().unwrap();
        let store = StateStore::at(state.path());
        let (_src, file) = postman_file(SAMPLE);
        let (window, ws) = open(cx, None, store.clone());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        let target = tempfile::tempdir().unwrap();
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                d.set_target(Some(target.path().to_path_buf()), window, cx);
                d.dev_fill(&file.to_string_lossy(), None, window, cx);
            })
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        assert!(
            target
                .path()
                .join("customers/create-customer.yaml")
                .is_file()
        );
        assert!(target.path().join(".env").is_file());
        ws.read_with(cx, |ws, _| {
            assert_eq!(ws.state.collection.as_ref().unwrap().root, target.path());
        });
        assert_eq!(store.recent()[0].path, target.path());
        // The summary stays up.
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        dialog.read_with(cx, |d, _| {
            assert!(d.error.is_none(), "{:?}", d.error);
            assert_eq!(d.postman_done.as_ref().unwrap().written, 5);
        });
    }

    const PETS: &str =
        include_str!("../../../curl-runner-core/tests/fixtures/openapi/small_v30.yaml");

    /// `text` in a temp file named `name`; the dir keeps it alive.
    fn temp_file(name: &str, text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        (dir, path)
    }

    #[gpui_kit::test]
    fn openapi_tab_previews_spec(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = temp_file("pets.yaml", PETS);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        dialog.update(cx, |d, cx| {
            d.set_file_import(ImportTab::OpenApi, file, cx);
            d.set_tab(ImportTab::OpenApi, cx);
        });
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.count(), 6);
            assert_eq!(d.import_label(), "Import 6 requests");
            let Some((_, Ok(import))) = &d.openapi else {
                panic!("{:?}", d.openapi.as_ref().map(|(_, r)| r.as_ref().err()));
            };
            assert_eq!(import.files[0].rel_path, Path::new("pets/list-pets.yaml"));
            // The Postman tab keeps its own pick.
            assert!(d.postman.is_none());
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.find("import-pick-openapi");
        });
    }

    #[gpui_kit::test]
    fn openapi_import_writes_and_summarizes(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = temp_file("pets.yaml", PETS);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        cx.update(|window, cx| {
            dialog.update(cx, |d, cx| {
                // The dev hook: a spec path fills the OpenAPI tab.
                d.dev_fill(&file.to_string_lossy(), None, window, cx)
            })
        });
        dialog.read_with(cx, |d, _| assert_eq!(d.tab, ImportTab::OpenApi));
        cx.update(|window, cx| window.render_frame(cx));
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        for rel in [
            "pets/list-pets.yaml",
            "pets/create-pet.yaml",
            "pets/get-pet.yaml",
            "pets/upload-photo.yaml",
            "store/place-order.yaml",
            "users/get-user.yaml",
        ] {
            assert!(dir.path().join(rel).is_file(), "{rel}");
        }
        let env = std::fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(env.contains("PETS_BASE_URL="), "{env}");
        assert!(env.contains("# PETS_API_KEY="), "{env}");
        // A spec has no secret values: none is in the Keychain yet.
        assert!(
            env.contains("# PETS_API_KEY is a secret: set it in the studio's Environments"),
            "{env}"
        );
        assert!(!env.contains("Keychain"), "{env}");
        // No line sets the secret.
        assert!(
            !env.lines().any(|l| l.starts_with("PETS_API_KEY=")),
            "{env}"
        );
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
        dialog.read_with(cx, |d, _| {
            assert!(d.error.is_none(), "{:?}", d.error);
            assert_eq!(d.count(), 0);
            assert!(d.postman_done.is_none());
            let done = d.openapi_done.as_ref().unwrap();
            assert_eq!(done.written, 6);
            let lines: Vec<String> = done.attention.iter().map(|(a, _)| a.to_string()).collect();
            assert!(
                lines.contains(&"Set the secret PETS_API_KEY".to_string()),
                "{lines:?}"
            );
            // Pick a file first, opening its request.
            let (first, opens) = &done.attention[0];
            assert!(matches!(first, Attention::PickFile { .. }), "{first}");
            assert_eq!(opens.as_deref(), Some(Path::new("pets/upload-photo.yaml")));
        });
        ws.read_with(cx, |ws, cx| {
            let marked = &ws.meta(cx).secret_names;
            assert!(marked.iter().any(|n| n == "PETS_API_KEY"), "{marked:?}");
        });
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.find("import-done");
        });
        // ⌘↵ on the summary is Done.
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    }

    #[test]
    fn drop_routes_by_content() {
        use super::route_text;
        assert_eq!(
            route_text(r#"{"openapi":"3.1.0","info":{}}"#),
            ImportTab::OpenApi
        );
        assert_eq!(route_text(SAMPLE), ImportTab::Postman);
        assert_eq!(
            route_text("swagger: \"2.0\"\ninfo:\n  title: X\n"),
            ImportTab::OpenApi
        );
        assert_eq!(route_text(PETS), ImportTab::OpenApi);
        assert_eq!(route_text("{ not json"), ImportTab::Postman);
        assert_eq!(route_text(""), ImportTab::Postman);
    }

    #[test]
    fn yaml_flow_style_routes_to_openapi() {
        use super::route_text;
        for spec in [
            "{openapi: 3.0.0, info: {title: X}}",
            "{ \"openapi\": 3.0.0, info: {title: X} }",
            "{info: {title: X}, swagger: '2.0'}",
            "{\n  swagger : \"2.0\",\n  info: {}\n}",
        ] {
            assert_eq!(route_text(spec), ImportTab::OpenApi, "{spec:?}");
        }
        for other in ["{info: {title: openapi}}", "{ not json", "{openapis: 1}"] {
            assert_eq!(route_text(other), ImportTab::Postman, "{other:?}");
        }
    }

    #[test]
    fn yaml_routes_by_a_key_at_column_0() {
        use super::route_text;
        for spec in [
            "\"openapi\": 3.0.0\n",
            "'swagger': '2.0'\n",
            "info:\n  title: X\nopenapi : 3.1.0\n",
            "openapi:\n  3.1.0\n",
            "\u{feff}openapi: 3.0.0\n",
        ] {
            assert_eq!(route_text(spec), ImportTab::OpenApi, "{spec:?}");
        }
        for other in [
            "info:\n  openapi: 3.0.0\n",
            " openapi: 3.0.0\n",
            "# openapi: 3.0.0\nitem: []\n",
            "openapis: 1\n",
            "openapi_version: 3\n",
            "\"openapi: 3\n",
            "openapi:x\n",
        ] {
            assert_eq!(route_text(other), ImportTab::Postman, "{other:?}");
        }
    }

    #[test]
    fn an_unreadable_drop_stays_on_its_tab() {
        use super::read_pick;
        let (tab, import) = read_pick(Path::new("/nonexistent/x.yaml"), ImportTab::OpenApi, true);
        assert_eq!(tab, ImportTab::OpenApi);
        assert!(matches!(import, Err(super::PickError::Failed(_))));
        let (_a, binary) = temp_file("bin.json", "");
        std::fs::write(&binary, [0xff, 0xfe, 0x00, 0x81]).unwrap();
        let (tab, import) = read_pick(&binary, ImportTab::OpenApi, true);
        assert_eq!(tab, ImportTab::OpenApi);
        assert!(import.is_err());
    }

    #[gpui_kit::test]
    fn a_finished_drop_does_not_pull_back_from_the_curl_tab(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = temp_file("pets.yaml", PETS);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        dialog.update(cx, |d, cx| {
            d.set_tab(ImportTab::Postman, cx);
            d.drop_file(file, cx);
            d.set_tab(ImportTab::Curl, cx);
        });
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.tab, ImportTab::Curl);
            assert!(d.reading.is_none());
        });
    }

    #[gpui_kit::test]
    fn a_spec_on_the_postman_tab_switches_to_openapi(cx: &mut TestAppContext) {
        let dir = collection();
        let (_src, file) = temp_file("pets.yaml", PETS);
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        dialog.update(cx, |d, cx| {
            d.set_tab(ImportTab::Postman, cx);
            d.drop_file(file, cx);
        });
        // Read off the UI thread: nothing to import meanwhile.
        dialog.read_with(cx, |d, _| {
            assert!(d.reading.is_some());
            assert_eq!(d.tab, ImportTab::Postman);
            assert_eq!(d.count(), 0);
        });
        cx.update(|window, cx| window.render_frame(cx));
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| {
            assert!(d.reading.is_none());
            assert_eq!(d.tab, ImportTab::OpenApi);
            assert_eq!(d.count(), 6);
            assert!(d.postman.is_none());
        });
        // And a collection dropped on the OpenAPI tab goes back to Postman.
        let (_c, collection) = postman_file(SAMPLE);
        dialog.update(cx, |d, cx| d.drop_file(collection, cx));
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.tab, ImportTab::Postman);
            assert_eq!(d.count(), 5);
        });
        // A file that can't be read stays on the tab, with its error.
        dialog.update(cx, |d, cx| {
            d.set_tab(ImportTab::OpenApi, cx);
            d.drop_file(PathBuf::from("/nonexistent/spec.yaml"), cx);
        });
        cx.run_until_parked();
        dialog.read_with(cx, |d, _| {
            assert_eq!(d.tab, ImportTab::OpenApi);
            assert!(matches!(&d.openapi, Some((_, Err(_)))));
        });
    }

    #[gpui_kit::test]
    fn openapi_tab_refuses_other_files_and_bad_specs(cx: &mut TestAppContext) {
        let dir = collection();
        let before = all_text(dir.path());
        let (_a, txt) = temp_file("pets.txt", PETS);
        let (_b, bad) = temp_file("bad.yaml", "openapi: 9.9.9\n");
        let (window, ws) = open(cx, Some(dir.path().to_path_buf()), StateStore::none());
        let cx = &mut VisualTestContext::from_window(window, cx);
        let dialog = cx.update(|window, cx| ws.update(cx, |ws, cx| ws.import(window, cx).unwrap()));
        dialog.update(cx, |d, cx| d.set_tab(ImportTab::OpenApi, cx));
        // Before a pick.
        cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
        dialog.read_with(cx, |d, _| {
            assert_eq!(
                d.error.as_deref(),
                Some("Choose an OpenAPI spec to import.")
            );
        });
        for (file, want) in [
            (txt, "pets.txt isn't a .json, .yaml or .yml file."),
            (bad, "Couldn't import the spec: "),
        ] {
            dialog.update(cx, |d, cx| d.set_file_import(ImportTab::OpenApi, file, cx));
            dialog.read_with(cx, |d, _| {
                assert_eq!(d.count(), 0);
                assert!(matches!(&d.openapi, Some((_, Err(_)))));
            });
            cx.update(|window, cx| window.render_frame(cx));
            cx.update(|window, cx| dialog.update(cx, |d, cx| d.import(window, cx)));
            dialog.read_with(cx, |d, _| {
                let e = d.error.as_deref().unwrap();
                assert!(e.starts_with(want), "{e}");
            });
        }
        assert_eq!(all_text(dir.path()), before);
        cx.update(|window, cx| assert!(window.has_active_dialog(cx)));
    }
}
