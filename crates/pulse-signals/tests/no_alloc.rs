#![allow(unsafe_code)]
//! Checks that signal propagation performs no heap allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use pulse_signals::{signal_graph, Signal};

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: forwards every call to the system allocator unchanged; only a thread-local counter is added.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[signal_graph]
fn control_loop(first: f64) -> f64 {
    let sensor_a = Signal::new(first);
    let sensor_b = Signal::new(1.0);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let scaled = Signal::map(&combined, |val| val * 0.5);
    let motor_out = Signal::map(&scaled, |val| val - 1.0);
    sensor_a.set(10.0);
    sensor_b.set(3.0);
    motor_out.get()
}

#[test]
fn propagation_does_not_allocate() {
    let before = ALLOCATIONS.with(Cell::get);
    let result = control_loop(0.0);
    let after = ALLOCATIONS.with(Cell::get);
    assert_eq!(result, 5.5);
    assert_eq!(after - before, 0, "signal propagation allocated");
}
