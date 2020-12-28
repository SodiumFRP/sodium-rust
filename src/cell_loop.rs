use crate::impl_::cell_loop::CellLoop as CellLoopImpl;
use crate::Cell;
use crate::SodiumCtx;

/// A forward reference for a [`Cell`] for creating dependency loops.
///
/// Both the creation of a `CellLoop` and filling it with the
/// referenced [`Cell`] by calling [`loop_`][CellLoop::loop_] _must_
/// occur within the same transaction, whether that is created by
/// calling [`SodiumCtx::transaction`] or
/// [`Transaction::new`][crate::Transaction::new].
pub struct CellLoop<A> {
    impl_: CellLoopImpl<A>,
}

impl<A> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            impl_: self.impl_.clone(),
        }
    }
}

impl<A: Send + Clone + 'static> CellLoop<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> CellLoop<A> {
        CellLoop {
            impl_: CellLoopImpl::new(&sodium_ctx.impl_),
        }
    }

    pub fn cell(&self) -> Cell<A> {
        Cell {
            impl_: self.impl_.cell(),
        }
    }

    pub fn loop_(&self, ca: &Cell<A>) {
        self.impl_.loop_(&ca.impl_);
    }
}
