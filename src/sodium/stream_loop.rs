use sodium::HasNode;
use sodium::IsStream;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use std::cell::RefCell;

pub struct StreamLoop<A> {
    stream: StreamWithSend<A>,
    assigned: bool
}

impl<A: 'static + Clone> IsStream<A> for StreamLoop<A> {
    fn to_stream_ref(&self) -> &Stream<A> {
        &self.stream.stream
    }
}

impl<A: 'static + Clone> StreamLoop<A> {

    pub fn new(sodium_ctx: &mut SodiumCtx) -> StreamLoop<A> {
        if sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.is_none()) {
            panic!("StreamLoop/CellLoop must be used within an explicit transaction");
        }
        StreamLoop {
            stream: StreamWithSend::new(sodium_ctx),
            assigned: false
        }
    }

    pub fn loop_(&mut self, sodium_ctx: &mut SodiumCtx, ea_out: Stream<A>) {
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
                    ea_out.listen_(
                        sodium_ctx,
                        self.stream.stream.data.clone().upcast(|x| x as &RefCell<HasNode>),
                        TransactionHandlerRef::new(
                            sodium_ctx2,
                            move |sodium_ctx, trans, a| {
                                let me = me.upgrade();
                                match me {
                                    Some(me) => {
                                        me.send(sodium_ctx, trans, a);
                                    },
                                    None => ()
                                }
                            }
                        )
                    )
                );
            }
        )
    }
}
