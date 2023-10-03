use std::sync::atomic::Ordering;

use crate::impl_::node::IsNode;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;
use crate::impl_::stream::Stream;
use crate::impl_::stream::WeakStream;

pub struct StreamSink<A> {
    stream: Stream<A>,
    sodium_ctx: SodiumCtx,
}

pub struct WeakStreamSink<A> {
    stream: WeakStream<A>,
    sodium_ctx: SodiumCtx,
}

impl<A> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            stream: self.stream.clone(),
            sodium_ctx: self.sodium_ctx.clone(),
        }
    }
}

impl<A: Send + 'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink {
            stream: Stream::new(sodium_ctx),
            sodium_ctx: sodium_ctx.clone(),
        }
    }

    pub fn new_with_coalescer<COALESCER: FnMut(&A, &A) -> A + Send + 'static>(
        sodium_ctx: &SodiumCtx,
        coalescer: COALESCER,
    ) -> StreamSink<A> {
        StreamSink {
            stream: Stream::_new_with_coalescer(sodium_ctx, coalescer),
            sodium_ctx: sodium_ctx.clone(),
        }
    }

    pub fn stream(&self) -> Stream<A> {
        self.stream.clone()
    }

    pub fn stream_ref(&self) -> &Stream<A> {
        &self.stream
    }

    pub fn send(&self, a: A) {
        self.sodium_ctx.transaction(|| {
            let node = self.stream_ref();
            node.data().changed.store(true, Ordering::SeqCst);
            self.sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                data.changed_nodes.push(node.box_clone());
            });
            self.stream._send(a);
        });
    }

    pub fn downgrade(this: &Self) -> WeakStreamSink<A> {
        WeakStreamSink {
            stream: Stream::downgrade(&this.stream),
            sodium_ctx: this.sodium_ctx.clone(),
        }
    }
}

impl<A> WeakStreamSink<A> {
    pub fn upgrade(&self) -> Option<StreamSink<A>> {
        let sodium_ctx = self.sodium_ctx.clone();
        self.stream
            .upgrade()
            .map(|stream: Stream<A>| StreamSink { stream, sodium_ctx })
    }
}
