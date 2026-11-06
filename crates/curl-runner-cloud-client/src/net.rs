//! libcurl setup shared by every call.

use curl::easy::Easy;
use std::time::Duration;

/// An easy handle for `url` with the shared TLS trust (OS store; no bundled
/// CA), bounded redirects (never https down to http), timeouts and user agent.
pub fn easy_for(url: &str, timeout: Duration) -> Result<Easy, curl::Error> {
    let mut easy = Easy::new();
    easy.url(url)?;
    easy.ssl_options(&curl_runner_core::transport::easy::native_ca_options())?;
    easy.follow_location(true)?;
    easy.max_redirections(5)?;
    if url.to_ascii_lowercase().starts_with("https://") {
        curl_runner_core::transport::easy::set_redirect_protocols(&easy, &["https"])?;
    }
    easy.timeout(timeout)?;
    easy.connect_timeout(Duration::from_secs(10).min(timeout))?;
    easy.useragent(&format!("curl-runner/{}", curl_runner_core::VERSION))?;
    Ok(easy)
}
