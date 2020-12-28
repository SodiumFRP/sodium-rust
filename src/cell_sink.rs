use crate::cell::Cell;
use crate::impl_::cell_sink::CellSink as CellSinkImpl;
use crate::sodium_ctx::SodiumCtx;

/// A [`Cell`] that allows values to be pushed into it, acting as a
/// bridge between the world of I/O and the world of FRP.
///
/// ## Note: This should only be used from _outside_ the context of
/// the Sodium system to inject data from I/O into the reactive system.
pub struct CellSink<A> {
    pub impl_: CellSinkImpl<A>,
}

impl<A> Clone for CellSink<A> {
    fn clone(&self) -> Self {
        CellSink {
            impl_: self.impl_.clone(),
        }
    }
}

impl<A: Clone + Send + 'static> CellSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx, a: A) -> CellSink<A> {
        CellSink {
            impl_: CellSinkImpl::new(&sodium_ctx.impl_, a),
        }
    }

    pub fn cell(&self) -> Cell<A> {
        Cell {
            impl_: self.impl_.cell(),
        }
    }

    pub fn send(&self, a: A) {
        self.impl_.send(a);
    }
}
