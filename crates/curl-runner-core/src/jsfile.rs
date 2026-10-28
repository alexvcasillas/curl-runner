//! Helpers shared by the snapshot and baseline files: both are JSON files TS
//! reads as untyped JS objects, builds paths for with `path.dirname/basename`
//! and must never overwrite when they cannot be parsed.

use crate::model::Json;
use indexmap::IndexMap;
use serde_json::Map;
use std::path::Path;
use std::sync::OnceLock;

/// A stored entry read like TS reads an untyped object: falsy values
/// (`null`, `false`, `0`, `""`) count as missing (`|| null`) and any other
/// non-object has no own fields (so it reads as an empty object).
pub(crate) fn untyped_entry(v: &Json) -> Option<&Map<String, Json>> {
    static EMPTY: OnceLock<Map<String, Json>> = OnceLock::new();
    let falsy = match v {
        Json::Null | Json::Bool(false) => true,
        Json::Number(n) => n.as_f64() == Some(0.0),
        Json::String(s) => s.is_empty(),
        _ => false,
    };
    match v {
        _ if falsy => None,
        Json::Object(o) => Some(o),
        _ => Some(EMPTY.get_or_init(Map::new)),
    }
}

/// String field, empty when missing or not a string.
pub(crate) fn text_field(o: &Map<String, Json>, key: &str) -> String {
    o.get(key)
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string()
}

/// `headers` of a stored entry: objects keep their values untouched, falsy
/// values are absent and other truthy values have no entries.
pub(crate) fn headers_field(o: &Map<String, Json>) -> Option<IndexMap<String, Json>> {
    match o.get("headers") {
        Some(Json::Object(h)) => Some(h.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        Some(Json::Null | Json::Bool(false)) | None => None,
        Some(Json::Number(n)) if n.as_f64() == Some(0.0) => None,
        Some(Json::String(s)) if s.is_empty() => None,
        Some(_) => Some(IndexMap::new()),
    }
}

/// Headers as a JS object (`headers || {}`).
pub(crate) fn headers_map(h: Option<&IndexMap<String, Json>>) -> Map<String, Json> {
    h.into_iter()
        .flatten()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// `path.dirname` and `path.basename` of a POSIX path.
pub(crate) fn split_path(p: &str) -> (&str, &str) {
    match p.rfind('/') {
        Some(0) => ("/", &p[1..]),
        Some(i) => (&p[..i], &p[i + 1..]),
        None => (".", p),
    }
}

/// `path.basename(p, path.extname(p))`.
pub(crate) fn stem(file: &str) -> &str {
    match file.rfind('.') {
        Some(i) if i > 0 => &file[..i],
        _ => file,
    }
}

/// Error for a file that exists but cannot be used (and so is never replaced).
pub(crate) fn unusable(kind: &str, path: &Path, why: &str) -> std::io::Error {
    std::io::Error::other(format!(
        "{kind} file {} cannot be used ({why}); fix or delete it, it was not overwritten",
        path.display()
    ))
}

/// Reads a JSON file shaped `{ <key>: {...} }`: `Ok(None)` when it does not
/// exist, `Err` when it exists but cannot be read, parsed (e.g. a
/// lone-surrogate escape `serde_json` rejects) or lacks the `key` object.
pub(crate) fn read_json_file(path: &Path, kind: &str, key: &str) -> std::io::Result<Option<Json>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(unusable(kind, path, &e.to_string())),
    };
    let v: Json = serde_json::from_str(&text).map_err(|e| unusable(kind, path, &e.to_string()))?;
    if !v.get(key).is_some_and(Json::is_object) {
        return Err(unusable(kind, path, &format!("no `{key}` object")));
    }
    Ok(Some(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn falsy_entries_are_missing_and_other_non_objects_are_empty() {
        for v in [json!(null), json!(false), json!(0), json!("")] {
            assert!(untyped_entry(&v).is_none(), "{v}");
        }
        assert!(untyped_entry(&json!(5)).unwrap().is_empty());
        assert!(untyped_entry(&json!([1])).unwrap().is_empty());
        assert_eq!(untyped_entry(&json!({"a": 1})).unwrap().len(), 1);
    }

    #[test]
    fn headers_field_follows_js_truthiness() {
        let h = |v: Json| headers_field(v.as_object().unwrap());
        assert_eq!(h(json!({})), None);
        assert_eq!(h(json!({"headers": null})), None);
        assert_eq!(h(json!({"headers": ""})), None);
        assert_eq!(h(json!({"headers": 7})), Some(IndexMap::new()));
        assert_eq!(h(json!({"headers": {"a": 1}})).unwrap()["a"], json!(1));
    }

    #[test]
    fn path_pieces_match_node() {
        assert_eq!(split_path("a.yaml"), (".", "a.yaml"));
        assert_eq!(split_path("/a.yaml"), ("/", "a.yaml"));
        assert_eq!(split_path("t/x/a.b.yaml"), ("t/x", "a.b.yaml"));
        assert_eq!(stem("a.b.yaml"), "a.b");
        assert_eq!(stem(".hidden"), ".hidden");
        assert_eq!(stem("noext"), "noext");
    }
}
