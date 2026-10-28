//! Minimal blocking HTTP GET over libcurl for the update paths.

use curl::easy::{Easy, List};
use std::time::Duration;

/// GET `url` (following redirects), failing on HTTP >= 400 and on bodies
/// larger than `max_bytes`.
pub fn get(
    url: &str,
    timeout: Duration,
    max_bytes: usize,
    headers: &[&str],
) -> Result<Vec<u8>, String> {
    let mut easy = Easy::new();
    easy.url(url).map_err(|e| e.to_string())?;
    // Same TLS trust as request transfers (OS store; no bundled CA).
    easy.ssl_options(&curl_runner_core::transport::easy::native_ca_options())
        .map_err(|e| e.to_string())?;
    easy.follow_location(true).map_err(|e| e.to_string())?;
    easy.max_redirections(5).map_err(|e| e.to_string())?;
    if url.to_ascii_lowercase().starts_with("https://") {
        // An https download never follows a redirect down to http.
        curl_runner_core::transport::easy::set_redirect_protocols(&easy, &["https"])
            .map_err(|e| e.to_string())?;
    }
    easy.timeout(timeout).map_err(|e| e.to_string())?;
    easy.connect_timeout(Duration::from_secs(10).min(timeout))
        .map_err(|e| e.to_string())?;
    easy.useragent(&format!("curl-runner/{}", curl_runner_core::VERSION))
        .map_err(|e| e.to_string())?;
    let mut list = List::new();
    for h in headers {
        list.append(h).map_err(|e| e.to_string())?;
    }
    easy.http_headers(list).map_err(|e| e.to_string())?;
    let mut body = Vec::new();
    {
        let mut t = easy.transfer();
        t.write_function(|d| {
            if body.len() + d.len() > max_bytes {
                return Ok(0); // abort: reported as a write error
            }
            body.extend_from_slice(d);
            Ok(d.len())
        })
        .map_err(|e| e.to_string())?;
        t.perform().map_err(|e| {
            if e.is_write_error() {
                format!("response larger than {max_bytes} bytes")
            } else {
                e.to_string()
            }
        })?;
    }
    let code = easy.response_code().map_err(|e| e.to_string())?;
    if code >= 400 {
        return Err(format!("HTTP {code} for {url}"));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_http_get_works_with_shared_tls_setup() {
        let srv = testkit::TestServer::start();
        let body = get(
            &format!("{}/echo", srv.base_url),
            Duration::from_secs(5),
            1 << 20,
            &[],
        )
        .unwrap();
        assert!(!body.is_empty());
        assert!(
            get(
                &format!("{}/status/404", srv.base_url),
                Duration::from_secs(5),
                1 << 20,
                &[]
            )
            .is_err()
        );
    }

    /// Live HTTPS against the OS trust store (the bug: no CA bundle in the
    /// static rustls libcurl made every real fetch fail): `cargo test -p
    /// curl-runner net::tests::https_get_uses_native_ca -- --ignored`.
    #[test]
    #[ignore = "needs network"]
    fn https_get_uses_native_ca() {
        let body = get(
            "https://registry.npmjs.org/@curl-runner/cli/latest",
            Duration::from_secs(10),
            1 << 20,
            &[],
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("\"version\""));
    }
}
