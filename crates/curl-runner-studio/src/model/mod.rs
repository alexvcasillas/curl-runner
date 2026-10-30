//! The studio's pure state model: no GPUI, so it compiles on every target.

pub mod collection;
pub mod diagnostics;
pub mod document;
// The run engine; the UI picks it up in later plan-2 tasks.
#[allow(dead_code)]
pub mod environments;
#[allow(dead_code)]
pub mod result_view;
#[allow(dead_code)]
pub mod run;
#[allow(dead_code)]
pub mod secrets;
pub mod templates;
