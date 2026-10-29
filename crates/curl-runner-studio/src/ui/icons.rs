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
        ] {
            assert!(svg.starts_with("<svg ") && svg.ends_with("</svg>"), "{svg}");
            assert!(svg.contains(r#"viewBox="0 0 16 16""#));
            assert_eq!(svg.matches('<').count(), svg.matches('>').count());
        }
        assert!(FOLDER.contains(r#"stroke-width="1.3""#));
    }
}
