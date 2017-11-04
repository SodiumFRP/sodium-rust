use sodium::gc::Gc;
use sodium::gc::GcDep;

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
