//! Wire protocol shared by the curl-runner Cloud server, CLI and studio.
//! Pure data types and validation. No I/O.

pub mod bundle;
pub mod error;
pub mod wire;

pub use bundle::*;
pub use error::ErrorCode;
pub use wire::*;
