use sodium::CoalesceHandler;
use sodium::IsStream;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamWithSend;
use sodium::Transaction;

pub struct StreamSink<A> {
    stream: StreamWithSend<A>,
    coalescer: CoalesceHandler<A>
}

impl<A> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            stream: self.stream.clone(),
            coalescer: self.coalescer.clone()
        }
    }
}

impl<A: Clone + 'static> IsStream<A> for StreamSink<A> {
    fn to_stream_ref(&self) -> &Stream<A> {
        self.stream.to_stream_ref()
    }

}

impl<A: Clone + 'static> StreamSink<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> StreamSink<A> {
        StreamSink::new_with_coalescer(
            sodium_ctx,
            |_,_| {
                panic!("send() called more than once per transaction, which isn't allowed. Did you want to combine the events? Then pass a combining function to your StreamSink constructor.");
            }
        )
    }

    pub fn new_with_coalescer<F>(sodium_ctx: &mut SodiumCtx, f: F) -> StreamSink<A> where F: Fn(&A,&A)->A + 'static {
        let out = StreamWithSend::new(sodium_ctx);
        StreamSink {
            stream: out.clone(),
            coalescer: CoalesceHandler::new(f, out.downgrade())
        }
    }

    pub fn send(&self, sodium_ctx: &mut SodiumCtx, a: &A) {
        Transaction::run_trans(
            sodium_ctx,
            |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction| {
                if sodium_ctx.with_data_ref(|ctx| ctx.in_callback > 0) {
                    panic!("You are not allowed to use send() inside a Sodium callback");
                }
                self.coalescer.run(sodium_ctx, trans, a);
            }
        )
    }
}
