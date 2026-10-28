//! `when` and `expect` rules (TS `validateWhenCondition`, `validateExpect`).

use super::issues::{
    Crash, Ctx, Res, Severity, as_num, has_key, is_object, not_plain_object, prop, truthy_prop,
};
use super::request::convert_to;
use curl_runner_core::jsval::{is_js_space, parse_int};
use curl_runner_core::model::Json;

const VALID_OPERATORS: &[&str] = &[
    "==",
    "!=",
    ">",
    "<",
    ">=",
    "<=",
    "contains",
    "matches",
    "exists",
    "not-exists",
];

pub fn validate_when(cx: &mut Ctx, when: &Json, path: &str) -> Res {
    if let Some(s) = when.as_str() {
        validate_when_string(cx, s, path);
        return Ok(());
    }
    if !is_object(when) {
        return Err(Crash(
            "when is not an Object. (evaluating '\"all\" in when')".into(),
        ));
    }
    let all = truthy_prop(when, "all");
    let any = truthy_prop(when, "any");
    let single = has_key(when, "left") || has_key(when, "operator") || has_key(when, "right");

    if all.is_none() && any.is_none() && !single {
        cx.add(
            path,
            Severity::Error,
            "When condition must have \"all\", \"any\", or \"left\"",
        );
        return Ok(());
    }
    let forms = [all.is_some(), any.is_some(), single];
    if forms.iter().filter(|f| **f).count() > 1 {
        cx.add(
            path,
            Severity::Warning,
            "When condition has multiple forms; behavior may be unexpected",
        );
    }

    for (key, list, empty) in [
        ("all", all, "Empty \"all\" array - condition always true"),
        ("any", any, "Empty \"any\" array - condition always false"),
    ] {
        let Some(list) = list else { continue };
        let p = format!("{path}.{key}");
        match list.as_array() {
            None => cx.add(&p, Severity::Error, "Must be an array"),
            Some(items) => {
                if items.is_empty() {
                    cx.add(&p, Severity::Warning, empty);
                }
                for (i, cond) in items.iter().enumerate() {
                    validate_condition(cx, cond, &format!("{p}[{i}]"))?;
                }
            }
        }
    }

    if single {
        validate_condition(cx, when, path)?;
    }
    Ok(())
}

fn validate_when_string(cx: &mut Ctx, when: &str, path: &str) {
    if !when.contains("store.") && !when.contains("env.") {
        cx.add(
            path,
            Severity::Warning,
            "When string should reference store.* or env.* values",
        );
    }
    // `/\s*(==|!=|>=|<=|>|<|contains|matches|exists|not-exists)\s*/`: the
    // `\s*` may match nothing, so this is a substring test.
    let has_op = ["==", "!=", ">", "<", "contains", "matches", "exists"]
        .iter()
        .any(|op| when.contains(op));
    if !has_op {
        cx.add(
            path,
            Severity::Warning,
            "When string should contain a valid operator (==, !=, >, <, >=, <=, contains, matches, exists)",
        );
    }
}

fn validate_condition(cx: &mut Ctx, cond: &Json, path: &str) -> Res {
    if cond.is_null() {
        return Err(Crash(
            "null is not an object (evaluating 'cond.left')".into(),
        ));
    }
    match truthy_prop(cond, "left") {
        None => cx.add(
            &format!("{path}.left"),
            Severity::Error,
            "Left operand is required",
        ),
        Some(left) => {
            let left = left.as_str().ok_or_else(|| {
                Crash("cond.left.startsWith is not a function. (In 'cond.left.startsWith(\"store.\")', 'cond.left.startsWith' is undefined)".into())
            })?;
            if !left.starts_with("store.") && !left.starts_with("env.") {
                cx.add(
                    &format!("{path}.left"),
                    Severity::Warning,
                    "Left operand should start with \"store.\" or \"env.\"",
                );
            }
        }
    }

    let op = prop(cond, "operator");
    match truthy_prop(cond, "operator") {
        None => cx.add(
            &format!("{path}.operator"),
            Severity::Error,
            "Operator is required",
        ),
        Some(o) if o.as_str().is_some_and(|s| VALID_OPERATORS.contains(&s)) => {}
        Some(o) => {
            cx.invalid_choice(o, VALID_OPERATORS, &format!("{path}.operator"), "operator")?;
        }
    }

    let op = op.and_then(Json::as_str);
    if op != Some("exists") && op != Some("not-exists") && prop(cond, "right").is_none() {
        cx.add(
            &format!("{path}.right"),
            Severity::Error,
            "Right operand is required",
        );
    }
    Ok(())
}

pub fn validate_expect(cx: &mut Ctx, expect: &Json, path: &str) -> Res {
    cx.check_unknown_keys(
        expect,
        &["failure", "status", "headers", "body", "responseTime"],
        path,
    )?;
    cx.boolean_field(expect, "failure", path);

    if let Some(s) = prop(expect, "status") {
        validate_status(cx, s, &format!("{path}.status"));
    }
    if truthy_prop(expect, "headers").is_some_and(not_plain_object) {
        cx.add(
            &format!("{path}.headers"),
            Severity::Error,
            "Expected headers must be an object",
        );
    }
    if let Some(rt) = prop(expect, "responseTime") {
        validate_response_time(cx, rt, &format!("{path}.responseTime"));
    }
    Ok(())
}

fn in_status_range(v: &Json) -> bool {
    as_num(v).is_some_and(|n| (100.0..=599.0).contains(&n))
}

fn validate_status(cx: &mut Ctx, status: &Json, path: &str) {
    if let Some(list) = status.as_array() {
        if list.is_empty() {
            cx.add(
                path,
                Severity::Warning,
                "Empty status array - no status will match",
            );
        }
        if !list.iter().all(in_status_range) {
            cx.add(
                path,
                Severity::Error,
                "Status codes must be numbers between 100-599",
            );
        }
    } else if let Some(s) = status.as_str() {
        let parsed = parse_int(s);
        if (100.0..=599.0).contains(&parsed) {
            cx.add_fix(
                path,
                Severity::Warning,
                "Status should be a number, not string",
                Some(convert_to(parsed)),
            );
        } else {
            cx.add(
                path,
                Severity::Error,
                "Status code must be a number between 100-599",
            );
        }
    } else if !in_status_range(status) {
        cx.add(
            path,
            Severity::Error,
            "Status code must be a number between 100-599",
        );
    }
}

/// One `[<>]=?\s*(\d+|\$\{[^}]+\})` at the start of `s`; returns the rest.
fn bound(s: &str) -> Option<&str> {
    let s = s.strip_prefix(['<', '>'])?;
    let s = s.strip_prefix('=').unwrap_or(s);
    let s = s.trim_start_matches(is_js_space);
    if let Some(rest) = s.strip_prefix("${") {
        let close = rest.find('}')?;
        return (close > 0).then(|| &rest[close + 1..]);
    }
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0).then(|| &s[digits..])
}

/// `/^[<>]=?\s*(\d+|\$\{[^}]+\})(\s*,\s*[<>]=?\s*(\d+|\$\{[^}]+\}))?$/`.
fn is_response_time(s: &str) -> bool {
    let Some(rest) = bound(s) else { return false };
    if rest.is_empty() {
        return true;
    }
    rest.trim_start_matches(is_js_space)
        .strip_prefix(',')
        .map(|r| r.trim_start_matches(is_js_space))
        .and_then(bound)
        .is_some_and(str::is_empty)
}

fn validate_response_time(cx: &mut Ctx, rt: &Json, path: &str) {
    let Some(s) = rt.as_str() else {
        cx.add(
            path,
            Severity::Error,
            "Response time must be a string (e.g., \"< 1000\")",
        );
        return;
    };
    if !is_response_time(s) {
        cx.add(
            path,
            Severity::Warning,
            "Response time should be like \"< 1000\", \"> 500, < 2000\"",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_time_pattern() {
        for ok in [
            "< 1000",
            "<=5",
            ">500, < 2000",
            "< ${T}",
            "> ${A},<${B}",
            "<\u{a0}5",
        ] {
            assert!(is_response_time(ok), "{ok}");
        }
        for bad in [
            "fast", "1000", "< ", "< ${}", "< 5,", "< 5 , ", "<5 x", "=< 5", "< 5\n",
        ] {
            assert!(!is_response_time(bad), "{bad}");
        }
    }
}
