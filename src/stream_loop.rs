use crate::impl_::stream_loop::StreamLoop as StreamLoopImpl;
use crate::SodiumCtx;
use crate::Stream;

/// A forward reference of a [`Stream`] for creating dependency loops.
pub struct StreamLoop<A> {
    pub impl_: StreamLoopImpl<A>,
}

impl<A: Send + Clone + 'static> StreamLoop<A> {
    /// Create a new `StreamLoop` in the given context.
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        StreamLoop {
            impl_: StreamLoopImpl::new(&sodium_ctx.impl_),
        }
    }

    /// Return a [`Stream`] that is equivalent to this `StreamLoop`
    /// once it has been resolved by calling
    /// [`loop_`][StreamLoop::loop_].
    pub fn stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.stream(),
        }
    }

    /// Resolve the loop to specify what this `StreamLoop` was a
    /// forward reference to.
    ///
    /// This function _must_ be invoked in the same transaction as the
    /// place where the `StreamLoop` is used. This requires you to
    /// create an explicit transaction, either with
    /// [`SodiumCtx::transaction`] or
    /// [`Transaction::new`][crate::Transaction::new].
    pub fn loop_(&self, sa: &Stream<A>) {
        self.impl_.loop_(&sa.impl_);
    }
}
