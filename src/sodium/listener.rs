use sodium::SodiumCtx;
use std::rc::Rc;

pub struct Listener {
    pub id: u32,
    unlisten: Rc<Fn()>
}

impl Clone for Listener {
    fn clone(&self) -> Listener {
        Listener {
            id: self.id,
            unlisten: self.unlisten.clone()
        }
    }
}

impl Listener {
    pub fn new<F>(sodium_ctx: &mut SodiumCtx, unlisten: F) -> Listener where F: Fn() + 'static {
        Listener {
            id: sodium_ctx.new_id(),
            unlisten: Rc::new(unlisten)
        }
    }

    pub fn unlisten(&self) {
        (self.unlisten)()
    }

    pub fn append(&self, sodium_ctx: &mut SodiumCtx, other: &Listener) -> Listener {
        let l1 = self.clone();
        let l2 = other.clone();
        Listener::new(
            sodium_ctx,
            move || {
                l1.unlisten();
                l2.unlisten();
            }
        )
    }
}
