use sodium::gc::Gc;
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
    impl Drop for A {
        fn drop(&mut self) {
            let count = self.count.upgrade().unwrap();
            let mut c = count.borrow_mut();
            *c = *c - 1;
        }
    }
    {
        let a = gc_ctx.new_gc(A::new(None, &count));
        let b = gc_ctx.new_gc(A::new(Some(a.clone()), &count));
        b.set_deps(vec![a.to_dep()]);
        let c = gc_ctx.new_gc(A::new(Some(b.clone()), &count));
        c.set_deps(vec![b.to_dep()]);
        a.x.set(Some(c.clone()));
        a.set_deps(vec![c.to_dep()]);
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
                .new_gc(RefCell::new(Value { value: 3 }))
                .upcast(|x| x as &RefCell<Inc>);
        (*a).borrow_mut().inc();
        let b = a.clone();
        (*b).borrow_mut().inc();
    }
}

#[test]
fn gc_upcast2() {
    let mut gc_ctx = GcCtx::new();
    let a = gc_ctx.new_gc(5);
    let a2 = a.clone();
    let b = gc_ctx.new_gc(move |x| x + *a).upcast(|x| x as &Fn(i32)->i32);
    b.add_deps(vec![a2.to_dep()]);
    b(4);
    b(3);
}
