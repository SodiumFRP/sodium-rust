use sodium::impl_::Dep;
use sodium::impl_::MemoLazy;
use sodium::impl_::Node;
use sodium::impl_::SodiumCtx;
use sodium::impl_::Cell;
use sodium::impl_::gc::Finalize;
use sodium::impl_::gc::Gc;
use sodium::impl_::gc::Trace;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub struct CellLoop<A> {
    cell: Cell<A>,
    init_value: Rc<UnsafeCell<Option<A>>>
}

impl<A: Trace + Finalize + Clone + 'static> CellLoop<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> CellLoop<A> {
        let init_value: Rc<UnsafeCell<Option<A>>> = Rc::new(UnsafeCell::new(None));
        let cell;
        {
            let init_value = init_value.clone();
            cell = Cell::new_lazy(
                sodium_ctx,
                sodium_ctx.new_lazy(move || {
                    let init_value = unsafe { &*(*init_value).get() };
                    if let &Some(ref val) = init_value {
                        val.clone()
                    } else {
                        panic!("CellLoop sampled before looped.")
                    }
                })
            );
        }
        CellLoop {
            cell,
            init_value
        }
    }

    pub fn loop_(&self, ca: Cell<A>) {
        let init_value = unsafe { &mut *(*self.init_value).get() };
        if init_value.is_some() {
            panic!("CellLoop looped more than once.");
        }
        *init_value = Some(ca.sample_no_trans());
        let value = self.cell._value().clone();
        let next_value = self.cell._next_value().clone();
        let update_deps = vec![ca.to_dep(), Dep { gc_dep: next_value.to_dep() }];
        let ca_node = ca._node().clone();
        let node = self.cell._node().clone();
        let sodium_ctx = node.sodium_ctx();
        node.set_update(
            move || {
                let sodium_ctx = &sodium_ctx;
                {
                    let next_value = unsafe { &mut *(*next_value).get() };
                    let ca = ca.clone();
                    let ca_next_value = unsafe { &*(*ca._next_value()).get() };
                    let x = ca_next_value.get().clone();
                    *next_value = sodium_ctx.new_lazy(move || {
                        x.clone()
                    });
                }
                let value = value.clone();
                let next_value = next_value.clone();
                sodium_ctx.post(move || {
                    let value = unsafe { &mut *(*value).get() };
                    let next_value = unsafe { &mut *(*next_value).get() };
                    *value = next_value.clone();
                });
                return true;
            },
            update_deps
        );
        node.ensure_bigger_than(ca_node.rank());
        node.add_dependencies(vec![ca_node]);
    }

    pub fn to_cell(&self) -> Cell<A> {
        self.cell.clone()
    }
}

impl<A: Trace + Finalize + Clone + 'static> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            cell: self.cell.clone(),
            init_value: self.init_value.clone()
        }
    }
}