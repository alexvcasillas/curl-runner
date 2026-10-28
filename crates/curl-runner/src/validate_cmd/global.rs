//! `global` rules (TS `validateGlobalConfig` and its sub-blocks: output,
//! connectionPool, ci, watch, profile, snapshot, diff, variables).

use super::issues::{
    Ctx, Res, Severity, as_num, entries, is_identifier, is_sensitive_value, js_string,
    not_plain_object, prop, truthy_prop,
};
use super::{request, security};
use curl_runner_core::model::Json;

pub(super) const VALID_GLOBAL_KEYS: &[&str] = &[
    "env",
    "security",
    "execution",
    "maxConcurrency",
    "continueOnError",
    "dryRun",
    "http2",
    "connectionPool",
    "ci",
    "ssl",
    "watch",
    "profile",
    "snapshot",
    "diff",
    "variables",
    "output",
    "defaults",
];
pub(super) const VALID_ENV_KEYS: &[&str] = &["environment", "redactSecrets"];
pub(super) const VALID_SECURITY_KEYS: &[&str] = &["allowedProtocols", "allowPaths"];
pub(super) const VALID_PROFILE_KEYS: &[&str] = &[
    "iterations",
    "warmup",
    "concurrency",
    "histogram",
    "exportFile",
];
/// `GlobalSnapshotConfig` (`global.snapshot`).
pub(super) const VALID_GLOBAL_SNAPSHOT_KEYS: &[&str] = &[
    "enabled",
    "name",
    "include",
    "exclude",
    "match",
    "dir",
    "updateMode",
    "ci",
];
/// `SnapshotConfig` (request-level `snapshot:`).
pub(super) const VALID_REQUEST_SNAPSHOT_KEYS: &[&str] =
    &["enabled", "name", "include", "exclude", "match"];
/// `GlobalDiffConfig` (`global.diff`).
pub(super) const VALID_GLOBAL_DIFF_KEYS: &[&str] = &[
    "enabled",
    "exclude",
    "match",
    "includeTimings",
    "dir",
    "label",
    "compareWith",
    "save",
    "outputFormat",
];
/// `DiffConfig` (request-level `diff:`).
pub(super) const VALID_REQUEST_DIFF_KEYS: &[&str] =
    &["enabled", "exclude", "match", "includeTimings"];
const VALID_EXECUTION_MODES: &[&str] = &["sequential", "parallel"];
const VALID_OUTPUT_FORMATS: &[&str] = &["json", "pretty", "raw"];
const VALID_PRETTY_LEVELS: &[&str] = &["minimal", "standard", "detailed"];
const VALID_SNAPSHOT_UPDATE_MODES: &[&str] = &["none", "all", "failing"];
const VALID_DIFF_OUTPUT_FORMATS: &[&str] = &["terminal", "json", "markdown"];

/// A truthy `field` that is not one of `valid` (strict equality).
fn bad_choice<'a>(obj: &'a Json, field: &str, valid: &[&str]) -> Option<&'a Json> {
    truthy_prop(obj, field).filter(|v| !v.as_str().is_some_and(|s| valid.contains(&s)))
}

pub fn validate_global(cx: &mut Ctx, config: &Json, path: &str) -> Res {
    cx.check_unknown_keys(config, VALID_GLOBAL_KEYS, path)?;

    if let Some(e) = bad_choice(config, "execution", VALID_EXECUTION_MODES) {
        cx.invalid_choice(
            e,
            VALID_EXECUTION_MODES,
            &format!("{path}.execution"),
            "execution mode",
        )?;
    }

    if let Some(mc) = prop(config, "maxConcurrency") {
        let p = format!("{path}.maxConcurrency");
        if cx.not_num_at_least(mc, 1.0) {
            let m = cx.num_msg(mc, "Must be a positive number");
            cx.add(&p, Severity::Error, m);
        }
        if prop(config, "execution").and_then(Json::as_str) == Some("sequential") {
            cx.add(
                &p,
                Severity::Warning,
                "maxConcurrency has no effect with sequential execution",
            );
        }
    }

    for f in ["continueOnError", "dryRun", "http2"] {
        cx.boolean_field(config, f, path);
    }

    if let Some(e) = truthy_prop(config, "env") {
        cx.check_unknown_keys(e, VALID_ENV_KEYS, &format!("{path}.env"))?;
    }
    if let Some(s) = truthy_prop(config, "security") {
        cx.check_unknown_keys(s, VALID_SECURITY_KEYS, &format!("{path}.security"))?;
    }
    if let Some(o) = truthy_prop(config, "output") {
        validate_output(cx, o, &format!("{path}.output"))?;
    }
    if let Some(p) = truthy_prop(config, "connectionPool") {
        validate_pool(cx, p, &format!("{path}.connectionPool"))?;
    }
    if let Some(c) = truthy_prop(config, "ci") {
        validate_ci(cx, c, &format!("{path}.ci"))?;
    }
    if let Some(s) = truthy_prop(config, "ssl") {
        security::validate_ssl(cx, s, &format!("{path}.ssl"))?;
    }
    if let Some(w) = truthy_prop(config, "watch") {
        validate_watch(cx, w, &format!("{path}.watch"))?;
    }
    if let Some(p) = truthy_prop(config, "profile") {
        validate_profile(cx, p, &format!("{path}.profile"))?;
    }
    if let Some(s) = truthy_prop(config, "snapshot") {
        validate_snapshot(
            cx,
            s,
            &format!("{path}.snapshot"),
            VALID_GLOBAL_SNAPSHOT_KEYS,
        )?;
    }
    if let Some(d) = truthy_prop(config, "diff") {
        validate_diff(cx, d, &format!("{path}.diff"), VALID_GLOBAL_DIFF_KEYS)?;
    }
    if let Some(v) = truthy_prop(config, "variables") {
        validate_variables(cx, v, &format!("{path}.variables"));
    }
    if let Some(d) = truthy_prop(config, "defaults") {
        request::validate_request_config(cx, d, &format!("{path}.defaults"), true)?;
    }
    Ok(())
}

pub fn validate_variables(cx: &mut Ctx, vars: &Json, path: &str) {
    if not_plain_object(vars) {
        cx.add(path, Severity::Error, "Must be an object");
        return;
    }
    for (key, value) in entries(vars) {
        let p = format!("{path}.{key}");
        if !is_identifier(key) {
            cx.add(
                &p,
                Severity::Warning,
                "Variable names should use alphanumeric characters and underscores",
            );
        }
        if let Some(v) = value.as_str()
            && is_sensitive_value(key, v)
        {
            cx.add(
                &p,
                Severity::Warning,
                "Potentially sensitive value hardcoded. Consider using environment variables like \"${env.VAR}\"",
            );
        }
    }
}

fn validate_output(cx: &mut Ctx, output: &Json, path: &str) -> Res {
    cx.check_unknown_keys(
        output,
        &[
            "verbose",
            "showHeaders",
            "showBody",
            "showMetrics",
            "format",
            "prettyLevel",
            "saveToFile",
            "dryRun",
        ],
        path,
    )?;
    if let Some(f) = bad_choice(output, "format", VALID_OUTPUT_FORMATS) {
        cx.invalid_choice(f, VALID_OUTPUT_FORMATS, &format!("{path}.format"), "format")?;
    }
    if let Some(l) = bad_choice(output, "prettyLevel", VALID_PRETTY_LEVELS) {
        cx.invalid_choice(
            l,
            VALID_PRETTY_LEVELS,
            &format!("{path}.prettyLevel"),
            "prettyLevel",
        )?;
    }
    for f in ["verbose", "showHeaders", "showBody", "showMetrics"] {
        cx.boolean_field(output, f, path);
    }
    Ok(())
}

/// `x !== undefined && (typeof x !== 'number' || x < min)` → error `msg`.
fn number_at_least(cx: &mut Ctx, obj: &Json, field: &str, min: f64, path: &str, msg: &str) {
    if let Some(v) = prop(obj, field).filter(|v| cx.not_num_at_least(v, min)) {
        let m = cx.num_msg(v, msg);
        cx.add(&format!("{path}.{field}"), Severity::Error, m);
    }
}

fn num_above(obj: &Json, field: &str, max: f64) -> bool {
    prop(obj, field).and_then(as_num).is_some_and(|n| n > max)
}

const POSITIVE: &str = "Must be a positive number";
const NON_NEGATIVE: &str = "Must be a non-negative number";

fn validate_pool(cx: &mut Ctx, pool: &Json, path: &str) -> Res {
    cx.check_unknown_keys(
        pool,
        &[
            "enabled",
            "maxStreamsPerHost",
            "keepaliveTime",
            "connectTimeout",
        ],
        path,
    )?;
    cx.boolean_field(pool, "enabled", path);
    number_at_least(cx, pool, "maxStreamsPerHost", 1.0, path, POSITIVE);
    if num_above(pool, "maxStreamsPerHost", 100.0) {
        cx.add(
            &format!("{path}.maxStreamsPerHost"),
            Severity::Warning,
            "Very high stream count may cause issues with some servers",
        );
    }
    number_at_least(cx, pool, "keepaliveTime", 0.0, path, NON_NEGATIVE);
    number_at_least(cx, pool, "connectTimeout", 0.0, path, NON_NEGATIVE);
    Ok(())
}

fn validate_ci(cx: &mut Ctx, ci: &Json, path: &str) -> Res {
    cx.check_unknown_keys(ci, &["strictExit", "failOn", "failOnPercentage"], path)?;
    cx.boolean_field(ci, "strictExit", path);
    number_at_least(cx, ci, "failOn", 0.0, path, NON_NEGATIVE);
    let pct = prop(ci, "failOnPercentage");
    if let Some(v) =
        pct.filter(|v| !cx.interpolated(v) && as_num(v).is_none_or(|n| !(0.0..=100.0).contains(&n)))
    {
        let m = cx.num_msg(v, "Must be a number between 0 and 100");
        cx.add(&format!("{path}.failOnPercentage"), Severity::Error, m);
    }
    if prop(ci, "failOn").is_some() && pct.is_some() {
        cx.add(
            path,
            Severity::Info,
            "Both failOn and failOnPercentage are set; failOn takes precedence",
        );
    }
    Ok(())
}

fn validate_watch(cx: &mut Ctx, watch: &Json, path: &str) -> Res {
    cx.check_unknown_keys(watch, &["enabled", "debounce", "clear"], path)?;
    number_at_least(cx, watch, "debounce", 0.0, path, NON_NEGATIVE);
    cx.boolean_field(watch, "enabled", path);
    cx.boolean_field(watch, "clear", path);
    Ok(())
}

fn validate_profile(cx: &mut Ctx, profile: &Json, path: &str) -> Res {
    cx.check_unknown_keys(profile, VALID_PROFILE_KEYS, path)?;
    number_at_least(cx, profile, "iterations", 1.0, path, POSITIVE);
    if num_above(profile, "iterations", 10_000.0) {
        cx.add(
            &format!("{path}.iterations"),
            Severity::Warning,
            "Very high iteration count may take long",
        );
    }
    number_at_least(cx, profile, "warmup", 0.0, path, NON_NEGATIVE);
    number_at_least(cx, profile, "concurrency", 1.0, path, POSITIVE);
    Ok(())
}

/// Shared by `snapshot` and `diff`: an enum field and a string `dir`.
fn choice_and_dir(cx: &mut Ctx, obj: &Json, field: &str, valid: &[&str], path: &str) {
    if let Some(v) = bad_choice(obj, field, valid) {
        cx.add(
            &format!("{path}.{field}"),
            Severity::Error,
            format!(
                "Invalid {field} \"{}\". Must be: {}",
                js_string(v),
                valid.join(", ")
            ),
        );
    }
    if truthy_prop(obj, "dir").is_some_and(|d| !d.is_string()) {
        cx.add(&format!("{path}.dir"), Severity::Error, "Must be a string");
    }
}

pub fn validate_snapshot(cx: &mut Ctx, snapshot: &Json, path: &str, keys: &[&str]) -> Res {
    cx.check_unknown_keys(snapshot, keys, path)?;
    choice_and_dir(
        cx,
        snapshot,
        "updateMode",
        VALID_SNAPSHOT_UPDATE_MODES,
        path,
    );
    Ok(())
}

pub fn validate_diff(cx: &mut Ctx, diff: &Json, path: &str, keys: &[&str]) -> Res {
    cx.check_unknown_keys(diff, keys, path)?;
    choice_and_dir(cx, diff, "outputFormat", VALID_DIFF_OUTPUT_FORMATS, path);
    Ok(())
}
