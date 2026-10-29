pub mod condition;
pub mod config;
pub mod convert;
pub mod diff;
pub mod discover;
pub mod env;
pub mod executor;
pub mod interpolate;
pub(crate) mod jsfile;
pub mod jsval;
pub mod model;
pub mod parser;
pub mod postprocess;
pub mod security;
pub mod snapshot;
pub mod store;
pub mod transport;
pub mod validate;
mod yaml_load;
pub mod yaml_text;

/// CLI version; `CURL_RUNNER_VERSION` at build time overrides the package version.
pub const VERSION: &str = match option_env!("CURL_RUNNER_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!VERSION.is_empty());
    }
}
