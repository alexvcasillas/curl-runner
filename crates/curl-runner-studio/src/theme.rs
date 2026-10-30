//! Design tokens from `tokens.json`, dark theme.

use std::collections::HashMap;
use std::sync::OnceLock;

const TOKENS_JSON: &str =
    include_str!("../../../docs/superpowers/specs/studio-design-system/tokens.json");

/// An sRGB color with straight alpha, each channel in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// The dark-theme color token `name`, with `{alias}` values resolved.
///
/// # Panics
///
/// Panics if `name` is not a color token in `tokens.json`.
pub fn color(name: &str) -> Rgba {
    match palette().get(name) {
        Some(&c) => c,
        None => panic!("unknown color token `{name}`"),
    }
}

/// The badge color for an HTTP method; unknown methods use `method-other`.
pub fn method_color(method: &str) -> Rgba {
    color(match method {
        "GET" => "method-get",
        "POST" => "method-post",
        "PUT" => "method-put",
        "PATCH" => "method-patch",
        "DELETE" => "method-delete",
        _ => "method-other",
    })
}

/// Token name to resolved dark color, parsed once.
fn palette() -> &'static HashMap<String, Rgba> {
    static PALETTE: OnceLock<HashMap<String, Rgba>> = OnceLock::new();
    PALETTE.get_or_init(|| {
        let tokens: serde_json::Value =
            serde_json::from_str(TOKENS_JSON).expect("tokens.json is valid JSON");
        let raw: HashMap<&str, &str> = tokens["color"]["tokens"]
            .as_array()
            .expect("color.tokens is an array")
            .iter()
            .map(|t| {
                let name = t["name"].as_str().expect("token name");
                let dark = t["value"]["dark"].as_str().expect("token dark value");
                (name, dark)
            })
            .collect();
        raw.keys()
            .map(|&name| (name.to_owned(), resolve(&raw, name)))
            .collect()
    })
}

fn resolve(raw: &HashMap<&str, &str>, name: &str) -> Rgba {
    let mut value = raw[name];
    // Aliases are one level deep today; the bound guards against cycles.
    for _ in 0..raw.len() {
        match value.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
            Some(target) => value = raw[target],
            None => return parse_hex(value),
        }
    }
    panic!("alias cycle at color token `{name}`")
}

/// Parses `#rrggbb` or `#rrggbbaa`.
fn parse_hex(value: &str) -> Rgba {
    let hex = value
        .strip_prefix('#')
        .filter(|h| matches!(h.len(), 6 | 8))
        .unwrap_or_else(|| panic!("bad color `{value}`"));
    let channel = |i: usize| {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16)
            .unwrap_or_else(|_| panic!("bad color `{value}`"));
        f32::from(byte) / 255.0
    };
    Rgba {
        r: channel(0),
        g: channel(2),
        b: channel(4),
        a: if hex.len() == 8 { channel(6) } else { 1.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a: 1.0,
        }
    }

    #[test]
    fn theme_bg_100_is_near_black() {
        assert_eq!(color("bg-100"), rgb(0x0a, 0x0a, 0x0a));
    }

    #[test]
    fn theme_text_gutter() {
        assert_eq!(color("text-gutter"), rgb(0x80, 0x80, 0x80));
    }

    #[test]
    fn theme_get_is_accent_blue() {
        assert_eq!(method_color("GET"), color("accent-blue"));
    }

    #[test]
    fn theme_unknown_method_is_other() {
        assert_eq!(method_color("TRACE"), color("method-other"));
    }

    #[test]
    fn theme_parses_alpha() {
        assert_eq!(
            color("overlay"),
            Rgba {
                a: f32::from(0xb3_u8) / 255.0,
                ..rgb(0, 0, 0)
            }
        );
    }

    #[test]
    fn theme_every_token_resolves() {
        let tokens: serde_json::Value = serde_json::from_str(TOKENS_JSON).unwrap();
        let names: Vec<&str> = tokens["color"]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(!names.is_empty());
        for name in names {
            color(name);
        }
    }
}
