//! Async session channels over tokio sockets.
//!
//! [`AsyncChan`] uses the protocol types from `pulse-core` (`Send`, `Recv`,
//! `Choose`, `Either`, `Close`). Its operations are `async` and mirror the
//! synchronous API. Each message is serialized as JSON in a length-prefixed
//! frame, so message types must implement `serde`.
//!
//! Transports: TCP ([`connect`], [`accept`]), Unix domain sockets
//! (`connect_unix` and `accept_unix`, Unix only), and an in-memory pipe
//! ([`pair`]) for tests. Any `AsyncRead + AsyncWrite` stream works through
//! [`AsyncChan::from_stream`].
//!
//! Each operation has a panicking form (`send`, `recv`, `offer`, `choose_left`,
//! `choose_right`) and a `try_` form that returns [`SessionError`]. Both consume
//! the endpoint. A failed `try_` operation leaves the stream in an unknown state,
//! so drop the endpoint after an error.
//!
//! Like `pulse-core`, an unfinished endpoint panics when dropped. Duality is
//! enforced by the types only when both peers name the same protocol.
//!
//! The receive operations are not cancellation safe. A future dropped mid-frame
//! desynchronizes the stream, so do not wrap them in `tokio::time::timeout`.

use std::{io, marker::PhantomData, time::Duration};

use pulse_core::{
    Choose, Client, Close, Dual, Either, Protocol, Recv, RecvTimeout, Send, Server, SessionError,
};
use serde::{de::DeserializeOwned, Serialize};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

const MESSAGE: u8 = 0;
const SELECT_LEFT: u8 = 1;
const SELECT_RIGHT: u8 = 2;
const MAX_FRAME: usize = 16 * 1024 * 1024;

/// An async endpoint whose available operations depend on protocol `P`.
///
/// `S` is the transport stream. It defaults to [`TcpStream`].
#[must_use = "session endpoints must be driven to Close"]
pub struct AsyncChan<P: Protocol, Role, S = TcpStream> {
    stream: Option<S>,
    state: PhantomData<(P, Role)>,
}

impl<P: Protocol, R, S> AsyncChan<P, R, S> {
    /// Wrap a stream as an endpoint of protocol `P`.
    ///
    /// The caller must match the peer's endpoint to the dual protocol. Prefer
    /// [`connect`], [`accept`], or [`pair`], which do this.
    pub fn from_stream(stream: S) -> Self {
        Self {
            stream: Some(stream),
            state: PhantomData,
        }
    }

    fn take_stream(&mut self) -> S {
        self.stream.take().expect("live session endpoint")
    }
}

impl<P: Protocol, R, S> Drop for AsyncChan<P, R, S> {
    fn drop(&mut self) {
        if self.stream.is_some() && !P::COMPLETE && !std::thread::panicking() {
            panic!("session endpoint abandoned before Close");
        }
    }
}

/// Connect as the client endpoint of protocol `P` over TCP.
pub async fn connect<P: Dual, A: ToSocketAddrs>(addr: A) -> io::Result<AsyncChan<P, Client>> {
    let stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    Ok(AsyncChan::from_stream(stream))
}

/// Accept a TCP connection as the server endpoint, which follows the dual of `P`.
pub async fn accept<P: Dual>(listener: &TcpListener) -> io::Result<AsyncChan<P::Dual, Server>> {
    let (stream, _) = listener.accept().await?;
    stream.set_nodelay(true)?;
    Ok(AsyncChan::from_stream(stream))
}

/// Connect as the client endpoint of protocol `P` over a Unix domain socket.
#[cfg(unix)]
pub async fn connect_unix<P: Dual>(
    path: impl AsRef<std::path::Path>,
) -> io::Result<AsyncChan<P, Client, UnixStream>> {
    Ok(AsyncChan::from_stream(UnixStream::connect(path).await?))
}

/// Accept a Unix domain socket connection as the server endpoint of `P`'s dual.
#[cfg(unix)]
pub async fn accept_unix<P: Dual>(
    listener: &UnixListener,
) -> io::Result<AsyncChan<P::Dual, Server, UnixStream>> {
    let (stream, _) = listener.accept().await?;
    Ok(AsyncChan::from_stream(stream))
}

/// The client and server endpoints returned by [`pair`].
pub type InMemoryPair<P> = (
    AsyncChan<P, Client, DuplexStream>,
    AsyncChan<<P as Dual>::Dual, Server, DuplexStream>,
);

/// Connected in-memory endpoints with exactly dual protocols.
///
/// `buffer` is the capacity in bytes of each direction. A send waits when the
/// buffer is full, so run the two peers on separate tasks.
pub fn pair<P: Dual>(buffer: usize) -> InMemoryPair<P> {
    let (client, server) = tokio::io::duplex(buffer);
    (
        AsyncChan::from_stream(client),
        AsyncChan::from_stream(server),
    )
}

/// Encode one frame: a tag, a big-endian length, then the payload.
async fn write_frame<S: AsyncWrite + Unpin>(
    stream: &mut S,
    tag: u8,
    payload: &[u8],
) -> Result<(), SessionError> {
    if payload.len() > MAX_FRAME {
        return Err(SessionError::TypeMismatch);
    }
    let written = async {
        stream.write_u8(tag).await?;
        stream.write_u32(payload.len() as u32).await?;
        stream.write_all(payload).await
    };
    written.await.map_err(|_| SessionError::Disconnected)
}

async fn read_frame<S: AsyncRead + Unpin>(stream: &mut S) -> Result<(u8, Vec<u8>), SessionError> {
    let tag = stream
        .read_u8()
        .await
        .map_err(|_| SessionError::Disconnected)?;
    read_frame_body(stream, tag).await
}

/// Read the rest of a frame whose tag has already been read.
async fn read_frame_body<S: AsyncRead + Unpin>(
    stream: &mut S,
    tag: u8,
) -> Result<(u8, Vec<u8>), SessionError> {
    let len = stream
        .read_u32()
        .await
        .map_err(|_| SessionError::Disconnected)? as usize;
    if len > MAX_FRAME {
        return Err(SessionError::TypeMismatch);
    }
    let mut payload = vec![0; len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|_| SessionError::Disconnected)?;
    Ok((tag, payload))
}

impl<M, P, R, S> AsyncChan<Send<M, P>, R, S>
where
    M: Serialize + DeserializeOwned + std::marker::Send + 'static,
    P: Protocol,
    S: AsyncWrite + Unpin,
{
    /// Send a message and advance. Panics if the peer disconnected.
    pub async fn send(self, message: M) -> AsyncChan<P, R, S> {
        self.try_send(message)
            .await
            .expect("session peer disconnected during send")
    }

    /// Send a message and advance, reporting failure instead of panicking.
    ///
    /// A message that cannot be encoded, or that exceeds the frame limit, reports
    /// [`SessionError::TypeMismatch`].
    pub async fn try_send(mut self, message: M) -> Result<AsyncChan<P, R, S>, SessionError> {
        let mut stream = self.take_stream();
        let payload = serde_json::to_vec(&message).map_err(|_| SessionError::TypeMismatch)?;
        write_frame(&mut stream, MESSAGE, &payload).await?;
        Ok(AsyncChan::from_stream(stream))
    }
}

impl<M, P, R, S> AsyncChan<Recv<M, P>, R, S>
where
    M: Serialize + DeserializeOwned + std::marker::Send + 'static,
    P: Protocol,
    S: AsyncRead + Unpin,
{
    /// Wait for a message and advance. Panics if the peer disconnected.
    pub async fn recv(self) -> (M, AsyncChan<P, R, S>) {
        self.try_recv()
            .await
            .expect("session peer disconnected during recv")
    }

    /// Wait for a message and advance, reporting failure instead of panicking.
    ///
    /// A frame that does not decode as `M` reports [`SessionError::TypeMismatch`].
    pub async fn try_recv(mut self) -> Result<(M, AsyncChan<P, R, S>), SessionError> {
        let mut stream = self.take_stream();
        let (tag, payload) = read_frame(&mut stream).await?;
        let message = decode_message(tag, &payload)?;
        Ok((message, AsyncChan::from_stream(stream)))
    }

    /// Wait up to `timeout` for a message.
    ///
    /// Only the frame's first byte is awaited under the timeout. Reading one
    /// byte is cancel safe, so on [`RecvTimeout::Elapsed`] nothing was consumed
    /// and the endpoint is returned intact to retry. The rest of the frame is
    /// read without a timeout, so the framing cannot desynchronize.
    pub async fn recv_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<(M, AsyncChan<P, R, S>), RecvTimeout<Self>> {
        let mut stream = self.take_stream();
        let mut first = [0u8; 1];
        match tokio::time::timeout(timeout, stream.read(&mut first)).await {
            Err(_) => return Err(RecvTimeout::Elapsed(AsyncChan::from_stream(stream))),
            Ok(Ok(1)) => {}
            Ok(_) => return Err(RecvTimeout::Failed(SessionError::Disconnected)),
        }
        let (tag, payload) = match read_frame_body(&mut stream, first[0]).await {
            Ok(frame) => frame,
            Err(error) => return Err(RecvTimeout::Failed(error)),
        };
        match decode_message(tag, &payload) {
            Ok(message) => Ok((message, AsyncChan::from_stream(stream))),
            Err(error) => Err(RecvTimeout::Failed(error)),
        }
    }
}

fn decode_message<M: DeserializeOwned>(tag: u8, payload: &[u8]) -> Result<M, SessionError> {
    if tag != MESSAGE {
        return Err(SessionError::TypeMismatch);
    }
    serde_json::from_slice(payload).map_err(|_| SessionError::TypeMismatch)
}

impl<L: Protocol, R: Protocol, Role, S: AsyncWrite + Unpin> AsyncChan<Choose<L, R>, Role, S> {
    /// Select the left continuation. Panics if the peer disconnected.
    pub async fn choose_left(self) -> AsyncChan<L, Role, S> {
        self.try_choose_left()
            .await
            .expect("session peer disconnected during choice")
    }

    /// Select the right continuation. Panics if the peer disconnected.
    pub async fn choose_right(self) -> AsyncChan<R, Role, S> {
        self.try_choose_right()
            .await
            .expect("session peer disconnected during choice")
    }

    /// Select the left continuation, reporting failure instead of panicking.
    pub async fn try_choose_left(mut self) -> Result<AsyncChan<L, Role, S>, SessionError> {
        let mut stream = self.take_stream();
        write_frame(&mut stream, SELECT_LEFT, &[]).await?;
        Ok(AsyncChan::from_stream(stream))
    }

    /// Select the right continuation, reporting failure instead of panicking.
    pub async fn try_choose_right(mut self) -> Result<AsyncChan<R, Role, S>, SessionError> {
        let mut stream = self.take_stream();
        write_frame(&mut stream, SELECT_RIGHT, &[]).await?;
        Ok(AsyncChan::from_stream(stream))
    }
}

impl<L: Protocol, R: Protocol, Role, S: AsyncRead + Unpin> AsyncChan<Either<L, R>, Role, S> {
    /// Wait for the peer's selection and return its typed continuation.
    /// Panics if the peer disconnected.
    pub async fn offer(self) -> Either<AsyncChan<L, Role, S>, AsyncChan<R, Role, S>> {
        self.try_offer()
            .await
            .expect("session peer disconnected during offer")
    }

    /// Wait for the peer's selection, reporting failure instead of panicking.
    pub async fn try_offer(
        mut self,
    ) -> Result<Either<AsyncChan<L, Role, S>, AsyncChan<R, Role, S>>, SessionError> {
        let mut stream = self.take_stream();
        let (tag, _) = read_frame(&mut stream).await?;
        match tag {
            SELECT_LEFT => Ok(Either::Left(AsyncChan::from_stream(stream))),
            SELECT_RIGHT => Ok(Either::Right(AsyncChan::from_stream(stream))),
            _ => Err(SessionError::TypeMismatch),
        }
    }
}

impl<R, S> AsyncChan<Close, R, S> {
    /// Release the completed endpoint. This does not perform a peer handshake.
    pub fn close(self) {}
}

impl<P: Protocol, R, S> AsyncChan<P, R, S> {
    /// Abort the session ungracefully without panicking this side.
    ///
    /// The peer's next receive or send fails with a disconnection. Prefer driving
    /// the protocol to [`Close`] and calling [`AsyncChan::close`].
    pub fn disconnect(mut self) {
        self.stream.take();
    }
}
