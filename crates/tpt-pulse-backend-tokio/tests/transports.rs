//! Tests for the in-memory and `try_` transport paths.

use tpt_pulse_backend_tokio::{pair, AsyncChan};
use tpt_pulse_core::{Choose, Client, Close, Dual, Either, Recv, RecvTimeout, Send, SessionError};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Ping(u32);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Pong(u32);

type Exchange = Send<Ping, Recv<Pong>>;

#[tokio::test]
async fn in_memory_pair_runs_a_round_trip() {
    let (client, server) = pair::<Exchange>(1024);
    let peer = tokio::spawn(async move {
        let (Ping(n), server) = server.recv().await;
        server.send(Pong(n + 1)).await.close();
    });
    let (Pong(reply), client) = client.send(Ping(41)).await.recv().await;
    client.close();
    peer.await.unwrap();
    assert_eq!(reply, 42);
}

#[tokio::test]
async fn try_recv_reports_disconnect() {
    let (client, server) = pair::<Recv<Pong>>(1024);
    server.disconnect();
    assert_eq!(
        client.try_recv().await.err(),
        Some(SessionError::Disconnected)
    );
}

#[tokio::test]
async fn try_send_reports_disconnect() {
    let (client, server) = pair::<Exchange>(1024);
    server.disconnect();
    assert_eq!(
        client.try_send(Ping(1)).await.err(),
        Some(SessionError::Disconnected)
    );
}

const MESSAGE_TAG: u8 = 0;
const SELECT_LEFT_TAG: u8 = 1;

#[tokio::test]
async fn try_recv_rejects_a_selection_frame() {
    let (mut raw, stream) = tokio::io::duplex(1024);
    let client: AsyncChan<Recv<Pong>, Client, _> = AsyncChan::from_stream(stream);
    raw.write_u8(SELECT_LEFT_TAG).await.unwrap();
    raw.write_u32(0).await.unwrap();
    assert_eq!(
        client.try_recv().await.err(),
        Some(SessionError::TypeMismatch)
    );
}

#[tokio::test]
async fn try_recv_rejects_a_payload_that_does_not_decode() {
    let (mut raw, stream) = tokio::io::duplex(1024);
    let client: AsyncChan<Recv<Pong>, Client, _> = AsyncChan::from_stream(stream);
    raw.write_u8(MESSAGE_TAG).await.unwrap();
    raw.write_u32(3).await.unwrap();
    raw.write_all(b"{{{").await.unwrap();
    assert_eq!(
        client.try_recv().await.err(),
        Some(SessionError::TypeMismatch)
    );
}

#[tokio::test]
async fn try_offer_follows_the_selected_branch() {
    type Pick = Choose<Send<Ping, Close>, Send<Pong, Close>>;
    let (client, server) = pair::<Pick>(1024);
    let peer = tokio::spawn(async move {
        match server.try_offer().await {
            Ok(Either::Right(server)) => server.try_recv().await.map(|(Pong(n), _)| n),
            _ => panic!("expected the right branch"),
        }
    });
    client
        .try_choose_right()
        .await
        .unwrap()
        .try_send(Pong(7))
        .await
        .unwrap()
        .close();
    assert_eq!(peer.await.unwrap(), Ok(7));
}

#[test]
fn dual_protocols_have_matching_shapes() {
    fn assert_dual<P: Dual>() {}
    assert_dual::<Exchange>();
}

#[tokio::test]
async fn recv_timeout_returns_the_endpoint_and_retries() {
    let (client, server) = pair::<Recv<Pong>>(1024);
    let client = match client.recv_timeout(Duration::from_millis(10)).await {
        Err(RecvTimeout::Elapsed(client)) => client,
        _ => panic!("expected the receive to time out"),
    };
    // Nothing was consumed, so the retry receives the reply once it is sent.
    let peer = tokio::spawn(async move { server.send(Pong(5)).await.close() });
    let (Pong(n), client) = client
        .recv_timeout(Duration::from_secs(5))
        .await
        .ok()
        .expect("reply after retry");
    client.close();
    peer.await.unwrap();
    assert_eq!(n, 5);
}
