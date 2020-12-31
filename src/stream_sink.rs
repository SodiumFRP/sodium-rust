use crate::impl_::stream_sink::StreamSink as StreamSinkImpl;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;

/// A [`Stream`] that allows values to be pushed into it, acting as an
/// interface between the world of I/O and the world of FRP.
///
/// ## Note: This should only be used from _outside_ the context of
/// the Sodium system to inject data from I/O into the reactive system.
pub struct StreamSink<A> {
    pub impl_: StreamSinkImpl<A>,
}

impl<A> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            impl_: self.impl_.clone(),
        }
    }
}

impl<A: Clone + Send + 'static> StreamSink<A> {
    /// Create a `StreamSink` that allows calling `send` on it once
    /// per transaction.
    ///
    /// If you call `send` more than once in a transaction on a
    /// `StreamSink` constructed with `StreamSink::new` it will
    /// panic. If you need to do this then use
    /// [`StreamSink::new_with_coalescer`].
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink {
            impl_: StreamSinkImpl::new(&sodium_ctx.impl_),
        }
    }

    /// Create a `StreamSink` that allows calling `send` one or more
    /// times per transaction.
    ///
    /// If you call `send` on the returned `StreamSink` more than once
    /// in a single transaction the events will be combined into a
    /// single event using the specified combining function. The
    /// combining function should be _associative_.
    pub fn new_with_coalescer<COALESCER: FnMut(&A, &A) -> A + Send + 'static>(
        sodium_ctx: &SodiumCtx,
        coalescer: COALESCER,
    ) -> StreamSink<A> {
        StreamSink {
            impl_: StreamSinkImpl::new_with_coalescer(&sodium_ctx.impl_, coalescer),
        }
    }

    /// Return a [`Stream`] that can be used in the creation of Sodium
    /// logic that will consume events pushed into this `StreamSink`
    /// from the I/O world.
    pub fn stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.stream(),
        }
    }

    /// Send a value to be made available to consumers of the `Stream`
    /// associated with this `StreamSink`.
    ///
    /// This method may not be called in handlers registered with
    /// [`Stream::listen`] or [`Cell::listen`][crate::Cell::listen].
    pub fn send(&self, a: A) {
        self.impl_.send(a);
    }
}
