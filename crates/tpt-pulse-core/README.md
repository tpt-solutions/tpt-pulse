# tpt-pulse-core

Synchronous, in-memory session-typed channels for Rust. A protocol is a Rust type
such as `Send<AuthReq, Recv<AuthOk, Close>>`. Each endpoint carries its protocol
state in its type, so the compiler rejects an operation that the protocol does not
allow at that point.

Part of [tpt-pulse](../../README.md). Most users should depend on the root
`tpt-pulse` crate and use its `prelude`, which re-exports everything here.

## Contents

- [Quick start](#quick-start)
- [Protocol types](#protocol-types)
- [Operations](#operations)
- [Branching](#branching)
- [Recursion](#recursion)
- [Errors and non-panicking operations](#errors-and-non-panicking-operations)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Feature and crate map](#feature-and-crate-map)

## Quick start

```rust
use tpt_pulse_core::{session_channel, Chan, Client, Either, Recv, Send};

struct AuthReq {
    user: String,
}
struct AuthOk(String);
struct AuthErr(String);
type LoginProto = Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>;

fn login_client(chan: Chan<LoginProto, Client>, user: &str) -> Result<String, String> {
    match chan.send(AuthReq { user: user.into() }).offer() {
        Either::Left(chan) => {
            let (AuthOk(message), chan) = chan.recv();
            chan.close();
            Ok(message)
        }
        Either::Right(chan) => {
            let (AuthErr(message), chan) = chan.recv();
            chan.close();
            Err(message)
        }
    }
}

fn main() {
    let (client, server) = session_channel::<LoginProto>();
    let peer = std::thread::spawn(move || {
        let (request, server) = server.recv();
        if request.user == "admin" {
            server.choose_left().send(AuthOk("Welcome, admin".into())).close();
        } else {
            server.choose_right().send(AuthErr("Unknown user".into())).close();
        }
    });
    println!("{:?}", login_client(client, "admin"));
    peer.join().expect("server completed");
}
```

`session_channel::<P>()` returns the client endpoint of type `Chan<P, Client>` and
the server endpoint of type `Chan<P::Dual, Server>`. The server's type is derived
from the protocol, so both sides agree on the protocol by construction.

The full runnable version, with both branches, is `examples/login.rs` at the
workspace root: `cargo run --example login`.

## Protocol types

| Type | Meaning |
| --- | --- |
| `Send<M, P>` | Send a message of type `M`, then continue as `P`. |
| `Recv<M, P>` | Receive a message of type `M`, then continue as `P`. |
| `Close` | End of the session. Default for the continuation parameter. |
| `Either<L, R>` | Protocol state with two branches. The peer picks one. |
| `Choose<L, R>` | The endpoint that picks a branch, used for the other side of `Either`. |
| `Rec<P>`, `Var` | Recursive protocols (see [Recursion](#recursion)). |

`Client` and `Server` are role markers. `Protocol` and `Dual` are sealed traits:
`Protocol` marks types that can name a session, and `Dual` maps a protocol to its
peer. You cannot implement them yourself. The `on_unimplemented` messages explain
which part of a protocol is invalid.

Message types are plain Rust types. Nothing is serialized in this crate, so a
message can be any `'static` type. The async transport in
[`tpt-pulse-backend-tokio`](../tpt-pulse-backend-tokio/README.md) adds the `serde`
requirement.

## Operations

Each operation consumes the endpoint and returns the next one, so an old endpoint
cannot be used again.

| Operation | On | Result |
| --- | --- | --- |
| `send(msg)` | `Chan<Send<M, P>, R>` | `Chan<P, R>` |
| `recv()` | `Chan<Recv<M, P>, R>` | `(M, Chan<P, R>)` |
| `offer()` | `Chan<Either<L, R>, R>` | `Either<Chan<L, R>, Chan<R, R>>` |
| `choose_left()` | `Chan<Choose<L, R>, R>` | `Chan<L, R>` |
| `choose_right()` | `Chan<Choose<L, R>, R>` | `Chan<R, R>` |
| `close()` | `Chan<Close, R>` | `()` |

`send` and `recv` panic if the peer has gone away. Use the `try_` forms to get a
`SessionError` instead (see below).

## Branching

`Either<L, R>` is a protocol state. The peer chooses, and the other side waits
with `offer()`, which returns the branch it was given. The chooser calls
`choose_left()` or `choose_right()`. Each branch has its own continuation type,
so the code after `offer()` must handle both branches with different protocols.

Branches can be nested. A choice that appears after a `send` is still checked: the
type of the endpoint after the `send` includes the choice.

## Recursion

`Rec<P>` names a protocol that repeats, and `Var` refers back to the loop head.
`Chan::enter` begins a pass through the loop. The loop head has the same endpoint
type on every iteration, so an ordinary Rust `loop` drives the protocol:

A client that sends a bounded number of ticks, then finishes:

```rust
use tpt_pulse_core::{Choose, Chan, Client, Close, Either, Rec, Recv, Send, Var};

struct Tick;
struct Done;

type Body = Choose<Send<Tick, Var>, Send<Done, Close>>;
type Counter = Rec<Body>;
type CounterPeer = Rec<Either<Recv<Tick, Var>, Recv<Done, Close>>>;

fn client(mut chan: Chan<Counter, Client>, ticks: u32) {
    let mut sent = 0;
    loop {
        let body = chan.enter();
        if sent == ticks {
            body.choose_right().send(Done).close();
            return;
        }
        chan = body.choose_left().send(Tick);
        sent += 1;
    }
}
```

`Chan::enter` begins one pass through the loop body. The server's endpoint has type
`CounterPeer` and follows the same loop with `offer()`. A loop needs an exit branch
that reaches `Close`, because an endpoint at `Rec<P>` must eventually close.

`Subst` is the trait that performs the substitution. You normally do not name it.
Recursion is available only in this two-party API. The multiparty checks in
[`tpt-pulse-multiparty`](../tpt-pulse-multiparty/README.md) cover loops in the text
format.

## Errors and non-panicking operations

Every blocking operation has a `try_` form that returns `Result<_, SessionError>`
instead of panicking:

- `try_send`, `try_recv`, `try_offer`, `try_choose_left`, `try_choose_right`.
- `recv_timeout` returns `Result<(M, Chan), RecvTimeout<Self>>`. `RecvTimeout::Elapsed`
  gives back the endpoint, still waiting, so the receive can be retried.
  `RecvTimeout::Failed` means the session failed, and the endpoint is consumed.
- `disconnect()` aborts a session on purpose, so the unfinished-endpoint check does
  not panic.

`SessionError` has two variants:

- `Disconnected`: the peer dropped its endpoint, or the transport closed.
- `TypeMismatch`: the peer sent a message or selection the protocol does not expect.

A failed `try_` operation consumes the endpoint, so after an error the session is over.

## Guarantees

- **Ordering.** Each operation is only available in the state that allows it. A `send`
  on a `Recv` endpoint does not compile (see `tests/compile_fail`).
- **Message types.** `recv` returns the declared type, and `send` accepts only that type.
- **Endpoint consumption.** Operations move the endpoint, so a session cannot be
  reused. The compile-fail tests cover reuse.
- **Peer compatibility.** `session_channel` gives the two ends types that are duals of
  each other. A mismatched peer does not compile.

## Limitations

- **Drop check is a runtime check.** An unfinished endpoint panics when dropped. That
  is not linear typing: `mem::forget`, process exit, and an explicit `disconnect()`
  bypass it.
- **Duality is per session.** It checks that two ends agree. It does not check
  deadlock freedom across several sessions. Use `tpt-pulse-multiparty` for that.
- **Synchronous and in-memory.** Messages travel over `std::sync::mpsc`, which
  allocates per message. Transport failure panics in the blocking API.
- **No peer handshake.** Closing an endpoint does not confirm that the peer finished.

## Feature and crate map

This crate has no feature flags and no runtime dependencies beyond the standard library.

| Need | Crate |
| --- | --- |
| Async sessions over TCP, Unix sockets, or any stream | [tpt-pulse-backend-tokio](../tpt-pulse-backend-tokio/README.md) |
| Several roles, with projection and deadlock checks | [tpt-pulse-multiparty](../tpt-pulse-multiparty/README.md) |
| Compile-time reactive signals | [tpt-pulse-signals](../tpt-pulse-signals/README.md) |

## Testing

- `cargo test -p tpt-pulse-core` runs the runtime tests (`tests/login.rs`,
  `tests/misuse.rs`) and the trybuild compile-fail cases in `tests/compile_fail.rs`,
  with their `.stderr` snapshots in `tests/ui/`.
- `cargo bench -p tpt-pulse-core` runs the exchange benchmark (`benches/exchange.rs`).
- To update a compile-fail snapshot after an intended diagnostic change, follow the
  steps in [CONTRIBUTING.md](../../CONTRIBUTING.md).

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
