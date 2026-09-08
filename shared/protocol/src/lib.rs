//! Control-plane protocol: length-prefixed JSON messages.

mod framing;
mod message;

pub use framing::*;
pub use message::*;
