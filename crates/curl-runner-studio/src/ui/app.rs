//! App setup: theme and fonts, the action and keymap table, the window.

use super::collection_settings::Page;
use super::open_shared;
use super::palette;
use super::settings_view::AppPage;
use super::state;
use super::workspace::{Workspace, quit_app};
use crate::model::backend::{Backend, DevControls, DevToggles, MOCK_LATENCY, MockBackend};
use crate::model::run::RunTarget;
use crate::model::secrets::{KeychainSecrets, SecretStore};
use crate::model::settings::Settings;
use crate::theme;
use gpui_kit::base::actions::{Cancel, Confirm, SelectDown, SelectUp};
use gpui_kit::component::{Theme, ThemeConfig};
use gpui_kit::*;
use std::borrow::Cow;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

actions!(
    studio,
    [
        /// Quit, asking first when files have unsaved changes (⌘Q).
        Quit,
        /// Pick a folder and open it as the collection (⌘O).
        OpenFolder,
        /// Open a shared link: paste it, review, add a read-only copy (⌘L).
        OpenSharedLink,
        /// Save the active file (⌘S).
        SaveFile,
        /// Close the active tab, asking first when it has unsaved changes (⌘W).
        CloseTab,
        /// Open the New dialog on a request (⌘N).
        NewRequest,
        /// Open the New dialog on a collection (⇧⌘N).
        NewCollection,
        /// Open the New dialog on the From curl template (⌘I).
        Import,
        /// Create what the New dialog describes (⌘↵ in the dialog).
        CreateNew,
        /// Open the command palette (⌘K).
        OpenPalette,
        /// Apply the first quick fix at the editor's cursor (⌘.).
        QuickFix,
        /// Show or hide the Problems panel (⇧⌘M).
        ToggleProblems,
        /// Save and run the active file (⌘↵).
        RunActive,
        /// Save and run only the request under the cursor (⌥⌘↵).
        RunRequestUnderCursor,
        /// Save and run every file of the collection.
        RunCollection,
        /// Copy the inspector's request as the curl command it ran as,
        /// redacted.
        CopyAsCurl,
        /// Run the inspector's request again.
        Rerun,
        /// Stop the run in flight (⌘. on the run screen).
        Stop,
        /// Leave the run screen for the editor; the run goes on (esc).
        CloseRun,
        /// Show the run history, or leave it when it is shown (⌘Y).
        OpenHistory,
        /// Leave the run history for the editor (esc).
        CloseHistory,
        /// Show the collection's environments and variables.
        OpenEnvironments,
        /// Close the variable drawer, else leave the collection settings
        /// or the app's Settings for the editor (esc).
        CloseSettings,
        /// Show the app's Settings (⌘,).
        OpenSettings,
    ]
);

actions!(
    studio_dev,
    [
        /// Developer › Offline: every backend call fails as offline.
        DevOffline,
        /// Developer › Conflict on next sync.
        DevConflictNextSync,
        /// Developer › Expire share links.
        DevExpireLinks,
        /// Developer › Fail next sign-in.
        DevFailNextSignIn,
        /// Developer › Reset mock: the first-launch state, sign-in screen
        /// included.
        DevResetMock,
    ]
);

/// The macOS menu bar. Quit goes through the workspace's unsaved-changes
/// check (see `init`). With `dev` (debug builds, a backend with developer
/// toggles), the app menu has Developer, checked as `dev` says.
pub fn menus(dev: Option<DevToggles>) -> Vec<Menu> {
    let mut app = vec![
        MenuItem::action("Settings…", OpenSettings),
        MenuItem::separator(),
    ];
    if let Some(t) = dev {
        app.push(MenuItem::submenu(
            Menu::new("Developer").items([
                MenuItem::action("Offline", DevOffline).checked(t.offline),
                MenuItem::action("Conflict on next sync", DevConflictNextSync)
                    .checked(t.conflict_next_sync),
                MenuItem::action("Expire share links", DevExpireLinks).checked(t.expire_links),
                MenuItem::action("Fail next sign-in", DevFailNextSignIn)
                    .checked(t.fail_next_sign_in),
                MenuItem::separator(),
                MenuItem::action("Reset mock", DevResetMock),
            ]),
        ));
        app.push(MenuItem::separator());
    }
    app.push(MenuItem::action("Quit curl-runner studio", Quit));
    vec![
        Menu::new("curl-runner studio").items(app),
        Menu::new("File").items([
            MenuItem::action("New Request…", NewRequest),
            MenuItem::action("New Collection…", NewCollection),
            MenuItem::action("Import curl…", Import),
            MenuItem::separator(),
            MenuItem::action("Open Folder…", OpenFolder),
            MenuItem::action("Open Shared Link…", OpenSharedLink),
            MenuItem::action("Save", SaveFile),
            MenuItem::separator(),
            MenuItem::action("Close Tab", CloseTab),
        ]),
        Menu::new("Run").items([
            MenuItem::action("Run", RunActive),
            MenuItem::action("Run Request Under Cursor", RunRequestUnderCursor),
            MenuItem::action("Run Collection", RunCollection),
            MenuItem::separator(),
            MenuItem::action("Stop", Stop),
            MenuItem::separator(),
            MenuItem::action("History", OpenHistory),
            MenuItem::action("Environments…", OpenEnvironments),
        ]),
    ]
}

/// One studio shortcut: its binding and what the Keyboard shortcuts page
/// calls it. Bindings that differ only by context share a label.
pub struct Shortcut {
    pub binding: KeyBinding,
    pub label: &'static str,
}

fn shortcut<A: Action>(
    keys: &'static str,
    action: A,
    context: Option<&str>,
    label: &'static str,
) -> Shortcut {
    Shortcut {
        binding: KeyBinding::new(keys, action, context),
        label,
    }
}

/// Every studio shortcut, the one list both [`keymap`] and the Keyboard
/// shortcuts page read. They apply anywhere in the window, the editor
/// included.
pub fn bindings() -> Vec<Shortcut> {
    vec![
        shortcut("cmd-q", Quit, None, "Quit"),
        shortcut("cmd-,", OpenSettings, None, "Settings"),
        shortcut("cmd-o", OpenFolder, None, "Open folder"),
        shortcut("cmd-l", OpenSharedLink, None, "Open a shared link"),
        shortcut("cmd-s", SaveFile, None, "Save file"),
        shortcut("cmd-w", CloseTab, None, "Close tab"),
        shortcut("cmd-n", NewRequest, None, "New request"),
        shortcut("cmd-shift-n", NewCollection, None, "New collection"),
        shortcut("cmd-i", Import, None, "Import curl"),
        shortcut(
            "cmd-enter",
            CreateNew,
            Some("NewDialog"),
            "Create, in the New dialog",
        ),
        // Bound after the Kit's `cmd-enter` in `Input` (a newline in a
        // textarea), so ⌘↵ in the dialog's fields creates instead.
        shortcut(
            "cmd-enter",
            CreateNew,
            Some("NewDialog > Input"),
            "Create, in the New dialog",
        ),
        shortcut("cmd-k", OpenPalette, None, "Command palette"),
        // Bound after the Kit's `cmd-.` (code actions menu), so it wins in
        // the editor; the menu stays on `ctrl-.` and the context menu.
        shortcut(
            "cmd-.",
            QuickFix,
            Some("Input && mode == editor"),
            "Quick fix",
        ),
        // ⌘. is Quick Fix in the editor; on the run screen (focused while
        // it is shown, never inside an editor) it stops the run.
        shortcut("cmd-.", Stop, Some("RunView"), "Stop the run"),
        shortcut("escape", CloseRun, Some("RunView"), "Leave the run screen"),
        shortcut("cmd-y", OpenHistory, None, "Run history"),
        shortcut(
            "escape",
            CloseHistory,
            Some("HistoryView"),
            "Leave run history",
        ),
        shortcut(
            "escape",
            CloseSettings,
            Some("CollectionSettings"),
            "Leave settings",
        ),
        shortcut(
            "escape",
            CloseSettings,
            Some("AppSettings"),
            "Leave settings",
        ),
        shortcut("escape", CloseSettings, Some("SignIn"), "Leave sign-in"),
        shortcut("cmd-shift-m", ToggleProblems, None, "Problems panel"),
        shortcut("cmd-enter", RunActive, None, "Run file"),
        shortcut(
            "alt-cmd-enter",
            RunRequestUnderCursor,
            None,
            "Run request under cursor",
        ),
        // Bound after the Kit's editor keys, so they win in the YAML editor.
        shortcut(
            "cmd-enter",
            RunActive,
            Some("Input && mode == editor"),
            "Run file",
        ),
        shortcut(
            "alt-cmd-enter",
            RunRequestUnderCursor,
            Some("Input && mode == editor"),
            "Run request under cursor",
        ),
        // The palette's list keys, as the Kit binds them for its `Command`.
        shortcut("up", SelectUp, Some(palette::CONTEXT), "Palette: previous"),
        shortcut("down", SelectDown, Some(palette::CONTEXT), "Palette: next"),
        shortcut(
            "enter",
            Confirm { secondary: false },
            Some(palette::CONTEXT),
            "Palette: run command",
        ),
        shortcut("escape", Cancel, Some(palette::CONTEXT), "Palette: close"),
    ]
}

/// The bindings [`init`] registers.
pub fn keymap() -> Vec<KeyBinding> {
    bindings().into_iter().map(|s| s.binding).collect()
}

pub const WINDOW_MIN_WIDTH: f32 = 960.;
const WINDOW_MIN_HEIGHT: f32 = 600.;

/// `960x600` (the `CURL_RUNNER_STUDIO_SIZE` dev hook), never below the
/// window's minimum.
fn parse_size(s: &str) -> Option<(f32, f32)> {
    let (w, h) = s.split_once('x')?;
    let (w, h): (f32, f32) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    Some((w.max(WINDOW_MIN_WIDTH), h.max(WINDOW_MIN_HEIGHT)))
}

/// Main.png: 44px.
pub const TITLE_BAR_HEIGHT: f32 = 44.;

pub fn run() {
    // `curl-runner://` links (the bundle registers the scheme): queued
    // until the workspace exists, then opened there.
    let (links, queued) = futures::channel::mpsc::unbounded();
    let app = application().with_assets(gpui_kit::assets::Assets);
    app.on_open_urls(open_shared::link_handler(links));
    app.run(move |cx| {
        gpui_kit::init(cx);
        init(cx);
        let (w, h) = dev_string("CURL_RUNNER_STUDIO_SIZE")
            .and_then(|s| parse_size(&s))
            .unwrap_or((1440., 900.));
        let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
        let background = dev_string("CURL_RUNNER_STUDIO_BACKGROUND").is_some();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("curl-runner studio".into()),
                appears_transparent: true,
                // Centres the traffic lights in the 44px title bar.
                traffic_light_position: Some(point(px(16.), px(15.))),
            }),
            app_owns_titlebar_drag: true,
            // Dev screenshots open behind the other windows, so they
            // never take the keyboard from whoever is typing.
            focus: !background,
            window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
            ..Default::default()
        };
        let store = dev_env("CURL_RUNNER_STUDIO_STATE")
            .map(state::StateStore::at)
            .unwrap_or_else(state::StateStore::app_support);
        let reopen = store
            .base()
            .is_none_or(|b| Settings::load(b).general.reopen_last_collection);
        let folder = dev_env("CURL_RUNNER_STUDIO_FOLDER")
            .or_else(|| store.last_folder().filter(|_| reopen))
            .filter(|p| p.is_dir());
        let file = dev_env("CURL_RUNNER_STUDIO_OPEN");
        let show = dev_string("CURL_RUNNER_STUDIO_SHOW");
        let run = dev_string("CURL_RUNNER_STUDIO_RUN");
        let line = dev_string("CURL_RUNNER_STUDIO_LINE").and_then(|l| l.parse::<u32>().ok());
        let tab = dev_string("CURL_RUNNER_STUDIO_TAB");
        let link = dev_string("CURL_RUNNER_STUDIO_LINK");
        let secrets: Arc<dyn SecretStore> = Arc::new(KeychainSecrets);
        // The only place the mock is named: views see a `Backend`.
        let backend: Arc<dyn Backend> = Arc::new(MockBackend::open(store.base(), MOCK_LATENCY));
        let dev_store = store.clone();
        let ws_backend = backend.clone();
        let (window_handle, workspace) = gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| {
                let mut ws = Workspace::new(folder, store, secrets, ws_backend, window, cx);
                // Before Welcome, when the screen was never answered;
                // a dev `SHOW` asks for something else.
                if show.is_none() {
                    ws.first_launch(window, cx);
                }
                if let Some(file) = file {
                    ws.open_file(&file, window, cx);
                    // As a click on its row does.
                    ws.expanded.insert(file);
                }
                if let Some(line) = line {
                    ws.dev_move_cursor(line, window, cx);
                }
                if let Some(tab) = tab {
                    ws.dev_show_tab(&tab);
                }
                if dev_string("CURL_RUNNER_STUDIO_BODY").as_deref() == Some("raw") {
                    ws.body_raw = true;
                }
                if let Some(link) = link {
                    // The credentials step, filled in, with
                    // `CURL_RUNNER_STUDIO_FILL` (`NAME=value,…`).
                    let fill = dev_string("CURL_RUNNER_STUDIO_FILL");
                    cx.defer_in(window, move |ws, window, cx| {
                        let view = ws.open_shared(None, window, cx);
                        view.update(cx, |v, cx| {
                            if let Some(fill) = &fill {
                                v.dev_credentials(fill);
                            }
                            v.receive(&link, window, cx);
                        });
                    });
                }
                if let Some(run) = run {
                    cx.defer_in(window, move |ws, window, cx| {
                        if run == "collection" {
                            ws.run(RunTarget::Collection, window, cx);
                        } else {
                            ws.run_active(window, cx);
                        }
                    });
                }
                if let Some(show) = show {
                    // Dialogs need the window's root, which exists once
                    // this builder returns.
                    cx.defer_in(window, move |ws, window, cx| match show.as_str() {
                        "new" | "duplicate" => {
                            let fill = dev_string("CURL_RUNNER_STUDIO_FILL").unwrap_or_default();
                            let duplicate = show == "duplicate";
                            if let Some(dialog) = ws.new_request(None, window, cx) {
                                dialog.update(cx, |d, cx| d.dev_fill(&fill, duplicate, window, cx));
                            }
                        }
                        "new-collection" => drop(ws.new_collection(window, cx)),
                        "import" | "import-folder" => {
                            let fill = dev_string("CURL_RUNNER_STUDIO_FILL").unwrap_or_default();
                            let folder = (show == "import-folder").then(|| PathBuf::from(&fill));
                            if let Some(dialog) = ws.import(window, cx) {
                                dialog.update(cx, |d, cx| d.dev_fill(&fill, folder, window, cx));
                            }
                        }
                        "palette" => {
                            let palette = ws.open_palette(window, cx);
                            if let Some(query) = dev_string("CURL_RUNNER_STUDIO_QUERY") {
                                palette.update(cx, |p, cx| p.set_query(&query, window, cx));
                            }
                            if let Some(pos) =
                                dev_string("CURL_RUNNER_STUDIO_SELECT").and_then(|s| s.parse().ok())
                            {
                                // Once the list is laid out, so it can scroll.
                                cx.spawn(async move |_, cx| {
                                    cx.background_executor()
                                        .timer(std::time::Duration::from_millis(500))
                                        .await;
                                    palette.update(cx, |p, cx| p.select(pos, cx));
                                })
                                .detach();
                            }
                        }
                        "problem" => ws.dev_show_first_problem(window, cx),
                        "signin" => {
                            let view = ws.open_sign_in(None, window, cx);
                            let url = dev_string("CURL_RUNNER_STUDIO_FILL");
                            let submit = dev_string("CURL_RUNNER_STUDIO_SUBMIT").is_some();
                            view.update(cx, |v, cx| {
                                v.dev_fill(url.as_deref(), window, cx);
                                if submit {
                                    v.dev_submit(cx);
                                }
                            });
                        }
                        "history" => ws.open_history(window, cx),
                        "settings" => {
                            let page = dev_string("CURL_RUNNER_STUDIO_PAGE")
                                .and_then(|p| AppPage::parse(&p))
                                .unwrap_or(AppPage::General);
                            ws.open_app_settings(page, window, cx);
                        }
                        "environments" => {
                            let env = dev_string("CURL_RUNNER_STUDIO_ENV");
                            let var = dev_string("CURL_RUNNER_STUDIO_VAR");
                            // After a `CURL_RUNNER_STUDIO_RUN`, so its
                            // captured values are listed.
                            cx.spawn_in(window, async move |ws, cx| {
                                for _ in 0..150 {
                                    let running = ws
                                        .update(cx, |ws, _| ws.runs.active.is_some())
                                        .unwrap_or(false);
                                    if !running {
                                        break;
                                    }
                                    cx.background_executor()
                                        .timer(std::time::Duration::from_millis(100))
                                        .await;
                                }
                                ws.update_in(cx, |ws, window, cx| {
                                    ws.open_settings(window, cx);
                                    ws.settings_on(env.as_deref(), var.as_deref(), window, cx);
                                })
                                .ok();
                            })
                            .detach();
                        }
                        "share" => {
                            ws.open_share(window, cx);
                            let fill = dev_string("CURL_RUNNER_STUDIO_FILL");
                            if let (Some(view), Some(fill)) = (ws.share_view(), fill) {
                                let emails = fill.split(',').map(String::from).collect();
                                view.update(cx, |v, cx| v.dev_invite(emails, cx));
                            }
                        }
                        "sharing" => {
                            ws.open_settings(window, cx);
                            if let Some(view) = &ws.settings_view {
                                view.update(cx, |v, cx| v.set_page(Page::Sharing, window, cx));
                            }
                        }
                        "export" => {
                            if let Some(view) = &ws.run_view {
                                view.update(cx, |v, cx| v.open_export_menu(cx));
                            }
                        }
                        _ => {}
                    });
                }
                ws
            })
        })
        .expect("failed to open the studio window");
        open_shared::pump_links(queued, window_handle, workspace.downgrade(), cx);
        if cfg!(debug_assertions) {
            install_dev_menu(backend, dev_store, window_handle, workspace.downgrade(), cx);
        }
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        if !background {
            cx.activate(true);
        }
    });
}

/// App menu › Developer: the backend's toggles (when it has them, the
/// mock), through [`Backend::dev`] only. Debug builds.
fn install_dev_menu(
    backend: Arc<dyn Backend>,
    store: state::StateStore,
    window: AnyWindowHandle,
    workspace: WeakEntity<Workspace>,
    cx: &mut App,
) {
    if backend.dev().is_none() {
        return;
    }
    let (b, ws) = (backend.clone(), workspace.clone());
    cx.on_action(move |_: &DevOffline, cx| {
        toggle_dev(&b, |d| d.set_offline(!d.offline()), cx);
        // A sync finds out at once: it fails offline, or flushes the
        // waiting changes once back.
        let ws = ws.clone();
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    ws.update(cx, |ws, cx| ws.sync_now(window, cx)).ok();
                })
                .ok();
        });
    });
    let b = backend.clone();
    cx.on_action(move |_: &DevConflictNextSync, cx| {
        toggle_dev(
            &b,
            |d| d.conflict_next_sync(!d.toggles().conflict_next_sync),
            cx,
        )
    });
    let b = backend.clone();
    cx.on_action(move |_: &DevExpireLinks, cx| {
        toggle_dev(&b, |d| d.expire_links(!d.toggles().expire_links), cx)
    });
    let b = backend.clone();
    cx.on_action(move |_: &DevFailNextSignIn, cx| {
        toggle_dev(
            &b,
            |d| d.fail_next_sign_in(!d.toggles().fail_next_sign_in),
            cx,
        )
    });
    let b = backend.clone();
    cx.on_action(move |_: &DevResetMock, cx| {
        if let Some(d) = b.dev() {
            d.reset();
        }
        store.set_connection(None);
        refresh_dev(&b, cx);
        // Dispatch has the window checked out: show the screen after.
        let workspace = workspace.clone();
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    workspace
                        .update(cx, |ws, cx| drop(ws.open_sign_in(None, window, cx)))
                        .ok();
                })
                .ok();
        });
    });
    refresh_dev(&backend, cx);
}

/// The menu's checkmarks and the windows, after a toggle changed.
fn refresh_dev(backend: &Arc<dyn Backend>, cx: &mut App) {
    cx.set_menus(menus(backend.dev().map(|d| d.toggles())));
    cx.refresh_windows();
}

fn toggle_dev(backend: &Arc<dyn Backend>, flip: fn(&dyn DevControls), cx: &mut App) {
    if let Some(d) = backend.dev() {
        flip(d);
    }
    refresh_dev(backend, cx);
}

/// Development aids for looking at a given state without clicking:
/// `CURL_RUNNER_STUDIO_FOLDER` opens that folder instead of the last one
/// (without remembering it), `CURL_RUNNER_STUDIO_OPEN` opens a file by
/// its path relative to the collection and `CURL_RUNNER_STUDIO_SHOW` opens
/// a dialog: `new` or `duplicate` (fields from `CURL_RUNNER_STUDIO_FILL`,
/// `name|METHOD|url`), `new-collection`, `import` (the curl text in
/// `CURL_RUNNER_STUDIO_FILL`, `\n` for newlines), `import-folder` (the
/// Folder of YAML tab on the folder in `CURL_RUNNER_STUDIO_FILL`) or
/// `palette` (with the
/// query in `CURL_RUNNER_STUDIO_QUERY` and the row in
/// `CURL_RUNNER_STUDIO_SELECT`, if set); `problem`
/// selects the open file's first problem and opens its popover and `export`
/// opens the run screen's Export menu (with
/// `CURL_RUNNER_STUDIO_RUN=collection`), `history` the History screen and
/// `environments` the collection's Environments (on the tab
/// `CURL_RUNNER_STUDIO_ENV` names, with `CURL_RUNNER_STUDIO_VAR` naming the
/// variable whose drawer opens) and `settings` the app's Settings (on the
/// page `CURL_RUNNER_STUDIO_PAGE` names: `general`, `editor`,
/// `appearance`, `accounts`, `shortcuts` or `updates`), `share` the Share
/// dialog (inviting the comma-separated addresses in
/// `CURL_RUNNER_STUDIO_FILL`), `sharing` the collection's Sharing page
/// and `signin` the
/// sign-in screen (with a server URL in `CURL_RUNNER_STUDIO_FILL`: the
/// Self-hosted card on it, tested; `CURL_RUNNER_STUDIO_SUBMIT=1` then
/// presses its primary button); hover and click states can't be
/// reached without a pointer. `CURL_RUNNER_STUDIO_LINE`
/// puts the cursor on a zero-based line, `CURL_RUNNER_STUDIO_TAB` picks the
/// inspector tab, `CURL_RUNNER_STUDIO_BODY=raw` shows the Checks tab's body
/// raw instead of diffed, `CURL_RUNNER_STUDIO_RUN` runs the open file once
/// (`collection`: the whole collection, on the run screen),
/// `CURL_RUNNER_STUDIO_SIZE` (`960x600`) sizes the window and
/// `CURL_RUNNER_STUDIO_STATE` keeps the studio's state (last folder, Recent,
/// settings, history) in that folder instead of Application Support, and
/// `CURL_RUNNER_STUDIO_BACKGROUND` opens the window without focusing it or
/// activating the app (for screenshots). `CURL_RUNNER_STUDIO_LINK` opens a
/// shared link as a `curl-runner://` event would (with
/// `CURL_RUNNER_STUDIO_FILL`, `NAME=value,…`: on the credentials step,
/// filled in).
///
/// Debug builds only: a release build ignores them all.
fn dev_env(name: &str) -> Option<PathBuf> {
    dev_value(std::env::var_os(name), DEV_HOOKS).map(PathBuf::from)
}

fn dev_string(name: &str) -> Option<String> {
    dev_env(name).map(|p| p.to_string_lossy().into_owned())
}

/// The dev variables are read in debug builds only.
const DEV_HOOKS: bool = cfg!(debug_assertions);

/// A dev variable's value when hooks are `enabled`; unset or empty means
/// "not set".
fn dev_value(v: Option<std::ffi::OsString>, enabled: bool) -> Option<std::ffi::OsString> {
    v.filter(|v| enabled && !v.is_empty())
}

/// Geist and Geist Mono (SIL OFL 1.1, `assets/fonts/OFL.txt`), one file
/// per weight and style the studio draws.
const EMBEDDED_FONTS: &[&[u8]] = &[
    include_bytes!("../../assets/fonts/Geist-Regular.ttf"),
    include_bytes!("../../assets/fonts/Geist-Medium.ttf"),
    include_bytes!("../../assets/fonts/Geist-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/Geist-Italic.ttf"),
    include_bytes!("../../assets/fonts/GeistMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/GeistMono-Medium.ttf"),
    include_bytes!("../../assets/fonts/GeistMono-SemiBold.ttf"),
];

/// Registers the embedded fonts. On failure the studio falls back to the
/// system families (see `font_families`).
fn load_fonts(cx: &App) {
    let fonts = EMBEDDED_FONTS.iter().map(|&b| Cow::Borrowed(b)).collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        log::error!("embedded fonts did not load: {e}");
    }
}

/// Theme, fonts and key bindings. Call after `gpui_kit::init`.
pub fn init(cx: &mut App) {
    load_fonts(cx);
    let installed = cx.text_system().all_font_names();
    let (ui, mono) = font_families(&installed);
    let _ = FONTS.set((ui.into(), mono.into()));
    match serde_json::from_value::<ThemeConfig>(theme_config(ui, mono)) {
        Ok(config) => Theme::update(cx, |t| t.apply_config(&Rc::new(config))),
        Err(e) => log::error!("studio theme does not parse: {e}"),
    }
    cx.bind_keys(keymap());
    cx.set_menus(menus(None));
    // Every Quit lands here (the workspace has no handler of its own). It
    // runs during dispatch, while the window is checked out, so defer.
    cx.on_action(|_: &Quit, cx| cx.defer(quit_app));
}

static FONTS: OnceLock<(SharedString, SharedString)> = OnceLock::new();

/// The interface font family in use.
pub fn ui_font() -> SharedString {
    FONTS.get().map_or(FALLBACK_UI.into(), |f| f.0.clone())
}

/// The code font family in use.
pub fn mono_font() -> SharedString {
    FONTS.get().map_or(FALLBACK_MONO.into(), |f| f.1.clone())
}

const FALLBACK_UI: &str = ".SystemUIFont";
const FALLBACK_MONO: &str = "Menlo";

/// Geist and Geist Mono (embedded, so present unless loading them failed);
/// otherwise the system UI font and Menlo. GPUI panics on a family it
/// cannot find, so never name a missing one.
fn font_families(installed: &[String]) -> (&'static str, &'static str) {
    let has = |family: &str| installed.iter().any(|f| f == family);
    (
        if has("Geist") { "Geist" } else { FALLBACK_UI },
        if has("Geist Mono") {
            "Geist Mono"
        } else {
            FALLBACK_MONO
        },
    )
}

/// `#rrggbbaa` for a dark token.
fn hex(name: &str) -> String {
    let c = theme::color(name);
    let byte = |v: f32| (v * 255.).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        byte(c.r),
        byte(c.g),
        byte(c.b),
        byte(c.a)
    )
}

/// Kit theme color key → studio token, as the design system README maps them.
const KIT_COLORS: &[(&str, &str)] = &[
    ("background", "bg-100"),
    ("foreground", "text-primary"),
    ("border", "border-200"),
    ("input.border", "border-200"),
    ("ring", "accent-blue"),
    ("caret", "text-primary"),
    ("selection.background", "selection"),
    ("muted.background", "surface-300"),
    ("muted.foreground", "text-secondary"),
    ("accent.background", "surface-300"),
    ("accent.foreground", "text-primary"),
    ("popover.background", "surface-raised"),
    ("popover.foreground", "text-primary"),
    ("primary.background", "primary"),
    ("primary.hover.background", "primary-hover"),
    ("primary.active.background", "primary-hover"),
    ("primary.foreground", "on-primary"),
    ("secondary.background", "bg-100"),
    ("secondary.hover.background", "surface-100"),
    ("secondary.active.background", "surface-200"),
    ("secondary.foreground", "text-primary"),
    ("link.foreground", "accent-blue"),
    ("link.hover.foreground", "accent-blue"),
    ("link.active.foreground", "accent-blue"),
    ("list.background", "bg-100"),
    ("list.hover.background", "surface-100"),
    ("list.active.background", "surface-200"),
    ("list.active.border", "surface-200"),
    ("danger.background", "danger"),
    ("danger.foreground", "danger"),
    ("warning.background", "warning"),
    ("warning.foreground", "warning"),
    ("success.background", "success"),
    ("success.foreground", "success"),
    ("info.background", "accent-blue"),
    ("info.foreground", "accent-blue"),
    ("title_bar.background", "bg-100"),
    ("title_bar.border", "border-100"),
    ("status_bar.background", "bg-100"),
    ("status_bar.border", "border-100"),
    ("sidebar.background", "bg-100"),
    ("sidebar.border", "border-100"),
    ("tab_bar.background", "bg-100"),
    ("tab.background", "bg-100"),
    ("tab.active.background", "bg-100"),
    ("tab.active.foreground", "text-primary"),
    ("tab.foreground", "text-secondary"),
    ("overlay", "overlay"),
    ("window.border", "border-100"),
    ("scrollbar.thumb.background", "border-200"),
    ("scrollbar.thumb.hover.background", "text-disabled"),
];

/// Kit editor colors → studio token.
const KIT_EDITOR: &[(&str, &str)] = &[
    ("editor.background", "bg-100"),
    ("editor.gutter.background", "bg-100"),
    ("editor.foreground", "text-primary"),
    ("editor.active_line.background", "surface-100"),
    ("editor.line_number", "text-gutter"),
    ("editor.active_line_number", "text-primary"),
    ("editor.invisible", "indent-guide"),
    ("error", "danger"),
    ("warning", "warning"),
    ("info", "accent-blue"),
];

/// Tree-sitter capture → studio syntax token.
const KIT_SYNTAX: &[(&str, &str)] = &[
    ("property", "syntax-key"),
    ("attribute", "syntax-key"),
    ("string", "syntax-string"),
    ("string.special", "syntax-string"),
    ("number", "syntax-number"),
    ("boolean", "syntax-number"),
    ("constant", "syntax-number"),
    ("comment", "syntax-comment"),
    ("comment.doc", "syntax-comment"),
    ("variable", "syntax-variable"),
    ("variable.special", "syntax-variable"),
    ("type", "syntax-variable"),
    ("punctuation", "syntax-punctuation"),
    ("punctuation.delimiter", "syntax-punctuation"),
    ("punctuation.bracket", "syntax-punctuation"),
    ("punctuation.special", "syntax-punctuation"),
    ("keyword", "syntax-punctuation"),
];

/// The GPUI-kit theme file for the studio: the dark tokens on the Kit's
/// colors, editor and syntax highlighting.
fn theme_config(ui_font: &str, mono_font: &str) -> serde_json::Value {
    use serde_json::{Map, Value, json};
    let map = |pairs: &[(&str, &str)], value: &dyn Fn(&str) -> Value| -> Map<String, Value> {
        pairs
            .iter()
            .map(|&(key, token)| (key.to_string(), value(token)))
            .collect()
    };
    let mut highlight = map(KIT_EDITOR, &|t| hex(t).into());
    highlight.insert(
        "syntax".into(),
        map(KIT_SYNTAX, &|t| json!({ "color": hex(t) })).into(),
    );
    json!({
        "name": "curl-runner studio",
        "mode": "dark",
        "font.family": ui_font,
        "font.size": 13.0,
        "mono_font.family": mono_font,
        "mono_font.size": 13.0,
        "radius": 6,
        "radius.lg": 8,
        "shadow": false,
        "colors": map(KIT_COLORS, &|t| hex(t).into()),
        "highlight": highlight,
    })
}

#[cfg(test)]
mod tests {
    use super::{DEV_HOOKS, ThemeConfig, dev_value, font_families, hex, keymap, theme_config};

    #[test]
    fn empty_dev_vars_count_as_unset() {
        assert_eq!(dev_value(Some("".into()), true), None);
        assert_eq!(dev_value(None, true), None);
        assert_eq!(
            dev_value(Some("a.yaml".into()), true),
            Some("a.yaml".into())
        );
    }

    #[test]
    fn dev_size_parses_and_respects_the_minimum() {
        assert_eq!(super::parse_size("960x600"), Some((960., 600.)));
        assert_eq!(super::parse_size("800x400"), Some((960., 600.)));
        assert_eq!(super::parse_size("wide"), None);
    }

    #[test]
    fn three_columns_fit_the_smallest_window() {
        let columns = super::super::sidebar::SIDEBAR_WIDTH
            + super::super::editor_pane::EDITOR_MIN_WIDTH
            + super::super::inspector::INSPECTOR_WIDTH;
        assert!(columns <= super::WINDOW_MIN_WIDTH, "{columns}");
    }

    #[test]
    fn dev_hooks_ignored_in_release() {
        assert_eq!(DEV_HOOKS, cfg!(debug_assertions));
        assert_eq!(dev_value(Some("a.yaml".into()), false), None);
        let got = dev_value(Some("a.yaml".into()), DEV_HOOKS);
        if cfg!(debug_assertions) {
            assert_eq!(got, Some("a.yaml".into()));
        } else {
            assert_eq!(got, None);
        }
    }

    #[test]
    fn fonts_fall_back_when_geist_is_missing() {
        let installed = ["Helvetica".to_string(), "Menlo".to_string()];
        assert_eq!(font_families(&installed), (".SystemUIFont", "Menlo"));
    }

    #[test]
    fn fonts_prefer_geist_when_installed() {
        let installed = ["Geist".to_string(), "Geist Mono".to_string()];
        assert_eq!(font_families(&installed), ("Geist", "Geist Mono"));
    }

    #[test]
    fn embedded_fonts_are_truetype_with_their_license() {
        assert_eq!(super::EMBEDDED_FONTS.len(), 7);
        for font in super::EMBEDDED_FONTS {
            assert_eq!(&font[..4], &[0, 1, 0, 0], "not a TrueType file");
        }
        let license = include_str!("../../assets/fonts/OFL.txt");
        assert!(license.contains("SIL Open Font License, Version 1.1"));
    }

    #[test]
    fn theme_config_maps_tokens() {
        let config: ThemeConfig =
            serde_json::from_value(theme_config("Geist", "Geist Mono")).unwrap();
        assert!(config.mode.is_dark());
        let highlight = config.highlight.expect("highlight section");
        assert_eq!(
            highlight.editor_background,
            Some(super::super::color("bg-100"))
        );
        assert_eq!(
            highlight.editor_line_number,
            Some(super::super::color("text-gutter"))
        );
    }

    #[test]
    fn hex_keeps_alpha() {
        assert_eq!(hex("bg-100"), "#0a0a0aff");
        assert_eq!(hex("overlay"), "#000000b3");
    }

    #[test]
    fn keymap_has_the_studio_shortcuts() {
        let keys: Vec<String> = keymap()
            .iter()
            .map(|b| {
                b.keystrokes()
                    .iter()
                    .map(|k| k.unparse(gpui_kit::OperatingSystem::Mac))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert_eq!(
            keys,
            [
                "cmd-q",
                "cmd-,",
                "cmd-o",
                "cmd-l",
                "cmd-s",
                "cmd-w",
                "cmd-n",
                "cmd-shift-n",
                "cmd-i",
                "cmd-enter",
                "cmd-enter",
                "cmd-k",
                "cmd-.",
                "cmd-.",
                "escape",
                "cmd-y",
                "escape",
                "escape",
                "escape",
                "escape",
                "cmd-shift-m",
                "cmd-enter",
                "alt-cmd-enter",
                "cmd-enter",
                "alt-cmd-enter",
                "up",
                "down",
                "enter",
                "escape"
            ]
        );
    }
}
