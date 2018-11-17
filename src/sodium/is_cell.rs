use sodium::Cell;
use sodium::CellLoop;
use sodium::CellSink;
use sodium::IsLambda1;
use sodium::IsLambda2;
use sodium::IsLambda3;
use sodium::IsLambda4;
use sodium::IsLambda5;
use sodium::IsLambda6;
use sodium::Listener;
use sodium::gc::Finalize;
use sodium::gc::Trace;

pub trait IsCell<A: Finalize + Trace + Clone + 'static>: Sized {
    fn to_cell(&self) -> Cell<A>;

    fn sample(&self) -> A {
        self.to_cell().sample()
    }

    fn map<B: Clone + Trace + Finalize + 'static,F:IsLambda1<A,B> + 'static>(
        &self,
        f: F
    ) -> Cell<B> {
        self.to_cell().map(f)
    }

    fn apply<B,F: IsLambda1<A,B> + Trace + Finalize + Clone + 'static,CF:IsCell<F>>(&self, cf: CF) -> Cell<B> where B: Trace + Finalize + Clone + 'static {
        self.to_cell().apply(cf)
    }

    fn lift2<B,C,CB:IsCell<B>,F: IsLambda2<A,B,C> + 'static>(&self, cb: CB, f: F) -> Cell<C> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static {
        self.to_cell().lift2(cb, f)
    }

    fn lift3<B,C,D,CB:IsCell<B>,CC:IsCell<C>,F: IsLambda3<A,B,C,D> + 'static>(&self, cb: CB, cc: CC, f: F) -> Cell<D> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static {
        self.to_cell().lift3(cb, cc, f)
    }

    fn lift4<B,C,D,E,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,F: IsLambda4<A,B,C,D,E> + 'static>(&self, cb: CB, cc: CC, cd: CD, f: F) -> Cell<E> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static, E: Clone + Trace + Finalize + 'static {
        self.to_cell().lift4(cb, cc, cd, f)
    }

    fn lift5<B,C,D,E,F,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,FN: IsLambda5<A,B,C,D,E,F> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, f: FN) -> Cell<F> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static, E: Clone + Trace + Finalize + 'static, F: Clone + Trace + Finalize + 'static {
        self.to_cell().lift5(cb, cc, cd, ce, f)
    }

    fn lift6<B,C,D,E,F,G,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,CF:IsCell<F>,FN: IsLambda6<A,B,C,D,E,F,G> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, cf: CF, f: FN) -> Cell<G> where B: Clone + Trace + Finalize + 'static, C: Clone + Trace + Finalize + 'static, D: Clone + Trace + Finalize + 'static, E: Clone + Trace + Finalize + 'static, F: Clone + Trace + Finalize + 'static, G: Clone + Trace + Finalize + 'static {
        self.to_cell().lift6(cb, cc, cd, ce, cf, f)
    }

    fn listen<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self.to_cell().listen(callback)
    }

    fn listen_weak<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self.to_cell().listen(callback)
    }
}

impl<A: Finalize + Trace + Clone + 'static> IsCell<A> for Cell<A> {
    fn to_cell(&self) -> Cell<A> {
        self.clone()
    }
}

impl<A: Finalize + Trace + Clone + 'static> IsCell<A> for CellLoop<A> {
    fn to_cell(&self) -> Cell<A> {
        Cell {
            impl_: self.impl_.to_cell()
        }
    }
}

impl<A: Finalize + Trace + Clone + 'static> IsCell<A> for CellSink<A> {
    fn to_cell(&self) -> Cell<A> {
        Cell {
            impl_: self.impl_.to_cell()
        }
    }
}

impl<'r, A: Finalize + Trace + Clone + 'static, CA: Clone + IsCell<A>> IsCell<A> for &'r CA {
    fn to_cell(&self) -> Cell<A> {
        (*self).to_cell()
    }
}
