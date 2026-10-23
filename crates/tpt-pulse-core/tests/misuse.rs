//! Safety-net behavior: abandonment panics, disconnection panics, clean exits.
//!
//! Also covers the `disconnect` escape hatch, which lets one side abort
//! mid-protocol without panicking; the peer's next operation fails instead.

use tpt_pulse_core::{session_channel, Recv, Send};
use std::panic;

fn panic_message(payload: &(dyn std::any::Any + std::marker::Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&'static str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown panic payload".into()
    }
}

#[test]
fn abandoning_mid_protocol_panics() {
    let result = panic::catch_unwind(|| {
        let (_client, _server) = session_channel::<Send<String>>();
        // Both endpoints drop unfinished; the first drop panics, the second
        // sees the active unwinding and releases the transport silently.
    });
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(panic_message(err.as_ref()).contains("abandoned before Close"));
}

#[test]
fn disconnect_escapes_the_abandonment_panic() {
    let (client, server) = session_channel::<Recv<String>>();
    client.disconnect();
    server.disconnect(); // Also non-Close, but no panic: already disconnected.
}

#[test]
fn dropping_completed_endpoints_is_allowed() {
    let (client, server) = session_channel::<Send<String>>();
    let peer = std::thread::spawn(move || {
        let (_, server) = server.recv();
        drop(server);
    });
    let client = client.send("hello".into());
    drop(client);
    peer.join().unwrap();
}

#[test]
fn recv_after_peer_disconnect_panics() {
    let (client, server) = session_channel::<Recv<String>>();
    server.disconnect();
    let result = panic::catch_unwind(move || {
        let _ = client.recv();
    });
    let err = result.unwrap_err();
    assert!(panic_message(err.as_ref()).contains("disconnected during recv"));
}

#[test]
fn send_after_peer_disconnect_panics() {
    let (client, server) = session_channel::<Send<String>>();
    server.disconnect();
    let result = panic::catch_unwind(move || {
        let _ = client.send("hello".into());
    });
    let err = result.unwrap_err();
    assert!(panic_message(err.as_ref()).contains("disconnected during send"));
}
