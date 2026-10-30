//! The studio's pure state model: no GPUI, so it compiles on every target.

pub mod collection;
pub mod diagnostics;
pub mod document;
pub mod environments;
// Parts of these (history, body diffs, Re-run failed, editing secrets) are
// picked up by later plan-2 tasks.
#[allow(dead_code)]
pub mod history;
#[allow(dead_code)]
pub mod result_view;
#[allow(dead_code)]
pub mod run;
#[allow(dead_code)]
pub mod secrets;
pub mod templates;
