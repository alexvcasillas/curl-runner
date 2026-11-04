//! Names every importer shares: file slugs and variable names.

/// Path of a file field an export left without one.
pub const PLACEHOLDER_FILE: &str = "./pick-a-file";

/// Names that curl-runner reads as dynamic variables.
const RESERVED_PREFIXES: [&str; 6] = [
    "DATE",
    "TIME",
    "UUID",
    "RANDOM",
    "CURRENT_TIME",
    "TIMESTAMP",
];

/// A source variable name as curl-runner can use it: only
/// `[A-Za-z0-9_]`, and `_VAR` after a name curl-runner treats specially.
pub fn var_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.starts_with("store.") || RESERVED_PREFIXES.iter().any(|p| out.starts_with(p)) {
        out.push_str("_VAR");
    }
    out
}

/// The longest slug, so a file name stays well under the file system's
/// limit.
const MAX_SLUG: usize = 100;

/// Lowercase ASCII letters and digits; any run of other characters becomes
/// `-`, trimmed; at most `MAX_SLUG` characters; empty falls back to
/// `fallback`. `.`, `/` and `\` are among the "other" characters, so a
/// slug is always a single plain component.
pub fn slug(name: &str, fallback: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let mut out = out.trim_start_matches('-');
    if let Some((end, _)) = out.char_indices().nth(MAX_SLUG) {
        out = &out[..end];
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() {
        fallback.into()
    } else {
        out.to_string()
    }
}
