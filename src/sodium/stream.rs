use sodium::Cell;
use sodium::cell;
use sodium::cell::CellImpl;
use sodium::CoalesceHandler;
use sodium::Dep;
use sodium::HandlerRefMut;
use sodium::cell::IsCellPrivate;
use sodium::IsCell;
use sodium::IsLambda1;
use sodium::IsLambda2;
use sodium::IsLambda3;
use sodium::IsLambda4;
use sodium::IsLambda5;
use sodium::IsLambda6;
use sodium::Lambda;
use sodium::Lazy;
use sodium::LazyCell;
use sodium::Listener;
use sodium::Node;
use sodium::HasNode;
use sodium::SodiumCtx;
use sodium::StreamLoop;
use sodium::StreamWithSend;
use sodium::Target;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use sodium::gc::Gc;
use sodium::gc::GcWeak;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Stream<A: Clone + 'static> {
    sodium_ctx: SodiumCtx,
    stream_impl: StreamImpl<A>
}

impl<A: Clone + 'static> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            sodium_ctx: self.sodium_ctx.clone(),
            stream_impl: self.stream_impl.clone()
        }
    }
}

pub trait HasStream<A: Clone + 'static> {
    fn stream(&self) -> Stream<A>;
}

impl<A: Clone + 'static> HasStream<A> for Stream<A> {
    fn stream(&self) -> Stream<A> {
        self.clone()
    }
}

pub fn make_stream<A: Clone + 'static>(sodium_ctx: SodiumCtx, stream_impl: StreamImpl<A>) -> Stream<A> {
    Stream {
        sodium_ctx,
        stream_impl
    }
}

pub fn get_sodium_ctx<A: Clone + 'static, SA: HasStream<A>>(sa: &SA) -> SodiumCtx {
    sa.stream().sodium_ctx.clone()
}

pub fn get_stream_impl<A: Clone + 'static, SA: HasStream<A>>(sa: &SA) -> StreamImpl<A> {
    sa.stream().stream_impl.clone()
}

pub trait IsStream<A: Clone + 'static> {
    fn stream(&self) -> Stream<A>;
    fn to_dep(&self) -> Dep;
    fn listen<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static;
    fn listen_once<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static;
    fn listen_weak<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static;
    fn map<B,F>(&self, f: F) -> Stream<B> where B: Clone + 'static, F: IsLambda1<A,B> + 'static;
    fn map_to<B>(&self, b: B) -> Stream<B> where B: Clone + 'static {
        self.map(move |a: &A| b.clone())
    }
    fn hold(&self, a: A) -> Cell<A> {
        self.hold_lazy(Lazy::new(move || a.clone()))
    }
    fn hold_lazy(&self, a: Lazy<A>) -> Cell<A>;
    fn snapshot_to<B,CB>(&self, cb: &CB) -> Stream<B> where B: Clone + 'static, CB: IsCell<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }
    fn snapshot<B,CB,C,F>(&self, cb: &CB, f: F) -> Stream<C> where B: Clone + 'static, CB: IsCell<B>, C: Clone + 'static, F: IsLambda2<A,B,C> + 'static;
    fn snapshot2<B,C,CB,CC,D,F>(&self, cb: &CB, cc: &CC, f: F) -> Stream<D> where B: Clone + 'static, C: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, D: Clone + 'static, F: IsLambda3<A,B,C,D> + 'static;
    fn snapshot3<B,C,D,CB,CC,CD,E,F>(&self, cb: &CB, cc: &CC, cd: &CD, f: F) -> Stream<E> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, E: Clone + 'static, F: IsLambda4<A,B,C,D,E> + 'static;
    fn snapshot4<B,C,D,E,CB,CC,CD,CE,F,FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> Stream<F> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, F: Clone + 'static, FN: IsLambda5<A,B,C,D,E,F> + 'static;
    fn snapshot5<B,C,D,E,F,CB,CC,CD,CE,CF,G,FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, f: FN) -> Stream<G> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, F: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, CF: IsCell<F>, G: Clone + 'static, FN: IsLambda6<A,B,C,D,E,F,G> + 'static;
    fn or_else<SA>(&self, sa: &SA) -> Stream<A> where SA: IsStream<A> {
        self.merge(sa, |l: &A, _r: &A| l.clone())
    }
    fn merge<SA,F>(&self, sa: &SA, f: F) -> Stream<A> where SA: IsStream<A>, F: IsLambda2<A,A,A> + 'static;
    fn filter<F>(&self, f: F) -> Stream<A> where F: IsLambda1<A,bool> + 'static;
    fn gate<CB>(&self, c: &CB) -> Stream<A> where CB: IsCell<bool> {
        self
            .snapshot(
                c,
                |a: &A, pred: &bool| {
                    if *pred {
                        Some(a.clone())
                    } else {
                        None
                    }
                }
            )
            .filter_option()
    }

    fn collect<B,S,F>(&self, init_state: S, f: F) -> Stream<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: Fn(&A,&S)->(B,S) + 'static
    {
        self.collect_lazy(Lazy::new(move || init_state.clone()), f)
    }

    fn collect_lazy<B,S,F>(&self, init_state: Lazy<S>, f: F) -> Stream<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: Fn(&A,&S)->(B,S) + 'static;

    fn accum<S,F>(&self, init_state: S, f: F) -> Cell<S>
        where S: Clone + 'static,
              F: Fn(&A,&S)->S + 'static
    {
        self.accum_lazy(Lazy::new(move || init_state.clone()), f)
    }

    fn accum_lazy<S,F>(&self, init_state: Lazy<S>, f: F) -> Cell<S>
        where S: Clone + 'static,
              F: Fn(&A,&S)->S + 'static;

    fn once(&self) -> Stream<A>;
}

pub trait IsStreamOption<A: Clone + 'static> {
    fn filter_option(&self) -> Stream<A>;
}

impl<A: Clone + 'static, S: HasStream<A>> IsStream<A> for S {
    fn stream(&self) -> Stream<A> {
        HasStream::stream(self)
    }

    fn to_dep(&self) -> Dep {
        get_stream_impl(self).to_dep()
    }

    fn listen<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static {
        get_stream_impl(self).listen(
            &mut get_sodium_ctx(self),
            f
        )
    }

    fn listen_once<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static {
        get_stream_impl(self).listen_once(
            &mut get_sodium_ctx(self),
            f
        )
    }

    fn listen_weak<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static {
        get_stream_impl(self).listen_once(
            &mut get_sodium_ctx(self),
            f
        )
    }

    fn map<B, F>(&self, f: F) -> Stream<B> where B: Clone + 'static, F: IsLambda1<A, B> + 'static {
        Stream {
            sodium_ctx: get_sodium_ctx(self),
            stream_impl: get_stream_impl(self).map(
                &mut get_sodium_ctx(self),
                f
            )
        }
    }

    fn hold_lazy(&self, a: Lazy<A>) -> Cell<A> {
        cell::make_cell(
            self.stream().sodium_ctx.clone(),
            self.stream().stream_impl.hold_lazy(
                &mut self.stream().sodium_ctx.clone(),
                a
            )
        )
    }

    fn snapshot<B, CB, C, F>(&self, cb: &CB, f: F) -> Stream<C> where B: Clone + 'static, CB: IsCell<B>, C: Clone + 'static, F: IsLambda2<A, B, C> + 'static {
        Stream {
            sodium_ctx: self.stream().sodium_ctx.clone(),
            stream_impl: self.stream().stream_impl.snapshot(
                &mut self.stream().sodium_ctx.clone(),
                &cell::get_cell_impl(&cb.cell()),
                f
            )
        }
    }

    fn snapshot2<B, C, CB, CC, D, F>(&self, cb: &CB, cc: &CC, f: F) -> Stream<D> where B: Clone + 'static, C: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, D: Clone + 'static, F: IsLambda3<A, B, C, D> + 'static {
        Stream {
            sodium_ctx: self.stream().sodium_ctx.clone(),
            stream_impl: self.stream().stream_impl.snapshot2(
                &mut self.stream().sodium_ctx.clone(),
                &cell::get_cell_impl(&cb.cell()),
                &cell::get_cell_impl(&cc.cell()),
                f
            )
        }
    }

    fn snapshot3<B, C, D, CB, CC, CD, E, F>(&self, cb: &CB, cc: &CC, cd: &CD, f: F) -> Stream<E> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, E: Clone + 'static, F: IsLambda4<A, B, C, D, E> + 'static {
        Stream {
            sodium_ctx: self.stream().sodium_ctx.clone(),
            stream_impl: self.stream().stream_impl.snapshot3(
                &mut self.stream().sodium_ctx.clone(),
                &cell::get_cell_impl(&cb.cell()),
                &cell::get_cell_impl(&cc.cell()),
                &cell::get_cell_impl(&cd.cell()),
                f
            )
        }
    }

    fn snapshot4<B, C, D, E, CB, CC, CD, CE, F, FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> Stream<F> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, F: Clone + 'static, FN: IsLambda5<A, B, C, D, E, F> + 'static {
        Stream {
            sodium_ctx: self.stream().sodium_ctx.clone(),
            stream_impl: self.stream().stream_impl.snapshot4(
                &mut self.stream().sodium_ctx.clone(),
                &cell::get_cell_impl(&cb.cell()),
                &cell::get_cell_impl(&cc.cell()),
                &cell::get_cell_impl(&cd.cell()),
                &cell::get_cell_impl(&ce.cell()),
                f
            )
        }
    }

    fn snapshot5<B, C, D, E, F, CB, CC, CD, CE, CF, G, FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, f: FN) -> Stream<G> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, F: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, CF: IsCell<F>, G: Clone + 'static, FN: IsLambda6<A, B, C, D, E, F, G> + 'static {
        Stream {
            sodium_ctx: self.stream().sodium_ctx.clone(),
            stream_impl: self.stream().stream_impl.snapshot5(
                &mut self.stream().sodium_ctx.clone(),
                &cell::get_cell_impl(&cb.cell()),
                &cell::get_cell_impl(&cc.cell()),
                &cell::get_cell_impl(&cd.cell()),
                &cell::get_cell_impl(&ce.cell()),
                &cell::get_cell_impl(&cf.cell()),
                f
            )
        }
    }

    fn merge<SA, F>(&self, sa: &SA, f: F) -> Stream<A> where SA: IsStream<A>, F: IsLambda2<A, A, A> + 'static {
        Stream {
            sodium_ctx: self.stream().sodium_ctx.clone(),
            stream_impl: self.stream().stream_impl.merge(
                &mut self.stream().sodium_ctx.clone(),
                &sa.stream().stream_impl,
                f
            )
        }
    }

    fn filter<F>(&self, f: F) -> Stream<A> where F: IsLambda1<A, bool> + 'static {
        Stream {
            sodium_ctx: get_sodium_ctx(self),
            stream_impl: get_stream_impl(self).filter(&mut get_sodium_ctx(self), f)
        }
    }

    fn collect_lazy<B,ST,F>(&self, init_state: Lazy<ST>, f: F) -> Stream<B>
        where B: Clone + 'static,
              ST: Clone + 'static,
              F: Fn(&A,&ST)->(B,ST) + 'static
    {
        Stream {
            sodium_ctx: get_sodium_ctx(self),
            stream_impl: get_stream_impl(self).collect_lazy(
                &mut get_sodium_ctx(self),
                init_state,
                f
            )
        }
    }

    fn accum_lazy<ST,F>(&self, init_state: Lazy<ST>, f: F) -> Cell<ST>
        where ST: Clone + 'static,
              F: Fn(&A,&ST)->ST + 'static
    {
        cell::make_cell(
            get_sodium_ctx(self),
            get_stream_impl(self).accum_lazy(
                &mut get_sodium_ctx(self),
                init_state,
                f
            )
        )
    }

    fn once(&self) -> Stream<A>
    {
        Stream {
            sodium_ctx: get_sodium_ctx(self),
            stream_impl: get_stream_impl(self).once(&mut get_sodium_ctx(self))
        }
    }
}

impl<A: Clone + 'static, S: HasStream<Option<A>>> IsStreamOption<A> for S {
    fn filter_option(&self) -> Stream<A> {
        Stream {
            sodium_ctx: get_sodium_ctx(self),
            stream_impl: StreamImpl::filter_option(
                &mut get_sodium_ctx(self),
                &get_stream_impl(self)
            )
        }
    }
}

pub struct StreamImpl<A> {
    pub data: Gc<RefCell<StreamData<A>>>
}

pub struct WeakStreamImpl<A> {
    pub data: GcWeak<RefCell<StreamData<A>>>
}

pub trait IsStreamPrivate<A: Clone + 'static> {
    fn to_stream_ref(&self) -> &StreamImpl<A>;

    fn to_stream(&self) -> StreamImpl<A> {
        self.to_stream_ref().clone()
    }

    fn to_dep(&self) -> Dep {
        Dep::new(self.to_stream_ref().clone().data)
    }

    fn listen<F>(&self, sodium_ctx: &mut SodiumCtx, handler: F) -> Listener where F: Fn(&A) + 'static {
        let l0 = self.listen_weak(sodium_ctx, handler);
        let l_id = Rc::new(RefCell::new(0));
        let l_id2 = l_id.clone();
        let sodium_ctx2 = sodium_ctx.clone();
        let l = Listener::new(
            sodium_ctx,
            move || {
                l0.unlisten();
                sodium_ctx2.with_data_mut(|ctx| ctx.keep_listeners_alive.remove(&(*l_id2.borrow())));
            }
        );
        *(*l_id).borrow_mut() = l.id;
        sodium_ctx.with_data_mut(|ctx| ctx.keep_listeners_alive.insert(l.id.clone(), l.clone()));
        l
    }

    fn listen_once<F>(&self, sodium_ctx: &mut SodiumCtx, handler: F) -> Listener where F: Fn(&A) + 'static {
        let listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
        let listener2 = listener.clone();
        let result = self.listen(
            sodium_ctx,
            move |a: &A| {
                let mut tmp = listener2.borrow_mut();
                let tmp2 = &mut *tmp;
                match tmp2 {
                    &mut Some(ref mut tmp3) => tmp3.unlisten(),
                    &mut None => ()
                }
                handler(a);
            }
        );
        *listener.borrow_mut() = Some(result.clone());
        result
    }

    fn listen_(&self, sodium_ctx: &mut SodiumCtx, target: Gc<RefCell<HasNode>>, action: TransactionHandlerRef<A>) -> Listener {
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans1| {
                self.listen2(sodium_ctx, target, trans1, action, false, false)
            }
        )
    }

    fn listen_weak<F>(&self, sodium_ctx: &mut SodiumCtx, action: F) -> Listener where F: Fn(&A) + 'static {
        let null_node = sodium_ctx.null_node();
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        return self.listen_(
            sodium_ctx,
            null_node,
            TransactionHandlerRef::new(
                sodium_ctx2,
                move |_sodium_ctx, _trans2, a| {
                    action(a)
                }
            )
        );
    }

    fn listen2(&self, sodium_ctx: &mut SodiumCtx, target: Gc<RefCell<HasNode>>, trans: &mut Transaction, action: TransactionHandlerRef<A>, suppress_earlier_firings: bool, weak_self: bool) -> Listener {
        let mut self_ = self.to_stream_ref().data.borrow_mut();
        let self__: &mut StreamData<A> = &mut *self_;
        let node_target;
        let regen;
        {
            let (node_target2, regen2) = (self__ as &mut HasNode).link_to(sodium_ctx, target.clone(), action.clone());
            node_target = node_target2;
            regen = regen2;
        }
        if regen {
            trans.with_data_mut(|data| data.to_regen = true);
        }
        let firings = self__.firings.clone();
        if !suppress_earlier_firings && !firings.is_empty() {
            let action = action.clone();
            trans.prioritized(
                sodium_ctx,
                target,
                HandlerRefMut::new(
                    move |sodium_ctx, trans2| {
                        for a in &firings {
                            sodium_ctx.with_data_mut(|ctx| ctx.in_callback = ctx.in_callback + 1);
                            action.run(sodium_ctx, trans2, a);
                            sodium_ctx.with_data_mut(|ctx| ctx.in_callback = ctx.in_callback - 1);
                        }
                    }
                )
            );
        }
        let s =
            if weak_self {
                WeakS(self.to_stream_ref().downgrade())
            } else {
                StrongS(self.to_stream_ref().clone())
            };
        ListenerImpl::new(s, action, node_target)
            .into_listener(sodium_ctx)
    }

    fn weak(&self, sodium_ctx: &mut SodiumCtx) -> StreamImpl<A> {
        let out = StreamWithSend::new(sodium_ctx);
        let out2 = out.downgrade();
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let l = Transaction::run_trans(
            sodium_ctx,
            |sodium_ctx, trans| {
                self.listen2(
                    sodium_ctx,
                    out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>),
                    trans,
                    TransactionHandlerRef::new(
                        sodium_ctx2,
                        move |sodium_ctx, trans, a| {
                            out2.send(sodium_ctx, trans, a)
                        }
                    ),
                    false,
                    true
                )
            }
        );
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn map<B:'static + Clone,F>(&self, sodium_ctx: &mut SodiumCtx, f: F) -> StreamImpl<B>
    where F: IsLambda1<A,B> + 'static
    {
        let out = StreamWithSend::new(sodium_ctx);
        let out2 = out.downgrade();
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let deps = f.deps();
        let l = self.listen_(
            sodium_ctx,
            out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>),
            TransactionHandlerRef::new_with_deps(
                sodium_ctx2,
                move |sodium_ctx, trans, a| {
                    out2.send(sodium_ctx, trans, &f.apply(a));
                },
                deps
            )
        );
        out
            .unsafe_add_cleanup(l)
            .to_stream()
    }

    fn map_to<B:'static + Clone>(&self, sodium_ctx: &mut SodiumCtx, b: B) -> StreamImpl<B> {
        return self.map(sodium_ctx, move |_: &A| b.clone());
    }

    fn hold(&self, sodium_ctx: &mut SodiumCtx, init_value: A) -> CellImpl<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, _trans| CellImpl::new_(sodium_ctx, self.to_stream_ref().clone(), Some(init_value))
        )
    }

    fn hold_lazy(&self, sodium_ctx: &mut SodiumCtx, initial_value: Lazy<A>) -> CellImpl<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans|
                self.hold_lazy_(sodium_ctx, trans, initial_value)
        )
    }

    fn hold_lazy_(&self, sodium_ctx: &mut SodiumCtx, _trans: &mut Transaction, initial_value: Lazy<A>) -> CellImpl<A> {
        LazyCell::new(sodium_ctx, self.to_stream_ref().clone(), initial_value).to_cell()
    }

    fn snapshot_to<CB,B>(&self, sodium_ctx: &mut SodiumCtx, c: &CB) -> StreamImpl<B> where CB: IsCellPrivate<B>, B: Clone + 'static {
        self.snapshot(sodium_ctx, c, |_a: &A, b: &B| b.clone())
    }

    fn snapshot<CB,B,C,F>(&self, sodium_ctx: &mut SodiumCtx, c: &CB, f: F) -> StreamImpl<C>
        where CB: IsCellPrivate<B>,
              B: Clone + 'static,
              C: Clone + 'static,
              F: IsLambda2<A,B,C> + 'static
    {
        let out = StreamWithSend::new(sodium_ctx);
        let l;
        {
            let out_node = out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>);
            let out = out.downgrade();
            let c = c.to_cell();
            let mut deps = vec![c.to_dep()];
            deps.append(&mut f.deps());
            let mut sodium_ctx2 = sodium_ctx.clone();
            let sodium_ctx2 = &mut sodium_ctx2;
            l = self.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new_with_deps(
                    sodium_ctx2,
                    move |sodium_ctx, trans, a|
                        out.send(sodium_ctx, trans, &f.apply(a, &c.sample_no_trans_())),
                    deps
                )
            );
        }
        out
            .unsafe_add_cleanup(l)
            .to_stream()
    }

    fn snapshot2<CB,CC,B,C,D,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, f: F) -> StreamImpl<D>
        where CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              F: IsLambda3<A,B,C,D> + 'static
    {
        let cc = cc.to_cell();
        let cc2 = cc.clone();
        let deps = f.deps();
        self.snapshot(
            sodium_ctx,
            cb,
            lambda!(
                move |a: &A, b: &B| {
                    f.apply(a, b, &cc.sample_no_trans_())
                },
                cc2.to_dep()
            ).add_deps_tunneled(deps)
        )
    }

    fn snapshot3<CB,CC,CD,B,C,D,E,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, f: F) -> StreamImpl<E>
        where CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              CD: IsCellPrivate<D>,
              B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: IsLambda4<A,B,C,D,E> + 'static
    {
        let cc = cc.to_cell();
        let cd = cd.to_cell();
        let cc2 = cc.clone();
        let cd2 = cd.clone();
        let deps = f.deps();
        self.snapshot(
            sodium_ctx,
            cb,
            lambda!(
                move |a: &A, b: &B| {
                    f.apply(a, b, &cc.sample_no_trans_(), &cd.sample_no_trans_())
                },
                cc2.to_dep(), cd2.to_dep()
            ).add_deps_tunneled(deps)
        )
    }

    fn snapshot4<CB,CC,CD,CE,B,C,D,E,F,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> StreamImpl<F>
        where CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              CD: IsCellPrivate<D>,
              CE: IsCellPrivate<E>,
              B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: Clone + 'static,
              FN: IsLambda5<A,B,C,D,E,F> + 'static
    {
        let cc = cc.to_cell();
        let cd = cd.to_cell();
        let ce = ce.to_cell();
        let cc2 = cc.clone();
        let cd2 = cd.clone();
        let ce2 = ce.clone();
        let deps = f.deps();
        self.snapshot(
            sodium_ctx,
            cb,
            lambda!(
                move |a:&A, b: &B| {
                    f.apply(a, b, &cc.sample_no_trans_(), &cd.sample_no_trans_(), &ce.sample_no_trans_())
                },
                cc2.to_dep(), cd2.to_dep(), ce2.to_dep()
            ).add_deps_tunneled(deps)
        )
    }

    fn snapshot5<CB,CC,CD,CE,CF,B,C,D,E,F,G,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, f: FN) -> StreamImpl<G>
        where CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              CD: IsCellPrivate<D>,
              CE: IsCellPrivate<E>,
              CF: IsCellPrivate<F>,
              B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: Clone + 'static,
              G: Clone + 'static,
              FN: IsLambda6<A,B,C,D,E,F,G> + 'static
    {
        let cc = cc.to_cell();
        let cd = cd.to_cell();
        let ce = ce.to_cell();
        let cf = cf.to_cell();
        let cc2 = cc.clone();
        let cd2 = cd.clone();
        let ce2 = ce.clone();
        let cf2 = cf.clone();
        let deps = f.deps();
        self.snapshot(
            sodium_ctx,
            cb,
            lambda!(
                move |a:&A, b: &B| {
                    f.apply(a, b, &cc.sample_no_trans_(), &cd.sample_no_trans_(), &ce.sample_no_trans_(), &cf.sample_no_trans_())
                },
                cc2.to_dep(), cd2.to_dep(), ce2.to_dep(), cf2.to_dep()
            ).add_deps_tunneled(deps)
        )
    }

    fn or_else<SA>(&self, sodium_ctx: &mut SodiumCtx, s: &SA) -> StreamImpl<A> where SA: IsStreamPrivate<A> {
        self.merge(sodium_ctx, s, |a: &A, _: &A| a.clone())
    }

    fn merge_<SA>(&self, sodium_ctx: &mut SodiumCtx, s: &SA) -> StreamImpl<A> where SA: IsStreamPrivate<A> {
        let out = StreamWithSend::<A>::new(sodium_ctx);
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let left = sodium_ctx.new_gc(RefCell::new(Node::new(sodium_ctx2, 0))).upcast(|x| x as &RefCell<HasNode>);
        let right = out.to_stream_ref().data.clone().upcast(|x| x as &RefCell<HasNode>);
        let (node_target, _) = left.borrow_mut().link_to(
            sodium_ctx,
            right.clone(),
            TransactionHandlerRef::new(
                sodium_ctx2,
                |_: &mut SodiumCtx, _: &mut Transaction, _: &A| ()
            )
        );
        let h;
        {
            let out = out.downgrade();
            h = TransactionHandlerRef::new(
                sodium_ctx2,
                move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &A| {
                    out.send(sodium_ctx, trans, a);
                }
            );
        }
        let l1 = self.listen_(sodium_ctx, left.clone(), h.clone());
        let l2 = s.listen_(sodium_ctx, right, h);
        out.unsafe_add_cleanup(l1).unsafe_add_cleanup(l2).unsafe_add_cleanup(Listener::new(
            sodium_ctx,
            move || {
                left.borrow_mut().unlink_to(&node_target);
            }
        )).to_stream()
    }

    fn merge<SA,F>(&self, sodium_ctx: &mut SodiumCtx, s: &SA, f: F) -> StreamImpl<A> where SA: IsStreamPrivate<A>, F: IsLambda2<A,A,A> + 'static {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction| {
                self.merge_(sodium_ctx, s).coalesce_(sodium_ctx,trans, f)
            }
        )
    }

    fn coalesce_<F>(&self, sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction, f: F) -> StreamImpl<A> where F: IsLambda2<A,A,A> + 'static {
        let deps = f.deps();
        let out = StreamWithSend::new(sodium_ctx);
        let h = CoalesceHandler::new(move |a, b| f.apply(a, b), out.downgrade());
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let l = self.listen2(
            sodium_ctx,
            out.to_stream_ref().data.clone().upcast(|x| x as &RefCell<HasNode>),
            trans1,
            h.to_transaction_handler(sodium_ctx2).set_deps_tunneled(deps),
            false,
            false
        );
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn last_firing_only_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> StreamImpl<A> {
        self.coalesce_(sodium_ctx, trans, |_: &A, a: &A| a.clone())
    }

    fn filter<F>(&self, sodium_ctx: &mut SodiumCtx, predicate: F) -> StreamImpl<A> where F: IsLambda1<A,bool> + 'static {
        let out = StreamWithSend::new(sodium_ctx);
        let l;
        {
            let out_node = out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>);
            let out = out.downgrade();
            let mut sodium_ctx2 = sodium_ctx.clone();
            let sodium_ctx2 = &mut sodium_ctx2;
            l = self.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    sodium_ctx2,
                    move |sodium_ctx, trans, a| {
                        if predicate.apply(a) {
                            out.send(sodium_ctx, trans, a);
                        }
                    }
                )
            );
        }
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn filter_option<S>(sodium_ctx: &mut SodiumCtx, self_: &S) -> StreamImpl<A> where S: IsStreamPrivate<Option<A>> {
        let out = StreamWithSend::new(sodium_ctx);
        let l;
        {
            let out_node = out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>);
            let out = out.downgrade();
            let mut sodium_ctx2 = sodium_ctx.clone();
            let sodium_ctx2 = &mut sodium_ctx2;
            l = self_.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    sodium_ctx2,
                    move |sodium_ctx, trans, oa| {
                        match oa {
                            &Some(ref a) => out.send(sodium_ctx, trans, a),
                            &None => ()
                        }
                    }
                )
            );
        }
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn gate<CB>(&self, sodium_ctx: &mut SodiumCtx, c: &CB) -> StreamImpl<A> where CB: IsCellPrivate<bool> {
        let s =
            self.snapshot(
                sodium_ctx,
                c,
                |a: &A, pred: &bool| {
                    if *pred {
                        Some(a.clone())
                    } else {
                        None
                    }
                }
            );
        StreamImpl::filter_option(sodium_ctx, &s)
    }

    fn collect<B,S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: S, f: F) -> StreamImpl<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: Fn(&A,&S)->(B,S) + 'static
    {
        self.collect_lazy(sodium_ctx, Lazy::new(move || init_state.clone()), f)
    }

    fn collect_lazy<B,S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: Lazy<S>, f: F) -> StreamImpl<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: Fn(&A,&S)->(B,S) + 'static
    {
        let ea = self.to_stream_ref().clone();
        let f = Rc::new(f);
        Transaction::run(
            sodium_ctx,
            move |sodium_ctx| {
                let mut es = StreamLoop::new(sodium_ctx);
                let s = IsStreamPrivate::hold_lazy(&es, sodium_ctx, init_state.clone());
                let f = f.clone();
                let f2 = move |a: &A, s: &S| f(a,s);
                let ebs = ea.snapshot(sodium_ctx, &s, f2);
                let eb = ebs.map(sodium_ctx, |&(ref b,ref _s): &(B,S)| b.clone());
                let es_out = ebs.map(sodium_ctx, |&(ref _b,ref s): &(B,S)| s.clone());
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                es.loop_(Stream {
                    sodium_ctx: sodium_ctx2.clone(),
                    stream_impl: es_out.weak(sodium_ctx2)
                });
                eb.keep_alive(sodium_ctx, es_out)
            }
        )
    }

    fn accum<S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: S, f: F) -> CellImpl<S>
        where S: Clone + 'static,
              F: Fn(&A,&S)->S + 'static
    {
        self.accum_lazy(sodium_ctx, Lazy::new(move || init_state.clone()), f)
    }

    fn accum_lazy<S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: Lazy<S>, f: F) -> CellImpl<S>
        where S: Clone + 'static,
              F: Fn(&A,&S)->S + 'static
    {
        let ea = self.to_stream_ref().clone();
        let f = Rc::new(f);
        Transaction::run(
            sodium_ctx,
            move |sodium_ctx| {
                let mut es = StreamLoop::new(sodium_ctx);
                let s = IsStreamPrivate::hold_lazy(&es, sodium_ctx, init_state.clone());
                let f = f.clone();
                let f2 = move |a: &A,s: &S| f(a,s);
                let es_out = ea.snapshot(sodium_ctx, &s, f2);
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                es.loop_(
                    Stream {
                        sodium_ctx: sodium_ctx2.clone(),
                        stream_impl: es_out.weak(sodium_ctx2)
                    }
                );
                es_out.hold_lazy(sodium_ctx, init_state.clone())
            }
        )
    }

    fn once(&self, sodium_ctx: &mut SodiumCtx) -> StreamImpl<A> {
        let out = StreamWithSend::new(sodium_ctx);
        let out_node = out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>);
        let l_cell = Rc::new(RefCell::new(None));
        let l;
        {
            let out = out.downgrade();
            let l_cell = l_cell.clone();
            let mut sodium_ctx2 = sodium_ctx.clone();
            let sodium_ctx2 = &mut sodium_ctx2;
            l = self.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    sodium_ctx2,
                    move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &A| {
                        let has_listener = l_cell.borrow().is_some();
                        if has_listener {
                            out.send(sodium_ctx, trans, a);
                            {
                                let l = l_cell.borrow();
                                let l: &Option<Listener> = &l;
                                match l.as_ref() {
                                    Some(l2) => l2.unlisten(),
                                    None => ()
                                }
                            }
                            *l_cell.borrow_mut() = None;
                        }
                    }
                )
            );
        }
        *l_cell.borrow_mut() = Some(l.clone());
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn keep_alive<X:'static>(&self, sodium_ctx: &mut SodiumCtx, object: X) -> StreamImpl<A> {
        let l = Listener::new(sodium_ctx, move || { let _object2 = &object; });
        self.add_cleanup(sodium_ctx, l)
    }

    fn unsafe_add_cleanup(&self, listener: Listener) -> &Self {
        self.to_stream_ref().data.add_deps(vec![listener.to_dep().gc_dep]);
        let mut data = self.to_stream_ref().data.borrow_mut();
        let data_: &mut StreamData<A> = &mut *data;
        data_.finalizers.push(listener);
        self
    }

    fn add_cleanup(&self, sodium_ctx: &mut SodiumCtx, cleanup: Listener) -> StreamImpl<A> {
        self
            .map(sodium_ctx, |a: &A| a.clone())
            .unsafe_add_cleanup(cleanup)
            .to_stream()
    }
}

impl<A: 'static + Clone> IsStreamPrivate<A> for StreamImpl<A> {
    fn to_stream_ref(&self) -> &StreamImpl<A> {
        self
    }
}

impl<A> Clone for StreamImpl<A> {
    fn clone(&self) -> Self {
        StreamImpl {
            data: self.data.clone()
        }
    }
}

impl<A> Clone for WeakStreamImpl<A> {
    fn clone(&self) -> Self {
        WeakStreamImpl {
            data: self.data.clone()
        }
    }
}

pub struct StreamData<A> {
    pub node: Node,
    pub finalizers: Vec<Listener>,
    pub firings: Vec<A>,
}

impl<A> Drop for StreamData<A> {
    fn drop(&mut self) {
        for finalizer in &self.finalizers {
            finalizer.unlisten();
        }
    }
}

impl<A> HasNode for StreamData<A> {
    fn node_ref(&self) -> &Node {
        &self.node
    }
    fn node_mut(&mut self) -> &mut Node {
        &mut self.node
    }
}

enum WeakOrStrongStream<A> {
    WeakS(WeakStreamImpl<A>),
    StrongS(StreamImpl<A>)
}

use self::WeakOrStrongStream::WeakS;
use self::WeakOrStrongStream::StrongS;

struct ListenerImpl<A> {
    event: WeakOrStrongStream<A>,

    #[allow(dead_code)]
    action: TransactionHandlerRef<A>,

    target: Target,
    done: bool
}

impl<A: Clone + 'static> ListenerImpl<A> {
    fn new(event: WeakOrStrongStream<A>, action: TransactionHandlerRef<A>, target: Target) -> ListenerImpl<A>
    {
        ListenerImpl {
            event: event,
            action: action,
            target: target,
            done: false
        }
    }

    fn into_listener(self, sodium_ctx: &mut SodiumCtx) -> Listener {
        let deps = match &self.event {
            &WeakS(ref s) => vec![self.action.to_dep()],
            &StrongS(ref s) => vec![self.action.to_dep(), s.to_dep()]
        };
        let self_ = RefCell::new(self);
        let l = Listener::new(
            sodium_ctx,
            move || {
                let mut self__ = self_.borrow_mut();
                let self___ = &mut *self__;
                if !self___.done {
                    let s_op =
                        match &self___.event {
                            &WeakS(ref s) => s.upgrade(),
                            &StrongS(ref s) => Some(s.clone())
                        };
                    match s_op {
                        Some(s) => {
                            let mut stream_data = s.data.borrow_mut();
                            HasNode::unlink_to(&mut *stream_data as &mut HasNode, &self___.target);
                        },
                        None => ()
                    }
                    self___.done = true;
                }
            }
        );
        l.set_deps(deps);
        l
    }
}

impl<A: Clone + 'static> StreamImpl<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> StreamImpl<A> {
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        StreamImpl {
            data: sodium_ctx.new_gc(RefCell::new(
                StreamData {
                    node: Node::new(sodium_ctx2, 0),
                    finalizers: Vec::new(),
                    firings: Vec::new()
                }
            ))
        }
    }

    pub fn downgrade(&self) -> WeakStreamImpl<A> {
        WeakStreamImpl {
            data: self.data.downgrade()
        }
    }

    pub fn or_else<IT>(sodium_ctx: &mut SodiumCtx, ss: IT) -> StreamImpl<A>
        where IT: Iterator<Item=StreamImpl<A>>
    {
        StreamImpl::merge(
            sodium_ctx,
            ss,
            |left, _right|
                left.clone()
        )
    }

    pub fn merge<IT,F>(sodium_ctx: &mut SodiumCtx, ss: IT, f: F) -> StreamImpl<A>
        where IT: Iterator<Item=StreamImpl<A>>,
              F: Fn(&A,&A)->A + 'static
    {
        let ss_vec: Vec<StreamImpl<A>> = ss.collect();
        let ss_vec_len = ss_vec.len();
        StreamImpl::merge_(sodium_ctx, &ss_vec, 0, ss_vec_len, f)
    }

    fn merge_<F>(sodium_ctx: &mut SodiumCtx, ss: &Vec<StreamImpl<A>>, start: usize, end: usize, f: F) -> StreamImpl<A>
        where F: Fn(&A,&A)->A + 'static
    {
        StreamImpl::merge__(sodium_ctx, ss, start, end, &Rc::new(f))
    }

    fn merge__<F>(sodium_ctx: &mut SodiumCtx, ss: &Vec<StreamImpl<A>>, start: usize, end: usize, f: &Rc<F>) -> StreamImpl<A>
        where F: Fn(&A,&A)->A + 'static
    {
        let len = end - start;
        if len == 0 {
            StreamImpl::new(sodium_ctx)
        } else if len == 1 {
            ss[start].clone()
        } else if len == 2 {
            let f2 = f.clone();
            let f3 = move |a: &A, b: &A| f2(a, b);
            ss[start].merge(sodium_ctx, &ss[start+1], f3)
        } else {
            let f2 = f.clone();
            let f3 = move |a: &A, b: &A| f2(a, b);
            let mid = (start + end) / 2;
            let s1 =
                StreamImpl::merge__(
                    sodium_ctx,
                    &ss,
                    start,
                    mid,
                    f
                );
            let s2 =
                StreamImpl::merge__(
                    sodium_ctx,
                    &ss,
                    mid,
                    end,
                    f
                );
            s1.merge(sodium_ctx, &s2, f3)
        }
    }
}

impl<A: Clone + 'static> WeakStreamImpl<A> {
    pub fn upgrade(&self) -> Option<StreamImpl<A>> {
        self.data.upgrade().map(|data| {
            StreamImpl {
                data: data
            }
        })
    }
}
