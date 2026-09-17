//! Synchronous, in-memory session-typed channels.
//!
//! Operations consume an endpoint and return its protocol continuation.
//! Abandoning an unfinished endpoint panics, except during existing unwinding.
//! This runtime check is not linear typing: `mem::forget` can bypass destructors.
//! Duality enforces peer compatibility, not deadlock freedom across sessions.

use std::{any::Any, marker::PhantomData, sync::mpsc};

mod branch;
pub use branch::{Choose, Either};

mod sealed {
    pub trait Sealed {}
}

/// A closed set of supported session protocol states.
pub trait Protocol: sealed::Sealed {
    /// Whether this state permits dropping the endpoint normally.
    const COMPLETE: bool = false;
}

/// Computes the peer's protocol, with an involutive duality guarantee.
pub trait Dual: Protocol {
    /// Matching peer protocol; taking its dual returns this protocol.
    type Dual: Dual<Dual = Self>;
}

/// Send a message, then follow `P`.
pub struct Send<M, P = Close>(PhantomData<fn(M) -> P>);
/// Receive a message, then follow `P`.
pub struct Recv<M, P = Close>(PhantomData<fn(M) -> P>);
/// Completed protocol. Dropping or explicitly closing this endpoint is valid.
pub struct Close;
/// The first endpoint's role.
pub struct Client;
/// The dual endpoint's role.
pub struct Server;

impl sealed::Sealed for Close {}
impl Protocol for Close {
    const COMPLETE: bool = true;
}
impl Dual for Close {
    type Dual = Close;
}
impl<M: std::marker::Send + 'static, P: Protocol> sealed::Sealed for Send<M, P> {}
impl<M: std::marker::Send + 'static, P: Protocol> sealed::Sealed for Recv<M, P> {}
impl<M: std::marker::Send + 'static, P: Protocol> Protocol for Send<M, P> {}
impl<M: std::marker::Send + 'static, P: Protocol> Protocol for Recv<M, P> {}
impl<M: std::marker::Send + 'static, P: Dual> Dual for Send<M, P> {
    type Dual = Recv<M, P::Dual>;
}
impl<M: std::marker::Send + 'static, P: Dual> Dual for Recv<M, P> {
    type Dual = Send<M, P::Dual>;
}

type Message = Box<dyn Any + std::marker::Send>;

struct Transport {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
}

/// An endpoint whose available operations depend on protocol `P`.
///
/// Messages must be owned (`Send + 'static`). Transport disconnection panics.
/// An unfinished endpoint panics on normal drop; during panic unwinding its
/// transport is released without a second panic. Endpoints cannot be cloned.
#[must_use = "session endpoints must be driven to Close"]
pub struct Chan<P: Protocol, Role> {
    transport: Option<Transport>,
    state: PhantomData<(P, Role)>,
}

impl<P: Protocol, R> Chan<P, R> {
    fn new(transport: Transport) -> Self {
        Self {
            transport: Some(transport),
            state: PhantomData,
        }
    }

    fn take_transport(&mut self) -> Transport {
        self.transport.take().expect("live session endpoint")
    }
}

impl<P: Protocol, R> Drop for Chan<P, R> {
    fn drop(&mut self) {
        if self.transport.is_some() && !P::COMPLETE && !std::thread::panicking() {
            panic!("session endpoint abandoned before Close");
        }
    }
}

impl<M: std::marker::Send + 'static, P: Protocol, R> Chan<Send<M, P>, R> {
    /// Send a message and advance. Panics if the peer disconnected.
    pub fn send(mut self, msg: M) -> Chan<P, R> {
        let transport = self.take_transport();
        assert!(
            transport.tx.send(Box::new(msg)).is_ok(),
            "session peer disconnected during send"
        );
        Chan::new(transport)
    }
}

impl<M: std::marker::Send + 'static, P: Protocol, R> Chan<Recv<M, P>, R> {
    /// Wait for a message and advance. Panics if the peer disconnected.
    pub fn recv(mut self) -> (M, Chan<P, R>) {
        let transport = self.take_transport();
        let message = transport
            .rx
            .recv()
            .expect("session peer disconnected during recv");
        let message = *message
            .downcast::<M>()
            .unwrap_or_else(|_| panic!("internal session message type mismatch"));
        (message, Chan::new(transport))
    }
}

impl<R> Chan<Close, R> {
    /// Release the completed endpoint. This does not perform a peer handshake.
    pub fn close(self) {}
}

impl<P: Protocol, R> Chan<P, R> {
    /// Abort the session ungracefully without panicking this side.
    ///
    /// The peer can receive already queued messages; subsequent receives or
    /// sends panic on disconnection. A peer already at [`Close`] can finish
    /// normally. Prefer driving the protocol to [`Close`] and calling
    /// [`Chan::close`].
    pub fn disconnect(mut self) {
        self.transport.take();
    }
}

/// Construct connected endpoints with exactly dual protocols.
///
/// Uses two unbounded standard-library queues; receives block until a message
/// arrives or the peer disconnects. Run communicating peers on separate threads
/// when either needs to wait. Messages are boxed, so this transport allocates.
pub fn session_channel<P: Dual>() -> (Chan<P, Client>, Chan<P::Dual, Server>) {
    let (client_tx, server_rx) = mpsc::channel();
    let (server_tx, client_rx) = mpsc::channel();
    (
        Chan::new(Transport {
            tx: client_tx,
            rx: client_rx,
        }),
        Chan::new(Transport {
            tx: server_tx,
            rx: server_rx,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        type Ping = Send<String, Recv<usize>>;
        let (client, server) = session_channel::<Ping>();
        let peer = std::thread::spawn(move || {
            let (message, server) = server.recv();
            server.send(message.len()).close();
        });
        let (length, client) = client.send("hello".into()).recv();
        assert_eq!(length, 5);
        client.close();
        peer.join().unwrap();
    }
}
