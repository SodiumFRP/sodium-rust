use crate::impl_::cell_sink::CellSink as CellSinkImpl;
use crate::sodium_ctx::SodiumCtx;
use crate::cell::Cell;

pub struct CellSink<A> {
    pub impl_: CellSinkImpl<A>
}

impl<A> Clone for CellSink<A> {
    fn clone(&self) -> Self {
        CellSink {
            impl_: self.impl_.clone()
        }
    }
}

impl<A:Clone+Send+'static> CellSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx, a: A) -> CellSink<A> {
        CellSink { impl_: CellSinkImpl::new(&sodium_ctx.impl_, a) }
    }

    pub fn cell(&self) -> Cell<A> {
        Cell { impl_: self.impl_.cell() }
    }

    pub fn send(&self, a: A) {
        self.impl_.send(a);
    }
}
