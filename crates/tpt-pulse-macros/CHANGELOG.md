# Changelog

All notable changes to `tpt-pulse-macros` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- `#[signal_graph]` lowers signal declarations and `set` into plain locals with inlined
  recomputation, checked for cycles at compile time.
- Sub-graphs: a `#[signal_graph]` function with `Signal` parameters can be called from
  another graph, through the internal `__compose!` macro.
- `global_protocol!` declares a multiparty protocol and verifies it at compile time, with
  the same statements as the text format, including `choice` and `loop`.
- `session_types!` emits a `tpt-pulse-core` protocol type for each role, so each endpoint
  has a type-level protocol and dual roles are checked by the compiler.

### Known gaps

- `session_types!` and `global_protocol!` take only plain messages. Choices and loops are
  in the text format and the `GlobalProtocol` API.
- Sub-graphs must be declared before their callers in the same module. Cross-crate and
  nested sub-graphs are not supported.
