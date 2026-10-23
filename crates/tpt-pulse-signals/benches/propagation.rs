#![allow(missing_docs)]
//! Cost of one signal propagation through a compiled graph.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tpt_pulse_signals::{signal_graph, Signal};

#[signal_graph]
fn propagate(input: f64) -> f64 {
    let sensor = Signal::new(input);
    let scaled = Signal::map(&sensor, |x| x * 0.5);
    let offset = Signal::map(&scaled, |x| x + 1.0);
    sensor.set(input + 1.0);
    offset.get()
}

fn propagation(c: &mut Criterion) {
    c.bench_function("signal graph propagation", |b| {
        b.iter(|| propagate(black_box(2.0)))
    });
}

criterion_group!(benches, propagation);
criterion_main!(benches);
