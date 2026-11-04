//! A mapped request written as one curl-runner YAML file. Disabled
//! fields stay in place as commented-out lines: removing the leading `# `
//! of one gives valid YAML with that field added.

use super::{Field, FieldValue, MappedAuth, MappedBody, MappedRequest};
use crate::model::Json;
use crate::snapshot::js_stringify;
use crate::yaml_text::{needs_quoting, quote_escaped, reads_back, scalar_text, yaml_key, yaml_str};

/// Field keys sit under `request:`'s maps, four spaces deep.
const ENTRY: &str = "    ";

/// The YAML file for `r`, headed with the source format and the
/// collection's name.
pub fn emit_request(r: &MappedRequest, source: &str, collection: &str) -> String {
    let mut out = vec![format!(
        "# Imported from {source}: {}",
        text_lines(collection).join(" ")
    )];
    for c in &r.comments {
        out.extend(text_lines(c).iter().map(|l| comment("", l)));
    }
    out.push(String::new());

    if !r.file_vars.is_empty() {
        out.push("global:".into());
        out.push("  variables:".into());
        for (k, v) in &r.file_vars {
            out.push(format!("{ENTRY}{}: {}", yaml_key(k), scalar(v)));
        }
        out.push(String::new());
    }

    out.push("request:".into());
    out.push(format!("  name: {}", scalar(&r.name)));
    out.push(format!("  method: {}", scalar(&r.method)));
    out.push(format!("  url: {}", scalar(&r.url)));
    fields(&mut out, "headers", &r.headers);
    fields(&mut out, "params", &r.params);
    match &r.auth {
        Some(MappedAuth::Bearer(token)) => {
            out.push("  auth:".into());
            out.push(format!("{ENTRY}type: bearer"));
            out.push(format!("{ENTRY}token: {}", scalar(token)));
        }
        Some(MappedAuth::Basic { username, password }) => {
            out.push("  auth:".into());
            out.push(format!("{ENTRY}type: basic"));
            out.push(format!("{ENTRY}username: {}", scalar(username)));
            out.push(format!("{ENTRY}password: {}", scalar(password)));
        }
        None => {}
    }
    match &r.body {
        MappedBody::None => {}
        MappedBody::Json(v) => json_body(&mut out, v),
        MappedBody::Text(t) => out.push(format!("  body: {}", text_body(t))),
        MappedBody::Form(f) => fields(&mut out, "formData", f),
    }
    if let Some(status) = r.expect_status {
        out.push("  expect:".into());
        out.push(format!("{ENTRY}status: {status}"));
    }
    format!("{}\n", out.join("\n"))
}

/// `name:` then one line per field, in order. Each note goes on its own
/// comment line above its field; a disabled field and its note get one
/// more `# `.
fn fields(out: &mut Vec<String>, name: &str, fields: &[Field]) {
    if fields.is_empty() {
        return;
    }
    out.push(format!("  {name}:"));
    for f in fields {
        let off = if f.disabled { "# " } else { "" };
        if let Some(note) = &f.note {
            let pad = format!("{ENTRY}{off}");
            out.extend(text_lines(note).iter().map(|l| comment(&pad, l)));
        }
        let value = match &f.value {
            FieldValue::Text(v) => scalar(v),
            FieldValue::File { path, .. } => format!("{{ file: {} }}", flow_scalar(path)),
        };
        out.push(format!("{ENTRY}{off}{}: {value}", yaml_key(&f.key)));
    }
}

/// An object or array body as block YAML in key order; a scalar as a
/// string holding its JSON text, which the runner sends verbatim (the
/// core serializer's rule).
fn json_body(out: &mut Vec<String>, v: &Json) {
    match v {
        Json::Object(_) | Json::Array(_) => json_entry(out, 2, "body:", v),
        scalar => out.push(format!("  body: {}", text_body(&js_stringify(scalar)))),
    }
}

/// `lead` (`key:` or `-`) and `v`, `pad` spaces deep.
fn json_entry(out: &mut Vec<String>, pad: usize, lead: &str, v: &Json) {
    let p = " ".repeat(pad);
    let children = |out: &mut Vec<String>| match v {
        Json::Object(o) => {
            for (k, v) in o {
                json_entry(out, pad + 2, &format!("{}:", yaml_key(k)), v);
            }
        }
        Json::Array(a) => {
            for v in a {
                json_entry(out, pad + 2, "-", v);
            }
        }
        _ => unreachable!(),
    };
    match v {
        Json::Object(o) if !o.is_empty() => {}
        Json::Array(a) if !a.is_empty() => {}
        Json::Object(_) => return out.push(format!("{p}{lead} {{}}")),
        Json::Array(_) => return out.push(format!("{p}{lead} []")),
        Json::Null => return out.push(format!("{p}{lead} null")),
        Json::String(s) => return out.push(format!("{p}{lead} {}", scalar(s))),
        other => return out.push(format!("{p}{lead} {}", js_stringify(other))),
    }
    if lead == "-" {
        // `- key: v` / `- - v`: the first child shares the dash's line.
        let mut nested = Vec::new();
        children(&mut nested);
        let first = nested.remove(0);
        out.push(format!("{p}- {}", &first[pad + 2..]));
        out.extend(nested);
    } else {
        out.push(format!("{p}{lead}"));
        children(out);
    }
}

/// A string body. Multi-line text is a `|` block when that reads back
/// exactly, otherwise a double-quoted string.
fn text_body(t: &str) -> String {
    if t.contains('\n') {
        if let Some(block) = block_scalar(t) {
            return block;
        }
        return quote_escaped(t);
    }
    scalar(t)
}

/// `|`, `|-` or `|+` by the trailing newlines, lines indented under
/// `body:`; `None` when the block would not read back as `t`.
fn block_scalar(t: &str) -> Option<String> {
    let chomp = match t.strip_suffix('\n') {
        None => "-",
        Some(rest) if rest.ends_with('\n') => "+",
        Some(_) => "",
    };
    let content = t.strip_suffix('\n').unwrap_or(t);
    let lines: Vec<String> = content
        .split('\n')
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{ENTRY}{l}")
            }
        })
        .collect();
    let block = format!("|{chomp}\n{}", lines.join("\n"));
    let check = format!("request:\n  body: {block}\n");
    reads_back(&check, &serde_json::json!({ "request": { "body": t } })).then_some(block)
}

/// A value scalar. `${var}` references stay plain where that reads back
/// the same (the studio's `unquote_references` rule); anything else gets
/// core's YAML quoting.
fn scalar(s: &str) -> String {
    if s.contains("${") && !s.contains(['\\', '"']) && scalar_text(s, false, false) == s {
        return s.to_string();
    }
    yaml_str(s)
}

/// A scalar inside an inline `{ ... }` map.
fn flow_scalar(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".into();
    }
    scalar_text(s, needs_quoting(s), true)
}

/// `# line`, or a bare `#` for an empty line, after `pad`.
fn comment(pad: &str, line: &str) -> String {
    if line.is_empty() {
        format!("{pad}#")
    } else {
        format!("{pad}# {line}")
    }
}

/// `s` split on everything YAML may read as a line break, so each piece
/// can sit on one comment line.
fn text_lines(s: &str) -> Vec<&str> {
    s.split("\r\n")
        .flat_map(|l| l.split(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}']))
        .collect()
}
