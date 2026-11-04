//! OpenAPI operations mapped to the shared import model. One request per
//! operation, in `paths` order and then method order.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use serde_json::{Map, Value};

use super::doc::{Doc, Resolved};
use super::sample::{Sampler, UnresolvedRef};
use crate::convert::import::{
    Attention, Field, FieldValue, ImportReport, ImportedVar, Mapped, MappedBody, MappedRequest,
    var_name,
};
use crate::convert::{SECRET_PARAMS, header_var, param_var, secret_header};
use crate::model::Json;

const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// `Authorization` schemes kept in front of a scrubbed credential.
const AUTH_SCHEMES: [&str; 5] = ["Bearer", "Basic", "Token", "Digest", "ApiKey"];

/// The longest API prefix.
const MAX_PREFIX: usize = 24;

/// Most example request paths an aggregated entry keeps.
const MAX_EXAMPLES: usize = 3;

/// The variable prefix for an API titled `title`: uppercase `[A-Z0-9_]`,
/// any run of other characters as one `_`, cut at a word boundary to at
/// most 24 characters. `API` when nothing is left.
pub fn api_prefix(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let mut out = out.trim_matches('_');
    if out.len() > MAX_PREFIX {
        // ASCII only, so any byte index is a char boundary. The cut is at
        // the last `_` within the first 24 characters, so a `_` right after
        // them doesn't count: the spec pins "Swagger Petstore - OpenAPI
        // 3.0" (a `_` at index 24) to `SWAGGER_PETSTORE`.
        let head = &out[..MAX_PREFIX];
        out = head.rfind('_').map_or(head, |i| &head[..i]);
    }
    if out.is_empty() {
        "API".into()
    } else {
        out.to_string()
    }
}

/// Map every operation of `doc`. Parts that can't be read are reported
/// and skipped; the rest still map.
pub fn map_document(doc: &Doc) -> Mapped {
    let title = doc.root["info"]["title"].as_str().unwrap_or("");
    let mut m = Mapper::new(doc, api_prefix(title));
    m.servers();
    if let Some(paths) = doc.root.get("paths").and_then(Value::as_object) {
        for (path, raw) in paths {
            m.path_item(path, raw);
        }
    }
    let webhooks = doc.root.get("webhooks").and_then(Value::as_object);
    for _ in webhooks.into_iter().flatten() {
        m.not_converted("webhook");
    }
    m.finish()
}

/// Attention entries keyed by identity, emitted in first-seen order.
#[derive(Default)]
pub(super) struct Tally {
    items: Vec<Attention>,
    /// Key → (index in `items`, number of the request last counted).
    keys: BTreeMap<String, (usize, usize)>,
}

impl Tally {
    /// Add `a` unless an equal entry is already there.
    pub(super) fn push(&mut self, a: Attention) {
        if !self.items.contains(&a) {
            self.items.push(a);
        }
    }

    /// Count a hit of `key` from request number `req` at `at`. `make` gives
    /// the first entry, with a count of 1 and `at` as its example. With
    /// `per_request`, a request counts once.
    pub(super) fn hit(
        &mut self,
        key: String,
        req: usize,
        at: &str,
        per_request: bool,
        make: impl FnOnce() -> Attention,
    ) {
        match self.keys.get_mut(&key) {
            Some((i, last)) => {
                if per_request && *last == req {
                    return;
                }
                *last = req;
                bump(&mut self.items[*i], at);
            }
            None => {
                self.keys.insert(key, (self.items.len(), req));
                self.items.push(make());
            }
        }
    }
}

/// One more hit on an aggregated entry.
fn bump(a: &mut Attention, at: &str) {
    match a {
        Attention::BadParams { count, examples }
        | Attention::UnresolvedRef {
            count, examples, ..
        }
        | Attention::SkippedBody {
            count, examples, ..
        }
        | Attention::ParamStyle { count, examples }
        | Attention::CookieParams { count, examples }
        | Attention::BodyShapes { count, examples } => {
            *count += 1;
            if examples.len() < MAX_EXAMPLES && !examples.iter().any(|e| e == at) {
                examples.push(at.to_string());
            }
        }
        Attention::AuthChoice { count, .. }
        | Attention::UnsupportedScheme { count, .. }
        | Attention::OwnServer { count, .. }
        | Attention::NotConverted { count, .. } => *count += 1,
        _ => {}
    }
}

pub(super) struct Mapper<'d> {
    pub(super) doc: &'d Doc,
    pub(super) sampler: Sampler<'d>,
    pub(super) prefix: String,
    base_var: String,
    /// The base URL variable's value.
    base_url: String,
    pub(super) vars: Vec<ImportedVar>,
    env_comments: Vec<(String, String)>,
    pub(super) tally: Tally,
    /// Secrets an `AccessToken` entry already asks for.
    pub(super) tokens: Vec<String>,
    /// Tags that got a request.
    folders: Vec<String>,
    requests: Vec<MappedRequest>,
    /// Path of the request being mapped.
    pub(super) at: String,
    /// Its number, for counting each request once.
    pub(super) req: usize,
    /// Its unresolved refs, once each.
    req_refs: Vec<UnresolvedRef>,
}

impl<'d> Mapper<'d> {
    fn new(doc: &'d Doc, prefix: String) -> Self {
        Self {
            doc,
            sampler: Sampler::new(doc),
            base_var: var_name(&format!("{prefix}_BASE_URL")),
            prefix,
            base_url: String::new(),
            vars: Vec::new(),
            env_comments: Vec::new(),
            tally: Tally::default(),
            tokens: Vec::new(),
            folders: Vec::new(),
            requests: Vec::new(),
            at: String::new(),
            req: 0,
            req_refs: Vec::new(),
        }
    }

    /// The base URL variable from the first server; the others become
    /// `.env` comments.
    fn servers(&mut self) {
        let doc = self.doc;
        let servers = doc
            .root
            .get("servers")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        let base = servers
            .first()
            .map(|s| self.server_url(s, true))
            .unwrap_or_default();
        if !base.contains("://") {
            self.tally.push(Attention::SetVariable {
                var: self.base_var.clone(),
            });
        }
        self.vars.push(ImportedVar {
            name: self.base_var.clone(),
            value: base.clone(),
            secret: false,
        });
        self.base_url = base;
        for s in servers.iter().skip(1) {
            let url = self.server_url(s, false);
            if !url.is_empty() {
                self.env_comments
                    .push((self.base_var.clone(), format!("Other server: {url}")));
            }
        }
    }

    /// `s.url` with its variables replaced by their defaults and no
    /// trailing `/`. With `report`, a variable without a default is
    /// reported.
    fn server_url(&mut self, s: &Value, report: bool) -> String {
        let url = s.get("url").and_then(Value::as_str).unwrap_or("");
        let vars = s.get("variables");
        let mut out = String::new();
        let mut rest = url;
        while let Some(open) = rest.find('{') {
            let Some(len) = rest[open..].find('}') else {
                break;
            };
            let name = &rest[open + 1..open + len];
            out.push_str(&rest[..open]);
            match vars.and_then(|v| v.get(name)?.get("default")) {
                Some(Value::String(d)) => out.push_str(d),
                Some(d @ (Value::Number(_) | Value::Bool(_))) => out.push_str(&d.to_string()),
                _ => {
                    if report {
                        self.tally.push(Attention::ServerVariable {
                            name: name.to_string(),
                        });
                    }
                    out.push_str(&rest[open..=open + len]);
                }
            }
            rest = &rest[open + len + 1..];
        }
        out.push_str(rest);
        out.trim_end_matches('/').to_string()
    }

    fn path_item(&mut self, path: &str, raw: &'d Value) {
        let bad = |reason: String| Attention::BadOperation {
            at: path.to_string(),
            reason,
        };
        let item = match self.doc.resolve(raw) {
            Resolved::Value(v) if v.is_object() => v,
            Resolved::Value(_) => {
                self.tally.push(bad("it isn't a path item object".into()));
                return;
            }
            Resolved::Missing(t) => {
                self.tally.push(bad(format!("couldn't resolve {t}")));
                return;
            }
            Resolved::External(t) => {
                self.tally
                    .push(bad(format!("can't follow the external reference {t}")));
                return;
            }
        };
        for method in METHODS {
            let Some(op) = item.get(method) else {
                continue;
            };
            if !op.is_object() {
                self.tally.push(Attention::BadOperation {
                    at: format!("{} {path}", method.to_ascii_uppercase()),
                    reason: "it isn't an operation object".into(),
                });
                continue;
            }
            let r = self.operation(path, method, op, item);
            self.requests.push(r);
        }
    }

    fn operation(
        &mut self,
        path: &str,
        method: &str,
        op: &'d Value,
        item: &'d Value,
    ) -> MappedRequest {
        let method = method.to_ascii_uppercase();
        let text = |v: &Value, k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let name = text(op, "summary")
            .or_else(|| text(op, "operationId"))
            .unwrap_or_else(|| format!("{method} {path}"));
        let tag = op
            .get("tags")
            .and_then(Value::as_array)
            .and_then(|t| t.first())
            .and_then(Value::as_str)
            .filter(|t| !t.is_empty());
        let folder = tag.or_else(|| path_folder(path));
        let mut r = MappedRequest {
            folders: folder.map(|t| vec![t.to_string()]).unwrap_or_default(),
            name,
            comments: Vec::new(),
            method,
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body: MappedBody::None,
            auth: None,
            file_vars: Vec::new(),
            expect_status: expect_status(op),
        };
        if let Some(t) = folder
            && !self.folders.iter().any(|f| f == t)
        {
            self.folders.push(t.to_string());
        }
        self.at = r.path();
        self.req = self.requests.len();

        r.comments = comment_lines(op.get("description"));
        if let Some(id) = text(op, "operationId") {
            r.comments.push(format!("operationId: {id}"));
        }
        if op.get("deprecated").and_then(Value::as_bool) == Some(true) {
            r.comments.push("Deprecated".into());
        }
        if let Some(url) = op.get("externalDocs").and_then(|d| text(d, "url")) {
            r.comments.push(url);
        }
        if let Some(t) = tag {
            let tags = self.doc.root.get("tags").and_then(Value::as_array);
            let described = tags
                .into_iter()
                .flatten()
                .find(|x| x.get("name").and_then(Value::as_str) == Some(t));
            r.comments
                .extend(comment_lines(described.and_then(|x| x.get("description"))));
        }

        let own = [op, item]
            .into_iter()
            .find_map(|v| v.get("servers")?.as_array()?.first());
        let base = match own.map(|s| self.server_url(s, true)) {
            Some(url) if url != self.base_url => {
                self.tally
                    .hit(format!("server\0{url}"), self.req, &self.at, true, || {
                        Attention::OwnServer {
                            url: url.clone(),
                            count: 1,
                        }
                    });
                url
            }
            _ => format!("${{{}}}", self.base_var),
        };

        let mut cookies = Vec::new();
        self.security(op, &mut r, &mut cookies);
        let path_values = self.params(op, item, &mut r, &mut cookies);
        if !cookies.is_empty() {
            r.headers
                .push(text_field("Cookie", cookies.join("; "), false, None));
        }
        r.url = format!("{base}{}", self.path_template(path, &path_values, &mut r));
        self.body(op, &mut r);

        let callbacks = op.get("callbacks").and_then(Value::as_object);
        for _ in callbacks.into_iter().flatten() {
            self.not_converted("callback");
        }
        let responses = op.get("responses").and_then(Value::as_object);
        for resp in responses.into_iter().flat_map(Map::values) {
            if let Resolved::Value(resp) = self.doc.resolve(resp)
                && let Some(links) = resp.get("links").and_then(Value::as_object)
            {
                for _ in links {
                    self.not_converted("link");
                }
            }
        }

        self.flush_refs();
        r
    }

    /// `path` with each `{p}` as `${p}`, and `r.file_vars` filled from
    /// `values`.
    fn path_template(
        &mut self,
        path: &str,
        values: &[(String, String)],
        r: &mut MappedRequest,
    ) -> String {
        let mut out = String::new();
        let mut rest = path;
        while let Some(open) = rest.find('{') {
            let Some(len) = rest[open..].find('}') else {
                break;
            };
            let raw = &rest[open + 1..open + len];
            let name = var_name(raw);
            if name != raw {
                self.tally.push(Attention::Renamed {
                    from: raw.to_string(),
                    to: name.clone(),
                });
            }
            out.push_str(&rest[..open]);
            out.push_str(&format!("${{{name}}}"));
            if !r.file_vars.iter().any(|(n, _)| *n == name) {
                let value = values
                    .iter()
                    .find(|(n, _)| n == raw)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                r.file_vars.push((name, value));
            }
            rest = &rest[open + len + 1..];
        }
        out.push_str(rest);
        out
    }

    /// Query and header parameters into `r`, cookies into `cookies`.
    /// Returns the path parameters' values by their raw names.
    fn params(
        &mut self,
        op: &'d Value,
        item: &'d Value,
        r: &mut MappedRequest,
        cookies: &mut Vec<String>,
    ) -> Vec<(String, String)> {
        // Path-item parameters, then the operation's; the operation's win
        // on the same `name` + `in`.
        let mut merged: Vec<&'d Value> = Vec::new();
        for raw in [item, op]
            .into_iter()
            .filter_map(|v| v.get("parameters")?.as_array())
            .flatten()
        {
            let Some(p) = self.resolve(raw) else {
                continue;
            };
            let key = |p: &Value| (p.get("name").cloned(), p.get("in").cloned());
            match merged.iter_mut().find(|m| key(m) == key(p)) {
                Some(slot) => *slot = p,
                None => merged.push(p),
            }
        }

        let mut path_values = Vec::new();
        for p in merged {
            let (Some(name), Some(place @ ("path" | "query" | "header" | "cookie"))) = (
                p.get("name")
                    .and_then(Value::as_str)
                    .filter(|n| !n.is_empty()),
                p.get("in").and_then(Value::as_str),
            ) else {
                self.tally
                    .hit("badparam".into(), self.req, &self.at, false, || {
                        Attention::BadParams {
                            count: 1,
                            examples: vec![self.at.clone()],
                        }
                    });
                continue;
            };
            let required =
                place == "path" || p.get("required").and_then(Value::as_bool) == Some(true);
            let sample = self.param_sample(p);
            // A disabled param is a commented-out line; its style matters
            // only once someone turns it on.
            if required && needs_style(p, place, &sample) {
                self.tally
                    .hit("style".into(), self.req, &self.at, false, || {
                        Attention::ParamStyle {
                            count: 1,
                            examples: vec![self.at.clone()],
                        }
                    });
            }
            let value = to_text(&sample);
            let note = note(p.get("description"));
            match place {
                "path" => {
                    // Path segments are mostly ids, so only a literal the
                    // document wrote counts as a credential.
                    let value = if is_secret_param(name) && self.has_literal(p) {
                        self.credential(&param_var(name), &value, false)
                    } else {
                        value
                    };
                    path_values.push((name.to_string(), value));
                }
                "query" => {
                    if r.params.iter().any(|f| f.key == name) {
                        continue;
                    }
                    let value = if is_secret_param(name) {
                        self.credential(&param_var(name), &value, false)
                    } else {
                        value
                    };
                    r.params.push(text_field(name, value, !required, note));
                }
                "header" => {
                    // The body sets it, and OpenAPI ignores it as a parameter.
                    if name.eq_ignore_ascii_case("content-type")
                        || r.headers.iter().any(|f| f.key.eq_ignore_ascii_case(name))
                    {
                        continue;
                    }
                    let value = if secret_header(name) {
                        let auth = name.eq_ignore_ascii_case("authorization");
                        self.credential(&header_var(name), &value, auth)
                    } else {
                        value
                    };
                    r.headers.push(text_field(name, value, !required, note));
                }
                "cookie" if required => {
                    // An auth scheme may already send it.
                    let set = format!("{name}=");
                    if cookies.iter().any(|c| c.starts_with(&set)) {
                        continue;
                    }
                    // A `Cookie` header is a secret, so every value is.
                    let value = self.credential(&param_var(name), &value, false);
                    cookies.push(format!("{name}={value}"));
                }
                "cookie" => {
                    self.tally
                        .hit("cookies".into(), self.req, &self.at, true, || {
                            Attention::CookieParams {
                                count: 1,
                                examples: vec![self.at.clone()],
                            }
                        });
                }
                _ => unreachable!("matched above"),
            }
        }
        path_values
    }

    /// Whether the document itself gives `p` a value: an example, or a
    /// schema `example`, `examples`, `default`, `enum` or `const`.
    fn has_literal(&self, p: &Value) -> bool {
        if p.get("example").is_some() || p.get("examples").is_some() {
            return true;
        }
        let Some(Resolved::Value(s)) = p.get("schema").map(|s| self.doc.resolve(s)) else {
            return false;
        };
        ["example", "examples", "default", "enum", "const"]
            .iter()
            .any(|k| s.get(*k).is_some())
    }

    /// A parameter's sample: its example, else one from its schema or its
    /// `content`'s first media type.
    fn param_sample(&mut self, p: &'d Value) -> Json {
        if let Some(v) = self.sampler.example(p) {
            return v;
        }
        if let Some(schema) = p.get("schema") {
            return self.sampler.value(schema);
        }
        let media = p
            .get("content")
            .and_then(Value::as_object)
            .and_then(|c| c.values().next());
        match media {
            Some(m) => match self.sampler.example(m) {
                Some(v) => v,
                None => m
                    .get("schema")
                    .map_or(Json::Null, |s| self.sampler.value(s)),
            },
            None => Json::Null,
        }
    }

    /// `${PREFIX_BASE}` as a new empty secret, for a value that names a
    /// credential. The value itself is never kept. With `scheme`, a
    /// leading auth scheme like `Bearer ` stays.
    pub(super) fn credential(&mut self, base: &str, value: &str, scheme: bool) -> String {
        let var = self.secret_var(&format!("{}_{base}", self.prefix));
        match value.split_once(' ') {
            Some((s, _)) if scheme && AUTH_SCHEMES.iter().any(|k| k.eq_ignore_ascii_case(s)) => {
                format!("{s} ${{{var}}}")
            }
            _ => format!("${{{var}}}"),
        }
    }

    /// Register `name` as an empty secret; returns its final name.
    pub(super) fn secret_var(&mut self, name: &str) -> String {
        self.add_var(name, true)
    }

    /// Register `name` as an empty variable, secret or not. A name already
    /// there is reused, and stays secret once one use is.
    pub(super) fn add_var(&mut self, name: &str, secret: bool) -> String {
        let name = var_name(name);
        match self.vars.iter_mut().find(|v| v.name == name) {
            Some(v) => v.secret |= secret,
            None => self.vars.push(ImportedVar {
                name: name.clone(),
                value: String::new(),
                secret,
            }),
        }
        name
    }

    /// Follow `v`'s `$ref`s. A reference that can't be followed is noted
    /// for the current request and gives `None`.
    pub(super) fn resolve(&mut self, v: &'d Value) -> Option<&'d Value> {
        match self.doc.resolve(v) {
            Resolved::Value(v) => Some(v),
            Resolved::Missing(target) => {
                self.unresolved(target, false);
                None
            }
            Resolved::External(target) => {
                self.unresolved(target, true);
                None
            }
        }
    }

    pub(super) fn unresolved(&mut self, target: String, external: bool) {
        self.sync_refs();
        let r = UnresolvedRef { target, external };
        if !self.req_refs.contains(&r) {
            self.req_refs.push(r);
        }
    }

    /// Take the sampler's unresolved refs into the current request's.
    fn sync_refs(&mut self) {
        for r in self.sampler.take_hits() {
            if !self.req_refs.contains(&r) {
                self.req_refs.push(r);
            }
        }
    }

    /// Count the current request's unresolved refs.
    fn flush_refs(&mut self) {
        self.sync_refs();
        for UnresolvedRef { target, external } in std::mem::take(&mut self.req_refs) {
            let at = self.at.clone();
            self.tally
                .hit(format!("ref\0{target}"), self.req, &at, true, || {
                    Attention::UnresolvedRef {
                        target,
                        external,
                        count: 1,
                        examples: vec![at.clone()],
                    }
                });
        }
    }

    fn not_converted(&mut self, what: &str) {
        self.tally
            .hit(format!("skip\0{what}"), self.req, &self.at, false, || {
                Attention::NotConverted {
                    what: what.to_string(),
                    count: 1,
                }
            });
    }

    fn finish(mut self) -> Mapped {
        for v in &self.vars {
            if !v.value.is_empty() || v.name == self.base_var {
                continue;
            }
            if !v.secret {
                self.tally.push(Attention::SetVariable {
                    var: v.name.clone(),
                });
            } else if !self.tokens.contains(&v.name) {
                self.tally.push(Attention::SetSecret {
                    var: v.name.clone(),
                });
            }
        }
        let description = self.doc.root["info"]["description"]
            .as_str()
            .map(|d| lines(d).join("\n"))
            .filter(|d| !d.is_empty());
        Mapped {
            report: ImportReport {
                folders: self.folders.len(),
                requests: self.requests.len(),
                attention: self.tally.items,
            },
            requests: self.requests,
            variables: self.vars,
            description,
            env_comments: self.env_comments,
        }
    }
}

/// The lowest documented `2xx`, else `200` for `2XX`.
fn expect_status(op: &Value) -> Option<u16> {
    let responses = op.get("responses")?.as_object()?;
    responses
        .keys()
        .filter_map(|k| k.parse::<u16>().ok())
        .filter(|s| (200..300).contains(s))
        .min()
        .or_else(|| {
            responses
                .keys()
                .any(|k| k.eq_ignore_ascii_case("2XX"))
                .then_some(200)
        })
}

/// A parameter whose `style`/`explode` the plain `key=a,b` form can't
/// express: a non-default style, a Swagger `tsv` collection format, an
/// object value, or an exploded array of more than one item.
fn needs_style(p: &Value, place: &str, sample: &Json) -> bool {
    if p.get("x-collectionFormat").is_some() {
        return true;
    }
    let default = if matches!(place, "query" | "cookie") {
        "form"
    } else {
        "simple"
    };
    let style = p.get("style").and_then(Value::as_str).unwrap_or(default);
    if style != default {
        return true;
    }
    let explode = p
        .get("explode")
        .and_then(Value::as_bool)
        .unwrap_or(style == "form");
    match sample {
        Json::Object(_) => true,
        Json::Array(items) => explode && items.len() > 1,
        _ => false,
    }
}

pub(super) fn is_secret_param(name: &str) -> bool {
    SECRET_PARAMS.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// A sample as a query, header or form value: arrays joined with `,`.
pub(super) fn to_text(v: &Json) -> String {
    match v {
        Json::String(s) => s.clone(),
        Json::Null => String::new(),
        Json::Array(items) => items.iter().map(to_text).collect::<Vec<_>>().join(","),
        other => other.to_string(),
    }
}

/// A media type without parameters, lowercased.
pub(super) fn essence(media: &str) -> String {
    media
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

pub(super) fn text_field(key: &str, value: String, disabled: bool, note: Option<String>) -> Field {
    Field {
        key: key.into(),
        value: FieldValue::Text(value),
        disabled,
        note,
    }
}

/// A description's non-blank lines, as plain text.
fn comment_lines(d: Option<&Value>) -> Vec<String> {
    let mut out = d.and_then(Value::as_str).map(lines).unwrap_or_default();
    out.retain(|l| !l.trim().is_empty());
    out
}

/// `text` as plain text, without blank lines at the edges. Inner blank
/// lines stay, so a README keeps its paragraphs.
fn lines(text: &str) -> Vec<String> {
    let plain = plain_text(text);
    let all: Vec<&str> = plain.lines().map(str::trim_end).collect();
    let Some(first) = all.iter().position(|l| !l.trim().is_empty()) else {
        return Vec::new();
    };
    let last = all
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .unwrap_or(first);
    all[first..=last].iter().map(|l| l.to_string()).collect()
}

/// HTML tags removed, then the common entities decoded. Only tag-shaped
/// text is removed, so `a < b` stays.
fn plain_text(text: &str) -> String {
    static TAG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"</?[A-Za-z][A-Za-z0-9-]*(\s[^<>]*)?/?>").expect("a valid pattern")
    });
    TAG.replace_all(text, "")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// The folder for an untagged operation: the path's first segment that
/// isn't a version (`v1`, `v2.1`), `api` or a `{template}`.
fn path_folder(path: &str) -> Option<&str> {
    let version = |s: &str| {
        s.strip_prefix(['v', 'V']).is_some_and(|rest| {
            rest.split('.')
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
    };
    path.split('/').find(|s| {
        !s.is_empty() && !version(s) && !s.eq_ignore_ascii_case("api") && !s.starts_with('{')
    })
}

/// A description on one line, as plain text.
pub(super) fn note(d: Option<&Value>) -> Option<String> {
    let text = plain_text(d?.as_str()?)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}
