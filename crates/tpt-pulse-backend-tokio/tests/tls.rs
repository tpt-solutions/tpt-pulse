//! A session over TLS with a self-signed certificate.

#![cfg(feature = "tls")]

use std::sync::Arc;

use rustls::pki_types::{PrivatePkcs8KeyDer, ServerName};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tpt_pulse_backend_tokio::tls::{accept, connect};
use tpt_pulse_core::{Close, Dual, Either, Recv, Send};

#[derive(Serialize, Deserialize)]
struct Request(String);

#[derive(Serialize, Deserialize)]
struct Reply(String);

// The client sends a request, and the server chooses how to reply.
type Login = Send<Request, Either<Recv<Reply, Close>, Recv<Reply, Close>>>;

/// A self-signed certificate for `localhost`, and the server and client configs that use it.
fn tls_configs() -> (Arc<rustls::ServerConfig>, Arc<rustls::ClientConfig>) {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".into()])
        .expect("generate a certificate");
    let cert = certified.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der());

    let server = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], key.into())
        .expect("server config");

    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert).expect("trust the certificate");
    let client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    (Arc::new(server), Arc::new(client))
}

#[tokio::test]
async fn a_login_runs_over_tls() {
    fn assert_dual<P: Dual>() {}
    assert_dual::<Login>();

    let (server_config, client_config) = tls_configs();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("address");
    let acceptor = TlsAcceptor::from(server_config);

    let peer = tokio::spawn(async move {
        let server = accept::<Login>(&listener, &acceptor)
            .await
            .expect("accept over TLS");
        let (Request(user), server) = server.recv().await;
        if user == "admin" {
            server
                .choose_left()
                .await
                .send(Reply("welcome".into()))
                .await
                .close();
        } else {
            server
                .choose_right()
                .await
                .send(Reply("unknown user".into()))
                .await
                .close();
        }
    });

    let domain = ServerName::try_from("localhost").expect("valid name");
    let client = connect::<Login, _>(addr, domain, client_config)
        .await
        .expect("connect over TLS");
    let reply = match client.send(Request("admin".into())).await.offer().await {
        Either::Left(ok) => ok.recv().await,
        Either::Right(_) => panic!("expected the left branch"),
    };
    let (Reply(text), client) = reply;
    client.close();
    peer.await.expect("server task");
    assert_eq!(text, "welcome");
}

#[tokio::test]
async fn a_client_that_does_not_trust_the_server_fails_the_handshake() {
    let (server_config, _) = tls_configs();
    let (_, untrusting) = tls_configs();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("address");
    let acceptor = TlsAcceptor::from(server_config);

    // The server side fails its handshake too, so its task just reports the error.
    let peer = tokio::spawn(async move { accept::<Login>(&listener, &acceptor).await.is_err() });

    let domain = ServerName::try_from("localhost").expect("valid name");
    let result = connect::<Login, _>(addr, domain, untrusting).await;
    assert!(
        result.is_err(),
        "a certificate from another authority must be rejected"
    );
    assert!(peer.await.expect("server task"));
}
