use sodium::gc::Finalize;
use sodium::gc::Gc;
use sodium::gc::GcCell;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use sodium::gc::GcCtx;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

#[test]
pub fn gc_loop() {
    let count = Rc::new(RefCell::new(0));
    let mut gc_ctx = GcCtx::new();
    struct A {
        count: Weak<RefCell<i32>>,
        x: Cell<Option<Gc<A>>>
    }
    impl A {
        fn new(x: Option<Gc<A>>, count: &Rc<RefCell<i32>>) -> A {
            {
                let mut c = count.borrow_mut();
                *c = *c + 1;
            }
            A {
                count: Rc::downgrade(&count),
                x: Cell::new(x)
            }
        }
    }
    impl Trace for A {
        fn trace(&self, f: &mut FnMut(&GcDep)) {
            match unsafe { &*self.x.as_ptr() } {
                &Some(ref a) => f(&a.to_dep()),
                &None => ()
            }
        }
    }
    impl Finalize for A {
        fn finalize(&mut self) {
            let count = self.count.upgrade().unwrap();
            let mut c = count.borrow_mut();
            *c = *c - 1;
            println!("finalise");
        }
    }
    impl Drop for A {
        fn drop(&mut self) {
            println!("drop");
        }
    }
    {
        let a = gc_ctx.new_gc(A::new(None, &count));
        let b = gc_ctx.new_gc(A::new(Some(a.clone()), &count));
        let c = gc_ctx.new_gc(A::new(Some(b.clone()), &count));
        a.x.set(Some(c.clone()));
        c.debug();
    }
    assert_eq!(0, *count.borrow());
}

#[test]
pub fn gc_twin_loop() {
    let count = Rc::new(RefCell::new(0));
    let mut gc_ctx = GcCtx::new();
    struct A {
        count: Weak<RefCell<i32>>,
        next1: Cell<Option<Gc<A>>>,
        next2: Cell<Option<Gc<A>>>
    }
    impl A {
        fn new(next1: Option<Gc<A>>, next2: Option<Gc<A>>, count: &Rc<RefCell<i32>>) -> A {
            {
                let mut c = count.borrow_mut();
                *c = *c + 1;
            }
            A {
                count: Rc::downgrade(&count),
                next1: Cell::new(next1),
                next2: Cell::new(next2)
            }
        }
    }
    impl Trace for A {
        fn trace(&self, f: &mut FnMut(&GcDep)) {
            let next1 = unsafe { &*self.next1.as_ptr() };
            let next2 = unsafe { &*self.next2.as_ptr() };
            next1.trace(f);
            next2.trace(f);
        }
    }
    impl Finalize for A {
        fn finalize(&mut self) {
            let count = self.count.upgrade().unwrap();
            let mut c = count.borrow_mut();
            *c = *c - 1;
            println!("finalise");
        }
    }
    impl Drop for A {
        fn drop(&mut self) {
            println!("drop");
        }
    }
    {
        let a = gc_ctx.new_gc(A::new(None, None, &count));
        let b = gc_ctx.new_gc(A::new(Some(a.clone()), Some(a.clone()), &count));
        let c = gc_ctx.new_gc(A::new(Some(b.clone()), Some(b.clone()), &count));
        a.next1.set(Some(c.clone()));
        a.next2.set(Some(c.clone()));
        c.debug();
    }
    assert_eq!(0, *count.borrow());
}

#[test]
pub fn gc_twin_loop_self() {
    let count = Rc::new(RefCell::new(0));
    let mut gc_ctx = GcCtx::new();
    struct A {
        count: Weak<RefCell<i32>>,
        next1: Cell<Option<Gc<A>>>,
        next2: Cell<Option<Gc<A>>>,
        self_: Cell<Option<Gc<A>>>
    }
    impl A {
        fn new(next1: Option<Gc<A>>, next2: Option<Gc<A>>, count: &Rc<RefCell<i32>>) -> A {
            {
                let mut c = count.borrow_mut();
                *c = *c + 1;
            }
            A {
                count: Rc::downgrade(&count),
                next1: Cell::new(next1),
                next2: Cell::new(next2),
                self_: Cell::new(None)
            }
        }
    }
    impl Trace for A {
        fn trace(&self, f: &mut FnMut(&GcDep)) {
            let next1 = unsafe { &*self.next1.as_ptr() };
            let next2 = unsafe { &*self.next2.as_ptr() };
            let self_ = unsafe { &*self.self_.as_ptr() };
            next1.trace(f);
            next2.trace(f);
            self_.trace(f);
        }
    }
    impl Finalize for A {
        fn finalize(&mut self) {
            let count = self.count.upgrade().unwrap();
            let mut c = count.borrow_mut();
            *c = *c - 1;
            println!("finalise");
        }
    }
    impl Drop for A {
        fn drop(&mut self) {
            println!("drop");
        }
    }
    {
        let a = gc_ctx.new_gc(A::new(None, None, &count));
        let b = gc_ctx.new_gc(A::new(Some(a.clone()), Some(a.clone()), &count));
        let c = gc_ctx.new_gc(A::new(Some(b.clone()), Some(b.clone()), &count));
        a.next1.set(Some(c.clone()));
        a.next2.set(Some(c.clone()));
        a.self_.set(Some(a.clone()));
        b.self_.set(Some(b.clone()));
        c.self_.set(Some(c.clone()));
        c.debug();
    }
    assert_eq!(0, *count.borrow());
}

#[test]
fn gc_weak() {
    let mut gc_ctx = GcCtx::new();
    let b;
    {
        let a = gc_ctx.new_gc(1);
        b = a.downgrade();
        assert!(b.upgrade().is_some());
    }
    assert!(b.upgrade().is_none());
}

#[test]
fn gc_deref() {
    let mut gc_ctx = GcCtx::new();
    let a = gc_ctx.new_gc(1);
    assert_eq!(*a, 1);
}

#[test]
fn gc_upcast() {
    struct Value {
        value: i32
    }
    impl Trace for Value {
        fn trace(&self, f: &mut FnMut(&GcDep)) {}
    }
    impl Finalize for Value {}
    trait Inc {
        fn inc(&mut self);
    }
    impl Inc for Value {
        fn inc(&mut self) {
            self.value = self.value + 1
        }
    }
    let mut gc_ctx = GcCtx::new();
    {
        let a =
            gc_ctx
                .new_gc(GcCell::new(Value { value: 3 }))
                .upcast(|x| x as &GcCell<Inc>);
        (*a).borrow_mut().inc();
        let b = a.clone();
        (*b).borrow_mut().inc();
    }
}
