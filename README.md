# tpt-pulse

Session-typed channels, compile-time reactive signals, and verified multiparty
protocols for Rust. Dual licensed **MIT OR Apache-2.0**, by **TPT Solutions**.

## Status

Implemented and tested:

- Synchronous two-party session channels (`tpt-pulse-core`).
- Async session channels over tokio TCP (`tpt-pulse-backend-tokio`).
- Compile-time signal graphs via `#[signal_graph]` (`tpt-pulse-signals`, `tpt-pulse-macros`).
- Multiparty protocol projection, deadlock checks, and per-role session types
  (`tpt-pulse-multiparty`, `global_protocol!`, `session_types!`).
- Acyclicity and deadlock checks run through the `tpt-telos` solver (`tpt-pulse-ir`).
- Leptos and Dioxus bridges for signal values (`tpt-pulse-backend-leptos`,
  `tpt-pulse-backend-dioxus`), with example apps that render on the server.
- Non-panicking `try_` operations returning `SessionError`, in `tpt-pulse-core` and
  `tpt-pulse-backend-tokio`.
- Recursive two-party protocols with `Rec`, `Var`, and `Chan::enter`.
- Async transports over TCP, Unix domain sockets, and an in-memory pipe.
- `cargo pulse check` and `cargo pulse mermaid` for protocol files
  (`crates/tpt-pulse-cli`, [format](crates/tpt-pulse-cli/README.md)).
- Mermaid and DOT export, and a `Monitor` that checks a trace against a role's program.
- `assert_protocol!` for checking a recorded trace in tests, and `simulate` for running
  every role of a protocol under a branch policy (`tpt-pulse-multiparty`).
- Optional TLS over tokio-rustls, behind the `tls` feature of `tpt-pulse-backend-tokio`.
- Typed peer stubs in TypeScript and Python, generated from JSON Schemas of the message
  types (`typescript_stubs`, `python_stubs`, behind the `schema` feature). They were
  type-checked by hand with `tsc --strict` and `mypy --strict`, not in CI.
- A starter project in [templates/starter](templates/starter/): one two-party session and a
  test that checks the client's trace.

Crates are not published. See [todo.md](todo.md), [spec.txt](spec.txt), and
[ARCHITECTURE.md](ARCHITECTURE.md). The spec is a design sketch; this README and
the tests describe what exists.

**Depends on [tpt-telos](https://github.com/tpt-solutions/tpt-telos)**, fetched from
git and pinned to a commit in `Cargo.toml`.

## Getting started

```sh
cargo run --example login         # synchronous session channels
cargo run --example login_async   # the same protocol over tokio TCP
cargo run --example robotics      # a compile-time signal graph
cargo run --example multiparty    # multiparty projection and deadlock detection
cargo run --example leptos_app    # a Leptos component driven by a signal graph (HTML)
cargo run --example dioxus_app    # a Dioxus component driven by a signal graph (HTML)
```

See [examples/](examples/README.md) for the source. The examples use `tpt_pulse::prelude::*`.
The protocol files for `cargo pulse` are in [examples/protocols/](examples/protocols/).

## Migrating from the sketch in spec.txt

The spec sketches APIs that the implementation refines:

- **Branching is explicit.** Spec §4 shows `chan.recv()` returning an `Either`.
  The implementation calls `offer()` on an `Either<L, R>` protocol state, as in
  `Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>`, so each branch has its own
  continuation type. Sending a sum *value* uses `Recv<Either<AuthOk, AuthErr>>`
  with plain `recv()`.
- **Signals are declared inside `#[signal_graph]` functions.** Write
  `#[signal_graph] fn robotics_control_loop() -> f64 { ... }`. The code keeps the
  spec's `Signal::new` / `map` / `map2` / `set` shape, and `.get()` reads the value.
  Signals outside a `#[signal_graph]` function cannot be used; their methods panic.
- **Async operations are `async`.** `chan.send(msg).await`, `chan.recv().await`,
  `chan.offer().await`, and so on. Connect with `connect::<P>(addr)` and accept with
  `accept::<P>(&listener)`, where the server endpoint has type `P::Dual`.
- **Multiparty protocols are checked, not just declared.** Use
  `global_protocol! { A -> B : Msg; B -> A : Reply; }` to build and check a protocol,
  `session_types! { Name { A -> B : Msg; B -> A : Reply; } }` to get each role's
  `tpt-pulse-core` type as `Name::A` and `Name::B`, or `Session::new` with hand-written
  per-role programs to check those.
- **UI values use `Live`.** Leptos and Dioxus components read and write
  `tpt_pulse_backend_leptos::Live` or `tpt_pulse_backend_dioxus::Live`. A graph's output
  becomes a memo with `derived`/`derived2` (Leptos) or `use_derived2` (Dioxus).

## Guarantees and limitations

Protocol ordering, message types, endpoint consumption, and dual peer types are
compile-time guarantees for `tpt-pulse-core`. Abandonment detection is a **runtime Drop
check**, not linear typing: unfinished endpoints panic on ordinary drop, but
`mem::forget`, process termination, and explicit `disconnect()` bypass that check.

Signal graphs are checked at compile time, and propagation generates no heap
allocation. Dependencies are limited to a single annotated function: closures
and `Signal::new` initializers may not read signals directly, and `set` applies only
to sources, as a top-level statement.

Multiparty checks are sound for asynchronous FIFO channels: if they pass, some
interleaving never leaves a receive waiting. The text format supports `choice` and
`loop`. The decider broadcasts each branch, so every role follows the same path.
The structural checks and a bounded simulation support the deadlock-freedom claim,
but they do not prove it for arbitrary protocols. `session_types!` and
`global_protocol!` take only plain messages, since their output is one `tpt-pulse-core`
type per role, and a choice needs an announcement to every role.

The synchronous transport allocates (boxed messages over `std::sync::mpsc`), and
transport failure panics. The tokio transport frames messages as JSON, so message
types need `serde`. Closing does not perform a peer handshake.

The UI bridges move values into framework signals. Leptos and Dioxus output was
checked on the server and, for Dioxus, by re-rendering the virtual DOM. Browser
behavior was not tested here.

## Development

The toolchain is pinned to Rust 1.97.1 in `rust-toolchain.toml`. The minimum supported
version is Rust 1.89, which CI checks. From the workspace root:

- `cargo test --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p tpt-pulse-backend-tokio --features tls`
- `cargo test -p tpt-pulse-multiparty --features schema`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`

The Leptos and Dioxus bridge tests also run on `wasm32-unknown-unknown` under Node, with
`wasm-bindgen-test` (see the `wasm` job in CI).

See [CONTRIBUTING.md](CONTRIBUTING.md) for test snapshot guidance.
