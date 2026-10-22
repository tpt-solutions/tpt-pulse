//! Round trip over a Unix domain socket. Runs only on Unix targets.

#![cfg(unix)]

use pulse_backend_tokio::{accept_unix, connect_unix};
use pulse_core::{Dual, Recv, Send};
use serde::{Deserialize, Serialize};
use tokio::net::UnixListener;

#[derive(Serialize, Deserialize)]
struct Ping(u32);

#[derive(Serialize, Deserialize)]
struct Pong(u32);

type Exchange = Send<Ping, Recv<Pong>>;

#[tokio::test]
async fn unix_socket_round_trip() {
    fn assert_dual<P: Dual>() {}
    assert_dual::<Exchange>();

    let path = std::env::temp_dir().join(format!("pulse-unix-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind");
    let peer = tokio::spawn(async move {
        let server = accept_unix::<Exchange>(&listener).await.expect("accept");
        let (Ping(n), server) = server.recv().await;
        server.send(Pong(n + 1)).await.close();
    });

    let client = connect_unix::<Exchange>(&path).await.expect("connect");
    let (Pong(reply), client) = client.send(Ping(41)).await.recv().await;
    client.close();
    peer.await.expect("server task");
    let _ = std::fs::remove_file(&path);
    assert_eq!(reply, 42);
}
