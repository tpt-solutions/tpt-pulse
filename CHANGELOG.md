# Changelog

## Unreleased

### Added

- Cargo workspace, dual licensing, contribution guidance, and CI checks.
- Synchronous session channels with sealed protocols, recursive duality,
  send/receive continuations, and explicit choice/offer branching.
- Runtime unfinished-endpoint detection and explicit disconnection.
- Prelude, runnable login example, both login branch tests, transport/drop tests,
  and compile-fail fixtures for invalid sequencing, types, reuse, and early close.
- `pulse-ir`: ordering checks (schedules as strict rank constraints) decided by
  the `tpt-telos` solver.
- `pulse-signals` and `pulse-macros`: `#[signal_graph]` lowers signal declarations
  and `set` into plain locals with inlined recomputation, checked for cycles at
  compile time. A counting-allocator test confirms zero allocation during propagation.
- `pulse-multiparty`: global protocols, per-role projection, and deadlock checks on
  local programs. `global_protocol!` verifies a protocol at compile time.
- `session_types!`: emits a `pulse-core` protocol type for each role, so each
  endpoint has a type-level protocol and dual roles are checked by the compiler.
- `pulse-backend-tokio`: async session channels over tokio TCP with length-prefixed
  JSON frames, plus a TCP login test and `login_async` example.
- `pulse-backend-leptos`: `Live` values and `derived`/`derived2` memos for Leptos.
- `pulse-backend-dioxus`: `Live` values with `use_live` and `use_derived2` hooks for Dioxus.
- Examples for the robotics signal graph, the multiparty protocol, and Leptos and
  Dioxus apps. The Leptos and Dioxus examples render HTML on the server.

### Known gaps

- Leptos and Dioxus apps were verified by server-side rendering, not in a browser.
- The `tpt-telos` solver reports `x - x > 0` as satisfiable because it does not merge
  duplicate variable terms. `pulse-ir` never emits duplicate terms, but the solver is wrong.

### Design clarifications

- `recv()` returns a message/continuation tuple; `offer()` returns branch endpoints.
- Runtime Drop checks are not compile-time linear typing or deadlock proofs.
- The signal and multiparty checks depend on `tpt-telos`, pulled from git at a pinned commit.
