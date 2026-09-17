# Changelog

## Unreleased

### Added

- Cargo workspace, dual licensing, contribution guidance, and CI checks.
- Synchronous session channels with sealed protocols, recursive duality,
  send/receive continuations, and explicit choice/offer branching.
- Runtime unfinished-endpoint detection and explicit disconnection.
- Prelude, runnable login example, both login branch tests, transport/drop tests,
  and compile-fail fixtures for invalid sequencing, types, reuse, and early close.
- Clearly labeled, non-publishable scaffolds for future signal, macro, IR,
  multiparty, and backend implementations.

### Design clarifications

- `recv()` returns a message/continuation tuple; `offer()` returns branch endpoints.
- Runtime Drop checks are not compile-time linear typing or deadlock proofs.
