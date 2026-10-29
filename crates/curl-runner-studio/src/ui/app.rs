//! App setup: theme and fonts, the action and keymap table, the window.

use super::palette;
use super::state;
use super::workspace::{Workspace, quit_app};
use crate::theme;
use gpui_kit::base::actions::{Cancel, Confirm, SelectDown, SelectUp};
use gpui_kit::component::{Theme, ThemeConfig};
use gpui_kit::*;
use std::borrow::Cow;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

actions!(
    studio,
    [
        /// Quit, asking first when files have unsaved changes (⌘Q).
        Quit,
        /// Pick a folder and open it as the collection (⌘O).
        OpenFolder,
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
    ]
);

/// The macOS menu bar. Quit goes through the workspace's unsaved-changes
/// check (see `init`).
pub fn menus() -> Vec<Menu> {
    vec![
        Menu::new("curl-runner studio").items([MenuItem::action("Quit curl-runner studio", Quit)]),
        Menu::new("File").items([
            MenuItem::action("New Request…", NewRequest),
            MenuItem::action("New Collection…", NewCollection),
            MenuItem::action("Import curl…", Import),
            MenuItem::separator(),
            MenuItem::action("Open Folder…", OpenFolder),
            MenuItem::action("Save", SaveFile),
            MenuItem::separator(),
            MenuItem::action("Close Tab", CloseTab),
        ]),
    ]
}

/// Every studio shortcut. They apply anywhere in the window, the editor
/// included.
pub fn keymap() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-o", OpenFolder, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-n", NewRequest, None),
        KeyBinding::new("cmd-shift-n", NewCollection, None),
        KeyBinding::new("cmd-i", Import, None),
        KeyBinding::new("cmd-enter", CreateNew, Some("NewDialog")),
        // Bound after the Kit's `cmd-enter` in `Input` (a newline in a
        // textarea), so ⌘↵ in the dialog's fields creates instead.
        KeyBinding::new("cmd-enter", CreateNew, Some("NewDialog > Input")),
        KeyBinding::new("cmd-k", OpenPalette, None),
        // Bound after the Kit's `cmd-.` (code actions menu), so it wins in
        // the editor; the menu stays on `ctrl-.` and the context menu.
        KeyBinding::new("cmd-.", QuickFix, Some("Input && mode == editor")),
        KeyBinding::new("cmd-shift-m", ToggleProblems, None),
        // The palette's list keys, as the Kit binds them for its `Command`.
        KeyBinding::new("up", SelectUp, Some(palette::CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(palette::CONTEXT)),
        KeyBinding::new(
            "enter",
            Confirm { secondary: false },
            Some(palette::CONTEXT),
        ),
        KeyBinding::new("escape", Cancel, Some(palette::CONTEXT)),
    ]
}

/// Main.png: 44px.
pub const TITLE_BAR_HEIGHT: f32 = 44.;

pub fn run() {
    application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            init(cx);
            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("curl-runner studio".into()),
                    appears_transparent: true,
                    // Centres the traffic lights in the 44px title bar.
                    traffic_light_position: Some(point(px(16.), px(15.))),
                }),
                app_owns_titlebar_drag: true,
                window_min_size: Some(size(px(960.), px(600.))),
                ..Default::default()
            };
            let store = state::StateStore::app_support();
            let folder = dev_env("CURL_RUNNER_STUDIO_FOLDER")
                .or_else(|| store.last_folder())
                .filter(|p| p.is_dir());
            let file = dev_env("CURL_RUNNER_STUDIO_OPEN");
            let show = std::env::var("CURL_RUNNER_STUDIO_SHOW").ok();
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    let mut ws = Workspace::new(folder, store, window, cx);
                    if let Some(file) = file {
                        ws.open_file(&file, window, cx);
                    }
                    if let Some(show) = show {
                        // Dialogs need the window's root, which exists once
                        // this builder returns.
                        cx.defer_in(window, move |ws, window, cx| match show.as_str() {
                            "new" | "duplicate" => {
                                let dialog = ws.new_request(None, window, cx);
                                let fill =
                                    std::env::var("CURL_RUNNER_STUDIO_FILL").unwrap_or_default();
                                let duplicate = show == "duplicate";
                                dialog.update(cx, |d, cx| d.dev_fill(&fill, duplicate, window, cx));
                            }
                            "new-collection" => drop(ws.new_collection(window, cx)),
                            "import" => drop(ws.import(window, cx)),
                            "palette" => {
                                let palette = ws.open_palette(window, cx);
                                if let Ok(query) = std::env::var("CURL_RUNNER_STUDIO_QUERY") {
                                    palette.update(cx, |p, cx| p.set_query(&query, window, cx));
                                }
                                if let Some(pos) = std::env::var("CURL_RUNNER_STUDIO_SELECT")
                                    .ok()
                                    .and_then(|s| s.parse().ok())
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
                            _ => {}
                        });
                    }
                    ws
                })
            })
            .expect("failed to open the studio window");
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}

/// Development aids for looking at a given state without clicking:
/// `CURL_RUNNER_STUDIO_FOLDER` opens that folder instead of the last one
/// (without remembering it), `CURL_RUNNER_STUDIO_OPEN` opens a file by
/// its path relative to the collection and `CURL_RUNNER_STUDIO_SHOW` opens
/// a dialog: `new` or `duplicate` (fields from `CURL_RUNNER_STUDIO_FILL`,
/// `name|METHOD|url`), `new-collection`, `import` or `palette` (with the
/// query in `CURL_RUNNER_STUDIO_QUERY` and the row in
/// `CURL_RUNNER_STUDIO_SELECT`, if set); `problem`
/// selects the open file's first problem and opens its popover (hover
/// states can't be reached without a pointer).
fn dev_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
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
    cx.set_menus(menus());
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
    use super::{ThemeConfig, font_families, hex, keymap, theme_config};

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
                "cmd-o",
                "cmd-s",
                "cmd-w",
                "cmd-n",
                "cmd-shift-n",
                "cmd-i",
                "cmd-enter",
                "cmd-enter",
                "cmd-k",
                "cmd-.",
                "cmd-shift-m",
                "up",
                "down",
                "enter",
                "escape"
            ]
        );
    }
}
