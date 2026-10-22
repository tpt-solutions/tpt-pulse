//! Async session channels over tokio TCP sockets.
//!
//! [`AsyncChan`] uses the protocol types from `pulse-core` (`Send`, `Recv`,
//! `Choose`, `Either`, `Close`). Its operations are `async` and mirror the
//! synchronous API. Each message is serialized as JSON in a length-prefixed
//! frame, so message types must implement `serde`.
//!
//! Like `pulse-core`, an unfinished endpoint panics when dropped, and transport
//! failures panic. Duality is enforced by the types only when both peers name
//! the same protocol, so both sides must use the same `P`.

use std::{io, marker::PhantomData};

use pulse_core::{Choose, Client, Close, Dual, Either, Protocol, Recv, Send, Server};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

const MESSAGE: u8 = 0;
const SELECT_LEFT: u8 = 1;
const SELECT_RIGHT: u8 = 2;
const MAX_FRAME: usize = 16 * 1024 * 1024;

/// An async endpoint whose available operations depend on protocol `P`.
#[must_use = "session endpoints must be driven to Close"]
pub struct AsyncChan<P: Protocol, Role> {
    stream: Option<TcpStream>,
    state: PhantomData<(P, Role)>,
}

impl<P: Protocol, R> AsyncChan<P, R> {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream: Some(stream),
            state: PhantomData,
        }
    }

    fn take_stream(&mut self) -> TcpStream {
        self.stream.take().expect("live session endpoint")
    }
}

impl<P: Protocol, R> Drop for AsyncChan<P, R> {
    fn drop(&mut self) {
        if self.stream.is_some() && !P::COMPLETE && !std::thread::panicking() {
            panic!("session endpoint abandoned before Close");
        }
    }
}

/// Connect as the client endpoint of protocol `P`.
pub async fn connect<P: Dual, A: ToSocketAddrs>(addr: A) -> io::Result<AsyncChan<P, Client>> {
    let stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    Ok(AsyncChan::new(stream))
}

/// Accept a connection as the server endpoint, which follows the dual of `P`.
pub async fn accept<P: Dual>(listener: &TcpListener) -> io::Result<AsyncChan<P::Dual, Server>> {
    let (stream, _) = listener.accept().await?;
    stream.set_nodelay(true)?;
    Ok(AsyncChan::new(stream))
}

async fn write_frame(stream: &mut TcpStream, tag: u8, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "message too large",
        ));
    }
    stream.write_u8(tag).await?;
    stream.write_u32(payload.len() as u32).await?;
    stream.write_all(payload).await
}

async fn read_frame(stream: &mut TcpStream) -> io::Result<(u8, Vec<u8>)> {
    let tag = stream.read_u8().await?;
    let len = stream.read_u32().await? as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut payload = vec![0; len];
    stream.read_exact(&mut payload).await?;
    Ok((tag, payload))
}

impl<M, P, R> AsyncChan<Send<M, P>, R>
where
    M: Serialize + DeserializeOwned + std::marker::Send + 'static,
    P: Protocol,
{
    /// Send a message and advance. Panics if the peer disconnected.
    pub async fn send(mut self, message: M) -> AsyncChan<P, R> {
        let mut stream = self.take_stream();
        let payload = serde_json::to_vec(&message).expect("message serializes to JSON");
        write_frame(&mut stream, MESSAGE, &payload)
            .await
            .expect("session peer disconnected during send");
        AsyncChan::new(stream)
    }
}

impl<M, P, R> AsyncChan<Recv<M, P>, R>
where
    M: Serialize + DeserializeOwned + std::marker::Send + 'static,
    P: Protocol,
{
    /// Wait for a message and advance. Panics if the peer disconnected.
    pub async fn recv(mut self) -> (M, AsyncChan<P, R>) {
        let mut stream = self.take_stream();
        let (tag, payload) = read_frame(&mut stream)
            .await
            .expect("session peer disconnected during recv");
        assert_eq!(tag, MESSAGE, "internal session message type mismatch");
        let message = serde_json::from_slice(&payload)
            .unwrap_or_else(|_| panic!("internal session message type mismatch"));
        (message, AsyncChan::new(stream))
    }
}

impl<L: Protocol, R: Protocol, Role> AsyncChan<Choose<L, R>, Role> {
    /// Select the left continuation. Panics if the peer disconnected.
    pub async fn choose_left(mut self) -> AsyncChan<L, Role> {
        let mut stream = self.take_stream();
        write_frame(&mut stream, SELECT_LEFT, &[])
            .await
            .expect("session peer disconnected during choice");
        AsyncChan::new(stream)
    }

    /// Select the right continuation. Panics if the peer disconnected.
    pub async fn choose_right(mut self) -> AsyncChan<R, Role> {
        let mut stream = self.take_stream();
        write_frame(&mut stream, SELECT_RIGHT, &[])
            .await
            .expect("session peer disconnected during choice");
        AsyncChan::new(stream)
    }
}

impl<L: Protocol, R: Protocol, Role> AsyncChan<Either<L, R>, Role> {
    /// Wait for the peer's selection and return its typed continuation.
    /// Panics if the peer disconnected.
    pub async fn offer(mut self) -> Either<AsyncChan<L, Role>, AsyncChan<R, Role>> {
        let mut stream = self.take_stream();
        let (tag, _) = read_frame(&mut stream)
            .await
            .expect("session peer disconnected during offer");
        match tag {
            SELECT_LEFT => Either::Left(AsyncChan::new(stream)),
            SELECT_RIGHT => Either::Right(AsyncChan::new(stream)),
            _ => panic!("internal session branch mismatch"),
        }
    }
}

impl<R> AsyncChan<Close, R> {
    /// Release the completed endpoint. This does not perform a peer handshake.
    pub fn close(self) {}
}
