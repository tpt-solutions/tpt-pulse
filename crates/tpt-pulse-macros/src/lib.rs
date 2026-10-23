//! Procedural macros for `tpt-pulse`.
//!
//! - [`signal_graph`] lowers a function that uses `Signal::new`, `map`, `map2`
//!   and `set` into plain locals with direct, inlined recomputation.
//! - [`global_protocol`] declares a multiparty protocol and checks it at compile time.
//! - [`session_types`] also emits a `tpt-pulse-core` protocol type for each role.
//!
//! The expansions name `::tpt_pulse_signals` and `::tpt_pulse_multiparty`, so a crate
//! using these macros must depend on those crates directly.

mod compose;
mod multiparty;
mod signal;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

/// Compile a function's signal declarations into a static update sequence.
///
/// # Examples
///
/// ```ignore
/// use tpt_pulse_signals::{signal_graph, Signal};
///
/// #[signal_graph]
/// fn robotics_control_loop() -> f64 {
///     let sensor_a = Signal::new(0.0);
///     let sensor_b = Signal::new(0.0);
///     let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
///     let motor_out = Signal::map(&combined, |val| val * 0.5);
///     sensor_a.set(10.0);
///     motor_out.get()
/// }
/// ```
#[proc_macro_attribute]
pub fn signal_graph(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[signal_graph] takes no arguments",
        )
        .to_compile_error()
        .into();
    }
    let item = parse_macro_input!(item as ItemFn);
    signal::expand(item)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Entry point of the sub-graph macros. Not part of the public API.
#[doc(hidden)]
#[proc_macro]
pub fn __compose(input: TokenStream) -> TokenStream {
    compose::compose_tokens(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Declare a multiparty protocol: `global_protocol! { A -> B : Msg; B -> A : Reply; }`.
///
/// Takes the same statements as the text format, including `choice` and `loop`.
/// Expands to a `tpt_pulse_multiparty::GlobalProtocol`. The protocol is projected at
/// compile time, so a malformed step is a compile error.
#[proc_macro]
pub fn global_protocol(input: TokenStream) -> TokenStream {
    multiparty::expand(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Declare a multiparty protocol and its per-role session types:
/// `session_types! { Login { Client -> Server : AuthReq; Server -> Client : AuthOk; } }`.
///
/// Expands to a module named after the protocol, with one `tpt-pulse-core` protocol
/// type per role. The steps are checked at compile time. Message types are
/// resolved from the module's enclosing scope.
#[proc_macro]
pub fn session_types(input: TokenStream) -> TokenStream {
    multiparty::expand_session_types(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
