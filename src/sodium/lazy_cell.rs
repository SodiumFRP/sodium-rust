use sodium::Cell;
use sodium::IsCell;

pub struct LazyCell<A> {
    pub cell: Cell<A>
}

impl<A: Clone + 'static> IsCell<A> for LazyCell<A> {
    fn to_cell_ref(&self) -> &Cell<A> {
        &self.cell
    }

    fn sample_no_trans_(&self) -> A {
        let mut data = (*self.cell.data).borrow_mut();
        let data = &mut data;
        if data.value.is_none() && data.lazy_init_value.is_some() {
            data.value = Some(data.lazy_init_value.as_ref().unwrap().get());
            data.lazy_init_value = None;
        }
        data.value.clone().unwrap()
    }
}
