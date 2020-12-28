use crate::impl_::sodium_ctx::SodiumCtx as SodiumCtxImpl;
use crate::Cell;
use crate::CellLoop;
use crate::CellSink;
use crate::Router;
use crate::Stream;
use crate::StreamLoop;
use crate::StreamSink;
use crate::Transaction;
use std::hash::Hash;

/// A context object representing a specific instance of a Sodium
/// system.
#[derive(Clone)]
pub struct SodiumCtx {
    pub impl_: SodiumCtxImpl,
}

impl Default for SodiumCtx {
    fn default() -> SodiumCtx {
        SodiumCtx::new()
    }
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            impl_: SodiumCtxImpl::new(),
        }
    }

    pub fn new_cell<A: Clone + Send + 'static>(&self, a: A) -> Cell<A> {
        Cell::new(self, a)
    }

    pub fn new_stream<A: Clone + Send + 'static>(&self) -> Stream<A> {
        Stream::new(self)
    }

    pub fn new_cell_sink<A: Clone + Send + 'static>(&self, a: A) -> CellSink<A> {
        CellSink::new(self, a)
    }

    pub fn new_stream_sink<A: Clone + Send + 'static>(&self) -> StreamSink<A> {
        StreamSink::new(self)
    }

    pub fn new_cell_loop<A: Clone + Send + 'static>(&self) -> CellLoop<A> {
        CellLoop::new(self)
    }

    pub fn new_stream_loop<A: Clone + Send + 'static>(&self) -> StreamLoop<A> {
        StreamLoop::new(self)
    }

    pub fn new_stream_sink_with_coalescer<
        A: Clone + Send + 'static,
        COALESCER: FnMut(&A, &A) -> A + Send + 'static,
    >(
        &self,
        coalescer: COALESCER,
    ) -> StreamSink<A> {
        StreamSink::new_with_coalescer(self, coalescer)
    }

    pub fn transaction<R, K: FnOnce() -> R>(&self, k: K) -> R {
        self.impl_.transaction(k)
    }

    pub fn new_transaction(&self) -> Transaction {
        Transaction::new(self)
    }

    pub fn post<K: FnMut() + Send + 'static>(&self, k: K) {
        self.impl_.post(k);
    }

    pub fn new_router<A, K>(
        &self,
        in_stream: &Stream<A>,
        selector: impl Fn(&A) -> Vec<K> + Send + Sync + 'static,
    ) -> Router<A, K>
    where
        A: Clone + Send + 'static,
        K: Send + Sync + Eq + Hash + 'static,
    {
        Router::new(self, in_stream, selector)
    }
}
