# tpt-pulse-macros

Procedural macros for tpt-pulse. The macros are re-exported by the crates that use
them, so you usually do not depend on this crate directly.

Part of [tpt-pulse](../../README.md).

## Macros

| Macro | Re-exported from | Purpose |
| --- | --- | --- |
| `#[signal_graph]` | `tpt_pulse_signals`, `tpt_pulse::prelude` | Lowers a signal graph into plain locals with inlined updates, and rejects cycles. |
| `global_protocol!` | `tpt_pulse::prelude` | Declares a multiparty protocol and checks it at compile time. |
| `session_types!` | `tpt_pulse::prelude` | Declares a multiparty protocol and emits a `tpt-pulse-core` type for each role. |

### `#[signal_graph]`

Applied to a function that creates signals with `Signal::new`, combines them with
`map` and `map2`, and updates sources with `set`. See
[tpt-pulse-signals](../tpt-pulse-signals/README.md) for the user-facing guide and the
sub-graph rules. This crate implements the expansion, including the internal
`__compose!` macro that inlines sub-graphs.

### `global_protocol!`

Takes the same statements as the text format: messages, `choice`, and `loop`.

```rust
use tpt_pulse::prelude::*;

let protocol = global_protocol! {
    Client -> Gateway : Request;
    Gateway -> Client : Answer;
};
let session = protocol.project().expect("protocol is well formed");
session.verify().expect("projected session is deadlock-free");
```

A malformed step is a compile error. The protocol is projected to each role, and the
projection is checked for structure and deadlock freedom. See
[tpt-pulse-multiparty](../tpt-pulse-multiparty/README.md).

### `session_types!`

Emits, for each role, a `tpt-pulse-core` protocol type. Each endpoint gets a type-level
protocol, and the compiler checks that the roles are duals of each other:

```rust
use tpt_pulse::prelude::*;

struct AuthReq;
struct AuthOk;

session_types! {
    Login {
        Client -> Server : AuthReq;
        Server -> Client : AuthOk;
    }
}
// Login::Client and Login::Server are now protocol types.
```

The labels name Rust message types, so those types must be in scope.

It takes only plain messages. A choice or loop needs an announcement to every role,
which this macro does not generate. Use `global_protocol!` and the text format for
those.

## How the expansions refer to other crates

The expansions name `::tpt_pulse_signals` and `::tpt_pulse_multiparty` by absolute
path. A crate that uses these macros must depend on those crates directly, not only
through the root `tpt-pulse` crate. The root crate's `prelude` already does this.

## Guarantees

- Cycles in a signal graph and deadlocks in a protocol are compile errors, not run-time
  failures.
- Expansion adds no run-time machinery to a signal graph.

## Limitations

- `session_types!` and `global_protocol!` take only plain messages, not choices or loops
  in the per-role types.
- Sub-graphs must be declared before their callers, in the same module.

## Testing

Macro behavior is tested through the crates that use it: `tpt-pulse-signals`
(`tests/robotics.rs`, `tests/composition.rs`, `tests/compile_fail.rs`) and
`tpt-pulse-multiparty`. The crate's own unit tests in
`src/multiparty.rs` cover the protocol expansion. `src/signal.rs` has the signal expansion tests.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
