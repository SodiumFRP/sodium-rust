use crate::impl_::stream_loop::StreamLoop as StreamLoopImpl;
use crate::SodiumCtx;
use crate::Stream;

/// A forward reference of a [`Stream`] for creating dependency loops.
pub struct StreamLoop<A> {
    pub impl_: StreamLoopImpl<A>,
}

impl<A: Send + Clone + 'static> StreamLoop<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        StreamLoop {
            impl_: StreamLoopImpl::new(&sodium_ctx.impl_),
        }
    }

    pub fn stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.stream(),
        }
    }

    pub fn loop_(&self, sa: &Stream<A>) {
        self.impl_.loop_(&sa.impl_);
    }
}
