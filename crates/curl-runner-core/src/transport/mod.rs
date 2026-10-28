//! HTTP transport over in-process libcurl (replaces spawning the `curl` binary).

pub mod curl_command;
pub mod easy;
pub mod response;

pub use curl_command::to_curl_command;
pub use easy::{Collector, build_easy};
pub use response::finish;

use crate::model::{GlobalConfig, HeaderBlock, Headers, Json, Metrics, RequestConfig};
use std::path::{Path, PathBuf};

/// A completed HTTP exchange.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub status: u16,
    /// Final (non-1xx) header block; lowercase names, repeats as `Many`.
    pub headers: Headers,
    /// Every header block (1xx, redirects, final), final last.
    pub header_history: Vec<HeaderBlock>,
    pub body: Json,
    pub metrics: Metrics,
}

/// Transfer could not complete (setup failure, DNS, connect, timeout, ...).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct TransportError {
    /// libcurl `CURLcode`, when the failure came from libcurl.
    pub code: Option<u32>,
    pub message: String,
}

impl TransportError {
    /// Map a failed transfer to the user-facing error. Timeouts use the
    /// documented `Request timeout after {ms}ms` / `Connection timeout after
    /// {ms}ms`; everything else mirrors curl's stderr `curl: (N) message`.
    pub fn from_curl(err: &curl::Error, req: &RequestConfig, global: &GlobalConfig) -> Self {
        let code = curl_code(err);
        let detail = err.extra_description().unwrap_or(err.description());
        let message = if err.is_operation_timedout() {
            // libcurl says "Connection timed out" whenever the transfer was
            // still resolving/connecting (incl. TLS), whichever timeout fired;
            // both apply then, so the smaller one is the one that expired.
            let connecting = detail.starts_with("Connection timed out")
                || detail.starts_with("Resolving timed out");
            let connect_ms = global
                .connection_pool
                .as_ref()
                .filter(|p| connecting && p.enabled == Some(true))
                .map(|p| p.connect_timeout.unwrap_or(easy::POOL_CONNECT_TIMEOUT_SECS) * 1000);
            match (connect_ms, req.timeout.filter(|t| *t > 0)) {
                (Some(c), r) if r.is_none_or(|r| c < r) => {
                    format!("Connection timeout after {c}ms")
                }
                (_, Some(r)) => format!("Request timeout after {r}ms"),
                _ => format!("curl: ({code}) {detail}"),
            }
        } else {
            format!("curl: ({code}) {detail}")
        };
        TransportError {
            code: Some(code),
            message,
        }
    }
}

/// `CURLcode` as u32 (`CURLcode` is `c_uint` on unix, `c_int` on Windows).
#[allow(clippy::unnecessary_cast)]
pub(crate) fn curl_code(err: &curl::Error) -> u32 {
    err.code() as u32
}

/// Resolve an UNCONFINED local path (`ssl.ca/cert/key`: TLS handshake
/// material, never transmitted, so exempt like in TS) against the working
/// directory, as TS did by handing it to the curl process. Confined paths
/// (formData files, `output`) go through `security::path_guard::resolve_safe`
/// in `build_easy`, also relative to `cwd`.
pub fn resolve_local_path(path: &str, cwd: &Path) -> Result<PathBuf, TransportError> {
    Ok(cwd.join(path))
}

/// Perform one request synchronously. `cwd` is the base of every relative
/// local path (`output`, formData files, `ssl`) and the confinement root unless
/// `security.allowPaths`.
pub fn send(
    req: &RequestConfig,
    global: &GlobalConfig,
    cwd: &Path,
) -> Result<Response, TransportError> {
    let easy = build_easy(req, global, cwd)?;
    if let Err(e) = easy.perform() {
        if let Some(msg) = easy.get_ref().output_error() {
            return Err(TransportError {
                code: Some(curl_code(&e)),
                message: msg.to_string(),
            });
        }
        return Err(TransportError::from_curl(&e, req, global));
    }
    finish(easy)
}

#[cfg(test)]
mod tests;
