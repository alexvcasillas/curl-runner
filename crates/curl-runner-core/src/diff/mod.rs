//! Response diffing: baseline files and comparison (`diff/*.ts`).
//!
//! No printing here; the terminal/json/markdown output lives in the CLI crate.

pub mod baseline;
pub mod differ;

pub use baseline::{BaselineManager, DEFAULT_BASELINE_DIR};
pub use differ::{DiffOrchestrator, ResponseDiffer};
