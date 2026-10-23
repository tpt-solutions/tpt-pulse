//! Synchronous, in-memory session-typed channels.
//!
//! Operations consume an endpoint and return its protocol continuation.
//! Abandoning an unfinished endpoint panics, except during existing unwinding.
//! This runtime check is not linear typing: `mem::forget` can bypass destructors.
//! Duality enforces peer compatibility, not deadlock freedom across sessions.

use std::{any::Any, fmt, marker::PhantomData, sync::mpsc, time::Duration};

mod branch;
pub use branch::{Choose, Either};

mod recursion;
pub use recursion::{Rec, Subst, Var};

mod sealed {
    pub trait Sealed {}
}

/// A closed set of supported session protocol states.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a session protocol",
    label = "use `Send`, `Recv`, `Either`, `Choose`, or `Close` here",
    note = "protocols are built only from the types in `tpt_pulse_core`"
)]
pub trait Protocol: sealed::Sealed {
    /// Whether this state permits dropping the endpoint normally.
    const COMPLETE: bool = false;
}

/// Computes the peer's protocol, with an involutive duality guarantee.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no dual protocol",
    label = "every branch and message here needs a matching peer step",
    note = "each `Send` pairs with a `Recv` of the same message type, and each `Either` with a `Choose`"
)]
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

/// Why a non-panicking session operation failed.
///
/// A failed operation consumes its endpoint, so the session cannot be resumed.
/// The exception is [`RecvTimeout::Elapsed`], which returns the endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The peer dropped its endpoint, or the transport closed.
    Disconnected,
    /// The peer sent a message or selection that the protocol does not expect.
    TypeMismatch,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "session peer disconnected"),
            Self::TypeMismatch => write!(f, "session peer sent an unexpected message"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Why [`Chan::recv_timeout`] did not produce a message.
pub enum RecvTimeout<C> {
    /// No message arrived in time. The endpoint is returned, still waiting, so
    /// the receive can be retried.
    Elapsed(C),
    /// The session failed. The endpoint is consumed.
    Failed(SessionError),
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

impl<M: std::marker::Send + 'static, P: Protocol, R> Chan<Send<M, P>, R> {
    /// Send a message and advance, reporting failure instead of panicking.
    ///
    /// On error the endpoint is consumed, and the message is dropped.
    pub fn try_send(mut self, msg: M) -> Result<Chan<P, R>, SessionError> {
        let transport = self.take_transport();
        transport
            .tx
            .send(Box::new(msg))
            .map_err(|_| SessionError::Disconnected)?;
        Ok(Chan::new(transport))
    }
}

impl<M: std::marker::Send + 'static, P: Protocol, R> Chan<Recv<M, P>, R> {
    /// Wait for a message and advance, reporting failure instead of panicking.
    ///
    /// On error the endpoint is consumed. Use [`Chan::recv_timeout`] to retry.
    pub fn try_recv(mut self) -> Result<(M, Chan<P, R>), SessionError> {
        let transport = self.take_transport();
        let message = transport
            .rx
            .recv()
            .map_err(|_| SessionError::Disconnected)?;
        let message = message
            .downcast::<M>()
            .map_err(|_| SessionError::TypeMismatch)?;
        Ok((*message, Chan::new(transport)))
    }

    /// Wait up to `timeout` for a message.
    ///
    /// On [`RecvTimeout::Elapsed`] the endpoint is returned still waiting, so the
    /// caller can retry or give up with [`Chan::disconnect`].
    pub fn recv_timeout(mut self, timeout: Duration) -> Result<(M, Chan<P, R>), RecvTimeout<Self>> {
        let transport = self.take_transport();
        match transport.rx.recv_timeout(timeout) {
            Ok(message) => match message.downcast::<M>() {
                Ok(message) => Ok((*message, Chan::new(transport))),
                Err(_) => Err(RecvTimeout::Failed(SessionError::TypeMismatch)),
            },
            Err(mpsc::RecvTimeoutError::Timeout) => Err(RecvTimeout::Elapsed(Chan::new(transport))),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(RecvTimeout::Failed(SessionError::Disconnected))
            }
        }
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

    #[test]
    fn try_send_reports_disconnect() {
        type Ping = Send<String, Recv<usize>>;
        let (client, server) = session_channel::<Ping>();
        server.disconnect();
        assert_eq!(
            client.try_send("hello".into()).err(),
            Some(SessionError::Disconnected)
        );
    }

    #[test]
    fn try_recv_delivers_a_message_queued_before_the_peer_left() {
        let (client, server) = session_channel::<Recv<usize>>();
        server.send(3).close();
        let (value, client) = client.try_recv().expect("queued message");
        assert_eq!(value, 3);
        client.close();
    }

    #[test]
    fn try_recv_reports_peer_gone() {
        let (client, server) = session_channel::<Recv<usize>>();
        server.disconnect();
        assert_eq!(client.try_recv().err(), Some(SessionError::Disconnected));
    }

    #[test]
    fn recv_timeout_returns_the_endpoint_when_nothing_arrives() {
        let (client, server) = session_channel::<Recv<usize>>();
        let client = match client.recv_timeout(Duration::from_millis(10)) {
            Err(RecvTimeout::Elapsed(client)) => client,
            _ => panic!("expected the receive to time out"),
        };
        server.send(9).close();
        let (value, client) = client
            .recv_timeout(Duration::from_secs(5))
            .ok()
            .expect("message after retry");
        assert_eq!(value, 9);
        client.close();
    }

    #[test]
    fn try_choose_and_try_offer_follow_the_selected_branch() {
        type Pick = Choose<Send<usize, Close>, Send<String, Close>>;
        let (client, server) = session_channel::<Pick>();
        let peer = std::thread::spawn(move || match server.try_offer() {
            Ok(Either::Right(server)) => server.try_recv().map(|(text, server)| {
                server.close();
                text
            }),
            _ => panic!("expected the right branch"),
        });
        client
            .try_choose_right()
            .unwrap()
            .try_send("hi".into())
            .unwrap()
            .close();
        assert_eq!(peer.join().unwrap(), Ok("hi".into()));
    }

    #[test]
    fn try_choose_reports_disconnect() {
        let (client, server) = session_channel::<Choose<Close, Close>>();
        server.disconnect();
        assert!(matches!(
            client.try_choose_left(),
            Err(SessionError::Disconnected)
        ));
    }
}
