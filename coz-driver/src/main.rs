use std::{hint::black_box, sync::mpsc, thread};

use sodium_rust::{SodiumCtx, StreamSink, StreamLoop, Stream, Operational, Cell};


fn main() {
    let sodium_ctx = SodiumCtx::new();
    let sodium_ctx = &sodium_ctx;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        while let Ok(v) = rx.recv() {
            black_box(v);
        }
    });

    let ss_input: StreamSink<i64>;
    let s_output;
    {
        let _t = sodium_ctx.new_transaction();

        ss_input = sodium_ctx.new_stream_sink();
        let sl_s_output: StreamLoop<Stream<i64>> = sodium_ctx.new_stream_loop();
        let c_s_output = sl_s_output.stream().hold(ss_input.stream());
        s_output = Cell::switch_s(&c_s_output);
        let s_output_next =
            s_output.snapshot(&c_s_output, |prime: &i64, old_s_output: &Stream<i64>| {
                let prime = *prime;
                old_s_output.filter(move |x: &i64| (*x % prime) != 0)
            });
        sl_s_output.loop_(&Operational::defer(&s_output_next));
    }

    let l = s_output.listen(move |prime: &i64| {
        tx.send(*prime).ok();
    });
    for x in 2..60_000 {
        coz::begin!("stream_send");
        ss_input.send(x);
        coz::end!("stream_send");
    }

    l.unlisten();
}
