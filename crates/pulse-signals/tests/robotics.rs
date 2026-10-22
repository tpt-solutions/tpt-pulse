//! Robotics signal graph: the spec section 4 example, checked against its documented output.

use pulse_signals::{signal_graph, Signal};

#[signal_graph]
fn robotics_control_loop(first: f64) -> (f64, f64) {
    let sensor_a = Signal::new(first);
    let sensor_b = Signal::new(0.0);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let motor_out = Signal::map(&combined, |val| val * 0.5);
    let before = motor_out.get();
    sensor_a.set(10.0);
    (before, motor_out.get())
}

#[test]
fn motor_output_follows_sensor_updates() {
    assert_eq!(robotics_control_loop(0.0), (0.0, 5.0));
}
