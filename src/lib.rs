//! Session-typed communication, compile-time reactive signals, and multiparty
//! protocol verification for Rust.
//!
//! - Synchronous and async two-party session channels: [`prelude`] and [`core`].
//! - Compile-time signal graphs, via `#[signal_graph]`: [`signals`].
//! - Multiparty protocols checked through `tpt-telos`: [`multiparty`] and
//!   `global_protocol!`.
//!
//! The async TCP backend is the separate `tpt-pulse-backend-tokio` crate.
//! Leptos and Dioxus bridges live in `tpt-pulse-backend-leptos` and `tpt-pulse-backend-dioxus`.

/// Synchronous session-typed channels.
pub use tpt_pulse_core as core;

/// Compile-time reactive signals.
pub use tpt_pulse_signals as signals;

/// Multiparty protocol descriptions and deadlock checks.
pub use tpt_pulse_multiparty as multiparty;

/// Common session protocol types and operations, plus the reactive and multiparty entry points.
pub mod prelude {
    pub use tpt_pulse_core::{
        session_channel, Chan, Choose, Client, Close, Dual, Either, Protocol, Rec, Recv, Send,
        Server, Var,
    };
    pub use tpt_pulse_macros::{global_protocol, session_types};
    pub use tpt_pulse_multiparty::assert_protocol;
    pub use tpt_pulse_multiparty::{Action, GlobalProtocol, Program, Session};
    pub use tpt_pulse_signals::{signal_graph, Signal};
}
