use sodium::Cell;
use sodium::IsCell;
use sodium::IsLambdaMut0;
use sodium::IsLambda1;
use sodium::IsLambda2;
use sodium::IsLambda3;
use sodium::IsLambda4;
use sodium::IsLambda5;
use sodium::IsLambda6;
use sodium::Listener;
use sodium::MemoLazy;
use sodium::Stream;
use sodium::StreamLoop;
use sodium::StreamSink;
use sodium::gc::Finalize;
use sodium::gc::Trace;

pub trait IsStream<A: Finalize + Trace + Clone + 'static> {
    fn to_stream(&self) -> Stream<A>;

    fn map<B: Clone + Trace + Finalize + 'static,F:IsLambda1<A,B> + 'static>(
        &self,
        f: F
    ) -> Stream<B> {
        self.to_stream().map(f)
    }

    fn map_to<B: Clone + Trace + Finalize + 'static>(&self, b: &B) -> Stream<B> {
        let b = b.clone();
        self.map(move |_a: &A| b.clone())
    }

    fn hold(&self, a: A) -> Cell<A> {
        self.to_stream().hold(a)
    }

    fn filter<PRED:IsLambda1<A,bool> + 'static>(&self, pred: PRED) -> Stream<A> {
        self.to_stream().filter(pred)
    }

    fn merge<SA: IsStream<A>, FN: Fn(&A,&A)->A+'static>(&self, sa: SA, f: FN) -> Stream<A> {
        self.to_stream().merge(sa, f)
    }

    fn gate<CA: IsCell<bool>>(&self, ca: CA) -> Stream<A> {
        self.to_stream().gate(ca)
    }

    fn collect<B,S,F>(&self, init_state: S, f: F) -> Stream<B>
        where B: Clone + Trace + Finalize + 'static,
              S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,(B,S)> + 'static
    {
        let sodium_ctx = self.to_stream().impl_._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        self.collect_lazy(sodium_ctx.new_lazy(move || init_state.clone()), f)
    }

    fn collect_lazy<B,S,F>(&self, init_state: MemoLazy<S>, f: F) -> Stream<B>
        where B: Clone + Trace + Finalize + 'static,
              S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,(B,S)> + 'static
    {
        self.to_stream().collect_lazy(init_state, f)
    }

    fn accum<S,F>(&self, init_state: S, f: F) -> Cell<S>
        where S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,S> + 'static
    {
        let sodium_ctx = self.to_stream().impl_._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        self.accum_lazy(sodium_ctx.new_lazy(move || init_state.clone()), f)
    }

    fn accum_lazy<S,F>(&self, init_state: MemoLazy<S>, f: F) -> Cell<S>
        where S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,S> + 'static
    {
        self.to_stream().accum_lazy(init_state, f)
    }

    fn once(&self) -> Stream<A> {
        self.to_stream().once()
    }

    fn or_else<SA: IsStream<A>>(&self, sa: SA) -> Stream<A> {
        self.merge(sa, |l, _r| l.clone())
    }

    fn snapshot<B,CB:IsCell<B>>(&self, cb: CB) -> Stream<B> where B: Trace + Finalize + Clone + 'static {
        self.to_stream().snapshot(cb)
    }

    fn snapshot2<B,C,CB:IsCell<B>,FN:IsLambda2<A,B,C> + 'static>(&self, cb: CB, f: FN) -> Stream<C> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static {
        self.to_stream().snapshot2(cb, f)
    }

    fn snapshot3<B,C,D,CB:IsCell<B>,CC:IsCell<C>,FN:IsLambda3<A,B,C,D> + 'static>(&self, cb: CB, cc: CC, f: FN) -> Stream<D> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static {
        self.to_stream().snapshot3(cb, cc, f)
    }

    fn snapshot4<B,C,D,E,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,FN:IsLambda4<A,B,C,D,E> + 'static>(&self, cb: CB, cc: CC, cd: CD, f: FN) -> Stream<E> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static {
        self.to_stream().snapshot4(cb, cc, cd, f)
    }

    fn snapshot5<B,C,D,E,F,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,FN:IsLambda5<A,B,C,D,E,F> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, f: FN) -> Stream<F> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, F: Trace + Finalize + Clone + 'static {
        self.to_stream().snapshot5(cb, cc, cd, ce, f)
    }

    fn snapshot6<B,C,D,E,F,G,CB:IsCell<B>,CC:IsCell<C>,CD:IsCell<D>,CE:IsCell<E>,CF:IsCell<F>,FN:IsLambda6<A,B,C,D,E,F,G> + 'static>(&self, cb: CB, cc: CC, cd: CD, ce: CE, cf: CF, f: FN) -> Stream<G> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, F: Trace + Finalize + Clone + 'static, G: Trace + Finalize + Clone + 'static {
        self.to_stream().snapshot6(cb, cc, cd, ce, cf, f)
    }

    fn add_cleanup<CLEANUP:IsLambdaMut0<()>+'static>(&self, cleanup: CLEANUP) {
        self.to_stream().add_cleanup(cleanup);
    }

    fn listen<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self.to_stream().listen(callback)
    }

    fn listen_weak<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self.to_stream().listen_weak(callback)
    }
}

impl<A: Finalize + Trace + Clone + 'static> IsStream<A> for Stream<A> {
    fn to_stream(&self) -> Stream<A> {
        self.clone()
    }
}

impl<A: Finalize + Trace + Clone + 'static> IsStream<A> for StreamLoop<A> {
    fn to_stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.to_stream()
        }
    }
}

impl<A: Finalize + Trace + Clone + 'static> IsStream<A> for StreamSink<A> {
    fn to_stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.to_stream()
        }
    }
}

impl<'r, A: Finalize + Trace + Clone + 'static, SA: Clone + IsStream<A>> IsStream<A> for &'r SA {
    fn to_stream(&self) -> Stream<A> {
        (*self).to_stream()
    }
}

pub trait IsStreamOption<A: Finalize + Trace + Clone + 'static> {
    fn to_stream_option(&self) -> Stream<Option<A>>;

    fn filter_option(&self) -> Stream<A> {
        self.to_stream_option().filter_option()
    }
}

impl<A: Finalize + Trace + Clone + 'static, SOA: IsStream<Option<A>> + Clone> IsStreamOption<A> for SOA {
    fn to_stream_option(&self) -> Stream<Option<A>> {
        self.clone().to_stream()
    }
}
