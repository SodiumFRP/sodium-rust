use sodium::impl_::Cell;
use sodium::impl_::Dep;
use sodium::impl_::Lambda;
use sodium::impl_::MemoLazy;
use sodium::impl_::SodiumCtx;
use sodium::gc::Finalize;
use sodium::gc::Gc;
use sodium::gc::Trace;
use std::cell::UnsafeCell;

pub struct CellSink<A> {
    next_value_op: Gc<UnsafeCell<Option<MemoLazy<A>>>>,
    cell: Cell<A>
}

impl<A: Trace + Finalize + Clone + 'static> CellSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> CellSink<A> {
        let mut gc_ctx = sodium_ctx.gc_ctx();
        let next_value_op = gc_ctx.new_gc_with_desc(UnsafeCell::new(None), String::from("CellSink::new_next_value"));
        let deps = vec![Dep { gc_dep: next_value_op.to_dep() }];
        CellSink {
            next_value_op: next_value_op.clone(),
            cell: Cell::_new(
                sodium_ctx,
                sodium_ctx.new_lazy(move || value.clone()),
                Lambda::new(
                    move || {
                        let next_value_op = unsafe { &*(*next_value_op).get() };
                        next_value_op.clone()
                    },
                    deps
                ),
                Vec::new(),
                || {},
                "CellSink::new"
            )
        }
    }

    pub fn send(&self, value: A) {
        let sodium_ctx = self.cell._node().sodium_ctx();
        sodium_ctx.transaction(|| {
            let next_value_op = unsafe { &mut *(*self.next_value_op).get() };
            *next_value_op = Some(sodium_ctx.new_lazy(move || value.clone()));
            self.cell._node().mark_dirty();
        });
    }

    pub fn to_cell(&self) -> Cell<A> {
        self.cell.clone()
    }
}

impl<A: Trace + Finalize + Clone + 'static> Clone for CellSink<A> {
    fn clone(&self) -> Self {
        CellSink {
            next_value_op: self.next_value_op.clone(),
            cell: self.cell.clone()
        }
    }
}
