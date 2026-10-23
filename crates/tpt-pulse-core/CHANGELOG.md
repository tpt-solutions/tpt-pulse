# Changelog

All notable changes to `tpt-pulse-core` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- Synchronous session channels with sealed protocols, recursive duality,
  send/receive continuations, and explicit choice/offer branching.
- Runtime unfinished-endpoint detection and explicit disconnection with `disconnect()`.
- `try_send`, `try_recv`, `recv_timeout`, `try_offer`, `try_choose_left`, and
  `try_choose_right`, which return `SessionError` instead of panicking. `recv_timeout`
  returns the endpoint on timeout so the receive can be retried.
- Recursive protocols with `Rec<P>`, `Var`, and `Chan::enter`. A loop head has one
  endpoint type on every iteration, so loops run in an ordinary Rust `loop`.
- `#[diagnostic::on_unimplemented]` messages for `Protocol` and `Dual`.
- Compile-fail fixtures for invalid sequencing, types, endpoint reuse, and early close.
- A runnable login example (`examples/login.rs` at the workspace root) that covers both
  branches.

### Known gaps

- Abandoning an unfinished endpoint panics on drop. This is a runtime check, not linear
  typing: `mem::forget` and process termination bypass it.
- Duality enforces peer compatibility, not deadlock freedom across sessions.
- Messages travel over `std::sync::mpsc`, which allocates per message, and transport
  failure panics in the blocking API.
- Closing does not perform a peer handshake.
