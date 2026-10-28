//! Security guards (port of the PR #124 hardening): URL protocol allow-list,
//! filesystem path confinement and secret redaction. Pure and side-effect
//! free; callers pass `cwd` explicitly.

pub mod path_guard;
pub mod redact;
pub mod url_guard;

use std::path::Path;

use crate::config::DEFAULT_ALLOWED_PROTOCOLS;
use crate::model::{FormFieldValue, GlobalConfig, RequestConfig};

/// A guard refusal. `Display` is the exact TS error text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct GuardError {
    pub message: String,
}

impl GuardError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// `security.allowedProtocols`, defaulting to `http, https` when unset or
/// empty (the TS resolver guarantees a non-empty list).
pub fn effective_protocols(global: &GlobalConfig) -> Vec<String> {
    match global
        .security
        .as_ref()
        .and_then(|s| s.allowed_protocols.as_ref())
        .filter(|p| !p.is_empty())
    {
        Some(p) => p.clone(),
        None => DEFAULT_ALLOWED_PROTOCOLS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

/// `security.allowPaths` (default false).
pub fn allow_paths(global: &GlobalConfig) -> bool {
    global
        .security
        .as_ref()
        .and_then(|s| s.allow_paths)
        .unwrap_or(false)
}

/// Confine the request's filesystem paths (`output`, formData files) like TS
/// `validateRequestPaths`. `ssl.ca/cert/key` are intentionally not confined
/// (TLS handshake material, never transmitted). Both are relative to
/// `cwd` (TS: `process.cwd()`) and must stay inside it unless `allowPaths`.
pub fn check_request_paths(
    req: &RequestConfig,
    global: &GlobalConfig,
    cwd: &Path,
) -> Result<(), GuardError> {
    let allow = allow_paths(global);
    if let Some(out) = req.output.as_deref().filter(|o| !o.is_empty()) {
        path_guard::resolve_safe(out, cwd, cwd, allow)?;
    }
    if let Some(fields) = &req.form_data {
        for (name, value) in fields {
            if let FormFieldValue::File(f) = value {
                path_guard::resolve_safe(&f.file, cwd, cwd, allow)
                    .map_err(|e| GuardError::new(format!("formData field \"{name}\": {e}")))?;
            }
        }
    }
    Ok(())
}

/// Paths first, then URL protocol, as in TS `executeRequest`.
pub fn check_request(
    req: &RequestConfig,
    global: &GlobalConfig,
    cwd: &Path,
) -> Result<(), GuardError> {
    check_request_paths(req, global, cwd)?;
    url_guard::check(&req.url, &effective_protocols(global))
}
