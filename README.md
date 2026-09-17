# tpt-pulse

Session-typed channels and planned compile-time reactive signals for Rust.
Dual licensed **MIT OR Apache-2.0**, by **TPT Solutions**.

## Status

Synchronous two-party channels are implemented and tested. Signal DAGs,
multiparty projection/verification, and Tokio/Leptos/Dioxus integrations are
**not implemented**; their crates are non-publishable scaffolds.
See [todo.md](todo.md), [spec.txt](spec.txt), and [ARCHITECTURE.md](ARCHITECTURE.md).
The spec is a design sketch, not a promise that all illustrated APIs exist.

## Getting started

From this checkout, run `cargo run --example login`. The example exercises
both successful and rejected login branches on two threads. It uses
`tpt_pulse::prelude::*`; see [the complete example](examples/login.rs).
For local integration, depend on the root package through a Cargo path dependency.
The crates have not been published. Metadata URLs follow the sibling tpt-telos
organization convention; repository hosting and release URLs need confirmation.

### Implemented API

- `session_channel::<P>()` returns `Chan<P, Client>` and `Chan<P::Dual, Server>`.
- `Send<M, P>` and `Recv<M, P>` default their continuation to `Close`.
- `send(message)` consumes the endpoint and returns its continuation.
- `recv()` consumes the endpoint and returns `(message, continuation)`.
- `Choose<L, R>` supports `choose_left()` and `choose_right()`.
- Its dual `Either<L, R>` supports `offer()`, returning
  `Either::Left(chan)` or `Either::Right(chan)` with typed continuations.
- Completed endpoints may be dropped or explicitly `close()`d.

The implemented login protocol is
`Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>`.
Unlike the pseudocode in spec section 4, branching is an explicit `offer()`
operation, not a special-case `recv()`. To send a sum *value* instead of branching
protocols, `Recv<Either<AuthOk, AuthErr>>` also works with ordinary tuple-returning
`recv()`; it does not produce branch-specific continuation channels.

### Guarantees and limitations

Protocol ordering, message types, endpoint consumption, and dual peer types are
compile-time guarantees. Abandonment detection is a **runtime Drop check**, not
linear typing: unfinished endpoints panic on ordinary drop, but avoid a second
panic during unwinding. `mem::forget`, process termination, and explicit
`disconnect()` bypass that check. `disconnect()` releases the transport;
queued messages can still be received before disconnection is observed.

The transport uses two unbounded `std::sync::mpsc` queues and boxed owned
`Send + 'static` messages. It allocates; sends enqueue and receives block.
Transport failure panics. Closing does not perform a peer handshake. Duality
alone does not prove deadlock freedom across multiple sessions. There is no
network transport or reactive zero-allocation claim in this implementation.

## Development

Use stable Rust (validated locally with Rust 1.97.1). Run `cargo test --workspace`,
`cargo fmt --all -- --check`, and
`cargo clippy --workspace --all-targets -- -D warnings`.
Build API documentation with `cargo doc --workspace --no-deps`.
See [CONTRIBUTING.md](CONTRIBUTING.md) for test snapshot guidance.
