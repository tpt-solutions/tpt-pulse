//! Compile-time reactive signals.
//!
//! Signals are declared inside a function marked with [`signal_graph`]. The
//! macro removes the reactive machinery at compile time: each signal becomes an
//! ordinary local, each `map`/`map2` becomes a closure call, and `set` recomputes
//! the affected downstream signals in order. There is no runtime graph, no
//! dynamic dispatch, and no heap allocation.
//!
//! [`Signal`] exists only so that the code type-checks as ordinary Rust. Its
//! methods are never called at run time; the macro rewrites every use.
//!
//! # Sub-graphs
//!
//! A `#[signal_graph]` function whose parameters are `Signal<T>` handles is a
//! sub-graph. A caller inlines it with `let out = sub(&a, &b);`, so a `set` in the
//! caller reaches the sub-graph's signals, and the cycle check covers the composed
//! graph. Limits: the sub-graph must be declared before its callers in the same
//! module, and give each sub-graph a distinct name, since the name identifies its macro. It may only declare signals,
//! must end with one of them, cannot call `set`, and cannot call another sub-graph.
//! A direct call to a sub-graph panics.
//!
//! ```
//! use tpt_pulse_signals::{signal_graph, Signal};
//!
//! #[signal_graph]
//! fn robotics_control_loop() -> f64 {
//!     let sensor_a = Signal::new(0.0);
//!     let sensor_b = Signal::new(0.0);
//!     let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
//!     let motor_out = Signal::map(&combined, |val| val * 0.5);
//!     sensor_a.set(10.0);
//!     motor_out.get()
//! }
//!
//! assert_eq!(robotics_control_loop(), 5.0);
//! ```

pub use tpt_pulse_macros::signal_graph;

/// Used by the macros that sub-graphs generate. Not part of the public API.
#[doc(hidden)]
pub use tpt_pulse_macros::__compose;

use std::marker::PhantomData;

/// A reactive value declared inside a `#[signal_graph]` function.
///
/// Outside that macro, `Signal` has no run-time behavior: calling its methods panics.
#[must_use]
pub struct Signal<T>(PhantomData<T>);

const OUTSIDE_MACRO: &str = "Signal values exist only inside a #[signal_graph] function";

impl<T> Signal<T> {
    /// Declare a source signal with an initial value. Lowered by `#[signal_graph]`.
    pub fn new(_initial: T) -> Self {
        panic!("{OUTSIDE_MACRO}")
    }

    /// Read the current value. Lowered by `#[signal_graph]` to a clone of the local.
    pub fn get(&self) -> T {
        panic!("{OUTSIDE_MACRO}")
    }

    /// Update a source signal and recompute its dependents. Lowered by `#[signal_graph]`.
    pub fn set(&self, _value: T) {
        panic!("{OUTSIDE_MACRO}")
    }

    /// Derive a signal from one source. Lowered by `#[signal_graph]`.
    pub fn map<U>(_source: &Signal<T>, _f: impl Fn(T) -> U) -> Signal<U> {
        panic!("{OUTSIDE_MACRO}")
    }

    /// Derive a signal from two sources. Lowered by `#[signal_graph]`.
    pub fn map2<A, B, U>(
        _first: &Signal<A>,
        _second: &Signal<B>,
        _f: impl Fn(A, B) -> U,
    ) -> Signal<U> {
        panic!("{OUTSIDE_MACRO}")
    }
}

/// Apply a one-input closure. Generic so the closure's parameter type is inferred
/// from the argument; inlined like a direct call. Not part of the public API.
#[doc(hidden)]
#[inline(always)]
pub fn __apply1<A, U>(f: impl FnOnce(A) -> U, a: A) -> U {
    f(a)
}

/// Apply a two-input closure. See [`__apply1`].
#[doc(hidden)]
#[inline(always)]
pub fn __apply2<A, B, U>(f: impl FnOnce(A, B) -> U, a: A, b: B) -> U {
    f(a, b)
}
