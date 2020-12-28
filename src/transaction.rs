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
    /// Create a new scoped transaction on the given context.
    pub fn new(sodium_ctx: &SodiumCtx) -> Transaction {
        Transaction {
            impl_: TransactionImpl::new(&sodium_ctx.impl_),
        }
    }

    /// Explicitly close this transaction.
    ///
    /// This transaction will close automatically when it goes out of
    /// scope and is dropped. This `close` method is an optional way
    /// to explicitly close the transaction before that.
    pub fn close(&self) {
        self.impl_.close();
    }
}
