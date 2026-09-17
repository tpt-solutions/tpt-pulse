use pulse_core::{session_channel, Recv};

fn main() {
    let (client, _server) = session_channel::<Recv<u32>>();
    let (message, _client) = client.recv();
    let _: String = message;
}
