use sodium::Dep;
use sodium::SodiumCtx;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use std::rc::Rc;

pub struct Listener {
    pub id: u32,
    unlisten: Rc<Fn()>,
    deps: Vec<Dep>
}

impl Clone for Listener {
    fn clone(&self) -> Listener {
        Listener {
            id: self.id,
            unlisten: self.unlisten.clone(),
            deps: self.deps.clone()
        }
    }
}

impl Trace for Listener {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        for dep in &self.deps {
            f(&dep.gc_dep)
        }
    }
}

impl Listener {
    pub fn new<F>(sodium_ctx: &mut SodiumCtx, unlisten: F) -> Listener where F: Fn() + 'static {
        Listener {
            id: sodium_ctx.new_id(),
            unlisten: Rc::new(unlisten) as Rc<Fn()>,
            deps: vec![]
        }
    }

    pub fn new_with_deps<F>(sodium_ctx: &mut SodiumCtx, unlisten: F, deps: Vec<Dep>) -> Listener where F: Fn() + 'static {
        Listener {
            id: sodium_ctx.new_id(),
            unlisten: Rc::new(unlisten) as Rc<Fn()>,
            deps: deps
        }
    }

    pub fn unlisten(&self) {
        (self.unlisten)()
    }

    pub fn append(&self, sodium_ctx: &mut SodiumCtx, other: &Listener) -> Listener {
        let l1 = self.clone();
        let l2 = other.clone();
        let l12 = l1.clone();
        let l22 = l2.clone();
        let mut deps = Vec::new();
        for dep in &self.deps {
            deps.push(dep.clone());
        }
        for dep in &other.deps {
            deps.push(dep.clone());
        }
        let l = Listener::new_with_deps(
            sodium_ctx,
            move || {
                l1.unlisten();
                l2.unlisten();
            },
            deps
        );
        l
    }
}
