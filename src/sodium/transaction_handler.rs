use sodium::Dep;
use sodium::SodiumCtx;
use sodium::Transaction;
use sodium::gc::Gc;
use sodium::gc::GcDep;
use sodium::gc::GcWeak;
use sodium::gc::Trace;
use std::any::Any;
use std::rc::Rc;
use std::rc::Weak;

pub struct WeakTransactionHandlerRef<A: ?Sized> {
    run: Weak<Fn(&mut SodiumCtx, &mut Transaction, &A)>,
    deps: Vec<GcDep>
}

impl<A:?Sized> Clone for WeakTransactionHandlerRef<A> {
    fn clone(&self) -> Self {
        WeakTransactionHandlerRef {
            run: self.run.clone(),
            deps: self.deps.clone()
        }
    }
}

impl<A:?Sized> WeakTransactionHandlerRef<A> {
    pub fn upgrade(&self) -> Option<TransactionHandlerRef<A>> {
        let deps = self.deps.clone();
        self.run.upgrade().map(|run| {
            TransactionHandlerRef {
                run: run,
                deps: deps
            }
        })
    }
}

pub struct TransactionHandlerRef<A: ?Sized> {
    run: Rc<Fn(&mut SodiumCtx, &mut Transaction, &A)>,
    deps: Vec<GcDep>
}

impl<A:?Sized> Clone for TransactionHandlerRef<A> {
    fn clone(&self) -> Self {
        TransactionHandlerRef {
            run: self.run.clone(),
            deps: self.deps.clone()
        }
    }
}

impl<A: ?Sized + 'static> TransactionHandlerRef<A> {
    pub fn new<F>(sodium_ctx: &mut SodiumCtx, f: F) -> TransactionHandlerRef<A>
    where
    F: Fn(&mut SodiumCtx, &mut Transaction, &A) + 'static
    {
        TransactionHandlerRef {
            run: Rc::new(f),
            deps: vec![]
        }
    }

    pub fn new_with_deps<F>(sodium_ctx: &mut SodiumCtx, f: F, deps: Vec<Dep>) -> TransactionHandlerRef<A>
    where
    F: Fn(&mut SodiumCtx, &mut Transaction, &A) + 'static
    {
        TransactionHandlerRef {
            run: Rc::new(f),
            deps: deps.iter().map(|dep| dep.gc_dep.clone()).collect()
        }
    }

    pub fn run(&self, sodium_ctx: &mut SodiumCtx, transaction: &mut Transaction, a: &A) {
        (self.run)(sodium_ctx, transaction, a)
    }

    pub fn downgrade(&self) -> WeakTransactionHandlerRef<A> {
        WeakTransactionHandlerRef {
            run: Rc::downgrade(&self.run),
            deps: self.deps.clone()
        }
    }
}
