use sodium::Dep;
use sodium::SodiumCtx;
use sodium::gc::Gc;

pub struct Listener {
    pub id: u32,
    unlisten: Gc<Fn()>
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
            unlisten: sodium_ctx.new_gc(unlisten).upcast(|a| a as &(Fn() + 'static))
        }
    }

    pub fn to_dep(&self) -> Dep {
        Dep::new(self.unlisten.clone())
    }

    pub fn set_deps(&self, deps: Vec<Dep>) {
        self.unlisten.set_deps(deps.into_iter().map(|dep| dep.gc_dep).collect());
    }

    pub fn unlisten(&self) {
        (self.unlisten)()
    }

    pub fn append(&self, sodium_ctx: &mut SodiumCtx, other: &Listener) -> Listener {
        let l1 = self.clone();
        let l2 = other.clone();
        let l12 = l1.clone();
        let l22 = l2.clone();
        let l = Listener::new(
            sodium_ctx,
            move || {
                l1.unlisten();
                l2.unlisten();
            }
        );
        l.unlisten.set_deps(vec![l12.unlisten.to_dep(), l22.unlisten.to_dep()]);
        l
    }
}
