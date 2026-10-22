//! Procedural macros for `tpt-pulse`.
//!
//! - [`signal_graph`] lowers a function that uses `Signal::new`, `map`, `map2`
//!   and `set` into plain locals with direct, inlined recomputation.
//! - [`global_protocol`] declares a multiparty protocol and checks it at compile time.
//!
//! The expansions name `::pulse_signals` and `::pulse_multiparty`, so a crate
//! using these macros must depend on those crates directly.

mod multiparty;
mod signal;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

/// Compile a function's signal declarations into a static update sequence.
///
/// # Examples
///
/// ```ignore
/// use pulse_signals::{signal_graph, Signal};
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

/// Declare a multiparty protocol: `global_protocol! { A -> B : Msg; B -> A : Reply; }`.
///
/// Expands to a `pulse_multiparty::GlobalProtocol`. The protocol is projected at
/// compile time, so a malformed step is a compile error.
#[proc_macro]
pub fn global_protocol(input: TokenStream) -> TokenStream {
    multiparty::expand(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
