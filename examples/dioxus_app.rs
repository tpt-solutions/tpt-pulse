//! Run with `cargo run --example dioxus_app`. Renders a Dioxus component whose
//! motor output is a `#[signal_graph]` over two sensor values, as HTML.
use dioxus::prelude::*;
use tpt_pulse_backend_dioxus::{use_derived2, use_live};
use tpt_pulse::signals::{signal_graph, Signal};

#[signal_graph]
fn motor_output(sensor_a_reading: f64, sensor_b_reading: f64) -> f64 {
    let sensor_a = Signal::new(sensor_a_reading);
    let sensor_b = Signal::new(sensor_b_reading);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let motor_out = Signal::map(&combined, |val| val * 0.5);
    motor_out.get()
}

#[allow(non_snake_case)]
fn Robotics() -> Element {
    let sensor_a = use_live(|| 2.0_f64);
    let sensor_b = use_live(|| 0.0_f64);
    let motor = use_derived2(sensor_a, sensor_b, motor_output);
    rsx! {
        section {
            p { "Sensor A: {sensor_a.get()}" }
            p { "Motor output: {motor()}" }
        }
    }
}

fn main() {
    let mut dom = VirtualDom::new(Robotics);
    dom.rebuild_in_place();
    println!("{}", dioxus_ssr::render(&dom));
}
