use sodium::HandlerRefMut;
use sodium::IsStream;
use sodium::IsNode;
use sodium::Node;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamData;
use sodium::Transaction;
use std::any::Any;

pub struct StreamWithSend<A> {
    pub stream: Stream<A>
}

impl<A> Clone for StreamWithSend<A> {
    fn clone(&self) -> Self {
        StreamWithSend {
            stream: self.stream.clone()
        }
    }
}

impl<A:'static + Clone> IsStream<A> for StreamWithSend<A> {
    fn to_stream_ref(&self) -> &Stream<A> {
        &self.stream
    }
}

impl<A:'static + Clone> StreamWithSend<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> StreamWithSend<A> {
        StreamWithSend {
            stream: Stream::new(sodium_ctx)
        }
    }

    pub fn send(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &A) {
        let targets;
        {
            let self2 = self.to_stream_ref().clone();
            let mut self_ = self.to_stream_ref().data.borrow_mut();
            let self__: &mut StreamData<A> = &mut *self_;
            if self__.firings.is_empty() {
                trans.last(move || {
                    let mut self_ = self2.data.borrow_mut();
                    let self__: &mut StreamData<A> = &mut *self_;
                    self__.firings.clear();
                })
            }
            self__.firings.push(a.clone());
            let mut node = self__.node.borrow_mut();
            let node2: &mut IsNode = &mut *node;
            let node3 = node2.downcast_to_node_mut();
            targets = node3.listeners.clone();
        }
        for target in targets {
            let target2;
            {
                let mut target_node = target.node.borrow_mut();
                let target_node2: &mut IsNode = &mut *target_node;
                let target_node3: &mut Node = target_node2.downcast_to_node_mut();
                target2 = target.clone();
            }
            let a2 = a.clone();
            trans.prioritized(
                sodium_ctx,
                target.node.clone(),
                HandlerRefMut::new(
                    move |sodium_ctx: &mut SodiumCtx, trans2| {
                        sodium_ctx.with_data_mut(|ctx| ctx.in_callback = ctx.in_callback + 1);
                        // Note: action here is a strong reference to a weak reference.
                        target2.action.run(sodium_ctx, trans2, &a2 as &Any);
                        sodium_ctx.with_data_mut(|ctx| ctx.in_callback = ctx.in_callback - 1);
                    }
                )
            )
        }
    }
}
