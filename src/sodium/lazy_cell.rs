use sodium::Cell;
use sodium::IsCell;
use sodium::HasCellDataGc;
use sodium::HasCellData;
use sodium::Lazy;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::gc::Gc;
use std::cell::RefCell;
use std::rc::Rc;

pub struct LazyCell<A> {
    pub cell: Cell<A>
}

/*
trait HasCellDataRc<A> {
    fn cell_data(&self) -> Rc<RefCell<HasCellData<A>>>;
}

impl<A: 'static> HasCellDataRc<A> for Cell<A> {
    fn cell_data(&self) -> Rc<RefCell<HasCellData<A>>> {
        self.data.clone() as Rc<RefCell<HasCellData<A>>>
    }
}

trait HasCellData<A> {
    fn cell_data_ref(&self) -> &CellData<A>;
    fn cell_data_mut(&mut self) -> &mut CellData<A>;
}
*/

impl<A: Clone + 'static> HasCellDataGc<A> for LazyCell<A> {
    fn cell_data(&self) -> Gc<RefCell<HasCellData<A>>> {
        self.cell.data.clone()
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