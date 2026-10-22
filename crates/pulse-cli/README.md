# cargo pulse

Check and render multiparty protocol files from the command line.

```sh
cargo install --path crates/pulse-cli          # from the workspace root
cargo pulse check examples/protocols/login.pulse
cargo pulse check examples/protocols/query.pulse
cargo pulse mermaid examples/protocols/query.pulse
```

## Commands

- `cargo pulse check <file>` parses and checks the protocol, then prints each
  role. A role without choices or loops shows its `pulse-core` type, for example
  `Client: Send<AuthReq, Recv<AuthOk, Close>>`. A role with choices or loops shows
  an outline of its actions instead.
  It exits with status 1 on a malformed protocol and 2 on usage or I/O errors.
- `cargo pulse mermaid <file>` prints a Mermaid sequence diagram. Choices become
  `alt` blocks and loops become `loop` blocks.

## Protocol file format

A protocol is a sequence of statements.

```text
// Comments run from // to the end of the line.
Client -> Server : AuthReq;
Server -> Client : AuthOk;

loop {
    Client -> Server : Ask;
    choice by Server {
        Server -> Client : Answer;
        continue;
    } or {
        Server -> Client : Done;
    }
}
```

Grammar:

```text
protocol   := statement*
statement  := role "->" role ":" label ";"
            | "loop" "{" statement* "}"
            | "choice" "by" role "{" statement* "}" ("or" "{" statement* "}")*
            | "continue" ";"
role       := identifier, not a keyword
label      := identifier
identifier := letter or digit or "_", one or more
```

- **Messages** run in order. Each role's projection is its own sends and receives.
- **Labels** are message names. They are the names of the Rust message types, so
  the Rust side should use the same names.
- **Choices** are made by the decider. It tells every other role which branch it
  took, so each role follows the same path. Branches are numbered from 0 in the
  order written.
- **Loops** run their body, then exit when the body falls off its end.
- **`continue`** repeats the innermost loop. It must be the last statement of a
  choice branch inside a loop. Every repeat therefore starts with a choice that
  all roles hear, so no role can spin without communicating.
- **Keywords** are `loop`, `choice`, `by`, `or`, and `continue`. They cannot be role names.

Every step ends with `;`. Blank lines and comments are ignored.

The Rust `global_protocol!` macro takes only plain messages so far. Use this
format, or `GlobalProtocol::parse`, for choices and loops.

## What `check` verifies

- The structure is well formed: no message to oneself, `continue` only where the
  grammar above allows it, every choice has a branch and at least one other role.
- A protocol without choices or loops is checked as a session. Every send has a
  matching receive, and the role programs cannot deadlock.
- A protocol with choices or loops relies on the broadcast of each choice. Every
  role follows the same path, so the projected roles cannot wait on each other.
  The workspace's simulation tests run these projections on FIFO queues under
  several branch policies, and check that none of them gets stuck.

Parsing reports the line of the problem, for example:

```text
bad.pulse: line 2: expected `->` after the sender
```
