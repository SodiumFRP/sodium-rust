use crate::impl_::transaction::Transaction as TransactionImpl;
use crate::SodiumCtx;

/// A scoped transaction marker.
///
/// An alternative to [`SodiumCtx::transaction`] that creates a struct
/// that will create a new transaction in the given [`SodiumCtx`] and
/// hold it open until the `Transaction` is dropped.
pub struct Transaction {
    impl_: TransactionImpl,
}

impl Transaction {
    pub fn new(sodium_ctx: &SodiumCtx) -> Transaction {
        Transaction {
            impl_: TransactionImpl::new(&sodium_ctx.impl_),
        }
    }

    // optional earily close
    pub fn close(&self) {
        self.impl_.close();
    }
}
