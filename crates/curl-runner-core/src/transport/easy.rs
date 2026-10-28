//! `RequestConfig` -> libcurl easy handle (`core/curl/args-builder.ts` option mapping).
//!
//! Also holds the request-shaping helpers shared with `curl_command` so the
//! dry-run command and the real transfer can never disagree.

use super::{TransportError, resolve_local_path};
use crate::model::{FormFieldValue, GlobalConfig, Json, RequestConfig};
use crate::security::{self, GuardError, path_guard};
use curl::easy::{Easy2, Form, Handler, HttpVersion, List, SslOpt, WriteError};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

/// curl tool defaults that libcurl does not apply on its own.
const CURL_TOOL_MAX_REDIRS: u32 = 50;
const CURL_TOOL_KEEPALIVE_SECS: u64 = 60;
/// TS `PooledCurlExecutor` defaults (seconds).
const POOL_KEEPALIVE_SECS: u64 = 60;
pub(crate) const POOL_CONNECT_TIMEOUT_SECS: u64 = 30;

/// libcurl handler: accumulates raw header lines and body bytes. With
/// `output` set the body streams to that file instead (curl `-o`), leaving the
/// in-memory body empty like TS.
#[derive(Debug, Default)]
pub struct Collector {
    headers: Vec<u8>,
    body: Vec<u8>,
    output: Option<OutputSink>,
}

#[derive(Debug)]
struct OutputSink {
    path: PathBuf,
    file: Option<File>,
    error: Option<String>,
}

impl Collector {
    fn with_output(path: Option<PathBuf>) -> Self {
        Collector {
            output: path.map(|path| OutputSink {
                path,
                file: None,
                error: None,
            }),
            ..Default::default()
        }
    }

    /// Why writing the `output` file failed (the transfer then fails with
    /// CURLE_WRITE_ERROR); use this message instead of libcurl's.
    pub fn output_error(&self) -> Option<&str> {
        self.output.as_ref()?.error.as_deref()
    }

    /// Raw header bytes as delivered by libcurl (every block, `-D -` format).
    pub fn header_bytes(&self) -> &[u8] {
        &self.headers
    }

    /// Raw body bytes (empty when streamed to `output`).
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// curl creates the `-o` file even for an empty body. Errors land in
    /// `output_error`.
    pub(crate) fn ensure_output_file(&mut self) {
        if let Some(out) = &mut self.output
            && out.file.is_none()
            && out.error.is_none()
        {
            match File::create(&out.path) {
                Ok(f) => out.file = Some(f),
                Err(e) => out.error = Some(output_err(&out.path, &e)),
            }
        }
    }
}

fn output_err(path: &Path, e: &std::io::Error) -> String {
    format!("Failed to write output file {}: {e}", path.display())
}

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        let Some(out) = &mut self.output else {
            self.body.extend_from_slice(data);
            return Ok(data.len());
        };
        if out.file.is_none() {
            match File::create(&out.path) {
                Ok(f) => out.file = Some(f),
                Err(e) => {
                    out.error = Some(output_err(&out.path, &e));
                    return Ok(0);
                }
            }
        }
        let file = out.file.as_mut().expect("output file opened above");
        match file.write_all(data) {
            Ok(()) => Ok(data.len()),
            Err(e) => {
                out.error = Some(output_err(&out.path, &e));
                Ok(0)
            }
        }
    }

    fn header(&mut self, data: &[u8]) -> bool {
        self.headers.extend_from_slice(data);
        true
    }
}

/// TS truthiness for optional strings: `None` and `""` are both absent.
pub(crate) fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().filter(|s| !s.is_empty())
}

/// HTTP method; TS `config.method || 'GET'`.
pub(crate) fn method(req: &RequestConfig) -> &str {
    req.method
        .as_deref()
        .filter(|m| !m.is_empty())
        .unwrap_or("GET")
}

/// Final URL with `params` appended (TS `URLSearchParams` encoding).
pub(crate) fn request_url(req: &RequestConfig) -> String {
    let mut url = req.url.clone();
    if let Some(params) = req.params.as_ref().filter(|p| !p.is_empty()) {
        let qs = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(&qs);
    }
    url
}

/// Body to send: TS `if (config.body)` (null/false/0/"" send nothing);
/// strings verbatim, anything else `JSON.stringify`. Never used with formData.
pub(crate) fn body_payload(req: &RequestConfig) -> Option<String> {
    if req.form_data.is_some() {
        return None;
    }
    match req.body.as_ref()? {
        Json::Null | Json::Bool(false) => None,
        Json::Number(n) if n.as_f64() == Some(0.0) => None,
        Json::String(s) if s.is_empty() => None,
        Json::String(s) => Some(s.clone()),
        // JS `JSON.stringify` (number text, integer-like keys first).
        other => Some(crate::snapshot::js_stringify(other)),
    }
}

/// Basic-auth `user:pass` when both are non-empty.
pub(crate) fn basic_auth(req: &RequestConfig) -> Option<(&str, &str)> {
    let a = req.auth.as_ref().filter(|a| a.auth_type == "basic")?;
    let u = a.username.as_deref().filter(|s| !s.is_empty())?;
    let p = a.password.as_deref().filter(|s| !s.is_empty())?;
    Some((u, p))
}

/// `-H` lines from `headers`.
pub(crate) fn user_header_lines(req: &RequestConfig) -> Vec<String> {
    req.headers
        .iter()
        .flatten()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect()
}

/// Bearer auth header line, when configured with a non-empty token.
pub(crate) fn bearer_line(req: &RequestConfig) -> Option<String> {
    let a = req.auth.as_ref().filter(|a| a.auth_type == "bearer")?;
    let t = a.token.as_deref().filter(|t| !t.is_empty())?;
    Some(format!("Authorization: Bearer {t}"))
}

/// Auto `Content-Type: application/json` when a body is sent and the exact
/// `Content-Type` key is absent or empty (TS check is case-sensitive).
pub(crate) fn needs_json_content_type(req: &RequestConfig) -> bool {
    let has_ct = req
        .headers
        .as_ref()
        .and_then(|h| h.get("Content-Type"))
        .is_some_and(|v| !v.is_empty());
    !has_ct && body_payload(req).is_some()
}

pub(crate) const JSON_CONTENT_TYPE: &str = "Content-Type: application/json";

/// Request header lines in TS order: user headers, bearer auth, auto
/// Content-Type.
fn header_lines(req: &RequestConfig) -> Vec<String> {
    let mut lines = user_header_lines(req);
    lines.extend(bearer_line(req));
    if needs_json_content_type(req) {
        lines.push(JSON_CONTENT_TYPE.to_string());
    }
    lines
}

/// JS `String(value)` for non-file form fields.
pub(crate) fn form_value_string(v: &FormFieldValue) -> String {
    match v {
        FormFieldValue::Bool(b) => b.to_string(),
        FormFieldValue::Number(n) => n.to_string(),
        FormFieldValue::Text(s) => s.clone(),
        FormFieldValue::File(f) => f.file.clone(),
    }
}

/// `-k`: `insecure: true` or `ssl.verify: false`.
pub(crate) fn insecure(req: &RequestConfig) -> bool {
    req.insecure == Some(true) || req.ssl.as_ref().and_then(|s| s.verify) == Some(false)
}

/// `followRedirects !== false`.
pub(crate) fn follows_redirects(req: &RequestConfig) -> bool {
    req.follow_redirects != Some(false)
}

/// Option-setting failure, formatted like curl's stderr.
fn cv(e: curl::Error) -> TransportError {
    let code = super::curl_code(&e);
    TransportError {
        code: Some(code),
        message: format!(
            "curl: ({code}) {}",
            e.extra_description().unwrap_or(e.description())
        ),
    }
}

fn guard_err(e: GuardError) -> TransportError {
    TransportError {
        code: None,
        message: e.message,
    }
}

// `CURLOPT_PROTOCOLS_STR` / `CURLOPT_REDIR_PROTOCOLS_STR` (libcurl >= 7.85):
// `CURLOPTTYPE_STRINGPOINT` (= OBJECTPOINT, 10000) + 318 / 319, verified
// against curl-sys's bundled `curl/curl.h`. The `curl` crate has no setter.
const CURLOPT_PROTOCOLS_STR: curl_sys::CURLoption = curl_sys::CURLOPTTYPE_OBJECTPOINT + 318;
const CURLOPT_REDIR_PROTOCOLS_STR: curl_sys::CURLoption = curl_sys::CURLOPTTYPE_OBJECTPOINT + 319;

/// Restrict transfer and redirect protocols to the allow-list: defence in
/// depth against redirects to `file://` and friends (TS `--proto` /
/// `--proto-redir`). libcurl rejects the whole list when it names a protocol
/// this build lacks (e.g. `ftp`), so the list is intersected with the
/// supported protocols; an unsupported entry simply stays blocked.
fn set_protocols(easy: &Easy2<Collector>, allowed: &[String]) -> Result<(), TransportError> {
    apply_protocols(
        easy.raw(),
        &[CURLOPT_PROTOCOLS_STR, CURLOPT_REDIR_PROTOCOLS_STR],
        allowed,
    )
    .map_err(|e| match e {
        None => TransportError {
            code: None,
            message: "Invalid protocol allow-list".into(),
        },
        Some(e) => cv(e),
    })
}

/// TLS trust setup shared by every libcurl handle we build (requests, the
/// version check and `upgrade` downloads): the static rustls libcurl has no
/// CA bundle, so verify against the OS trust store.
pub fn native_ca_options() -> SslOpt {
    let mut o = SslOpt::new();
    o.native_ca(true);
    o
}

/// Limit the protocols a redirect may switch to on a plain `curl::easy::Easy`
/// handle (the start URL's protocol is unaffected).
pub fn set_redirect_protocols(
    easy: &curl::easy::Easy,
    allowed: &[&str],
) -> Result<(), curl::Error> {
    let allowed: Vec<String> = allowed.iter().map(|s| s.to_string()).collect();
    apply_protocols(easy.raw(), &[CURLOPT_REDIR_PROTOCOLS_STR], &allowed)
        .map_err(|e| e.unwrap_or_else(|| curl::Error::new(curl_sys::CURLE_BAD_FUNCTION_ARGUMENT)))
}

/// The protocols this libcurl build supports, lowercase (computed once).
pub(crate) fn supported_protocols() -> &'static [String] {
    static SUPPORTED: LazyLock<Vec<String>> = LazyLock::new(|| {
        curl::Version::get()
            .protocols()
            .map(|p| p.to_ascii_lowercase())
            .collect()
    });
    &SUPPORTED
}

/// `Err(None)` is an interior NUL in the list.
fn apply_protocols(
    raw: *mut curl_sys::CURL,
    opts: &[curl_sys::CURLoption],
    allowed: &[String],
) -> Result<(), Option<curl::Error>> {
    let supported = supported_protocols();
    let usable: Vec<String> = allowed
        .iter()
        .map(|p| p.to_ascii_lowercase())
        .filter(|p| supported.contains(p))
        .collect();
    let list = std::ffi::CString::new(usable.join(",")).map_err(|_| None)?;
    for &opt in opts {
        // SAFETY: `raw` is a live easy handle; libcurl copies string
        // options, and `list` outlives the call.
        let rc = unsafe { curl_sys::curl_easy_setopt(raw, opt, list.as_ptr()) };
        if rc != curl_sys::CURLE_OK {
            return Err(Some(curl::Error::new(rc)));
        }
    }
    Ok(())
}

/// Build a configured easy handle (not yet performed). Usable standalone
/// (`send`) or added to a `curl::multi::Multi`, then passed to `finish`.
pub fn build_easy(
    req: &RequestConfig,
    global: &GlobalConfig,
    cwd: &Path,
) -> Result<Easy2<Collector>, TransportError> {
    // Guards run before any filesystem or network access (TS order: paths,
    // then URL protocol).
    security::check_request(req, global, cwd).map_err(guard_err)?;
    let allow = security::allow_paths(global);
    let output = match non_empty(&req.output) {
        Some(o) => Some(path_guard::resolve_safe(o, cwd, cwd, allow).map_err(guard_err)?),
        None => None,
    };
    let mut easy = Easy2::new(Collector::with_output(output));

    easy.url(&request_url(req)).map_err(cv)?;
    set_protocols(&easy, &security::effective_protocols(global))?;
    let m = method(req);
    easy.custom_request(m).map_err(cv)?;
    if m.eq_ignore_ascii_case("HEAD") {
        easy.nobody(true).map_err(cv)?;
    }
    easy.useragent(&format!("curl/{}", curl::Version::get().version()))
        .map_err(cv)?;

    let mut list = List::new();
    for line in header_lines(req) {
        list.append(&line).map_err(cv)?;
    }
    easy.http_headers(list).map_err(cv)?;

    if let Some((u, p)) = basic_auth(req) {
        easy.username(u).map_err(cv)?;
        easy.password(p).map_err(cv)?;
    }

    if let Some(fields) = &req.form_data {
        if !fields.is_empty() {
            easy.httppost(build_form(fields, cwd, allow)?).map_err(cv)?;
        }
    } else if let Some(body) = body_payload(req) {
        easy.post_fields_copy(body.as_bytes()).map_err(cv)?;
    }

    if let Some(ms) = req.timeout.filter(|t| *t > 0) {
        easy.timeout(Duration::from_millis(ms)).map_err(cv)?;
    }

    if follows_redirects(req) {
        easy.follow_location(true).map_err(cv)?;
        let max = req
            .max_redirects
            .filter(|m| *m > 0)
            .map_or(CURL_TOOL_MAX_REDIRS, |m| {
                u32::try_from(m).unwrap_or(u32::MAX)
            });
        easy.max_redirections(max).map_err(cv)?;
    }

    if let Some(proxy) = non_empty(&req.proxy) {
        easy.proxy(proxy).map_err(cv)?;
    }

    if insecure(req) {
        easy.ssl_verify_peer(false).map_err(cv)?;
        easy.ssl_verify_host(false).map_err(cv)?;
    }
    let ca = req.ssl.as_ref().and_then(|s| non_empty(&s.ca));
    if ca.is_none() {
        // The static rustls libcurl has no default CA bundle: without this
        // every HTTPS handshake fails ("no server certificate verifier").
        // Use the OS trust store, like the system curl does.
        easy.ssl_options(&native_ca_options()).map_err(cv)?;
    }
    if let Some(ssl) = &req.ssl {
        if let Some(ca) = ca {
            easy.cainfo(resolve_local_path(ca, cwd)?).map_err(cv)?;
        }
        if let Some(cert) = non_empty(&ssl.cert) {
            easy.ssl_cert(resolve_local_path(cert, cwd)?).map_err(cv)?;
        }
        if let Some(key) = non_empty(&ssl.key) {
            easy.ssl_key(resolve_local_path(key, cwd)?).map_err(cv)?;
        }
    }

    // Connection pool settings apply only when pooling is enabled (TS uses
    // them solely in PooledCurlExecutor, which also forces --http2).
    let pool = global
        .connection_pool
        .as_ref()
        .filter(|p| p.enabled == Some(true));
    if req.http2 == Some(true) || pool.is_some() {
        easy.http_version(HttpVersion::V2).map_err(cv)?;
    }
    let keepalive = pool.map_or(CURL_TOOL_KEEPALIVE_SECS, |p| {
        p.keepalive_time.unwrap_or(POOL_KEEPALIVE_SECS)
    });
    easy.tcp_keepalive(true).map_err(cv)?;
    if keepalive > 0 {
        easy.tcp_keepidle(Duration::from_secs(keepalive))
            .map_err(cv)?;
        easy.tcp_keepintvl(Duration::from_secs(keepalive))
            .map_err(cv)?;
    }
    if let Some(p) = pool {
        let secs = p.connect_timeout.unwrap_or(POOL_CONNECT_TIMEOUT_SECS);
        easy.connect_timeout(Duration::from_secs(secs))
            .map_err(cv)?;
    }

    Ok(easy)
}

/// Content-Type of a file part without an explicit `contentType`, as the curl
/// tool (`-F 'f=@path;filename=x'`, libcurl mime API) picks it: guessed from
/// the part filename (custom, else the path's basename), then from the path,
/// else `application/octet-stream`. libcurl's `curl_formadd` (used by the
/// `curl` crate) would guess from the path only.
pub(crate) fn file_part_content_type(filename: Option<&str>, path: &Path) -> &'static str {
    filename
        .and_then(guess_content_type)
        .or_else(|| guess_content_type(&path.to_string_lossy()))
        .unwrap_or("application/octet-stream")
}

/// libcurl `Curl_mime_contenttype`: case-insensitive extension table.
fn guess_content_type(name: &str) -> Option<&'static str> {
    const TYPES: [(&str, &str); 10] = [
        (".gif", "image/gif"),
        (".jpg", "image/jpeg"),
        (".jpeg", "image/jpeg"),
        (".png", "image/png"),
        (".svg", "image/svg+xml"),
        (".txt", "text/plain"),
        (".htm", "text/html"),
        (".html", "text/html"),
        (".pdf", "application/pdf"),
        (".xml", "application/xml"),
    ];
    let n = name.as_bytes();
    TYPES
        .iter()
        .find(|(ext, _)| {
            n.len() >= ext.len() && n[n.len() - ext.len()..].eq_ignore_ascii_case(ext.as_bytes())
        })
        .map(|(_, t)| *t)
}

fn build_form(
    fields: &crate::model::FormDataConfig,
    cwd: &Path,
    allow: bool,
) -> Result<Form, TransportError> {
    let mut form = Form::new();
    let mut missing = Vec::new();
    for (name, value) in fields {
        let added = match value {
            FormFieldValue::File(f) => {
                let path = path_guard::resolve_safe(&f.file, cwd, cwd, allow).map_err(|e| {
                    guard_err(GuardError::new(format!("formData field \"{name}\": {e}")))
                })?;
                if !path.is_file() {
                    missing.push(format!("{name}: {}", f.file));
                    continue;
                }
                let filename = non_empty(&f.filename);
                let ct = non_empty(&f.content_type)
                    .unwrap_or_else(|| file_part_content_type(filename, &path));
                let mut part = form.part(name);
                part.file(&path).content_type(ct);
                if let Some(n) = filename {
                    part.filename(n);
                }
                part.add()
            }
            other => form
                .part(name)
                .contents(form_value_string(other).as_bytes())
                .add(),
        };
        added.map_err(|e| TransportError {
            code: None,
            message: format!("formData field \"{name}\": {}", e.description()),
        })?;
    }
    if !missing.is_empty() {
        return Err(TransportError {
            code: None,
            message: format!("File(s) not found: {}", missing.join(", ")),
        });
    }
    Ok(form)
}
