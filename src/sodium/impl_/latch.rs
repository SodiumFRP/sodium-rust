use sodium::gc::Finalize;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub struct Latch<A> {
    data: Rc<UnsafeCell<LatchData<A>>>
}

struct LatchData<A> {
    thunk: Box<Fn()->A>,
    val: A
}

impl<A: Trace> Trace for Latch<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let self_ = unsafe { &*(*self.data).get() };
        self_.val.trace(f);
    }
}

impl<A: Finalize> Finalize for Latch<A> {
    fn finalize(&mut self) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.val.finalize();
    }
}

impl<A:Clone + 'static> Latch<A> {
    pub fn const_(value:  A) -> Latch<A> {
        Latch::new(move || value.clone())
    }
}

impl<A> Latch<A> {
    pub fn new<F: Fn()->A + 'static>(thunk: F) -> Latch<A> {
        let val = thunk();
        Latch {
            data: Rc::new(UnsafeCell::new(
                LatchData {
                    thunk: Box::new(thunk),
                    val
                }
            ))
        }
    }

    pub fn get(&self) -> &A {
        let self_ = unsafe { &*(*self.data).get() };
        &self_.val
    }

    pub fn get_mut(&mut self) -> &mut A {
        let self_ = unsafe { &mut *(*self.data).get() };
        &mut self_.val
    }

    pub fn reset(&mut self) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.val = (self_.thunk)();
    }
}

impl<A: Clone> Clone for Latch<A> {
    fn clone(&self) -> Self {
        Latch {
            data: self.data.clone()
        }
    }
}
