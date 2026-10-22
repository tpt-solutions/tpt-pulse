#![allow(missing_docs)]
//! Cost of one request/reply exchange on the synchronous transport.
//!
//! Both endpoints run on one thread. The queues are unbounded, so the client can
//! send before the server receives, and the benchmark measures the protocol and
//! transport cost rather than thread startup.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pulse_core::{session_channel, Recv, Send};

fn exchange(c: &mut Criterion) {
    c.bench_function("sync request/reply, one thread", |b| {
        b.iter(|| {
            let (client, server) = session_channel::<Send<u64, Recv<u64>>>();
            let client = client.send(black_box(1));
            let (value, server) = server.recv();
            server.send(value + 1).close();
            let (reply, client) = client.recv();
            client.close();
            black_box(reply)
        })
    });
}

criterion_group!(benches, exchange);
criterion_main!(benches);
