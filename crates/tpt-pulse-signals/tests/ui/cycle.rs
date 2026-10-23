use tpt_pulse_signals::{signal_graph, Signal};

#[signal_graph]
fn feedback() {
    let sensor = Signal::map(&combined, |v| v);
    let combined = Signal::map(&sensor, |v| v + 1.0);
}

fn main() {}
