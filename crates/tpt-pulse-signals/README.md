# tpt-pulse-signals

Compile-time reactive signals for Rust. A signal graph is declared inside one
function marked `#[signal_graph]`. The macro removes the reactive machinery at
compile time: each signal becomes an ordinary local, each `map` or `map2` becomes
a direct closure call, and `set` recomputes the affected downstream signals in
order.

There is no runtime graph, no dynamic dispatch, and no heap allocation during
propagation. A counting-allocator test checks the last point.

Part of [tpt-pulse](../../README.md). Most users should depend on the root
`tpt-pulse` crate, which re-exports this one as `tpt_pulse::signals`.

## Contents

- [Quick start](#quick-start)
- [The API](#the-api)
- [Sub-graphs](#sub-graphs)
- [Dependencies and the cycle check](#dependencies-and-the-cycle-check)
- [Using signals from UI frameworks](#using-signals-from-ui-frameworks)
- [Guarantees](#guarantees)
- [Limitations](#limitations)

## Quick start

```rust
use tpt_pulse_signals::{signal_graph, Signal};

#[signal_graph]
fn robotics_control_loop(sensor_a_reading: f64) -> f64 {
    let sensor_a = Signal::new(sensor_a_reading);
    let sensor_b = Signal::new(0.0);
    let combined = Signal::map2(&sensor_a, &sensor_b, |a, b| a + b);
    let motor_out = Signal::map(&combined, |val| val * 0.5);

    sensor_a.set(10.0);
    motor_out.get()
}

fn main() {
    assert_eq!(robotics_control_loop(2.0), 5.0);
}
```

The full version with printed updates is `examples/robotics.rs`:
`cargo run --example robotics`.

## The API

Inside a `#[signal_graph]` function:

| Form | Meaning |
| --- | --- |
| `Signal::new(initial)` | A source signal holding `initial`. Its value can be changed with `set`. |
| `Signal::map(&s, f)` | A signal whose value is `f(s)`. |
| `Signal::map2(&a, &b, f)` | A signal whose value is `f(a, b)`. |
| `s.set(value)` | Changes a source signal and recomputes its dependents. Top-level statement only. |
| `s.get()` | Reads the current value. |

`Signal<T>` is a marker type. Its methods panic if called outside a
`#[signal_graph]` function, because the macro rewrites every use. The type exists so
the code type-checks as ordinary Rust, including in IDEs.

The function returns a plain value, usually taken with `get()` from its output signal.

## Sub-graphs

A `#[signal_graph]` function whose parameters are `Signal<T>` handles, and which returns
a `Signal<T>`, is a sub-graph. A caller inlines it with an ordinary-looking call, passing
its signals by reference:

```rust
use tpt_pulse_signals::{signal_graph, Signal};

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
```

Sub-graph calls are inlined, so a `set` in the caller reaches the sub-graph's signals,
and the cycle check covers the composed graph. Limits, from the crate docs:

- A sub-graph must be declared before its callers in the same module.
- Each sub-graph needs a distinct name, since the name identifies its macro.
- A sub-graph may only declare signals, must end with one of them, and cannot call
  `set` or another sub-graph.
- A direct call to a sub-graph outside a graph panics.

Cross-crate sub-graphs and nested sub-graphs are not supported yet.

## Dependencies and the cycle check

The macro records which signal depends on which, then orders the updates. The order
is checked with [`tpt-pulse-ir`](../tpt-pulse-ir/README.md). A cyclic graph is a
compile error:

```text
error: signal graph is cyclic: circular ordering: sensor -> combined -> sensor; rejected by tpt-telos
```

The check runs before code is generated, so a cyclic graph never compiles.

## Using signals from UI frameworks

The graph itself is framework-neutral. To connect it to a UI, use a bridge:

- [`tpt-pulse-backend-leptos`](../tpt-pulse-backend-leptos/README.md): `Live` values,
  and `derived` or `derived2` memos.
- [`tpt-pulse-backend-dioxus`](../tpt-pulse-backend-dioxus/README.md): `Live` values,
  and the `use_derived2` hook.

A bridge reads the graph's inputs and writes its output, so the graph itself does not
change.

## Guarantees

- **Compile-time order.** Updates happen in a fixed order computed by the macro, so
  `set` does not sort dependencies at run time.
- **No allocation during propagation.** `tests/no_alloc.rs` installs a counting global
  allocator and confirms that `set` and `get` allocate nothing.
- **Cycles are rejected at compile time.** Signals cannot depend on themselves, directly
  or through other signals.

## Limitations

- **Dependencies are limited to one annotated function.** Closures and `Signal::new`
  initializers may not read signals directly.
- **`set` applies only to sources**, and only as a top-level statement.
- **Sub-graphs are in-module only** (see [Sub-graphs](#sub-graphs)).
- **No DOT export.** The macro lowers the graph away at compile time, so there is no
  runtime structure to draw.
- **The crate is not published** (`publish = false`).

## Testing and benchmarks

- `cargo test -p tpt-pulse-signals` runs the robotics and composition tests, the
  allocation test, and the trybuild cases in `tests/compile_fail.rs` (with snapshots in
  `tests/ui/`).
- `cargo bench -p tpt-pulse-signals` runs the propagation benchmark
  (`benches/propagation.rs`).

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
