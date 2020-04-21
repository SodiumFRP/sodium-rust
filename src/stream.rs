use crate::cell::Cell;
use crate::impl_::dep::Dep;
use crate::impl_::stream::Stream as StreamImpl;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::{lambda1, lambda2};
use crate::Lazy;
use crate::listener::Listener;
use crate::sodium_ctx::SodiumCtx;

pub struct Stream<A> {
    pub impl_: StreamImpl<A>
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            impl_: self.impl_.clone()
        }
    }
}

impl<A:Clone+Send+'static> Stream<Option<A>> {
    pub fn filter_option(&self) -> Stream<A> {
        self.filter(|a: &Option<A>| a.is_some()).map(|a: &Option<A>| a.clone().unwrap())
    }
}

impl<A:Clone+Send+'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream {
            impl_: StreamImpl::new(&sodium_ctx.impl_)
        }
    }

    // use as dependency to lambda1, lambda2, etc.
    pub fn to_dep(&self) -> Dep {
        self.impl_.to_dep()
    }

    pub fn snapshot<B:Clone+Send+'static,C:Clone+Send+'static,FN:IsLambda2<A,B,C>+Send+Sync+'static>(&self, cb: &Cell<B>, f: FN) -> Stream<C> {
        Stream { impl_: self.impl_.snapshot(&cb.impl_, f) }
    }

    pub fn snapshot1<B:Send+Clone+'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    pub fn snapshot3<B:Send+Clone+'static,C:Send+Clone+'static,D:Send+Clone+'static,FN:IsLambda3<A,B,C,D>+Send+Sync+'static>(&self, cb: &Cell<B>, cc: &Cell<C>, mut f: FN) -> Stream<D> {
        let deps: Vec<Dep>;
        if let Some(deps2) = f.deps_op() {
            deps = deps2.clone();
        } else {
            deps = Vec::new();
        }
        let cc = cc.clone();
        self.snapshot(cb, lambda2(move |a: &A, b: &B| f.call(a, b, &cc.sample()), deps))
    }

    pub fn map<B:Send+Clone+'static,FN:IsLambda1<A,B>+Send+Sync+'static>(&self, f: FN) -> Stream<B> {
        Stream { impl_: self.impl_.map(f) }
    }

    pub fn map_to<B:Send+Sync+Clone+'static>(&self, b: B) -> Stream<B> {
        self.map(move |_:&A| b.clone())
    }

    pub fn filter<PRED:IsLambda1<A,bool>+Send+Sync+'static>(&self, pred: PRED) -> Stream<A> {
        Stream { impl_: self.impl_.filter(pred) }
    }

    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> {
        self.merge(s2, |lhs:&A, _rhs:&A| lhs.clone())
    }

    pub fn merge<FN:IsLambda2<A,A,A>+Send+Sync+'static>(&self, s2: &Stream<A>, f: FN) -> Stream<A> {
        Stream { impl_: self.impl_.merge(&s2.impl_, f) }
    }

    pub fn hold(&self, a: A) -> Cell<A> {
        Cell { impl_: self.impl_.hold(a) }
    }

    pub fn hold_lazy(&self, a: Lazy<A>) -> Cell<A> {
        Cell { impl_: self.impl_.hold_lazy(a) }
    }

    pub fn gate(&self, cpred: &Cell<bool>) -> Stream<A> {
        let cpred = cpred.clone();
        let cpred_dep = cpred.to_dep();
        self.filter(lambda1(move |_: &A| cpred.sample(), vec![cpred_dep]))
    }

    pub fn once(&self) -> Stream<A> {
        Stream { impl_: self.impl_.once() }
    }

    pub fn collect<B,S,F>(&self, init_state: S, f: F) -> Stream<B>
        where B: Send + Clone + 'static,
              S: Send + Clone + 'static,
              F: IsLambda2<A,S,(B,S)> + Send + Sync + 'static
    {
        self.collect_lazy(Lazy::new(move || init_state.clone()), f)
    }

    pub fn collect_lazy<B,S,F>(&self, init_state: Lazy<S>, f: F) -> Stream<B>
        where B: Send + Clone + 'static,
              S: Send + Clone + 'static,
              F: IsLambda2<A,S,(B,S)> + Send + Sync + 'static
    {
        Stream { impl_: self.impl_.collect_lazy(init_state, f) }
    }

    pub fn accum<S,F>(&self, init_state: S, f: F) -> Cell<S>
        where S: Send + Clone + 'static,
              F: IsLambda2<A,S,S> + Send + Sync + 'static
    {
        self.accum_lazy(Lazy::new(move || init_state.clone()), f)
    }

    pub fn accum_lazy<S,F>(&self, init_state: Lazy<S>, f: F) -> Cell<S>
        where S: Send + Clone + 'static,
              F: IsLambda2<A,S,S> + Send + Sync + 'static
    {
        Cell { impl_: self.impl_.accum_lazy(init_state, f) }
    }

    pub fn listen_weak<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen_weak(k) }
    }

    pub fn listen<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener {
        Listener { impl_: self.impl_.listen(k) }
    }
}
