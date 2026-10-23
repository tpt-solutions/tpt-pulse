# Changelog

All notable changes to `tpt-pulse-multiparty` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- Global protocols, per-role projection, and deadlock checks on local programs.
  `global_protocol!` verifies a protocol at compile time.
- `GlobalProtocol::parse` for the protocol file format, with parse errors that report a
  line number.
- `GlobalProtocol::to_mermaid` for sequence diagrams.
- `choice` (decider-broadcast branches) and `loop` with `continue` in the text format.
  Projection, the `Monitor` for branching roles, and Mermaid `alt`/`loop` blocks.
- `Monitor`, which checks an observed event sequence against one role's program, and
  `finish()`, which requires the whole program to be followed.
- `assert_protocol!`, re-exported from the prelude, which checks a trace of events against
  one role's program.
- `simulate`, which runs every role of a protocol under a branch policy and returns the
  event trace.
- Feature `schema`: `Schemas` maps each label to the JSON Schema of its message type, and
  `simulate_messages` plays every role, generates each payload, and checks it against its
  schema before it is sent.
- Feature `schema`: `typescript_stubs` and `python_stubs`, which write message types from
  the schemas and each role's projected program as a typed constant, with choices and
  loops nested.

### Known gaps

- Choices and loops are checked structurally and by a bounded simulation. This is not a
  proof of deadlock freedom for arbitrary protocols.
- `cargo pulse check` does not report deadlocks for branching protocols, because the
  broadcast projection rules them out by construction. Hand-written role programs can
  still deadlock, and `Session::verify` reports that for them.
- The socket wrapper for `Monitor` is not done. Frames carry no message labels, so a
  wrapper cannot map bytes to protocol events until labels are added to the wire format.
- The generated TypeScript and Python stubs were checked by hand with `tsc --strict` and
  `mypy --strict`, not in CI.
- The simulation plays all roles together. There is no generated per-role Rust mock type.
