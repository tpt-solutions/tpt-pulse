use pulse_core::{session_channel, Send};

fn main() {
    let (client, _server) = session_channel::<Send<u32>>();
    client.close();
}
