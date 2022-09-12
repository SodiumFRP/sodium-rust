use criterion::{black_box, criterion_group, criterion_main, Criterion};

use sodium_rust::SodiumCtx;

fn stream(c: &mut Criterion) {
    let mut stream_send = c.benchmark_group("Stream::send");
    stream_send.bench_function("simple", |b| {
        b.iter_with_large_drop(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let listener = sink
                .stream()
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
            (ctx, sink, listener)
        })
    });

    stream_send.bench_function("+map", |b| {
        b.iter_with_large_drop(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let listener = sink
                .stream()
                .map(|v: &u16| *v + 10)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
            (ctx, sink, listener)
        })
    });
}

fn cell(c: &mut Criterion) {
    let mut cell_send = c.benchmark_group("Cell::send");
    cell_send.bench_function("simple", |b| {
        b.iter_with_large_drop(|| {
            let ctx = SodiumCtx::new();

            let sink = ctx.new_cell_sink(0_u16);
            let cell = sink.cell();

            for v in 0_u16..1000 {
                sink.send(black_box(v));
                let s = black_box(cell.sample());
                assert_eq!(v, s);
            }
        })
    });

    cell_send.bench_function("+map", |b| {
        b.iter_with_large_drop(|| {
            let ctx = SodiumCtx::new();

            let sink = ctx.new_cell_sink(0_u16);
            let cell = sink.cell().map(|v: &u16| *v + 10);

            for v in 0_u16..1000 {
                sink.send(black_box(v));
                let s = black_box(cell.sample());
                assert_eq!(v + 10, s);
            }
        })
    });
}

criterion_group!(benches, stream, cell);
criterion_main!(benches);
