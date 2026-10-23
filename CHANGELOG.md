# Changelog

## Unreleased

### Added

- Cargo workspace, dual licensing, contribution guidance, and CI checks.
- Synchronous session channels with sealed protocols, recursive duality,
  send/receive continuations, and explicit choice/offer branching.
- Runtime unfinished-endpoint detection and explicit disconnection.
- Prelude, runnable login example, both login branch tests, transport/drop tests,
  and compile-fail fixtures for invalid sequencing, types, reuse, and early close.
- `tpt-pulse-ir`: ordering checks (schedules as strict rank constraints) decided by
  the `tpt-telos` solver.
- `tpt-pulse-signals` and `tpt-pulse-macros`: `#[signal_graph]` lowers signal declarations
  and `set` into plain locals with inlined recomputation, checked for cycles at
  compile time. A counting-allocator test confirms zero allocation during propagation.
- `tpt-pulse-multiparty`: global protocols, per-role projection, and deadlock checks on
  local programs. `global_protocol!` verifies a protocol at compile time.
- `session_types!`: emits a `tpt-pulse-core` protocol type for each role, so each
  endpoint has a type-level protocol and dual roles are checked by the compiler.
- `tpt-pulse-backend-tokio`: async session channels over tokio TCP with length-prefixed
  JSON frames, plus a TCP login test and `login_async` example.
- `tpt-pulse-backend-leptos`: `Live` values and `derived`/`derived2` memos for Leptos.
- `tpt-pulse-backend-dioxus`: `Live` values with `use_live` and `use_derived2` hooks for Dioxus.
- Examples for the robotics signal graph, the multiparty protocol, and Leptos and
  Dioxus apps. The Leptos and Dioxus examples render HTML on the server.
- `tpt-pulse-core`: `try_send`, `try_recv`, `recv_timeout`, `try_offer`, `try_choose_left`,
  and `try_choose_right`, which return `SessionError` instead of panicking. `recv_timeout`
  returns the endpoint on timeout so the receive can be retried. `disconnect()` aborts
  a session without panicking.
- `tpt-pulse-core`: recursive protocols with `Rec<P>`, `Var`, and `Chan::enter`. A loop head
  has one endpoint type on every iteration, so loops run in an ordinary Rust `loop`.
- `tpt-pulse-core`: `#[diagnostic::on_unimplemented]` messages for `Protocol` and `Dual`.
- `tpt-pulse-backend-tokio`: `try_` forms of every operation, a generic transport
  (`AsyncChan::from_stream`), an in-memory `pair` for tests, Unix domain sockets
  (`connect_unix`, `accept_unix`), and `disconnect()`.
- `tpt-pulse-multiparty`: `GlobalProtocol::parse` for the protocol file format,
  `GlobalProtocol::to_mermaid` for sequence diagrams, and `Monitor`, which checks an
  observed trace against one role's program.
- `tpt-pulse-ir`: `OrderGraph::to_dot` for Graphviz output.
- `cargo pulse` (`crates/tpt-pulse-cli`): `check` and `mermaid` commands over protocol files.
  Examples: `examples/protocols/login.pulse` and `examples/protocols/query.pulse`.
- `tpt-pulse-multiparty`: `choice` (decider-broadcast branches) and `loop` with `continue`
  in the text format. Projection, the `Monitor` for branching roles, and Mermaid
  `alt`/`loop` blocks. Parse errors now report a line number.
- `tpt-pulse-backend-tokio`: `recv_timeout`, which returns the endpoint on timeout so the
  receive can be retried.
- `assert_protocol!` (`tpt-pulse-multiparty`, re-exported from the prelude): checks a trace of
  events against one role's program.
- `rust-version = "1.89"` for every crate, with a CI job that checks the workspace on 1.89.
- `cargo deny` policy (`deny.toml`) and CI job.
- Benchmarks for the sync exchange and signal propagation, compiled in CI.
- `Cargo.lock` is no longer ignored, so builds are reproducible.
- `global_protocol!` takes choice and loop, with the same statements as the text format.
- `tpt_pulse_multiparty::simulate`: runs every role of a protocol under a branch policy, and
  returns the event trace. A building block for mock peers.
- `tpt-pulse-backend-tokio` `tls` feature: `connect` and `accept` over tokio-rustls (ring provider).
- `templates/starter`: a starter two-party session with a test.
- The Leptos and Dioxus bridge tests run as wasm32 under Node, with `wasm-bindgen-test`.
- `tpt-pulse-signals`: sub-graphs. A `#[signal_graph]` fn with `Signal` parameters can be called from another graph, and `set` in the caller reaches its signals. Limits are in the crate docs.
- Mock peers with payloads (`simulate_messages`), TypeScript and Python stubs, and the schema registry (feature `schema`).

### Known gaps

- Leptos and Dioxus apps were verified by server-side rendering, not in a browser.
- `session_types!` and `global_protocol!` take only plain messages. Choices and loops
  are in the text format and `GlobalProtocol` API. Recursion exists only in the
  two-party `tpt-pulse-core` API.
- Choices and loops are checked structurally and by a bounded simulation. This is not
  a proof of deadlock freedom for arbitrary protocols.
- `cargo pulse check` does not report deadlocks for branching protocols, because
  the broadcast projection rules them out by construction. Hand-written role
  programs can still deadlock, and `Session::verify` reports that for them.
- Tokio `offer` has no timeout. `recv_timeout` covers receives only.
- Signal graphs have no DOT export, because the macro lowers them away at compile time.
- The `tpt-telos` solver reports `x - x > 0` as satisfiable because it does not merge
  duplicate variable terms. `tpt-pulse-ir` never emits duplicate terms, but the solver is wrong.

### Design clarifications

- `recv()` returns a message/continuation tuple; `offer()` returns branch endpoints.
- Runtime Drop checks are not compile-time linear typing or deadlock proofs.
- The signal and multiparty checks depend on `tpt-telos`, pulled from git at a pinned commit.
