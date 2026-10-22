//! Bridge from pulse values into Dioxus reactive signals.
//!
//! A `#[signal_graph]` function computes its output from its inputs in a
//! single, allocation-free pass. This crate connects those inputs and outputs
//! to Dioxus: [`Live`] holds a value that components read and write, and
//! [`use_derived2`] recomputes a graph whenever its inputs change.

use dioxus::prelude::{use_memo, use_signal, Memo, Signal};
use dioxus::signals::{ReadableExt, WritableExt};

/// A value shared between pulse code and Dioxus components.
///
/// `Live` is a copyable handle to a Dioxus [`Signal`]. Reads inside a component
/// subscribe it to the value, and writes re-render subscribers.
#[derive(Clone, Copy, PartialEq)]
pub struct Live<T: 'static>(Signal<T>);

impl<T: 'static> Live<T> {
    /// Create a value. Must run inside a Dioxus runtime, such as a component.
    pub fn new(initial: T) -> Self {
        Self(Signal::new(initial))
    }

    /// Read the current value, subscribing the surrounding component.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.0.read().clone()
    }

    /// Replace the value and notify subscribers.
    pub fn set(&self, value: T) {
        let mut signal = self.0;
        signal.set(value);
    }

    /// The underlying Dioxus signal, for code that works with `Signal` directly.
    pub fn signal(&self) -> Signal<T> {
        self.0
    }
}

impl<T: 'static> From<Signal<T>> for Live<T> {
    fn from(signal: Signal<T>) -> Self {
        Self(signal)
    }
}

/// Create a component-local [`Live`] value with an initial state.
pub fn use_live<T: 'static>(initial: impl FnOnce() -> T) -> Live<T> {
    Live(use_signal(initial))
}

/// Recompute `f` of two inputs whenever either changes. Call from a component.
pub fn use_derived2<A, B, U>(
    first: Live<A>,
    second: Live<B>,
    f: impl Fn(A, B) -> U + 'static,
) -> Memo<U>
where
    A: Clone + 'static,
    B: Clone + 'static,
    U: PartialEq + Clone + 'static,
{
    use_memo(move || f(first.get(), second.get()))
}
