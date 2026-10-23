//! `Live` values drive derived hooks, and updates re-render the component.

#![allow(non_snake_case)]

use std::cell::RefCell;

use dioxus::core::{NoOpMutations, VirtualDom};
use dioxus::prelude::*;
use tpt_pulse_backend_dioxus::{use_derived2, use_live, Live};
use wasm_bindgen_test::wasm_bindgen_test;

thread_local! {
    static SENSORS: RefCell<Option<(Live<f64>, Live<f64>)>> = const { RefCell::new(None) };
}

fn Root() -> Element {
    let sensor_a = use_live(|| 2.0_f64);
    let sensor_b = use_live(|| 0.0_f64);
    use_hook(|| SENSORS.with(|slot| *slot.borrow_mut() = Some((sensor_a, sensor_b))));
    let motor = use_derived2(sensor_a, sensor_b, |a, b| (a + b) * 0.5);
    rsx! { p { "motor: {motor()}" } }
}

#[wasm_bindgen_test]
fn derived_output_re_renders_when_an_input_changes() {
    let mut dom = VirtualDom::new(Root);
    dom.rebuild_in_place();
    assert!(dioxus_ssr::render(&dom).contains("motor: 1"));

    let (sensor_a, _sensor_b) = SENSORS
        .with(|slot| *slot.borrow())
        .expect("root component registered its sensors");
    sensor_a.set(10.0);
    dom.render_immediate(&mut NoOpMutations);
    assert!(dioxus_ssr::render(&dom).contains("motor: 5"));
}
