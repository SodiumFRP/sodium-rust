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
    stream_send.bench_function("merge2", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sa = ctx.new_stream_sink();
            let sb = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sa
                .stream()
                .merge(&sb.stream(), |a: &u16, b: &u16| *a + *b)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                if v % 2 == 0 {
                    sa.send(black_box(v));
                } else {
                    sb.send(black_box(v));
                }
            }
        })
    });
    stream_send.bench_function("merge3", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sa = ctx.new_stream_sink();
            let sb = ctx.new_stream_sink();
            let sc = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sa
                .stream()
                .merge(&sb.stream(), |a: &u16, b: &u16| *a + *b)
                .merge(&sc.stream(), |s: &u16, c: &u16| *s + *c)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                match v % 3 {
                    2 => sa.send(black_box(v)),
                    1 => sb.send(black_box(v)),
                    _ => sc.send(black_box(v)),
                }
            }
        })
    });
    stream_send.bench_function("merge4", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();
            let sa = ctx.new_stream_sink();
            let sb = ctx.new_stream_sink();
            let sc = ctx.new_stream_sink();
            let sd = ctx.new_stream_sink();

            let mut values: Vec<u16> = Vec::new();
            let _listener = sa
                .stream()
                .merge(&sb.stream(), |a: &u16, b: &u16| *a + *b)
                .merge(&sc.stream(), |s: &u16, c: &u16| *s + *c)
                .merge(&sd.stream(), |s: &u16, d: &u16| *s + *d)
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                match v % 4 {
                    3 => sa.send(black_box(v)),
                    2 => sb.send(black_box(v)),
                    1 => sc.send(black_box(v)),
                    _ => sd.send(black_box(v)),
                }
            }
        })
    });
    stream_send.bench_function("stream_loop unused", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let sa = ctx.new_stream_sink();
            let _sb = ctx.transaction(|| {
                let sb = ctx.new_stream_loop();
                sb.loop_(&sa.stream());
                sb
            });

            let mut values: Vec<u16> = Vec::new();
            let _listener = sa
                .stream()
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sa.send(black_box(v));
            }
        })
    });
    stream_send.bench_function("stream_loop used", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let sa = ctx.new_stream_sink();
            let sb = ctx.transaction(|| {
                let sb = ctx.new_stream_loop();
                sb.loop_(&sa.stream());
                sb
            });

            let mut values: Vec<u16> = Vec::new();
            let _listener = sb
                .stream()
                .listen(move |v: &u16| values.push(black_box(*v)));

            for v in 0_u16..1000 {
                sa.send(black_box(v));
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
    cell_send.bench_function("+map2", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let sink = ctx.new_cell_sink(0_u16);
            let cell = sink.cell().map(|v: &u16| *v + 10).map(|v: &u16| *v - 5);

            for v in 0_u16..1000 {
                sink.send(black_box(v));
                let s = black_box(cell.sample());
                assert_eq!(v + 5, s);
            }
        })
    });
    cell_send.bench_function("+map3", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let sink = ctx.new_cell_sink(0_u16);
            let cell = sink
                .cell()
                .map(|v: &u16| *v + 10)
                .map(|v: &u16| *v - 5)
                .map(|v: &u16| *v + 2);

            for v in 0_u16..1000 {
                sink.send(black_box(v));
                let s = black_box(cell.sample());
                assert_eq!(v + 7, s);
            }
        })
    });
}

fn snapshot(c: &mut Criterion) {
    let mut snapshot = c.benchmark_group("snapshot");
    snapshot.bench_function("one", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let stream_sink = ctx.new_stream_sink();
            let cell_sink = ctx.new_cell_sink(0_u16);
            let sum_stream = stream_sink
                .stream()
                .snapshot(&cell_sink.cell(), |v: &u16, add: &u16| *v + *add);

            let mut values: Vec<u16> = Vec::new();
            let _listener = sum_stream.listen(move |v: &u16| values.push(black_box(*v)));

            for add in 1_u16..50 {
                for v in 0_u16..100 {
                    stream_sink.send(v);
                }
                cell_sink.send(add);
            }
        });
    });
    snapshot.bench_function("two", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let stream_sink = ctx.new_stream_sink();
            let ca = ctx.new_cell_sink(0_u16);
            let cb = ctx.new_cell_sink(10_u16);
            let sum_stream = stream_sink.stream().snapshot3(
                &ca.cell(),
                &cb.cell(),
                |v: &u16, a: &u16, b: &u16| *v + *a + *b,
            );

            let mut values: Vec<u16> = Vec::new();
            let _listener = sum_stream.listen(move |v: &u16| values.push(black_box(*v)));

            for add in 1_u16..50 {
                for v in 0_u16..100 {
                    stream_sink.send(v);
                }
                if add % 2 == 0 {
                    ca.send(add);
                } else {
                    cb.send(add + 10);
                }
            }
        });
    });
    snapshot.bench_function("three", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let stream_sink = ctx.new_stream_sink();
            let ca = ctx.new_cell_sink(0_u16);
            let cb = ctx.new_cell_sink(10_u16);
            let cc = ctx.new_cell_sink(100_u16);
            let sum_stream = stream_sink.stream().snapshot4(
                &ca.cell(),
                &cb.cell(),
                &cc.cell(),
                |v: &u16, a: &u16, b: &u16, c: &u16| *v + *a + *b + *c,
            );

            let mut values: Vec<u16> = Vec::new();
            let _listener = sum_stream.listen(move |v: &u16| values.push(black_box(*v)));

            for add in 1_u16..50 {
                for v in 0_u16..100 {
                    stream_sink.send(v);
                }
                match add % 3 {
                    2 => ca.send(add),
                    1 => cb.send(add + 10),
                    _ => cc.send(add + 100),
                }
            }
        });
    });
    snapshot.bench_function("four", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let stream_sink = ctx.new_stream_sink();
            let ca = ctx.new_cell_sink(0_u16);
            let cb = ctx.new_cell_sink(10_u16);
            let cc = ctx.new_cell_sink(100_u16);
            let cd = ctx.new_cell_sink(1000_u16);
            let sum_stream = stream_sink.stream().snapshot5(
                &ca.cell(),
                &cb.cell(),
                &cc.cell(),
                &cd.cell(),
                |v: &u16, a: &u16, b: &u16, c: &u16, d: &u16| *v + *a + *b + *c + *d,
            );

            let mut values: Vec<u16> = Vec::new();
            let _listener = sum_stream.listen(move |v: &u16| values.push(black_box(*v)));

            for add in 1_u16..50 {
                for v in 0_u16..100 {
                    stream_sink.send(v);
                }
                match add % 4 {
                    3 => ca.send(add),
                    2 => cb.send(add + 10),
                    1 => cc.send(add + 100),
                    _ => cd.send(add + 1000),
                }
            }
        });
    });
    snapshot.bench_function("five", |b| {
        b.iter(|| {
            let ctx = SodiumCtx::new();

            let stream_sink = ctx.new_stream_sink();
            let ca = ctx.new_cell_sink(0_u16);
            let cb = ctx.new_cell_sink(10_u16);
            let cc = ctx.new_cell_sink(100_u16);
            let cd = ctx.new_cell_sink(1000_u16);
            let ce = ctx.new_cell_sink(5_u16);
            let sum_stream = stream_sink.stream().snapshot6(
                &ca.cell(),
                &cb.cell(),
                &cc.cell(),
                &cd.cell(),
                &ce.cell(),
                |v: &u16, a: &u16, b: &u16, c: &u16, d: &u16, e: &u16| *v + *a + *b + *c + *d + *e,
            );

            let mut values: Vec<u16> = Vec::new();
            let _listener = sum_stream.listen(move |v: &u16| values.push(black_box(*v)));

            for add in 1_u16..50 {
                for v in 0_u16..100 {
                    stream_sink.send(v);
                }
                match add % 5 {
                    4 => ca.send(add),
                    3 => cb.send(add + 10),
                    2 => cc.send(add + 100),
                    1 => cd.send(add + 1000),
                    _ => ce.send(add + 5),
                }
            }
        });
    });
}

criterion_group!(benches, snapshot, cell, stream);
criterion_main!(benches);
