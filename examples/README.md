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
- `cargo run --example leptos_app`: [leptos_app.rs](leptos_app.rs). A Leptos component
  whose motor output is a `#[signal_graph]`, rendered to HTML on the server.
- `cargo run --example dioxus_app`: [dioxus_app.rs](dioxus_app.rs). The same
  component in Dioxus, rendered to HTML on the server.

- `cargo pulse check examples/protocols/login.pulse`: [protocols/login.pulse](protocols/login.pulse).
  The same exchange as a protocol file for the `cargo pulse` command. See
  [crates/tpt-pulse-cli](../crates/tpt-pulse-cli/README.md) for the format.
- `cargo pulse check examples/protocols/query.pulse`: [protocols/query.pulse](protocols/query.pulse).
  A loop with a choice by the server, and an auditor that hears only some branches.

These examples are demonstrations, not real authentication. The Leptos and Dioxus
apps print HTML; running them in a browser needs a client-side build, which is not
set up here.
