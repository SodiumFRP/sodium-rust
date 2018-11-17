use sodium::Cell;
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
use sodium::MemoLazy;
use sodium::gc::Finalize;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use sodium::impl_;

pub struct Stream<A> {
    pub impl_: impl_::Stream<A>
}

impl<A: Clone + Trace + Finalize + 'static> Stream<Option<A>> {
    pub fn filter_option(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.filter_option()
        }
    }
}

impl<A: Clone + Trace + Finalize + 'static> Stream<A> {

    pub fn to_dep(&self) -> Dep {
        self.impl_.to_dep()
    }

    pub fn map<B: Clone + Trace + Finalize + 'static,F:IsLambda1<A,B> + 'static>(
        &self,
        f: F
    ) -> Stream<B> {
        Stream {
            impl_: self.impl_.map(f)
        }
    }

    pub fn hold(&self, a: A) -> Cell<A> {
        Cell {
            impl_: self.impl_.hold(a)
        }
    }

    pub fn filter<PRED:IsLambda1<A,bool> + 'static>(&self, pred: PRED) -> Stream<A> {
        Stream {
            impl_: self.impl_.filter(pred)
        }
    }

    pub fn merge<SA:IsStream<A>, FN:Fn(&A,&A)->A+'static>(&self, sa: SA, f: FN) -> Stream<A> {
        Stream {
            impl_: self.impl_.merge(sa.to_stream().impl_, f)
        }
    }

    pub fn gate<CA:IsCell<bool>>(&self, ca: CA) -> Stream<A> {
        Stream {
            impl_: self.impl_.gate(ca.to_cell().impl_)
        }
    }

    pub fn collect_lazy<B,S,F>(&self, init_state: MemoLazy<S>, f: F) -> Stream<B>
        where B: Clone + Trace + Finalize + 'static,
              S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,(B,S)> + 'static
    {
        Stream {
            impl_: self.impl_.collect_lazy(init_state, f)
        }
    }

    pub fn accum_lazy<S,F>(&self, init_state: MemoLazy<S>, f: F) -> Cell<S>
        where S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,S> + 'static
    {
        Cell {
            impl_: self.impl_.accum_lazy(init_state, f)
        }
    }

    pub fn once(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.once()
        }
    }

    pub fn snapshot<B,CB:IsCell<B>>(&self, cb: CB) -> Stream<B> where B: Trace + Finalize + Clone + 'static {
        Stream {
            impl_: self.impl_.snapshot(cb.to_cell().impl_)
        }
    }

    pub fn snapshot2<B,C,CB:IsCell<B>,FN:IsLambda2<A,B,C> + 'static>(&self, cb: CB, f: FN) -> Stream<C> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static {
        Stream {
            impl_: self.impl_.snapshot2(cb.to_cell().impl_, f)
        }
    }

    pub fn snapshot3<B,C,D,CB:IsCell<B>,CC:IsCell<C>,FN:IsLambda3<A,B,C,D> + 'static>(&self, cb: CB, cc: CC, f: FN) -> Stream<D> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static {
        Stream {
            impl_: self.impl_.snapshot3(cb.to_cell().impl_, cc.to_cell().impl_, f)
        }
    }

    pub fn snapshot4<B,C,D,E,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,FN:IsLambda4<A,B,C,D,E> + 'static>(&self, cb: CB, cc: CC, cd: CD, f: FN) -> Stream<E> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static {
        Stream {
            impl_: self.impl_.snapshot4(cb.to_cell().impl_, cc.to_cell().impl_, cd.to_cell().impl_, f)
        }
    }

    pub fn snapshot5<B,C,D,E,F,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,FN:IsLambda5<A,B,C,D,E,F> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, f: FN) -> Stream<F> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, F: Trace + Finalize + Clone + 'static {
        Stream {
            impl_: self.impl_.snapshot5(cb.to_cell().impl_, cc.to_cell().impl_, cd.to_cell().impl_, ce.to_cell().impl_, f)
        }
    }

    pub fn snapshot6<B,C,D,E,F,G,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,CF:IsCell<F>,FN:IsLambda6<A,B,C,D,E,F,G> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, cf: CF, f: FN) -> Stream<G> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, F: Trace + Finalize + Clone + 'static, G: Trace + Finalize + Clone + 'static {
        Stream {
            impl_: self.impl_.snapshot6(cb.to_cell().impl_, cc.to_cell().impl_, cd.to_cell().impl_, ce.to_cell().impl_, cf.to_cell().impl_, f)
        }
    }

    pub fn listen<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self.impl_.listen(callback)
    }
}

impl<A: Clone + Trace + Finalize + 'static> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            impl_: self.impl_.clone()
        }
    }
}

impl<A: Clone + Trace + Finalize + 'static> Finalize for Stream<A> {
    fn finalize(&mut self) {}
}

impl<A: Clone + Trace + Finalize + 'static> Trace for Stream<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.impl_.trace(f);
    }
}
