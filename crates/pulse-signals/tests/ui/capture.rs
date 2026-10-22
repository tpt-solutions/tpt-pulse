use pulse_signals::{signal_graph, Signal};

#[signal_graph]
fn captures() {
    let sensor = Signal::new(1.0);
    let doubled = Signal::map(&sensor, |v| v + sensor.get());
}

fn main() {}
