//! Transport abstractions: TCP now, QUIC/TLS hooks for later.

mod tcp;
mod traits;

pub use tcp::*;
pub use traits::*;
