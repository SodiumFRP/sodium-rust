use sodium::SodiumCtx;
use std::ops::Deref;
use std::rc::Rc;

pub struct HandlerRef<A> {
    run: Rc<Fn(&mut SodiumCtx,&A)>
}

impl<A> Clone for HandlerRef<A> {
    fn clone(&self) -> Self {
        HandlerRef {
            run: self.run.clone()
        }
    }
}

impl<A> HandlerRef<A> {
    pub fn new<F>(f: F) -> HandlerRef<A> where F: Fn(&mut SodiumCtx,&A) + 'static {
        HandlerRef {
            run: Rc::new(f)
        }
    }

    pub fn run(&self, sodium_ctx: &mut SodiumCtx, a: &A) {
        (self.run)(sodium_ctx, a);
    }
}

pub struct HandlerRefMut<A> {
    run: Rc<Fn(&mut SodiumCtx, &mut A)>
}

impl<A> Clone for HandlerRefMut<A> {
    fn clone(&self) -> HandlerRefMut<A> {
        HandlerRefMut {
            run: self.run.clone()
        }
    }
}

impl<A> HandlerRefMut<A> {
    pub fn new<F>(f: F) -> HandlerRefMut<A> where F: Fn(&mut SodiumCtx, &mut A) + 'static {
        HandlerRefMut {
            run: Rc::new(f)
        }
    }

    pub fn run(&self, sodium_ctx: &mut SodiumCtx, a: &mut A) {
        (self.run)(sodium_ctx, a)
    }
}
