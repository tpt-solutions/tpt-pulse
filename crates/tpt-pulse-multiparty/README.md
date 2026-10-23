# tpt-pulse-multiparty

Multiparty session protocols for Rust. Describe a protocol among several roles
globally, check it, project it to a local program for each role, and verify that
each role's program is deadlock-free. The crate also provides runtime monitors,
mock peers, and generated peer stubs for other languages.

Part of [tpt-pulse](../../README.md).

## Contents

- [The protocol language](#the-protocol-language)
- [Writing protocols in Rust](#writing-protocols-in-rust)
- [Projection and verification](#projection-and-verification)
- [Runtime checks: Monitor and assert_protocol!](#runtime-checks-monitor-and-assert_protocol)
- [Simulation and mock peers](#simulation-and-mock-peers)
- [Stubs for other languages](#stubs-for-other-languages)
- [Mermaid and the CLI](#mermaid-and-the-cli)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Features](#features)

## The protocol language

A protocol is a sequence of statements, from the text format:

```text
Client -> Gateway : Request;
Gateway -> Backend : Forward;
Backend -> Gateway : Reply;
Gateway -> Client : Answer;
```

Statements:

- **Message** `from -> to : label;`. The label names the message type.
- **Choice** `choice by D { ... } or { ... }`. The decider `D` picks a branch, and
  branches are numbered from 0 in the order written.
- **Loop** `loop { ... }`. A branch that ends with `continue;` repeats the loop. A body
  that falls off its end exits.

The grammar is in [`crates/tpt-pulse-cli/README.md`](../tpt-pulse-cli/README.md).

Choices are broadcast. The decider tells every other role which branch it took, so each
role follows the same path without seeing the rest of the protocol. That broadcast is
what keeps every projected path sequential.

## Writing protocols in Rust

Macros in the root crate's prelude check protocols at compile time:

```rust
use tpt_pulse::prelude::*;

let protocol = global_protocol! {
    Client -> Gateway : Request;
    Gateway -> Backend : Forward;
    Backend -> Gateway : Reply;
    Gateway -> Client : Answer;
};
```

`global_protocol!` produces a `GlobalProtocol`. `GlobalProtocol::parse` reads the text
format at run time, so the same protocol can live in a file. `session_types!` also
emits a `tpt-pulse-core` type for each role. See
[tpt-pulse-macros](../tpt-pulse-macros/README.md).

The types in this crate are:

- `GlobalProtocol`: the list of statements, with `roles()`, `project()`, and
  `to_mermaid()`.
- `Statement` and `Step`: the global description.
- `Program` and `Action`: a role's local program is a list of `Action`s. `Send` and `Recv`
  are the plain steps. A decider uses `Choose`, which announces a branch to every other
  role, and the other roles use `Offer`, which waits for that announcement. Loops are
  nested in the same list.
- `Session`: a set of role programs. `Session::new` builds one by hand, and `verify()`
  checks it.
- `branch_label(index)`: the label the decider sends for branch `index`.

## Projection and verification

`GlobalProtocol::project()` checks the structure and returns each role's `Program`.
Errors are `ProtocolError`, and parse errors are `ParseError`, which carry a line
number.

`Session::verify()` checks that the programs can run to completion over asynchronous
FIFO channels. It is sound for that model: if it passes, some interleaving never leaves
a receive waiting. It reports deadlocks for hand-written programs, which the projection
rules out by construction.

```rust
use tpt_pulse::prelude::*;

let session = global_protocol! {
    Client -> Server : Ping;
    Server -> Client : Pong;
}
.project()
.expect("protocol is well formed");
session.verify().expect("projected session is deadlock-free");
```

The full example, which also shows a hand-written deadlock being rejected, is
`examples/multiparty.rs`: `cargo run --example multiparty`.

## Runtime checks: Monitor and assert_protocol!

- `Monitor::new(program)` checks an observed sequence of events against one role's
  program. `observe(&action)` checks each event as it arrives, and `finish()` requires
  that the whole program was followed. Errors are `MonitorError`.
- `assert_protocol!` checks a recorded trace of `Action::send` and `Action::recv`
  events against one role's program. It is re-exported from the prelude and meant for
  tests.

Frames on the wire do not carry message labels, so a socket wrapper cannot yet map bytes
to protocol events. The monitor works on events you supply.

## Simulation and mock peers

- `simulate(protocol, policy, max_steps)` plays every role of a protocol. The `policy`
  closure receives the number of branches at each choice and returns the index to take.
  The result is an `Outcome`: `Finished(Trace)` when every role completed, or
  `CutOff(Trace)` when `max_steps` was reached while roles could still move (typical of
  loops that never exit). A run that goes wrong is a `SimulationError`. A `Trace` is a
  list of `(role, Action)` events.
- `simulate_messages(protocol, schemas, policy, max_steps, payload)` (feature `schema`)
  does the same run and also builds each payload with the `payload` closure, which is
  given the sending role and label. Each payload is checked against its label's JSON
  Schema before it is sent, and the run stops at the first payload that breaks it. It
  returns the `Outcome` and the `Message` values in order.

The simulation runs all roles together. It does not generate a Rust mock type for each
role.

## Stubs for other languages

With feature `schema`, two generators write peer code from the protocol:

- `typescript_stubs(protocol, schemas)` writes TypeScript message types and each role's
  projected program as a typed constant.
- `python_stubs(protocol, schemas)` writes the same shape in Python.

`Schemas` maps each label to the JSON Schema of its Rust message type, from `schemars`.
It also lists the labels a protocol uses that have no schema. Choices and loops are
nested in the generated programs. Errors are `StubError`.

The TypeScript output type-checks under `tsc --strict`, and the Python output under
`mypy --strict` on Python 3.13. Both were checked by hand. They are not CI steps yet.

## Mermaid and the CLI

- `GlobalProtocol::to_mermaid()` returns a Mermaid sequence diagram. Choices become
  `alt` blocks and loops become `loop` blocks.
- `cargo pulse check <file>` and `cargo pulse mermaid <file>` run the same checks from
  the command line. See [tpt-pulse-cli](../tpt-pulse-cli/README.md).

## Guarantees

- Projection checks structure, and every projected path is a sequential run.
- `Session::verify` is sound for asynchronous FIFO channels.
- Stubs and simulation payloads are checked against the schema of their message type.

## Limitations

- **Bounded check, not a proof.** Choices and loops are checked structurally and by a
  bounded simulation. This is not a proof of deadlock freedom for arbitrary protocols.
- **Plain messages only** in the per-role Rust types from `session_types!` and
  `global_protocol!`. Choices and loops are in the text format and the `GlobalProtocol`
  API.
- **`cargo pulse check` does not report deadlocks for branching protocols**, because the
  broadcast projection rules them out by construction. Hand-written role programs can
  still deadlock, and `Session::verify` reports that for them.
- **No socket monitor.** See [Runtime checks](#runtime-checks-monitor-and-assert_protocol).
- **Generated stubs are not CI-checked** against `tsc` or `mypy`.

## Features

- `schema`: JSON Schema registry (`Schemas`), typed mock peers (`simulate_messages`), and
  the TypeScript and Python stub generators. Adds `schemars`, `jsonschema`, and
  `serde_json` as dependencies.

Run the feature's tests with `cargo test -p tpt-pulse-multiparty --features schema`.

## Testing

`cargo test -p tpt-pulse-multiparty` runs the protocol, simulation, assertion, and monitor
tests. With `--features schema`, it also runs the mock, schema, TypeScript, and Python
tests.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
