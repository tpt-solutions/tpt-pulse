//! TLS session channels, built on `tokio-rustls` and the `ring` crypto provider.
//!
//! These wrap the TCP functions with a handshake. The endpoint type is the same
//! [`AsyncChan`], with a TLS stream underneath, so the protocol code does not change.
//!
//! The caller supplies the rustls configuration. A client must trust the server's
//! certificate through its config, and a server must be configured with its own.

use std::{io, sync::Arc};

use tpt_pulse_core::{Client, Dual, Server};
use rustls::pki_types::ServerName;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_rustls::{client, server, TlsAcceptor, TlsConnector};

use crate::AsyncChan;

/// Connect as the client endpoint of protocol `P`, over TLS.
///
/// `domain` is the name the server's certificate must match.
pub async fn connect<P: Dual, A: ToSocketAddrs>(
    addr: A,
    domain: ServerName<'static>,
    config: Arc<rustls::ClientConfig>,
) -> io::Result<AsyncChan<P, Client, client::TlsStream<TcpStream>>> {
    let tcp = TcpStream::connect(addr).await?;
    tcp.set_nodelay(true)?;
    let stream = TlsConnector::from(config).connect(domain, tcp).await?;
    Ok(AsyncChan::from_stream(stream))
}

/// Accept a connection as the server endpoint of `P`'s dual, performing the TLS handshake.
pub async fn accept<P: Dual>(
    listener: &TcpListener,
    acceptor: &TlsAcceptor,
) -> io::Result<AsyncChan<P::Dual, Server, server::TlsStream<TcpStream>>> {
    let (tcp, _) = listener.accept().await?;
    tcp.set_nodelay(true)?;
    let stream = acceptor.accept(tcp).await?;
    Ok(AsyncChan::from_stream(stream))
}
