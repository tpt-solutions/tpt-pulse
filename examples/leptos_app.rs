//! Run with `cargo run --example leptos_app`. Renders a Leptos component whose
//! motor output is a `#[signal_graph]` over two sensor values, as HTML.
use leptos::prelude::*;
use pulse_backend_leptos::{derived2, Live};
use tpt_pulse::signals::{signal_graph, Signal};

#[signal_graph]
fn motor_output(sensor_a_reading: f64, sensor_b_reading: f64) -> f64 {
    let sensor_a = Signal::new(sensor_a_reading);
    let sensor_b = Signal::new(sensor_b_reading);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let motor_out = Signal::map(&combined, |val| val * 0.5);
    motor_out.get()
}

#[component]
fn Robotics() -> impl IntoView {
    let sensor_a = Live::new(2.0);
    let sensor_b = Live::new(0.0);
    let motor = derived2(sensor_a, sensor_b, motor_output);
    view! {
        <section>
            <p>"Sensor A: " {move || sensor_a.get()}</p>
            <p>"Motor output: " {move || motor.get()}</p>
        </section>
    }
}

fn main() {
    let html = Owner::new().with(|| Robotics().to_html());
    println!("{html}");
}
