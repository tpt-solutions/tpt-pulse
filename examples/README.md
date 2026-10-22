# Examples

Run each example from the workspace root.

- `cargo run --example login`: [login.rs](login.rs). Synchronous session channels
  with a client and server on two threads, covering both authentication branches.
- `cargo run --example login_async`: [login_async.rs](login_async.rs). The same
  protocol over tokio TCP sockets.
- `cargo run --example robotics`: [robotics.rs](robotics.rs). A `#[signal_graph]`
  function whose signals update without runtime dependency tracking.
- `cargo run --example multiparty`: [multiparty.rs](multiparty.rs). A three-role
  protocol checked at compile time, and a hand-written deadlock that is detected.

These examples are demonstrations, not real authentication. The Leptos and Dioxus
UI demos are deferred to a later release.
