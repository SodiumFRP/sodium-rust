use criterion::{black_box, criterion_group, criterion_main, Criterion};

use sodium_rust::SodiumCtx;

fn stream(c: &mut Criterion) {
    let mut stream_send = c.benchmark_group("Stream::send");
    stream_send.bench_function("simple", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+map", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .map(|v: &u16| *v + 10)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+mapx2", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .map(|v: &u16| black_box(*v + 10))
                .map(|v: &u16| black_box(*v - 5))
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+mapx3", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .map(|v: &u16| black_box(*v + 10))
                .map(|v: &u16| black_box(*v - 5))
                .map(|v: &u16| black_box(*v * 2))
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+filter", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .filter(|v: &u16| *v % 3 == 0 && *v % 5 == 0)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+map+filter", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .map(|v: &u16| *v + 1)
                .filter(|v: &u16| *v % 3 == 0 && *v % 5 == 0)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+map+filter+map", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .map(|v: &u16| *v + 1)
                .filter(|v: &u16| *v % 3 == 0 && *v % 5 == 0)
                .map(|v: &u16| *v - 4)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });

    stream_send.bench_function("+map+filter+map+filter", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sink = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sink
                .stream()
                .map(|v: &u16| *v + 1)
                .filter(|v: &u16| *v % 3 == 0)
                .map(|v: &u16| *v - 4)
                .filter(|v: &u16| *v % 5 == 0)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sink.send(black_box(v));
            }
        })
    });
}

fn cell(c: &mut Criterion) {
    let mut cell_send = c.benchmark_group("Cell::send");
    cell_send.bench_function("simple", |b| {
        b.iter(|| {
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
        b.iter(|| {
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

criterion_group!(benches, cell, stream);
criterion_main!(benches);
