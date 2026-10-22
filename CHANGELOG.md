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
- `pulse-backend-tokio`: async session channels over tokio TCP with length-prefixed
  JSON frames, plus a TCP login test and `login_async` example.
- Examples for the robotics signal graph and the multiparty protocol.

### Deferred

- Leptos and Dioxus integration crates remain empty scaffolds.

### Design clarifications

- `recv()` returns a message/continuation tuple; `offer()` returns branch endpoints.
- Runtime Drop checks are not compile-time linear typing or deadlock proofs.
- The signal and multiparty checks depend on `tpt-telos`, pulled from git at a pinned commit.
