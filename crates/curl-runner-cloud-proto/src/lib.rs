//! Wire protocol shared by the curl-runner Cloud server, CLI and studio.
//! Pure data types and validation. No I/O. Sync types live in [`sync`].

pub mod bundle;
pub mod error;
pub mod inputs;
pub mod sync;
pub mod wire;

pub use bundle::*;
pub use error::ErrorCode;
pub use inputs::*;
pub use wire::*;
