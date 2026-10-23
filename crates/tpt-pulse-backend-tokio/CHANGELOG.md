# Changelog

All notable changes to `tpt-pulse-backend-tokio` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- Async session channels over tokio TCP, with length-prefixed JSON frames, plus a TCP
  login test and the `login_async` example.
- `try_` forms of every operation, returning `SessionError`.
- `recv_timeout`, which returns the endpoint on timeout so the receive can be retried.
- A generic transport (`AsyncChan::from_stream`) and an in-memory `pair` for tests.
- Unix domain sockets (`connect_unix`, `accept_unix`).
- `disconnect()` to abort a session on purpose.
- Feature `tls`: `connect` and `accept` over tokio-rustls with the `ring` provider.

### Known gaps

- Receive operations are not cancellation safe, so they must not be wrapped in
  `tokio::time::timeout`.
- `offer` has no timeout. `recv_timeout` covers receives only.
- Frames carry no message labels, so a socket wrapper cannot yet feed the multiparty
  `Monitor`.
- The protocol uses JSON on the wire, so message types must implement `serde`.
