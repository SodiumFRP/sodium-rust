use crate::impl_::transaction::Transaction as TransactionImpl;
use crate::SodiumCtx;

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
