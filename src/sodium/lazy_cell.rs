use sodium::Cell;
use sodium::IsCell;
use sodium::Lazy;
use sodium::SodiumCtx;
use sodium::Stream;

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

impl<A:Clone + 'static> LazyCell<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx, event: Stream<A>, lazy_init_value: Lazy<A>) -> LazyCell<A> {
        let mut cell = Cell::new_(
            sodium_ctx,
            event,
            None
        );
        cell.with_cell_data_mut(
            move |data|
                data.lazy_init_value = Some(lazy_init_value)
        );
        LazyCell {
            cell: cell
        }
    }
}