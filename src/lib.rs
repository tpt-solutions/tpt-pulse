//! Session-typed communication, compile-time reactive signals, and multiparty
//! protocol verification for Rust.
//!
//! - Synchronous and async two-party session channels: [`prelude`] and [`core`].
//! - Compile-time signal graphs, via `#[signal_graph]`: [`signals`].
//! - Multiparty protocols checked through `tpt-telos`: [`multiparty`] and
//!   `global_protocol!`.
//!
//! The async TCP backend is the separate `pulse-backend-tokio` crate.
//! Leptos and Dioxus bridges live in `pulse-backend-leptos` and `pulse-backend-dioxus`.

/// Synchronous session-typed channels.
pub use pulse_core as core;

/// Compile-time reactive signals.
pub use pulse_signals as signals;

/// Multiparty protocol descriptions and deadlock checks.
pub use pulse_multiparty as multiparty;

/// Common session protocol types and operations, plus the reactive and multiparty entry points.
pub mod prelude {
    pub use pulse_core::{
        session_channel, Chan, Choose, Client, Close, Dual, Either, Protocol, Rec, Recv, Send,
        Server, Var,
    };
    pub use pulse_macros::{global_protocol, session_types};
    pub use pulse_multiparty::assert_protocol;
    pub use pulse_multiparty::{Action, GlobalProtocol, Program, Session};
    pub use pulse_signals::{signal_graph, Signal};
}
