use sodium::Cell;
use sodium::IsCell;
use sodium::gc::Finalize;
use sodium::gc::Trace;
use sodium::impl_;

pub struct CellLoop<A> {
    pub impl_: impl_::CellLoop<A>
}

impl<A: Trace + Finalize + Clone + 'static> CellLoop<A> {

    pub fn loop_<CA:IsCell<A>>(&self, ca: CA) {
        self.impl_.loop_(ca.to_cell().impl_);
    }

    pub fn to_cell(&self) -> Cell<A> {
        Cell {
            impl_: self.impl_.to_cell()
        }
    }
}

impl<A: Trace + Finalize + Clone + 'static> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            impl_: self.impl_.clone()
        }
    }
}
