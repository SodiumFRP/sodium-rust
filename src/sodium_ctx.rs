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
    /// Create a new Sodium FRP context.
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            impl_: SodiumCtxImpl::new(),
        }
    }

    /// Create a new constant value [`Cell`] in this context.
    pub fn new_cell<A: Clone + Send + 'static>(&self, a: A) -> Cell<A> {
        Cell::new(self, a)
    }

    /// Create a new stream that will never fire in this context.
    pub fn new_stream<A: Clone + Send + 'static>(&self) -> Stream<A> {
        Stream::new(self)
    }

    /// Create a new [`CellSink`] for interfacing I/O and FRP.
    pub fn new_cell_sink<A: Clone + Send + 'static>(&self, a: A) -> CellSink<A> {
        CellSink::new(self, a)
    }

    /// Create a new [`StreamSink`] for interfacing I/O and FRP.
    pub fn new_stream_sink<A: Clone + Send + 'static>(&self) -> StreamSink<A> {
        StreamSink::new(self)
    }

    /// Create a new [`CellLoop`] to act as a forward reference for a
    /// [`Cell`] that will be created later.
    pub fn new_cell_loop<A: Clone + Send + 'static>(&self) -> CellLoop<A> {
        CellLoop::new(self)
    }

    /// Create a new [`StreamLoop`] to act as a forward reference for
    /// a [`Stream`] that will be created later.
    pub fn new_stream_loop<A: Clone + Send + 'static>(&self) -> StreamLoop<A> {
        StreamLoop::new(self)
    }

    /// Create a new [`StreamSink`] with a combining function that
    /// allows [`send`][CellSink::send]ing multiple event values per
    /// transaction.
    pub fn new_stream_sink_with_coalescer<
        A: Clone + Send + 'static,
        COALESCER: FnMut(&A, &A) -> A + Send + 'static,
    >(
        &self,
        coalescer: COALESCER,
    ) -> StreamSink<A> {
        StreamSink::new_with_coalescer(self, coalescer)
    }

    /// Run the given function inside a single Sodium transaction,
    /// closing the transaction after the function returns.
    pub fn transaction<R, K: FnOnce() -> R>(&self, k: K) -> R {
        self.impl_.transaction(k)
    }

    /// Create a new scoped transaction object.
    ///
    /// The Sodium transaction on this context will be held open until
    /// the returned [`Transaction`] is dropped or
    /// [Transaction::close] is called explicitly.
    pub fn new_transaction(&self) -> Transaction {
        Transaction::new(self)
    }

    /// Execute the given code after the current transaction is
    /// closed, or immediately if there is no current transaction.
    pub fn post<K: FnMut() + Send + 'static>(&self, k: K) {
        self.impl_.post(k);
    }

    /// Create a new [`Router`] in this context.
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
