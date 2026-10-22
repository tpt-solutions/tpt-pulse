//! Per-role protocol types from `session_types!` are exact duals, so the
//! endpoints built from them run end to end.

use tpt_pulse::prelude::*;

#[derive(Debug, PartialEq)]
struct Request(u32);

#[derive(Debug, PartialEq)]
struct Reply(u32);

session_types! {
    Ping {
        Client -> Server : Request;
        Server -> Client : Reply;
    }
}

/// Typed so the compiler checks that the server endpoint is the dual of the client's.
fn serve(chan: Chan<Ping::Server, Server>) {
    let (Request(n), chan) = chan.recv();
    chan.send(Reply(n + 1)).close();
}

#[test]
fn per_role_types_are_dual_and_run() {
    let (client, server) = session_channel::<Ping::Client>();
    let peer = std::thread::spawn(move || serve(server));
    let (reply, client) = client.send(Request(41)).recv();
    client.close();
    peer.join().expect("server thread");
    assert_eq!(reply, Reply(42));
}
