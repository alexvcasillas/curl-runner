//! `--debug` dump: the token stream, AST and IR as JSON in the key order the
//! TS objects had (`JSON.stringify` omits `undefined` and writes non-finite
//! numbers as `null`).

use super::types::{
    Auth, Body, CurlAst, CurlRunnerIr, FormField, FormFile, NumText, Source, Ssl, UnsupportedFlag,
};
use crate::model::Json;
use serde_json::{Map, Number};

pub fn object(entries: Vec<(&str, Json)>) -> Json {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k.to_string(), v);
    }
    Json::Object(m)
}

pub fn strings(items: &[String]) -> Json {
    Json::Array(items.iter().cloned().map(Json::String).collect())
}

fn s(v: &str) -> Json {
    Json::String(v.to_string())
}

fn num(n: f64) -> Json {
    Number::from_f64(n).map_or(Json::Null, Json::Number)
}

fn unsupported(flags: &[UnsupportedFlag]) -> Json {
    Json::Array(
        flags
            .iter()
            .map(|f| {
                let mut e = vec![("flag", s(&f.flag))];
                if let Some(v) = &f.value {
                    e.push(("value", s(v)));
                }
                object(e)
            })
            .collect(),
    )
}

pub fn ast(a: &CurlAst) -> Json {
    let mut m = Map::new();
    m.insert("url".into(), s(&a.url));
    m.insert(
        "headers".into(),
        Json::Array(
            a.headers
                .iter()
                .map(|(k, v)| object(vec![("key", s(k)), ("value", s(v))]))
                .collect(),
        ),
    );
    m.insert("unsupportedFlags".into(), unsupported(&a.unsupported_flags));

    let opt = |v: &Option<String>| v.as_deref().map(s);
    let list = |v: &Vec<String>| (!v.is_empty()).then(|| strings(v));
    for key in &a.order {
        let value: Option<Json> = match *key {
            "method" => opt(&a.method),
            "user" => opt(&a.user),
            "output" => opt(&a.output),
            "proxy" => opt(&a.proxy),
            "maxTime" => a.max_time.map(num),
            "maxRedirs" => a.max_redirs.map(num),
            "cacert" => opt(&a.cacert),
            "cert" => opt(&a.cert),
            "key" => opt(&a.key),
            "cookie" => opt(&a.cookie),
            "cookieJar" => opt(&a.cookie_jar),
            "userAgent" => opt(&a.user_agent),
            "referer" => opt(&a.referer),
            "get" => Some(Json::Bool(a.get)),
            "head" => Some(Json::Bool(a.head)),
            "location" => Some(Json::Bool(a.location)),
            "insecure" => Some(Json::Bool(a.insecure)),
            "compressed" => Some(Json::Bool(a.compressed)),
            "http2" => Some(Json::Bool(a.http2)),
            "silent" => Some(Json::Bool(a.silent)),
            "verbose" => Some(Json::Bool(a.verbose)),
            "data" => list(&a.data),
            "dataRaw" => list(&a.data_raw),
            "dataBinary" => list(&a.data_binary),
            "dataUrlencode" => list(&a.data_urlencode),
            "form" => list(&a.form),
            "formString" => list(&a.form_string),
            _ => None,
        };
        if let Some(v) = value {
            m.insert((*key).to_string(), v);
        }
    }
    Json::Object(m)
}

fn auth(a: &Auth) -> Json {
    let mut e = vec![("type", s(&a.kind))];
    for (k, v) in [
        ("username", &a.username),
        ("password", &a.password),
        ("token", &a.token),
    ] {
        if let Some(v) = v {
            e.push((k, s(v)));
        }
    }
    object(e)
}

fn form_file(f: &FormFile) -> Json {
    let mut e = vec![("file", s(&f.file))];
    let filename = f.filename.as_deref().map(|v| ("filename", s(v)));
    let ctype = f.content_type.as_deref().map(|v| ("contentType", s(v)));
    if f.type_first {
        e.extend(ctype);
        e.extend(filename);
    } else {
        e.extend(filename);
        e.extend(ctype);
    }
    object(e)
}

fn ssl(x: &Ssl) -> Json {
    let mut e = Vec::new();
    for (k, v) in [("ca", &x.ca), ("cert", &x.cert), ("key", &x.key)] {
        if let Some(v) = v {
            e.push((k, s(v)));
        }
    }
    object(e)
}

fn string_map(m: &super::jsutil::JsMap<String>) -> Json {
    object(
        m.entries()
            .into_iter()
            .map(|(k, v)| (k.as_str(), s(v)))
            .collect(),
    )
}

pub fn ir(i: &CurlRunnerIr) -> Json {
    let mut e: Vec<(&str, Json)> = vec![
        ("method", s(&i.method)),
        ("url", s(&i.url)),
        ("headers", string_map(&i.headers)),
    ];
    let mut meta = vec![
        (
            "source",
            s(match i.source {
                Source::Curl => "curl",
                Source::Yaml => "yaml",
            }),
        ),
        ("warnings", strings(&i.warnings)),
    ];
    if !i.unsupported_flags.is_empty() {
        meta.push(("unsupportedFlags", unsupported(&i.unsupported_flags)));
    }
    e.push(("metadata", object(meta)));

    if let Some(p) = i.params.as_ref().filter(|p| !p.is_empty()) {
        e.push(("params", string_map(p)));
    }
    if let Some(b) = &i.body {
        let content = match b {
            Body::Json(j) => j.clone(),
            Body::Urlencoded(c) | Body::Raw(c) | Body::Binary(c) => s(c),
        };
        e.push((
            "body",
            object(vec![("type", s(b.type_name())), ("content", content)]),
        ));
    }
    if let Some(f) = &i.form_data {
        e.push((
            "formData",
            object(
                f.entries()
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.as_str(),
                            match v {
                                FormField::Text(t) => s(t),
                                FormField::File(f) => form_file(f),
                            },
                        )
                    })
                    .collect(),
            ),
        ));
    }
    if let Some(a) = &i.auth {
        e.push(("auth", auth(a)));
    }
    if i.insecure {
        e.push(("insecure", Json::Bool(true)));
    }
    if let Some(f) = i.follow_redirects {
        e.push(("followRedirects", Json::Bool(f)));
    }
    if let Some(m) = &i.max_redirects {
        e.push((
            "maxRedirects",
            match m {
                NumText::Num(n) => num(*n),
                NumText::Text(t) => s(t),
            },
        ));
    }
    if let Some(t) = i.timeout {
        e.push(("timeout", num(t)));
    }
    if let Some(p) = &i.proxy {
        e.push(("proxy", s(p)));
    }
    if let Some(o) = &i.output {
        e.push(("output", s(o)));
    }
    if i.http2 {
        e.push(("http2", Json::Bool(true)));
    }
    if let Some(x) = &i.ssl {
        e.push(("ssl", ssl(x)));
    }
    if let Some(n) = &i.name {
        e.push(("name", s(n)));
    }
    object(e)
}
