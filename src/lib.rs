//! Session-typed communication for Rust.
//!
//! Synchronous channels are available now; reactive signals and backend
//! integrations are planned. See the project TODO for implementation status.

/// Synchronous session-typed channels.
pub use pulse_core as core;

/// Common session protocol types and operations.
pub mod prelude {
    pub use pulse_core::{
        session_channel, Chan, Choose, Client, Close, Dual, Either, Protocol, Recv, Send, Server,
    };
}
