use sodium::IsStream;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamData;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;

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
        if sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.borrow().is_none()) {
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
        let me = self.to_stream_ref().downgrade();
        Transaction::run(
            sodium_ctx,
            move |sodium_ctx| {
                let me2 = me.clone();
                let me_ = me.upgrade().unwrap();
                let me_data = me_.data.borrow();
                let me_data_: &StreamData<A> = &*me_data;
                me_.unsafe_add_cleanup(
                    ea_out.listen_(
                        sodium_ctx,
                        me_data_.node.clone(),
                        TransactionHandlerRef::new(
                            move |sodium_ctx, trans, a| {
                                let me_ = me2.upgrade();
                                match me_ {
                                    Some(me3) => {
                                        let me4 = StreamWithSend {
                                            stream: me3
                                        };
                                        me4.send(sodium_ctx, trans, a);
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
