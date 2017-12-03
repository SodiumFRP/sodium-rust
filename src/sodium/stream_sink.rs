use sodium::CoalesceHandler;
use sodium::stream;
use sodium::stream::HasStream;
use sodium::stream::IsStreamPrivate;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::stream::StreamImpl;
use sodium::StreamWithSend;
use sodium::Transaction;

pub struct StreamSink<A> {
    sodium_ctx: SodiumCtx,
    stream: StreamWithSend<A>,
    coalescer: CoalesceHandler<A>
}

impl<A> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            sodium_ctx: self.sodium_ctx.clone(),
            stream: self.stream.clone(),
            coalescer: self.coalescer.clone()
        }
    }
}

impl<A: Clone + 'static> HasStream<A> for StreamSink<A> {
    fn stream(&self) -> Stream<A> {
        stream::make_stream(
            self.sodium_ctx.clone(),
            self.stream.stream.clone()
        )
    }
}


impl<A: Clone + 'static> IsStreamPrivate<A> for StreamSink<A> {
    fn to_stream_ref(&self) -> &StreamImpl<A> {
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
            sodium_ctx: sodium_ctx.clone(),
            stream: out.clone(),
            coalescer: CoalesceHandler::new(f, out.downgrade())
        }
    }

    pub fn send(&self, a: &A) {
        let mut sodium_ctx = self.sodium_ctx.clone();
        let sodium_ctx = &mut sodium_ctx;
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
