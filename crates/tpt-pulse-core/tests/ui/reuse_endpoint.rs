use tpt_pulse_core::{session_channel, Send};

fn main() {
    let (client, _server) = session_channel::<Send<u32>>();
    let _finished = client.send(1);
    client.send(2);
}
