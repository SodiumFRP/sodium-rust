use sodium::SodiumCtx;
use sodium::Transaction;
use std::any::Any;
use std::rc::Rc;
use std::rc::Weak;

pub struct WeakTransactionHandlerRef<A: ?Sized> {
    run: Weak<Fn(&mut SodiumCtx, &mut Transaction, &A)>
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
    run: Rc<Fn(&mut SodiumCtx, &mut Transaction, &A)>
}

impl<A:?Sized> Clone for TransactionHandlerRef<A> {
    fn clone(&self) -> Self {
        TransactionHandlerRef {
            run: self.run.clone()
        }
    }
}

impl<A: ?Sized> TransactionHandlerRef<A> {
    pub fn new<F>(f: F) -> TransactionHandlerRef<A>
    where
    F: Fn(&mut SodiumCtx, &mut Transaction, &A) + 'static
    {
        TransactionHandlerRef {
            run: Rc::new(f)
        }
    }

    pub fn into_any(self) -> TransactionHandlerRef<Any>
    where A: Any + Sized
    {
        let self_ = self;
        TransactionHandlerRef::new(
            move |sodium_ctx: &mut SodiumCtx, transaction: &mut Transaction, a: &Any| {
                match a.downcast_ref::<A>() {
                    Some(a2) => self_.run(sodium_ctx, transaction, a2),
                    None => ()
                }
            }
        )
    }

    pub fn run(&self, sodium_ctx: &mut SodiumCtx, transaction: &mut Transaction, a: &A) {
        (self.run)(sodium_ctx, transaction, a)
    }

    pub fn downgrade(&self) -> WeakTransactionHandlerRef<A> {
        WeakTransactionHandlerRef {
            run: Rc::downgrade(&self.run)
        }
    }
}
