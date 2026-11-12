//! Blocking client for the curl-runner Cloud API over libcurl. It is shared
//! by the CLI and the studio, so it has no terminal output and no CLI-only
//! dependencies.

pub mod client;
pub mod link;
pub mod net;
pub mod publish;
pub mod retry;

pub use client::{
    Client, CloudError, CollectionRow, Latest, Me, NetError, Reply, TeamInfo, User, Verified,
    check_server, describe, versions_path,
};

/// Text from a server, safe to print: control characters (a terminal reads
/// escape sequences from them) become `\u{..}` escapes.
pub fn escape_control(s: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control() {
            let _ = write!(out, "\\u{{{:x}}}", c as u32);
        } else {
            out.push(c);
        }
    }
    out
}
