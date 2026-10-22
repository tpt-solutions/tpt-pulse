# tpt-pulse

Session-typed channels, compile-time reactive signals, and verified multiparty
protocols for Rust. Dual licensed **MIT OR Apache-2.0**, by **TPT Solutions**.

## Status

Implemented and tested:

- Synchronous two-party session channels (`pulse-core`).
- Async session channels over tokio TCP (`pulse-backend-tokio`).
- Compile-time signal graphs via `#[signal_graph]` (`pulse-signals`, `pulse-macros`).
- Multiparty protocol projection, deadlock checks, and per-role session types
  (`pulse-multiparty`, `global_protocol!`, `session_types!`).
- Acyclicity and deadlock checks run through the `tpt-telos` solver (`pulse-ir`).
- Leptos and Dioxus bridges for signal values (`pulse-backend-leptos`,
  `pulse-backend-dioxus`), with example apps that render on the server.

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

See [examples/](examples/) for the source. The examples use `tpt_pulse::prelude::*`.

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
  `pulse-core` type as `Name::A` and `Name::B`, or `Session::new` with hand-written
  per-role programs to check those.
- **UI values use `Live`.** Leptos and Dioxus components read and write
  `pulse_backend_leptos::Live` or `pulse_backend_dioxus::Live`. A graph's output
  becomes a memo with `derived`/`derived2` (Leptos) or `use_derived2` (Dioxus).

## Guarantees and limitations

Protocol ordering, message types, endpoint consumption, and dual peer types are
compile-time guarantees for `pulse-core`. Abandonment detection is a **runtime Drop
check**, not linear typing: unfinished endpoints panic on ordinary drop, but
`mem::forget`, process termination, and explicit `disconnect()` bypass that check.

Signal graphs are checked at compile time, and propagation generates no heap
allocation. Dependencies are limited to a single annotated function: closures
and `Signal::new` initializers may not read signals directly, and `set` applies only
to sources, as a top-level statement.

Multiparty checks are sound for asynchronous FIFO channels: if they pass, some
interleaving never leaves a receive waiting. They do not model arbitrary state
machines. `session_types!` generates types for sequential protocols only, since that
is what a global description can express.

The synchronous transport allocates (boxed messages over `std::sync::mpsc`), and
transport failure panics. The tokio transport frames messages as JSON, so message
types need `serde`. Closing does not perform a peer handshake.

The UI bridges move values into framework signals. Leptos and Dioxus output was
checked on the server and, for Dioxus, by re-rendering the virtual DOM. Browser
behavior was not tested here.

## Development

Use stable Rust (validated locally with Rust 1.97.1). From the workspace root:

- `cargo test --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`

See [CONTRIBUTING.md](CONTRIBUTING.md) for test snapshot guidance.
