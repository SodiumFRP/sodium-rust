use sodium::Dep;
use sodium::SodiumCtx;
use sodium::Transaction;
use sodium::gc::Gc;
use sodium::gc::GcDep;
use sodium::gc::GcWeak;
use std::any::Any;
use std::rc::Rc;
use std::rc::Weak;

pub struct WeakTransactionHandlerRef<A: ?Sized> {
    run: GcWeak<Fn(&mut SodiumCtx, &mut Transaction, &A)>
}

impl<A:?Sized> Clone for WeakTransactionHandlerRef<A> {
    fn clone(&self) -> Self {
        WeakTransactionHandlerRef {
            run: self.run.clone()
        }
    }
}

impl<A:?Sized> WeakTransactionHandlerRef<A> {
    pub fn upgrade(&self) -> Option<TransactionHandlerRef<A>> {
        self.run.upgrade().map(|run| {
            TransactionHandlerRef {
                run: run
            }
        })
    }
}

pub struct TransactionHandlerRef<A: ?Sized> {
    run: Gc<Fn(&mut SodiumCtx, &mut Transaction, &A)>
}

impl<A:?Sized> Clone for TransactionHandlerRef<A> {
    fn clone(&self) -> Self {
        TransactionHandlerRef {
            run: self.run.clone()
        }
    }
}

impl<A: ?Sized + 'static> TransactionHandlerRef<A> {
    pub fn new<F>(sodium_ctx: &mut SodiumCtx, f: F) -> TransactionHandlerRef<A>
    where
    F: Fn(&mut SodiumCtx, &mut Transaction, &A) + 'static
    {
        TransactionHandlerRef {
            run: sodium_ctx.new_gc(f).upcast(|a| a as &(Fn(&mut SodiumCtx, &mut Transaction, &A) + 'static))
        }
    }

    pub fn new_with_deps<F>(sodium_ctx: &mut SodiumCtx, f: F, deps: Vec<Dep>) -> TransactionHandlerRef<A>
    where
    F: Fn(&mut SodiumCtx, &mut Transaction, &A) + 'static
    {
        let t = TransactionHandlerRef::new(sodium_ctx, f);
        t.set_deps(deps);
        t
    }

    pub fn to_dep(&self) -> Dep {
        Dep::new(self.run.clone())
    }

    pub fn set_deps(&self, deps: Vec<Dep>) {
        self.run.set_deps(deps.into_iter().map(|dep| dep.gc_dep).collect());
    }

    pub fn run(&self, sodium_ctx: &mut SodiumCtx, transaction: &mut Transaction, a: &A) {
        (self.run)(sodium_ctx, transaction, a)
    }

    pub fn downgrade(&self) -> WeakTransactionHandlerRef<A> {
        WeakTransactionHandlerRef {
            run: Gc::downgrade(&self.run)
        }
    }
}
