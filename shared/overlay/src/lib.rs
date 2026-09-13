//! Virtual overlay network for **management mesh** only.
//!
//! - Admin + Server = equal Overlay **servers** (DHCP / hole-punch assist / relay)
//! - Edge (+ each server process) = Overlay **nodes** with a virtual IP
//! - Intranet penetration (public port mapping) is a separate layer

mod config;
mod dhcp;
mod frame;
mod mgmt;
mod node;
mod peer;
mod punch;
mod tun_dev;

pub use config::*;
pub use dhcp::*;
pub use frame::*;
pub use mgmt::*;
pub use node::*;
pub use peer::*;
pub use punch::*;
pub use tun_dev::*;
