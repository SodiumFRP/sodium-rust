use sodium::HandlerRefMut;
use sodium::IsStream;
use sodium::HasNode;
use sodium::Node;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamData;
use sodium::Transaction;
use sodium::WeakStream;
use std::any::Any;

pub struct StreamWithSend<A> {
    pub stream: Stream<A>
}

pub struct WeakStreamWithSend<A> {
    pub stream: WeakStream<A>
}

impl<A> Clone for StreamWithSend<A> {
    fn clone(&self) -> Self {
        StreamWithSend {
            stream: self.stream.clone()
        }
    }
}

impl<A> Clone for WeakStreamWithSend<A> {
    fn clone(&self) -> Self {
        WeakStreamWithSend {
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

    pub fn downgrade(&self) -> WeakStreamWithSend<A> {
        WeakStreamWithSend {
            stream: self.stream.downgrade()
        }
    }

    pub fn send(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &A) {
        let targets;
        {
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
            }
            let node = self.to_stream_ref().data.clone();
            let node2 = node.borrow();
            let node4 = node2.node_ref();
            targets = node4.listeners.clone();
        }
        for target in targets {
            let target2;
            let target_node2;
            {
                match target.node.upgrade() {
                    Some(target_node) => {
                        target_node2 = target_node.clone();
                        let mut target_node = target_node.borrow_mut();
                        let target_node2: &mut HasNode = &mut *target_node;
                        let _target_node3: &mut Node = target_node2.node_mut();
                        target2 = target.clone();
                    },
                    None => continue
                }
            }
            let a2 = a.clone();
            trans.prioritized(
                sodium_ctx,
                target_node2,
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

impl<A: Clone + 'static> WeakStreamWithSend<A> {
    pub fn upgrade(&self) -> Option<StreamWithSend<A>> {
        self.stream
            .upgrade()
            .map(
                |stream|
                    StreamWithSend {
                        stream: stream
                    }
            )
    }

    pub fn send(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &A) {
        for stream_with_send in self.upgrade().iter() {
            stream_with_send.send(sodium_ctx, trans, a);
        }
    }
}
