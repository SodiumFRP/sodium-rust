use sodium::impl_::gc::Gc;
use sodium::impl_::gc::GcDep;

pub struct Dep {
    pub gc_dep: GcDep
}

impl Dep {
    pub fn new<A: 'static + ?Sized>(a: Gc<A>) -> Dep {
        Dep {
            gc_dep: a.to_dep()
        }
    }
}

impl Clone for Dep {
    fn clone(&self) -> Self {
        Dep {
            gc_dep: self.gc_dep.clone()
        }
    }
}
