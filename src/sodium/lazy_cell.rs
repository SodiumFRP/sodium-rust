use sodium::Cell;
use sodium::IsCell;

pub struct LazyCell<A> {
    pub cell: Cell<A>
}

impl<A: Clone + 'static> IsCell<A> for LazyCell<A> {
    fn to_cell_ref(&self) -> &Cell<A> {
        &self.cell
    }

    // Implementation moved to Cell, because a cast from LazyCell to Cell will not use the correct
    // sample_no_trans_ implementation.
    //fn sample_no_trans_(&self) -> A;
}
