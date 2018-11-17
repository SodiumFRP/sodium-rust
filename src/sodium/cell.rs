use sodium::Dep;
use sodium::IsCell;
use sodium::IsStream;
use sodium::IsLambda1;
use sodium::IsLambda2;
use sodium::IsLambda3;
use sodium::IsLambda4;
use sodium::IsLambda5;
use sodium::IsLambda6;
use sodium::Listener;
use sodium::Stream;
use sodium::gc::Finalize;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use sodium::impl_;

pub struct Cell<A> {
    pub impl_: impl_::Cell<A>
}

impl<A: Clone + Trace + Finalize + 'static> Cell<A> {

    pub fn to_dep(&self) -> Dep {
        self.impl_.to_dep()
    }

    pub fn sample(&self) -> A {
        self.impl_.sample()
    }

    pub fn map<B: Clone + Trace + Finalize + 'static,F:IsLambda1<A,B> + 'static>(
        &self,
        f: F
    ) -> Cell<B> {
        Cell {
            impl_: self.impl_.map(f)
        }
    }

    pub fn apply<B,F: IsLambda1<A,B> + Trace + Finalize + Clone + 'static,CF:IsCell<F>>(&self, cf: CF) -> Cell<B> where B: Trace + Finalize + Clone + 'static {
        Cell {
            impl_: self.impl_.apply(cf.to_cell().impl_)
        }
    }

    pub fn lift2<B,C,CB:IsCell<B>,F: IsLambda2<A,B,C> + 'static>(&self, cb: CB, f: F) -> Cell<C> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static {
        Cell {
            impl_: self.impl_.lift2(cb.to_cell().impl_, f)
        }
    }

    pub fn lift3<B,C,D,CB:IsCell<B>,CC:IsCell<C>,F: IsLambda3<A,B,C,D> + 'static>(&self, cb: CB, cc: CC, f: F) -> Cell<D> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static {
        Cell {
            impl_: self.impl_.lift3(cb.to_cell().impl_, cc.to_cell().impl_, f)
        }
    }

    pub fn lift4<B,C,D,E,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,F: IsLambda4<A,B,C,D,E> + 'static>(&self, cb: CB, cc: CC, cd: CD, f: F) -> Cell<E> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static, E: Clone + Trace + Finalize + 'static {
        Cell {
            impl_: self.impl_.lift4(cb.to_cell().impl_, cc.to_cell().impl_, cd.to_cell().impl_, f)
        }
    }

    pub fn lift5<B,C,D,E,F,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,FN: IsLambda5<A,B,C,D,E,F> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, f: FN) -> Cell<F> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static, E: Clone + Trace + Finalize + 'static, F: Clone + Trace + Finalize + 'static {
        Cell {
            impl_: self.impl_.lift5(cb.to_cell().impl_, cc.to_cell().impl_, cd.to_cell().impl_, ce.to_cell().impl_, f)
        }
    }

    pub fn lift6<B,C,D,E,F,G,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,CF:IsCell<F>,FN: IsLambda6<A,B,C,D,E,F,G> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, cf: CF, f: FN) -> Cell<G> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static, E: Clone + Trace + Finalize + 'static, F: Clone + Trace + Finalize + 'static, G: Clone + Trace + Finalize + 'static {
        Cell {
            impl_: self.impl_.lift6(cb.to_cell().impl_, cc.to_cell().impl_, cd.to_cell().impl_, ce.to_cell().impl_, cf.to_cell().impl_, f)
        }
    }

    pub fn switch_s<SA:IsStream<A> + Trace + Finalize + Clone + 'static,CSA:IsCell<SA>>(csa: CSA) -> Stream<A> {
        Stream {
            impl_: impl_::Cell::switch_s(csa.to_cell().impl_.map(|sa:&SA| sa.to_stream().impl_))
        }
    }

    pub fn switch_c<CA:IsCell<A> + Trace + Finalize + Clone + 'static,CCA:IsCell<CA>>(cca: CCA) -> Cell<A> {
        Cell {
            impl_: impl_::Cell::switch_c(cca.to_cell().impl_.map(|ca:&CA| ca.to_cell().impl_))
        }
    }

    pub fn listen<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self.impl_.listen(callback)
    }
}

impl<A: Clone + Trace + Finalize + 'static> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            impl_: self.impl_.clone()
        }
    }
}

impl<A: Trace> Trace for Cell<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.impl_.trace(f);
    }
}

impl<A: Finalize> Finalize for Cell<A> {
    fn finalize(&mut self) {
        self.impl_.finalize();
    }
}
