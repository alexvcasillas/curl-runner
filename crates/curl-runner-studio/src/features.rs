//! Compile-time switches.

/// Whether this build has the server-backed UI (sign-in, share, sync,
/// shared links) and the mock behind it. Off in every distributed build;
/// `cargo run --features server` for development. UI code branches on
/// this; `cfg(feature = "server")` is only for what must not be compiled
/// in (the mock, its fixtures, the Developer menu).
pub const SERVER: bool = cfg!(feature = "server");
