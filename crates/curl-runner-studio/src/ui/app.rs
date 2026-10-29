//! App setup: theme and fonts, the action and keymap table, the window.

use super::state;
use super::workspace::Workspace;
use crate::theme;
use gpui_kit::component::{Theme, ThemeConfig};
use gpui_kit::*;
use std::rc::Rc;
use std::sync::OnceLock;

actions!(
    studio,
    [
        /// Pick a folder and open it as the collection (⌘O).
        OpenFolder,
        /// Save the active file (⌘S).
        SaveFile,
        /// Close the active tab, asking first when it has unsaved changes (⌘W).
        CloseTab,
        /// Open the New request dialog (⌘N). Wired by the dialogs task.
        NewRequest,
        /// Open the New collection dialog (⇧⌘N). Wired by the dialogs task.
        NewCollection,
        /// Open the New dialog on the From curl template (⌘I). Wired by the
        /// dialogs task.
        Import,
    ]
);

/// Every studio shortcut. They apply anywhere in the window, the editor
/// included.
pub fn keymap() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-o", OpenFolder, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-n", NewRequest, None),
        KeyBinding::new("cmd-shift-n", NewCollection, None),
        KeyBinding::new("cmd-i", Import, None),
    ]
}

pub const TITLE_BAR_HEIGHT: f32 = 48.;

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
                    // Centres the traffic lights in the 48px title bar.
                    traffic_light_position: Some(point(px(16.), px(17.))),
                }),
                app_owns_titlebar_drag: true,
                window_min_size: Some(size(px(960.), px(600.))),
                ..Default::default()
            };
            let folder = state::last_folder().filter(|p| p.is_dir());
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Workspace::new(folder, window, cx))
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

/// Theme, fonts and key bindings. Call after `gpui_kit::init`.
pub fn init(cx: &mut App) {
    let installed = cx.text_system().all_font_names();
    let (ui, mono) = font_families(&installed);
    let _ = FONTS.set((ui.into(), mono.into()));
    match serde_json::from_value::<ThemeConfig>(theme_config(ui, mono)) {
        Ok(config) => Theme::update(cx, |t| t.apply_config(&Rc::new(config))),
        Err(e) => log::error!("studio theme does not parse: {e}"),
    }
    cx.bind_keys(keymap());
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

/// Geist and Geist Mono when installed; otherwise the system UI font and
/// Menlo. GPUI panics on a family it cannot find, so never name a missing one.
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
            ["cmd-o", "cmd-s", "cmd-w", "cmd-n", "cmd-shift-n", "cmd-i"]
        );
    }
}
