# tpt-pulse-backend-dioxus

A bridge from tpt-pulse signal graphs to [Dioxus](https://dioxuslabs.com) reactive
signals. A `#[signal_graph]` function computes its output from its inputs in one
allocation-free pass. This crate connects those inputs and outputs to Dioxus
components.

Part of [tpt-pulse](../../README.md). The graph itself is described in
[tpt-pulse-signals](../tpt-pulse-signals/README.md).

## Contents

- [Quick start](#quick-start)
- [The API](#the-api)
- [Server rendering and the example](#server-rendering-and-the-example)
- [Testing](#testing)
- [Guarantees](#guarantees)
- [Limitations](#limitations)

## Quick start

```rust
use dioxus::prelude::*;
use tpt_pulse_backend_dioxus::{use_derived2, use_live};
use tpt_pulse_signals::{signal_graph, Signal};

#[signal_graph]
fn motor_output(sensor_a_reading: f64, sensor_b_reading: f64) -> f64 {
    let sensor_a = Signal::new(sensor_a_reading);
    let sensor_b = Signal::new(sensor_b_reading);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let motor_out = Signal::map(&combined, |val| val * 0.5);
    motor_out.get()
}

#[component]
fn Robotics() -> Element {
    let sensor_a = use_live(|| 2.0);
    let sensor_b = use_live(|| 0.0);
    let motor = use_derived2(sensor_a, sensor_b, motor_output);
    rsx! {
        section {
            p { "Sensor A: {sensor_a.get()}" }
            p { "Motor output: {motor()}" }
        }
    }
}
```

`use_live` creates a `Live` value in a component. `use_derived2` returns a memo that
reruns the graph whenever either input changes. Call the memo to read it, as in
`{motor()}`, and the component updates when the value changes.

## The API

| Item | Purpose |
| --- | --- |
| `Live<T>` | A reactive value. `Live::new(initial)`, `get()`, `set(value)`, and `signal()` for the underlying Dioxus `Signal`. |
| `use_live(|| initial)` | A hook that creates a `Live<T>` for the component. |
| `use_derived2(a, b, f)` | A hook that returns a `Memo` of `f(a, b)`. It recomputes when either input changes. Call the memo to read it. `U` must be `Clone + PartialEq`. |

Pass a `#[signal_graph]` function as `f`. As with the Leptos bridge, the graph does not
need to know about Dioxus.

## Server rendering and the example

The example renders the component to HTML with `dioxus-ssr`, with no browser needed:

```text
cargo run --example dioxus_app
```

It prints the HTML for the component, with the motor output computed by the signal graph.

## Testing

`cargo test -p tpt-pulse-backend-dioxus` runs the `Live` tests on the host. The bridge
tests also run as `wasm32-unknown-unknown` under Node with `wasm-bindgen-test`. That is a
separate CI job (`wasm`).

## Guarantees

- `use_derived2` recomputes the graph when either input changes, through Dioxus's own
  signals.
- The graph runs in the same single pass as in the signals crate.

## Limitations

- **Verified by server rendering and virtual DOM re-rendering, not in a browser.** Browser
  behavior was not tested.
- **Values are `'static`.** `Live<T>` requires `T: 'static`.
- **Dioxus 0.7 with the `signals` and `hooks` features.** The API follows that version.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
