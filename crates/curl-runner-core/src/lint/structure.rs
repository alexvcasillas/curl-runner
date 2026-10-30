//! Document-level rules (TS `collectDefinedVariables`, `validateYamlStructure`,
//! `validateCollection`, the `request`/`requests` entry points and
//! `checkDuplicateRequestNames`). Each pass is one entry of [`super::PASSES`].

use super::global;
use super::issues::{
    Crash, Ctx, Fix, FixValue, Res, Severity, is_object, object_keys, prop, truthy, truthy_prop,
};
use super::request::validate_request_config;
use crate::model::Json;

const VALID_TOP_LEVEL_KEYS: &[&str] = &["version", "global", "collection", "requests", "request"];
const VALID_COLLECTION_KEYS: &[&str] =
    &["name", "description", "variables", "defaults", "requests"];

/// `[...v]` throws unless `v` is iterable (arrays, strings).
fn spread(v: Option<&Json>) -> Res {
    match v {
        Some(x) if truthy(Some(x)) && !(x.is_array() || x.is_string()) => Err(Crash(
            "Spread syntax requires ...iterable[Symbol.iterator] to be a function".into(),
        )),
        _ => Ok(()),
    }
}

/// `collectDefinedVariables`. The sets it fills are never read by TS, so
/// only the errors it throws matter.
pub fn collect_defined_variables(content: &Json, _cx: &mut Ctx) -> Res {
    if content.is_null() {
        return Err(Crash(
            "null is not an object (evaluating 'content.global')".into(),
        ));
    }
    spread(prop(content, "requests"))?;
    if let Some(c) = prop(content, "collection").filter(|c| !c.is_null()) {
        spread(prop(c, "requests"))?;
    }
    Ok(())
}

/// `validateYamlStructure`.
pub fn validate_structure(content: &Json, cx: &mut Ctx) -> Res {
    if !is_object(content) {
        return Err(Crash(
            "content is not an Object. (evaluating '\"request\" in content')".into(),
        ));
    }
    let has = |k: &str| prop(content, k).is_some();
    let (req, reqs, coll) = (has("request"), has("requests"), has("collection"));
    if !req && !reqs && !coll {
        cx.add(
            "",
            Severity::Error,
            "Missing request(s). Must have \"request\", \"requests\", or \"collection\"",
        );
    }
    if req && reqs {
        cx.add(
            "",
            Severity::Warning,
            "Both \"request\" and \"requests\" defined; both will be executed",
        );
    }
    if coll && (req || reqs) {
        cx.add(
            "",
            Severity::Warning,
            "Collection mixed with request/requests; may cause confusion",
        );
    }
    for key in object_keys(content)? {
        if !VALID_TOP_LEVEL_KEYS.contains(&key.as_str()) {
            cx.add_fix(
                &key,
                Severity::Warning,
                format!("Unknown top-level key \"{key}\""),
                Some(Fix {
                    description: format!("Remove unknown key \"{key}\""),
                    value: FixValue::Delete,
                }),
            );
        }
    }
    if prop(content, "version").is_some_and(|v| !v.is_string()) {
        cx.add("version", Severity::Warning, "Version should be a string");
    }
    Ok(())
}

pub fn validate_global_pass(content: &Json, cx: &mut Ctx) -> Res {
    let Some(g) = truthy_prop(content, "global") else {
        return Ok(());
    };
    // The runner does not interpolate `global`, so numbers must be literal.
    cx.literal_only = true;
    let res = global::validate_global(cx, g, "global");
    cx.literal_only = false;
    res
}

pub fn validate_collection_pass(content: &Json, cx: &mut Ctx) -> Res {
    let Some(c) = truthy_prop(content, "collection") else {
        return Ok(());
    };
    let path = "collection";
    cx.check_unknown_keys(c, VALID_COLLECTION_KEYS, path)?;
    if truthy_prop(c, "name").is_none() {
        cx.add(
            "collection.name",
            Severity::Warning,
            "Collection name is recommended",
        );
    }
    match truthy_prop(c, "requests").and_then(Json::as_array) {
        None => cx.add(
            "collection.requests",
            Severity::Error,
            "Collection must have a requests array",
        ),
        Some(list) => {
            if list.is_empty() {
                cx.add(
                    "collection.requests",
                    Severity::Warning,
                    "Collection has no requests",
                );
            }
            for (i, r) in list.iter().enumerate() {
                validate_request_config(cx, r, &format!("{path}.requests[{i}]"), false)?;
            }
        }
    }
    if let Some(v) = truthy_prop(c, "variables") {
        global::validate_variables(cx, v, "collection.variables");
    }
    if let Some(d) = truthy_prop(c, "defaults") {
        validate_request_config(cx, d, "collection.defaults", true)?;
    }
    Ok(())
}

pub fn validate_request_pass(content: &Json, cx: &mut Ctx) -> Res {
    match truthy_prop(content, "request") {
        Some(r) => validate_request_config(cx, r, "request", false),
        None => Ok(()),
    }
}

pub fn validate_requests_pass(content: &Json, cx: &mut Ctx) -> Res {
    let Some(reqs) = truthy_prop(content, "requests") else {
        return Ok(());
    };
    match reqs.as_array() {
        None => cx.add("requests", Severity::Error, "Must be an array"),
        Some(list) => {
            for (i, r) in list.iter().enumerate() {
                validate_request_config(cx, r, &format!("requests[{i}]"), false)?;
            }
        }
    }
    Ok(())
}

/// `checkDuplicateRequestNames`: entries are `path:name`, split on `:` (so a
/// name is cut at its first colon, as in TS).
pub fn check_duplicate_names(_content: &Json, cx: &mut Ctx) -> Res {
    let mut names: Vec<(String, Vec<String>)> = Vec::new();
    for entry in &cx.request_names {
        let mut parts = entry.split(':');
        let path = parts.next().unwrap_or_default().to_string();
        let name = parts.next().unwrap_or_default().to_string();
        match names.iter_mut().find(|(n, _)| *n == name) {
            Some((_, paths)) => paths.push(path),
            None => names.push((name, vec![path])),
        }
    }
    for (name, paths) in names {
        if paths.len() > 1 {
            cx.add(
                &paths[1],
                Severity::Warning,
                format!("Duplicate request name \"{name}\" (also at {})", paths[0]),
            );
        }
    }
    Ok(())
}
