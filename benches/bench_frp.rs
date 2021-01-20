use criterion::{criterion_group, criterion_main, black_box, Criterion};

use sodium_rust::SodiumCtx;

fn stream(c: &mut Criterion) {
    c.bench_function("stream.send", |b| b.iter_with_large_drop(|| {
        let ctx = SodiumCtx::new();
        let sink = ctx.new_stream_sink();

        let mut values: Vec<u8> = Vec::new();
        let listener = sink.stream().listen(move |v: &u8| values.push(*v));

        for v in 0_u8..100 {
            sink.send(black_box(v));
        }
        (ctx, sink, listener)
    }));
}

criterion_group!(benches, sodium);
criterion_main!(benches);
