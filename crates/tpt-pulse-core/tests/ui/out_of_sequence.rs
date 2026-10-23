use tpt_pulse_core::{session_channel, Recv, Send};

fn main() {
    let (client, _server) = session_channel::<Send<String, Recv<u32>>>();
    client.recv();
}
