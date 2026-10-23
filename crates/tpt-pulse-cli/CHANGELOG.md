# Changelog

All notable changes to `tpt-pulse-cli` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- `cargo pulse check <file>`: parses and checks a protocol file, then prints each role.
  Exits with status 1 for a malformed protocol and 2 for usage or I/O errors.
- `cargo pulse mermaid <file>`: prints a Mermaid sequence diagram, with `alt` blocks for
  choices and `loop` blocks for loops.
- Example protocol files: `examples/protocols/login.pulse` and
  `examples/protocols/query.pulse`.

### Known gaps

- `check` does not report deadlocks for branching protocols, because the broadcast
  projection rules them out by construction.
