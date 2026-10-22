//! `Live` values and derived memos update when their inputs change.

use leptos::prelude::{Get, Owner};
use pulse_backend_leptos::{derived, derived2, Live};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn derived_memos_follow_their_inputs() {
    Owner::new().with(|| {
        let sensor_a = Live::new(2.0_f64);
        let sensor_b = Live::new(0.0_f64);
        let motor = derived2(sensor_a, sensor_b, |a, b| (a + b) * 0.5);
        assert_eq!(motor.get(), 1.0);

        sensor_a.set(10.0);
        assert_eq!(motor.get(), 5.0);

        let doubled = derived(sensor_b, |b| b * 2.0);
        sensor_b.set(3.0);
        assert_eq!(doubled.get(), 6.0);
    });
}

#[wasm_bindgen_test]
fn live_round_trips_through_rw_signal() {
    Owner::new().with(|| {
        let live = Live::new(String::from("idle"));
        let raw = Live::from(live.rw_signal());
        raw.set(String::from("running"));
        assert_eq!(live.get(), "running");
    });
}
