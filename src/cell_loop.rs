use crate::Cell;
use crate::SodiumCtx;
use crate::impl_::cell_loop::CellLoop as CellLoopImpl;

pub struct CellLoop<A> {
    impl_: CellLoopImpl<A>
}

impl<A> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            impl_: self.impl_.clone()
        }
    }
}

impl<A:Send+Clone+'static> CellLoop<A> {

    pub fn new(sodium_ctx: &SodiumCtx) -> CellLoop<A> {
        CellLoop {
            impl_: CellLoopImpl::new(&sodium_ctx.impl_)
        }
    }

    pub fn cell(&self) -> Cell<A> {
        Cell { impl_: self.impl_.cell() }
    }

    pub fn loop_(&self, ca: &Cell<A>) {
        self.impl_.loop_(&ca.impl_);
    }
}
