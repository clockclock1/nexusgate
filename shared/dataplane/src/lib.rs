//! Data plane: bidirectional copy, UDP/HTTP/HTTPS/relay scaffolds.

pub mod http;
pub mod https;
pub mod relay;
pub mod tcp;
pub mod udp;

pub use tcp::copy_bidirectional;
