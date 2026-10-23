//! Sub-graphs: a `#[signal_graph]` function with `Signal` parameters, called from a
//! caller graph. The caller's `set` must reach the sub-graph's signals.

use tpt_pulse_signals::{signal_graph, Signal};

/// Adds two inputs and halves the sum. Only usable as a sub-graph.
#[allow(dead_code)]
#[signal_graph]
fn fuse(a: Signal<f64>, b: Signal<f64>) -> Signal<f64> {
    let total = Signal::map2(&a, &b, |x, y| x + y);
    let scaled = Signal::map(&total, |t| t * 0.5);
    scaled
}

#[signal_graph]
fn control(sensor: f64) -> f64 {
    let sensor_a = Signal::new(sensor);
    let sensor_b = Signal::new(0.0);
    let motor = fuse(&sensor_a, &sensor_b);
    let out = Signal::map(&motor, |m| m + 1.0);
    sensor_a.set(10.0);
    out.get()
}

#[signal_graph]
fn two_calls(sensor: f64) -> f64 {
    let sensor_a = Signal::new(sensor);
    let sensor_b = Signal::new(1.0);
    let first = fuse(&sensor_a, &sensor_b);
    let second = fuse(&sensor_b, &sensor_a);
    let sum = Signal::map2(&first, &second, |x, y| x + y);
    sensor_b.set(3.0);
    sum.get()
}

#[test]
fn a_set_in_the_caller_reaches_the_sub_graph() {
    // Before the set: fuse(2, 0) = 1, so out = 2. After it: fuse(10, 0) = 5, so out = 6.
    assert_eq!(control(2.0), 6.0);
}

#[test]
fn a_sub_graph_called_twice_is_inlined_twice() {
    // Before the set: fuse(2, 1) = 1.5 each, so sum = 3. After b = 3: fuse = 2.5 each, so sum = 5.
    assert_eq!(two_calls(2.0), 5.0);
}
