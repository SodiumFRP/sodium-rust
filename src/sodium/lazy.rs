use std::rc::Rc;

pub struct Lazy<A> {
    f: Rc<Fn()->A>
}

impl<A> Clone for Lazy<A> {
    fn clone(&self) -> Self {
        Lazy {
            f: self.f.clone()
        }
    }
}

impl<A:'static> Lazy<A> {
    pub fn new<F>(f: F) -> Lazy<A> where F: Fn()->A + 'static {
        Lazy {
            f: Rc::new(f)
        }
    }

    pub fn get(&self) -> A {
        (&self.f)()
    }

    pub fn map<F,B:'static>(&self, f: F) -> Lazy<B> where F: Fn(&A)->B + 'static {
        let self_ = self.clone();
        Lazy::new(move || f(&self_.get()))
    }

    pub fn lift2<F,B:'static,C:'static>(&self, b: &Lazy<B>, f: F) -> Lazy<C> where F: Fn(&A,&B)->C + 'static {
        let a = self.clone();
        let b = b.clone();
        Lazy::new(move || {
            f(&a.get(), &b.get())
        })
    }

    pub fn lift3<F,B:'static,C:'static,D:'static>(&self, b: &Lazy<B>, c: &Lazy<C>, f: F) -> Lazy<D> where F: Fn(&A,&B,&C)->D + 'static {
        let a = self.clone();
        let b = b.clone();
        let c = c.clone();
        Lazy::new(move || {
            f(&a.get(), &b.get(), &c.get())
        })
    }

    pub fn lift4<F,B:'static,C:'static,D:'static,E:'static>(&self, b: &Lazy<B>, c: &Lazy<C>, d: &Lazy<D>, f: F) -> Lazy<E> where F: Fn(&A,&B,&C,&D)->E + 'static {
        let a = self.clone();
        let b = b.clone();
        let c = c.clone();
        let d = d.clone();
        Lazy::new(move || {
            f(&a.get(), &b.get(), &c.get(), &d.get())
        })
    }
}
