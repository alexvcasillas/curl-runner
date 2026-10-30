//! The app's settings (board: Settings), kept in `{base}/settings.json`
//! under the studio's Application Support folder. A missing, unreadable or
//! corrupt file reads as the defaults; out-of-range values are clamped.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE: &str = "settings.json";

/// The editor's font sizes, in points.
pub const FONT_SIZES: std::ops::RangeInclusive<u8> = 10..=20;
/// The indent widths the Editor page offers.
pub const TAB_SIZES: [u8; 3] = [2, 4, 8];

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub editor: EditorSettings,
    pub appearance: AppearanceSettings,
    pub sync: SyncSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// Open the last collection on launch.
    pub reopen_last_collection: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        GeneralSettings {
            reopen_last_collection: true,
        }
    }
}

/// Applied live to every open editor. There is no `format_on_save`: the
/// studio has no YAML formatter yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    pub font_size: u8,
    pub tab_size: u8,
    pub word_wrap: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        EditorSettings {
            font_size: 13,
            tab_size: 2,
            // The Kit's editor wraps by default.
            word_wrap: true,
        }
    }
}

impl EditorSettings {
    /// The editor's line height for its font size: 18px at 13px, as on the
    /// boards.
    pub fn line_height(&self) -> f32 {
        (f32::from(self.font_size) * 18. / 13.).round()
    }
}

/// Dark only for now; Light and System are a later build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: Theme,
}

/// Accounts & sync › Sync. The syncing itself follows them (plan 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncSettings {
    /// Sync after each save.
    pub automatic: bool,
    /// Also on a metered (personal hotspot) connection.
    pub metered: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        SyncSettings {
            automatic: true,
            metered: false,
        }
    }
}

fn path(base: &Path) -> PathBuf {
    base.join(FILE)
}

impl Settings {
    /// The settings saved in `base`, or the defaults.
    pub fn load(base: &Path) -> Settings {
        std::fs::read_to_string(path(base))
            .ok()
            .and_then(|t| serde_json::from_str::<Settings>(&t).ok())
            .unwrap_or_default()
            .clamped()
    }

    /// Writes them to `base`, atomically.
    pub fn save(&self, base: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(base)?;
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        super::write_atomic(&path(base), &text)
    }

    /// Values a hand-edited file may carry, brought in range. Only Dark
    /// can be picked today.
    fn clamped(mut self) -> Settings {
        let e = &mut self.editor;
        e.font_size = e.font_size.clamp(*FONT_SIZES.start(), *FONT_SIZES.end());
        if !TAB_SIZES.contains(&e.tab_size) {
            e.tab_size = EditorSettings::default().tab_size;
        }
        self.appearance.theme = Theme::Dark;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorSettings, GeneralSettings, Settings, Theme};

    #[test]
    fn settings_roundtrip_and_defaults() {
        let d = tempfile::tempdir().unwrap();
        // Nothing saved: the defaults.
        let defaults = Settings::load(d.path());
        assert_eq!(defaults, Settings::default());
        assert_eq!(defaults.editor.font_size, 13);
        assert_eq!(defaults.editor.tab_size, 2);
        assert!(defaults.editor.word_wrap);
        assert!(defaults.general.reopen_last_collection);
        assert_eq!(defaults.appearance.theme, Theme::Dark);
        assert!(defaults.sync.automatic);
        assert!(!defaults.sync.metered);

        let s = Settings {
            general: GeneralSettings {
                reopen_last_collection: false,
            },
            editor: EditorSettings {
                font_size: 15,
                tab_size: 4,
                word_wrap: false,
            },
            ..Settings::default()
        };
        let base = d.path().join("nested");
        s.save(&base).unwrap();
        assert_eq!(Settings::load(&base), s);

        // A file from an older build misses keys: they read as defaults.
        std::fs::write(base.join("settings.json"), r#"{"editor":{"font_size":14}}"#).unwrap();
        let partial = Settings::load(&base);
        assert_eq!(partial.editor.font_size, 14);
        assert_eq!(partial.editor.tab_size, 2);
        assert!(partial.general.reopen_last_collection);

        // Corrupt or out of range.
        std::fs::write(base.join("settings.json"), "{not json").unwrap();
        assert_eq!(Settings::load(&base), Settings::default());
        std::fs::write(
            base.join("settings.json"),
            r#"{"editor":{"font_size":99,"tab_size":3},"appearance":{"theme":"light"}}"#,
        )
        .unwrap();
        let clamped = Settings::load(&base);
        assert_eq!(clamped.editor.font_size, 20);
        assert_eq!(clamped.editor.tab_size, 2);
        assert_eq!(clamped.appearance.theme, Theme::Dark);
    }

    #[test]
    fn line_height_follows_the_font_size() {
        let at = |font_size| {
            EditorSettings {
                font_size,
                ..EditorSettings::default()
            }
            .line_height()
        };
        assert_eq!(at(13), 18.);
        assert_eq!(at(10), 14.);
        assert_eq!(at(20), 28.);
    }
}
