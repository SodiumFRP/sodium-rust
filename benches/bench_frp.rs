use criterion::{criterion_group, criterion_main, black_box, Criterion};

use sodium_rust::SodiumCtx;

fn stream(c: &mut Criterion) {
    c.bench_function("stream.send", |b| b.iter_with_large_drop(|| {
        let ctx = SodiumCtx::new();
        let sink = ctx.new_stream_sink();

        let mut values: Vec<u16> = Vec::new();
        let listener = sink.stream().listen(move |v: &u16| values.push(black_box(*v)));

        for v in 0_u16..1000 {
            sink.send(black_box(v));
        }
        (ctx, sink, listener)
    }));
}

fn cell(c: &mut Criterion) {
    c.bench_function("cell.send", |b| b.iter_with_large_drop(|| {
        let ctx = SodiumCtx::new();

        let sink = ctx.new_cell_sink(0_u16);
        let cell = sink.cell();

        for v in 0_u16..1000 {
            sink.send(black_box(v));
            let s = black_box(cell.sample());
            assert_eq!(v, s);
        }
    }));
}

criterion_group!(benches, stream, cell);
criterion_main!(benches);
