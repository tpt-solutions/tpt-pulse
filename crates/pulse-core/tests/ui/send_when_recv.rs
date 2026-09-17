use pulse_core::{session_channel, Recv};

fn main() {
    let (client, _server) = session_channel::<Recv<String>>();
    client.send(String::new());
}
