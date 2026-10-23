# tpt-pulse-backend-leptos

A bridge from tpt-pulse signal graphs to [Leptos](https://leptos.dev) reactive
signals. A `#[signal_graph]` function computes its output from its inputs in one
allocation-free pass. This crate connects those inputs and outputs to Leptos
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
use leptos::prelude::*;
use tpt_pulse_backend_leptos::{derived2, Live};
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
```

Each `Live` value is a reactive signal that components can read and write. `derived2`
runs the graph whenever either input changes, and returns a memo that the view reads.

## The API

| Item | Purpose |
| --- | --- |
| `Live<T>` | A reactive value. `Live::new(initial)`, `get()`, `set(value)`, and `rw_signal()` for the underlying Leptos `RwSignal`. |
| `derived(input, f)` | A memo whose value is `f(input)`. Recomputed when `input` changes. |
| `derived2(a, b, f)` | A memo whose value is `f(a, b)`. Recomputed when either input changes. |

`T` must be `Send + Sync + 'static`, as Leptos requires for its signals.

Pass a `#[signal_graph]` function as `f`. The graph recomputes from its inputs on every
change, so the graph itself does not need to know about Leptos.

## Server rendering and the example

The example renders the component to HTML on the server, with no browser needed:

```text
cargo run --example leptos_app
```

It prints the HTML for the `Robotics` component, with the motor output computed by the
signal graph.

## Testing

`cargo test -p tpt-pulse-backend-leptos` runs the `Live` tests on the host. The bridge
tests also run as `wasm32-unknown-unknown` under Node with `wasm-bindgen-test`. That
is a separate CI job (`wasm`).

## Guarantees

- A `derived` or `derived2` memo updates when its inputs change, through Leptos's own
  reactivity.
- The graph runs in the same single pass as in the signals crate.

## Limitations

- **Verified by server rendering, not in a browser.** The example renders HTML on the
  server. Interactive behavior in a browser was not tested.
- **Values must be `Send + Sync`.**
- **Leptos 0.8.** The API follows that version. Newer Leptos releases may need changes.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
