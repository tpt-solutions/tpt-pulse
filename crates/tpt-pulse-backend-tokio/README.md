# tpt-pulse-backend-tokio

Async session channels over tokio sockets. The protocol types are the same as in
[tpt-pulse-core](../tpt-pulse-core/README.md): `Send`, `Recv`, `Choose`, `Either`, and
`Close`. Each operation is `async` and mirrors the synchronous API. Each message is
serialized as JSON in a length-prefixed frame, so message types must implement `serde`.

Part of [tpt-pulse](../../README.md). The same login protocol as the synchronous
example, over TCP, is `examples/login_async.rs`: `cargo run --example login_async`.

## Contents

- [Quick start](#quick-start)
- [Transports](#transports)
- [Operations](#operations)
- [TLS](#tls)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Features](#features)

## Quick start

```rust
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tpt_pulse_backend_tokio::{accept, connect, AsyncChan};
use tpt_pulse_core::{Client, Close, Dual, Either, Recv, Send, Server};

#[derive(Serialize, Deserialize)]
struct AuthReq {
    user: String,
}
#[derive(Serialize, Deserialize)]
struct AuthOk(String);
#[derive(Serialize, Deserialize)]
struct AuthErr(String);

type LoginProto = Send<AuthReq, Either<Recv<AuthOk, Close>, Recv<AuthErr, Close>>>;

async fn login_client(chan: AsyncChan<LoginProto, Client>, user: &str) -> Result<String, String> {
    match chan.send(AuthReq { user: user.into() }).await.offer().await {
        Either::Left(chan) => {
            let (AuthOk(message), chan) = chan.recv().await;
            chan.close();
            Ok(message)
        }
        Either::Right(chan) => {
            let (AuthErr(message), chan) = chan.recv().await;
            chan.close();
            Err(message)
        }
    }
}

async fn login_server(chan: AsyncChan<<LoginProto as Dual>::Dual, Server>) {
    let (request, chan) = chan.recv().await;
    if request.user == "admin" {
        chan.choose_left().await.send(AuthOk("Welcome, admin".into())).await.close();
    } else {
        chan.choose_right().await.send(AuthErr("Unknown user".into())).await.close();
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local address");
    let server = tokio::spawn(async move {
        let chan = accept::<LoginProto>(&listener).await.expect("accept");
        login_server(chan).await;
    });
    let client = connect::<LoginProto, _>(addr).await.expect("connect");
    println!("{:?}", login_client(client, "admin").await);
    server.await.expect("server task");
}
```

The server endpoint has type `P::Dual`, so `accept::<P>` and `connect::<P>` agree on the
protocol. Both operations are `async`, and each operation consumes the endpoint and
returns the next one, as in the core crate.

## Transports

| Function | Transport | Notes |
| --- | --- | --- |
| `connect::<P>(addr)` / `accept::<P>(&listener)` | TCP | `addr` is anything `ToSocketAddrs`. |
| `connect_unix::<P>(path)` / `accept_unix::<P>(...)` | Unix domain socket | Unix only. |
| `pair::<P>(buffer)` | In-memory pipe | Returns both endpoints. Intended for tests. |
| `AsyncChan::from_stream(stream)` | Any `AsyncRead + AsyncWrite` | For other transports. |

`AsyncChan<P, Role, S>` has a default stream type of `TcpStream`, so a TCP endpoint is
written `AsyncChan<P, Client>`.

## Operations

Each operation comes in two forms:

- The panicking form: `send`, `recv`, `offer`, `choose_left`, `choose_right`. A transport
  failure panics.
- The `try_` form: `try_send`, `try_recv`, `try_offer`, `try_choose_left`,
  `try_choose_right`. It returns `Result<_, SessionError>`.

Both forms consume the endpoint. A failed `try_` operation leaves the stream in an unknown
state, so drop the endpoint after an error.

`recv_timeout` returns the endpoint on timeout, so the receive can be retried. The
`try_offer` form has no timeout (see the limitations). `disconnect()` aborts a session on
purpose, and `close()` ends it normally.

Like the core crate, an unfinished endpoint panics when dropped, unless it was closed or
disconnected.

## TLS

The `tls` feature adds `tls::connect` and `tls::accept` over `tokio-rustls` with the `ring`
crypto provider. They do the TLS handshake and then return the same `AsyncChan` type, so
the protocol code does not change. The caller supplies the rustls configuration: a client
must trust the server's certificate through its config, and a server must be configured
with its own certificate.

TLS is off by default, so the default build has no TLS dependencies.

## Guarantees

- The protocol is enforced by the types, as in the core crate. Duality holds only when
  both peers name the same protocol.
- Each message is a tagged, length-prefixed JSON frame. A payload larger than the frame
  limit is reported as `SessionError::TypeMismatch` by the `try_` forms.

## Limitations

- **Receives are not cancellation safe.** A future dropped in the middle of a frame leaves
  the stream out of step. Do not wrap receives in `tokio::time::timeout`. Use
  `recv_timeout` instead.
- **`offer` has no timeout.** `recv_timeout` covers receives only.
- **Frames carry no message labels.** A socket wrapper therefore cannot map bytes to
  protocol events, so the multiparty `Monitor` cannot yet watch a live socket.
- **JSON only.** The wire format is JSON, which is larger than a binary format.
- **Peer handshake.** Closing does not confirm that the peer finished.

## Features

- `tls`: `tls::connect` and `tls::accept` over `tokio-rustls`. Adds `rustls` and
  `tokio-rustls`.

## Testing

- `cargo test -p tpt-pulse-backend-tokio` runs the TCP login test, the transport tests
  (in-memory pipe and generic streams), and the Unix socket test.
- `cargo test -p tpt-pulse-backend-tokio --features tls` also runs the TLS test.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
