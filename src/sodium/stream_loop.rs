use sodium::HasNode;
use sodium::Stream;
use sodium::stream;
use sodium::stream::HasStream;
use sodium::stream::IsStreamPrivate;
use sodium::SodiumCtx;
use sodium::stream::StreamImpl;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use std::cell::RefCell;

pub struct StreamLoop<A> {
    sodium_ctx: SodiumCtx,
    stream: StreamWithSend<A>,
    assigned: bool
}

impl<A: 'static + Clone> IsStreamPrivate<A> for StreamLoop<A> {
    fn to_stream_ref(&self) -> &StreamImpl<A> {
        &self.stream.stream
    }
}

impl<A: Clone + 'static> HasStream<A> for StreamLoop<A> {
    fn stream(&self) -> Stream<A> {
        stream::make_stream(
            self.sodium_ctx.clone(),
            self.stream.stream.clone()
        )
    }
}

impl<A: 'static + Clone> StreamLoop<A> {

    pub fn new(sodium_ctx: &mut SodiumCtx) -> StreamLoop<A> {
        if sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.is_none()) {
            panic!("StreamLoop/CellLoop must be used within an explicit transaction");
        }
        StreamLoop {
            sodium_ctx: sodium_ctx.clone(),
            stream: StreamWithSend::new(sodium_ctx),
            assigned: false
        }
    }

    pub fn loop_(&mut self, ea_out: Stream<A>) {
        let mut sodium_ctx = stream::get_sodium_ctx(&ea_out);
        let sodium_ctx = &mut sodium_ctx;
        if self.assigned {
            panic!("StreamLoop looped more than once");
        }
        self.assigned = true;
        Transaction::run(
            sodium_ctx,
            move |sodium_ctx| {
                let me = self.stream.downgrade();
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                self.unsafe_add_cleanup(
                    stream::get_stream_impl(&ea_out).listen_(
                        sodium_ctx,
                        self.stream.stream.data.clone().upcast(|x| x as &RefCell<HasNode>),
                        TransactionHandlerRef::new(
                            sodium_ctx2,
                            move |sodium_ctx, trans, a| {
                                me.send(sodium_ctx, trans, a);
                            }
                        )
                    )
                );
            }
        )
    }
}
