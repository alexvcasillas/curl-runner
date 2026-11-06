//! The Cloud API client lives in `curl-runner-cloud-client`; this module
//! keeps what only the CLI needs (server resolution from flags and the
//! environment) and re-exports the rest under the old paths.

use super::Env;
use crate::version_check::{HOOKS_ENABLED, hook};

pub use curl_runner_cloud_client::{Client, CloudError, Verified, check_server, describe};

pub const DEFAULT_SERVER: &str = "https://api.curl-runner.com";
pub const SERVER_ENV: &str = "CURL_RUNNER_SERVER";
pub const TEST_SERVER_ENV: &str = "CURL_RUNNER_TEST_SERVER";

/// The API base URL: `--server`, else the test hook (debug and `test-hooks`
/// builds), else `CURL_RUNNER_SERVER`, else the hosted service.
pub fn resolve_server(flag: Option<&str>, env: Env) -> String {
    let url = flag
        .map(str::to_string)
        .or_else(|| hook(env, TEST_SERVER_ENV, HOOKS_ENABLED))
        .or_else(|| env(SERVER_ENV).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| DEFAULT_SERVER.to_string());
    url.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(a, _)| *a == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn server_resolution_order() {
        let env = env_of(&[
            (SERVER_ENV, "https://env.example.com/"),
            (TEST_SERVER_ENV, "http://t/"),
        ]);
        assert_eq!(
            resolve_server(Some("https://flag.example.com/"), &env),
            "https://flag.example.com"
        );
        assert_eq!(resolve_server(None, &env), "http://t");
        let env = env_of(&[(SERVER_ENV, "https://env.example.com/")]);
        assert_eq!(resolve_server(None, &env), "https://env.example.com");
        assert_eq!(resolve_server(None, &env_of(&[])), DEFAULT_SERVER);
    }
}
