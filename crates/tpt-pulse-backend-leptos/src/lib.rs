//! Bridge from pulse values into Leptos reactive signals.
//!
//! A `#[signal_graph]` function computes its output from its inputs in a
//! single, allocation-free pass. This crate connects those inputs and outputs
//! to Leptos: [`Live`] holds a value that components read and write, and
//! [`derived`] and [`derived2`] recompute a graph whenever its inputs change.
//!
//! ```ignore
//! let sensor_a = Live::new(2.0);
//! let sensor_b = Live::new(0.0);
//! let motor = derived2(sensor_a, sensor_b, motor_output);
//! // Reading `motor` inside a view updates it when either sensor changes.
//! ```

use leptos::prelude::{Get, Memo, RwSignal, Set};

/// A value shared between pulse code and Leptos components.
///
/// `Live` is a copyable handle to a Leptos [`RwSignal`]. Reads in a reactive
/// context subscribe the caller; writes notify subscribers.
#[derive(Clone, Copy)]
pub struct Live<T: Send + Sync + 'static>(RwSignal<T>);

impl<T: Send + Sync + 'static> Live<T> {
    /// Create a value with an initial state. Must run inside a reactive owner.
    pub fn new(initial: T) -> Self {
        Self(RwSignal::new(initial))
    }

    /// Read the current value, subscribing the surrounding reactive context.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.0.get()
    }

    /// Replace the value and notify subscribers.
    pub fn set(&self, value: T) {
        self.0.set(value);
    }

    /// The underlying Leptos signal, for code that works with `RwSignal` directly.
    pub fn rw_signal(&self) -> RwSignal<T> {
        self.0
    }
}

impl<T: Send + Sync + 'static> From<RwSignal<T>> for Live<T> {
    fn from(signal: RwSignal<T>) -> Self {
        Self(signal)
    }
}

/// Recompute `f` of one input whenever that input changes.
pub fn derived<A, U>(input: Live<A>, f: impl Fn(A) -> U + Send + Sync + 'static) -> Memo<U>
where
    A: Clone + Send + Sync + 'static,
    U: PartialEq + Send + Sync + 'static,
{
    Memo::new(move |_| f(input.get()))
}

/// Recompute `f` of two inputs whenever either changes.
pub fn derived2<A, B, U>(
    first: Live<A>,
    second: Live<B>,
    f: impl Fn(A, B) -> U + Send + Sync + 'static,
) -> Memo<U>
where
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    U: PartialEq + Send + Sync + 'static,
{
    Memo::new(move |_| f(first.get(), second.get()))
}
