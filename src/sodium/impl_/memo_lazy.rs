use sodium::impl_::Dep;
use sodium::impl_::IsLambda0;
use sodium::impl_::gc::Finalize;
use sodium::impl_::gc::Gc;
use sodium::impl_::gc::GcCtx;
use sodium::impl_::gc::GcDep;
use sodium::impl_::gc::Trace;
use std::cell::UnsafeCell;

pub struct MemoLazy<A> {
    data: Gc<MemoLazyData<A>>
}

pub struct MemoLazyData<A> {
    thunk: Box<IsLambda0<A>>,
    val_op: UnsafeCell<Option<A>>
}

impl<A: Trace> Trace for MemoLazy<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        f(&self.data.to_dep());
    }
}

impl<A: Finalize> Finalize for MemoLazy<A> {
    fn finalize(&mut self) {
    }
}

impl<A: Trace> Trace for MemoLazyData<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        for dep in self.thunk.deps() {
            let Dep { gc_dep: dep2 } = dep;
            f(&dep2);
        }
        self.val_op.trace(f);
    }
}

impl<A: Finalize> Finalize for MemoLazyData<A> {
    fn finalize(&mut self) {
        self.val_op.finalize();
    }    
}

impl<A: Trace + Finalize + 'static> MemoLazy<A> {
    pub fn new<F: IsLambda0<A> + 'static>(gc_ctx: &mut GcCtx, thunk: F) -> MemoLazy<A> {
        MemoLazy {
            data: gc_ctx.new_gc_with_desc(
                MemoLazyData {
                    thunk: Box::new(thunk),
                    val_op: UnsafeCell::new(None)
                },
                String::from("MemoLazy::new")
            )
        }
    }

    pub fn get(&self) -> &A {
        let self_ = &*self.data;
        let val_op = unsafe { &*self_.val_op.get() };
        match val_op {
            Some(ref val) => val,
            None => {
                let val = (self_.thunk).apply();
                unsafe {
                    *self_.val_op.get() = Some(val);
                }
                self.get()
            }
        }
    }

    pub fn get_mut(&mut self) -> &mut A {
        let val_op;
        {
            let self_ = &*self.data;
            val_op = unsafe { &mut *self_.val_op.get() };
        }
        match val_op {
            Some(ref mut val) => val,
            None => {
                {
                    let self_ = &*self.data;
                    let val = (self_.thunk).apply();
                    unsafe {
                        *self_.val_op.get() = Some(val);
                    }
                }
                self.get_mut()
            }
        }
    }
}

impl<A: Clone> Clone for MemoLazy<A> {
    fn clone(&self) -> Self {
        MemoLazy {
            data: self.data.clone()
        }
    }
}
