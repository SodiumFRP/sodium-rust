use crate::impl_::sodium_ctx::SodiumCtx;

pub struct Transaction {
    sodium_ctx: SodiumCtx,
    done: bool,
}

impl Transaction {
    pub fn new(sodium_ctx: &SodiumCtx) -> Transaction {
        sodium_ctx.enter_transaction();
        Transaction {
            sodium_ctx: sodium_ctx.clone(),
            done: false,
        }
    }

    // optional earily close
    pub fn close(&mut self) {
        if !self.done {
            self.sodium_ctx.leave_transaction();
            self.done = true;
        }
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.close();
    }
}
