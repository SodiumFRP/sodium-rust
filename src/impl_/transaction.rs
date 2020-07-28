use crate::impl_::sodium_ctx::SodiumCtx;

pub struct Transaction {
    sodium_ctx: SodiumCtx,
    done: std::cell::Cell<bool>,
}

impl Transaction {
    pub fn new(sodium_ctx: &SodiumCtx) -> Transaction {
        sodium_ctx.enter_transaction();
        Transaction {
            sodium_ctx: sodium_ctx.clone(),
            done: std::cell::Cell::new(false),
        }
    }

    // optional earily close
    pub fn close(&self) {
        if !self.done.get() {
            self.sodium_ctx.leave_transaction();
            self.done.set(true);
        }
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.close();
    }
}
