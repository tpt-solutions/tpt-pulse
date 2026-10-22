//! A starter session. Edit the protocol, then run `cargo run` and `cargo test`.

use tpt_pulse::prelude::*;

// The message types. The protocol refers to them by name.
struct Ask;
struct Answer;

// `session_types!` gives each role its own pulse-core type. The client's
// endpoint is `Chan<Query::Client, Client>`, and the server's is the dual.
session_types! {
    Query {
        Client -> Server : Ask;
        Server -> Client : Answer;
    }
}

fn main() {
    let (client, server) = session_channel::<Query::Client>();
    let peer = std::thread::spawn(move || {
        let (Ask, server) = server.recv();
        server.send(Answer).close();
    });
    let (Answer, client) = client.send(Ask).recv();
    client.close();
    peer.join().expect("server thread");
    println!("the server answered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_client_follows_the_protocol() {
        let protocol = global_protocol! {
            Client -> Server : Ask;
            Server -> Client : Answer;
        };
        assert_protocol!(
            protocol,
            "Client",
            [
                Action::send("Server", "Ask"),
                Action::recv("Server", "Answer"),
            ]
        );
    }
}
