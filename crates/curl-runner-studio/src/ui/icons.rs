//! The mockups' 16px stroke glyphs, drawn from inline SVG so they match
//! the design exactly without growing the Kit's asset bundle.

use gpui_kit::component::Icon;

macro_rules! glyph {
    ($stroke:literal, $body:literal) => {
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#000" stroke-linecap="round" stroke-linejoin="round" stroke-width=""##,
            $stroke,
            r##"">"##,
            $body,
            "</svg>"
        )
    };
}

pub const CHEVRON_DOWN: &str = glyph!("1.5", r#"<path d="M5 6l3 3 3-3"/>"#);
pub const CHEVRON_RIGHT: &str = glyph!("1.5", r#"<path d="M6 5l3 3-3 3"/>"#);
pub const FOLDER: &str = glyph!("1.3", r#"<path d="M2 4.5h4l1.5 1.5H14v6.5H2z"/>"#);
pub const FILTER: &str = glyph!("1.5", r#"<path d="M2 4h12M4 8h8M6 12h4"/>"#);
pub const SEARCH: &str = glyph!(
    "1.5",
    r#"<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/>"#
);
pub const PLUS: &str = glyph!("1.5", r#"<path d="M8 3v10M3 8h10"/>"#);
pub const ERROR: &str = glyph!(
    "1.75",
    r#"<circle cx="8" cy="8" r="6"/><path d="M8 5v3.5M8 10.75v.5"/>"#
);
pub const WARNING: &str = glyph!(
    "1.75",
    r#"<path d="M8 2l6.5 11.5h-13z"/><path d="M8 6.5v3M8 11.5v.5"/>"#
);
pub const INFO: &str = glyph!(
    "1.75",
    r#"<circle cx="8" cy="8" r="6"/><path d="M8 7.5V11M8 5.25v.5"/>"#
);
pub const FOLDER_OPEN: &str = glyph!(
    "1.3",
    r#"<path d="M2 12.5v-8h4l1.5 1.5H13v1.5"/><path d="M2 12.5l1.8-5h10.7l-1.8 5z"/>"#
);
pub const FOLDER_PLUS: &str = glyph!(
    "1.3",
    r#"<path d="M2 4.5h4l1.5 1.5H14v6.5H2z"/><path d="M8 7.5v3.5M6.25 9.25h3.5"/>"#
);
pub const FILE_PLUS: &str = glyph!(
    "1.4",
    r#"<path d="M9 2H3.5v12h9V5.5z"/><path d="M9 2v3.5h3.5"/><path d="M8 7.5v4M6 9.5h4"/>"#
);
pub const IMPORT: &str = glyph!(
    "1.5",
    r#"<path d="M8 2.5V10M5 7l3 3 3-3"/><path d="M3 9v4h10V9"/>"#
);
pub const PANEL_BOTTOM: &str = glyph!(
    "1.4",
    r#"<rect x="2" y="2.5" width="12" height="11" rx="1.5"/><path d="M2 9.5h12"/>"#
);
pub const CHECK: &str = glyph!("1.7", r#"<path d="M3 8.5l3 3 7-7"/>"#);
pub const CROSS: &str = glyph!("1.7", r#"<path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/>"#);
pub const COPY: &str = glyph!(
    "1.4",
    r#"<rect x="5" y="5" width="8.5" height="8.5" rx="1.5"/><path d="M3 10.5V3.5A1 1 0 014 2.5h7"/>"#
);
pub const RERUN: &str = glyph!(
    "1.5",
    r#"<path d="M13 8a5 5 0 11-1.5-3.5"/><path d="M13 2.5v3h-3"/>"#
);
pub const DOWNLOAD: &str = glyph!(
    "1.5",
    r#"<path d="M8 2.5v8M4.5 7L8 10.5 11.5 7M3 13.5h10"/>"#
);
/// Slower than the previous run (History).
pub const TREND: &str = glyph!("1.6", r#"<path d="M3 11l4-4 3 3 3-5"/>"#);
/// A clock with a back arrow: the run history.
pub const HISTORY: &str = glyph!(
    "1.4",
    r#"<path d="M2.5 8a5.5 5.5 0 101.6-3.9"/><path d="M2.5 2.5v2.5H5"/><path d="M8 5.5V8l1.75 1.75"/>"#
);
/// The board's dashed ring for a queued request.
pub const QUEUED: &str = glyph!(
    "1.5",
    r#"<circle cx="8" cy="8" r="5.5" stroke-dasharray="2 2"/>"#
);
/// A request that did not run.
pub const SKIPPED: &str = glyph!(
    "1.5",
    r#"<circle cx="8" cy="8" r="5.5"/><path d="M5.5 8h5"/>"#
);
pub const CHEVRON_LEFT: &str = glyph!("1.5", r#"<path d="M10 4L6 8l4 4"/>"#);
/// A key: a Keychain value.
pub const KEY: &str = glyph!(
    "1.5",
    r#"<circle cx="5.5" cy="10.5" r="3"/><path d="M7.7 8.3L13 3M11 5l1.5 1.5"/>"#
);
/// Reveal a masked value.
pub const EYE: &str = glyph!(
    "1.4",
    r#"<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/>"#
);
/// Mask it again.
pub const EYE_OFF: &str = glyph!(
    "1.4",
    r#"<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/><path d="M2.5 13.5l11-11"/>"#
);
/// The Settings nav's glyphs, from the board.
pub const GENERAL: &str = glyph!(
    "1.5",
    r#"<rect x="2" y="2" width="12" height="12" rx="2"/><path d="M5 6h6M5 10h4"/>"#
);
pub const CODE: &str = glyph!("1.5", r#"<path d="M5 4L1.5 8 5 12M11 4l3.5 4-3.5 4"/>"#);
/// Half filled: the filled half carries its own fill.
pub const APPEARANCE: &str = glyph!(
    "1.5",
    r##"<circle cx="8" cy="8" r="5.5"/><path d="M8 2.5v11"/><path d="M8 2.5a5.5 5.5 0 010 11z" fill="#000"/>"##
);
pub const USER: &str = glyph!(
    "1.5",
    r#"<circle cx="8" cy="5.5" r="2.75"/><path d="M2.75 14a5.25 5.25 0 0110.5 0"/>"#
);
pub const KEYBOARD: &str = glyph!(
    "1.5",
    r#"<rect x="1.5" y="4" width="13" height="8" rx="1.5"/><path d="M4 7h1M7 7h1M10 7h2M5 9.5h6"/>"#
);
pub const UPDATES: &str = glyph!("1.5", r#"<path d="M13 8a5 5 0 11-1.5-3.5M13 2.5v3h-3"/>"#);
pub const CLOUD: &str = glyph!(
    "1.4",
    r#"<path d="M4.5 12.5a3 3 0 01-.3-6 4 4 0 017.6-.9 3.2 3.2 0 01.2 6.9z"/>"#
);
pub const SERVER: &str = glyph!(
    "1.4",
    r#"<rect x="2.5" y="2.5" width="11" height="4.5" rx="1"/><rect x="2.5" y="9" width="11" height="4.5" rx="1"/><path d="M5 4.75h.5M5 11.25h.5"/>"#
);
pub const SYNC: &str = glyph!(
    "1.5",
    r#"<path d="M2.5 7a5.5 5.5 0 019.7-2.5M13.5 9a5.5 5.5 0 01-9.7 2.5"/><path d="M12.5 2v3h-3M3.5 14v-3h3"/>"#
);
/// Where a credential lives.
pub const LOCK: &str = glyph!(
    "1.5",
    r#"<rect x="3" y="7" width="10" height="7" rx="1.5"/><path d="M5.5 7V5a2.5 2.5 0 015 0v2"/>"#
);
pub const LINK: &str = glyph!(
    "1.5",
    r#"<path d="M6.5 9.5l3-3"/><path d="M7 4.5l1-1a2.8 2.8 0 014 4l-1 1M9 11.5l-1 1a2.8 2.8 0 01-4-4l1-1"/>"#
);
pub const MAIL: &str = glyph!(
    "1.5",
    r#"<rect x="2" y="3.5" width="12" height="9" rx="1.5"/><path d="M2.5 4.5L8 8.5l5.5-4"/>"#
);
/// The GitHub mark, filled.
pub const GITHUB: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="#000"><path d="M8 1a7 7 0 00-2.2 13.6c.35.07.48-.15.48-.34v-1.2c-1.95.42-2.36-.94-2.36-.94-.32-.81-.78-1.03-.78-1.03-.64-.43.05-.42.05-.42.7.05 1.07.72 1.07.72.63 1.07 1.64.76 2.04.58.06-.45.24-.76.44-.94-1.55-.18-3.19-.78-3.19-3.46 0-.76.27-1.39.72-1.88-.07-.18-.31-.89.07-1.85 0 0 .59-.19 1.92.72a6.6 6.6 0 013.5 0c1.33-.91 1.92-.72 1.92-.72.38.96.14 1.67.07 1.85.45.49.72 1.12.72 1.88 0 2.69-1.64 3.28-3.2 3.45.25.22.48.65.48 1.31v1.94c0 .19.13.41.48.34A7 7 0 008 1z"/></svg>"##;
/// Filled, unlike the stroke glyphs.
pub const PLAY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="#000"><path d="M4 2.5v11l9-5.5z"/></svg>"##;

/// An icon from one of the glyphs above; size and color it like any `Icon`.
pub fn glyph(svg: &'static str) -> Icon {
    Icon::default().data(svg.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_well_formed_svg() {
        for svg in [
            CHEVRON_DOWN,
            CHEVRON_RIGHT,
            FOLDER,
            FILTER,
            SEARCH,
            PLUS,
            ERROR,
            WARNING,
            INFO,
            PLAY,
            FOLDER_OPEN,
            FOLDER_PLUS,
            FILE_PLUS,
            IMPORT,
            PANEL_BOTTOM,
            CHECK,
            CROSS,
            COPY,
            RERUN,
            DOWNLOAD,
            TREND,
            HISTORY,
            QUEUED,
            SKIPPED,
            CHEVRON_LEFT,
            KEY,
            EYE,
            EYE_OFF,
            GENERAL,
            CODE,
            APPEARANCE,
            USER,
            KEYBOARD,
            UPDATES,
            CLOUD,
            SERVER,
            SYNC,
            LOCK,
            LINK,
            MAIL,
            GITHUB,
        ] {
            assert!(svg.starts_with("<svg ") && svg.ends_with("</svg>"), "{svg}");
            assert!(svg.contains(r#"viewBox="0 0 16 16""#));
            assert_eq!(svg.matches('<').count(), svg.matches('>').count());
        }
        assert!(FOLDER.contains(r#"stroke-width="1.3""#));
    }
}
