//! curl-runner studio: a macOS desktop app for YAML request suites.

// Off macOS nothing renders, so only the model's own tests use it.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod theme;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod model;

#[cfg(target_os = "macos")]
mod ui;

#[cfg(target_os = "macos")]
fn main() {
    ui::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("curl-runner studio is macOS only");
    std::process::exit(1);
}
