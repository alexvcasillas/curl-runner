//! The importer core every format shares: the mapped model, the YAML
//! emitter, file planning and the writer.

mod emit;
mod model;
mod names;
mod report;
mod write;

pub use emit::emit_request;
pub use model::{Field, FieldValue, ImportedVar, Mapped, MappedAuth, MappedBody, MappedRequest};
pub use names::{PLACEHOLDER_FILE, slug, var_name};
pub use report::{Attention, ImportReport};
pub use write::{Import, ImportFile, WriteOutcome, plan_import, write_import};

#[cfg(test)]
mod tests;
