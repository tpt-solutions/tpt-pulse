//! Run with `cargo run --example robotics`. A signal graph that is compiled into a
//! static update sequence, with no runtime dependency tracking.
use tpt_pulse::prelude::*;

#[signal_graph]
fn robotics_control_loop(sensor_a_reading: f64) -> f64 {
    let sensor_a = Signal::new(sensor_a_reading);
    let sensor_b = Signal::new(0.0);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let motor_out = Signal::map(&combined, |val| val * 0.5);

    println!("initial motor output: {}", motor_out.get());
    sensor_a.set(10.0);
    println!("after sensor_a = 10.0: {}", motor_out.get());
    sensor_b.set(-4.0);
    println!("after sensor_b = -4.0: {}", motor_out.get());
    motor_out.get()
}

fn main() {
    let final_output = robotics_control_loop(2.0);
    assert_eq!(final_output, 3.0);
}
