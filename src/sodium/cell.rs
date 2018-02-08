use sodium::Dep;
use sodium::HandlerRefMut;
use sodium::HasNode;
use sodium::IsStream;
use sodium::IsLambda1;
use sodium::IsLambda2;
use sodium::IsLambda3;
use sodium::IsLambda4;
use sodium::IsLambda5;
use sodium::IsLambda6;
use sodium::Lambda;
use sodium::Node;
use sodium::stream::IsStreamPrivate;
use sodium::Lazy;
use sodium::Listener;
use sodium::Operational;
use sodium::SodiumCtx;
use sodium::stream;
use sodium::Stream;
use sodium::stream::StreamImpl;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use sodium::WeakStreamWithSend;
use sodium::gc::Gc;
use sodium::gc::GcCell;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::Deref;

pub struct Cell<A: Clone + 'static> {
    sodium_ctx: SodiumCtx,
    cell_impl: CellImpl<A>
}

impl<A: Clone + 'static> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            sodium_ctx: self.sodium_ctx.clone(),
            cell_impl: self.cell_impl.clone()
        }
    }
}

pub trait HasCell<A: Clone + 'static> {
    fn cell(&self) -> Cell<A>;
}

impl<A: Clone + 'static> HasCell<A> for Cell<A> {
    fn cell(&self) -> Cell<A> {
        self.clone()
    }
}

pub fn make_cell<A: Clone + 'static>(sodium_ctx: SodiumCtx, cell_impl: CellImpl<A>) -> Cell<A> {
    Cell {
        sodium_ctx,
        cell_impl
    }
}

pub fn get_sodium_ctx<A: Clone + 'static, CA: HasCell<A>>(ca: &CA) -> SodiumCtx {
    ca.cell().sodium_ctx.clone()
}

pub fn get_cell_impl<A: Clone + 'static, CA: HasCell<A>>(ca: &CA) -> CellImpl<A> {
    ca.cell().cell_impl.clone()
}

pub trait IsCell<A: Clone + 'static> {
    fn cell(&self) -> Cell<A>;
    fn to_dep(&self) -> Dep;
    fn listen<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static;
    fn listen_weak<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static;
    fn sample(&self) -> A;
    fn sample_lazy(&self) -> Lazy<A>;
    fn map<B,F>(&self, f: F) -> Cell<B> where B: Clone + 'static, F: IsLambda1<A,B> + 'static;
    fn lift2<B,C,CB,F>(&self, cb: &CB, f: F) -> Cell<C> where B: Clone + 'static, C: Clone + 'static, CB: IsCell<B>, F: IsLambda2<A,B,C> + 'static;
    fn lift3<B,C,D,CB,CC,F>(&self, cb: &CB, cc: &CC, f: F) -> Cell<D> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, F: IsLambda3<A,B,C,D> + 'static;
    fn lift4<B,C,D,E,CB,CC,CD,F>(&self, cb: &CB, cc: &CC, cd: &CD, f: F) -> Cell<E> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, F: IsLambda4<A,B,C,D,E> + 'static;
    fn lift5<B,C,D,E,F,CB,CC,CD,CE,FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> Cell<F> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, F: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, FN: IsLambda5<A,B,C,D,E,F> + 'static;
    fn lift6<B,C,D,E,F,G,CB,CC,CD,CE,CF,FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, f: FN) -> Cell<G> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, F: Clone + 'static, G: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, CF: IsCell<F>, FN: IsLambda6<A,B,C,D,E,F,G> + 'static;
    fn apply<B,CF,F>(&self, cf: &CF) -> Cell<B> where B: Clone + 'static, CF: IsCell<Rc<F>>, F: IsLambda1<A,B> + 'static;
}

impl<A: Clone + 'static, SA: HasCell<A>> IsCell<A> for SA {
    fn cell(&self) -> Cell<A> {
        HasCell::cell(self)
    }

    fn to_dep(&self) -> Dep {
        get_cell_impl(self).to_dep()
    }

    fn listen<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static {
        get_cell_impl(self).listen(&mut get_sodium_ctx(self), f)
    }

    fn listen_weak<F>(&self, f: F) -> Listener where F: Fn(&A) + 'static {
        Transaction::run(
            &mut get_sodium_ctx(self),
            |sodium_ctx| {
                Operational
                    ::value(self)
                    .listen_weak(f)
            }
        )
    }

    fn sample(&self) -> A {
        get_cell_impl(self).sample(&mut get_sodium_ctx(self))
    }

    fn sample_lazy(&self) -> Lazy<A> {
        get_cell_impl(self).sample_lazy(&mut get_sodium_ctx(self))
    }

    fn map<B, F>(&self, f: F) -> Cell<B> where B: Clone + 'static, F: IsLambda1<A, B> + 'static {
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: get_cell_impl(self).map(&mut get_sodium_ctx(self), f)
        }
    }

    fn lift2<B, C, CB, F>(&self, cb: &CB, f: F) -> Cell<C> where B: Clone + 'static, C: Clone + 'static, CB: IsCell<B>, F: IsLambda2<A, B, C> + 'static {
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: get_cell_impl(self).lift2(
                &mut get_sodium_ctx(self),
                &cb.cell().cell_impl.clone(),
                f
            )
        }
    }

    fn lift3<B, C, D, CB, CC, F>(&self, cb: &CB, cc: &CC, f: F) -> Cell<D> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, F: IsLambda3<A, B, C, D> + 'static {
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: get_cell_impl(self).lift3(
                &mut get_sodium_ctx(self),
                &cb.cell().cell_impl.clone(),
                &cc.cell().cell_impl.clone(),
                f
            )
        }
    }

    fn lift4<B, C, D, E, CB, CC, CD, F>(&self, cb: &CB, cc: &CC, cd: &CD, f: F) -> Cell<E> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, F: IsLambda4<A, B, C, D, E> + 'static {
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: get_cell_impl(self).lift4(
                &mut get_sodium_ctx(self),
                &cb.cell().cell_impl.clone(),
                &cc.cell().cell_impl.clone(),
                &cd.cell().cell_impl.clone(),
                f
            )
        }
    }

    fn lift5<B, C, D, E, F, CB, CC, CD, CE, FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> Cell<F> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, F: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, FN: IsLambda5<A, B, C, D, E, F> + 'static {
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: get_cell_impl(self).lift5(
                &mut get_sodium_ctx(self),
                &cb.cell().cell_impl.clone(),
                &cc.cell().cell_impl.clone(),
                &cd.cell().cell_impl.clone(),
                &ce.cell().cell_impl.clone(),
                f
            )
        }
    }

    fn lift6<B, C, D, E, F, G, CB, CC, CD, CE, CF, FN>(&self, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, f: FN) -> Cell<G> where B: Clone + 'static, C: Clone + 'static, D: Clone + 'static, E: Clone + 'static, F: Clone + 'static, G: Clone + 'static, CB: IsCell<B>, CC: IsCell<C>, CD: IsCell<D>, CE: IsCell<E>, CF: IsCell<F>, FN: IsLambda6<A, B, C, D, E, F, G> + 'static {
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: get_cell_impl(self).lift6(
                &mut get_sodium_ctx(self),
                &cb.cell().cell_impl.clone(),
                &cc.cell().cell_impl.clone(),
                &cd.cell().cell_impl.clone(),
                &ce.cell().cell_impl.clone(),
                &cf.cell().cell_impl.clone(),
                f
            )
        }
    }

    fn apply<B, CF, F>(&self, cf: &CF) -> Cell<B> where B: Clone + 'static, CF: IsCell<Rc<F>>, F: IsLambda1<A,B> + 'static {
        let mut sodium_ctx = get_sodium_ctx(self);
        let sodium_ctx = &mut sodium_ctx;
        let cf2: CellImpl<Rc<F>> = get_cell_impl(&cf.cell());
        let ca2: CellImpl<A> = get_cell_impl(&self.cell());
        let cb = CellImpl::apply(sodium_ctx, &cf2, &ca2);
        Cell {
            sodium_ctx: get_sodium_ctx(self),
            cell_impl: cb
        }
    }
}

impl<A: Clone + 'static> Cell<A> {
    pub fn switch_c<CCA,CA>(cca: &CCA) -> Cell<A>
        where CCA: IsCell<CA>,
               CA: IsCell<A> + Clone + 'static
    {
        let mut sodium_ctx = get_sodium_ctx(&cca.cell());
        let sodium_ctx = &mut sodium_ctx;
        make_cell(
            sodium_ctx.clone(),
            CellImpl::switch_c(
                sodium_ctx,
                &get_cell_impl(
                    &cca.cell().map(|ca: &CA| get_cell_impl(&ca.cell()))
                )
            )
        )
    }

    pub fn switch_s<CSA,SA>(csa: &CSA) -> Stream<A>
        where CSA: IsCell<SA>,
              SA: IsStream<A> + Clone + 'static
    {
        let mut sodium_ctx = get_sodium_ctx(&csa.cell());
        let sodium_ctx = &mut sodium_ctx;
        stream::make_stream(
            sodium_ctx.clone(),
            CellImpl::switch_s(
                sodium_ctx,
                &get_cell_impl(
                    &csa.cell().map(|sa: &SA| stream::get_stream_impl(&sa.stream()))
                )
            )
        )
    }
}

pub trait IsCellPrivate<A: Clone + 'static> {
    fn to_cell(&self) -> CellImpl<A>;

    fn to_dep(&self) -> Dep {
        Dep::new(self.to_cell().data)
    }

    fn with_cell_data_ref<F,R>(&self, f: F) -> R where F: FnOnce(&CellData<A>)->R {
        let data1 = self.to_cell();
        let data2 = (*data1.data).borrow();
        let data3 = data2.borrow();
        f(data3.cell_data_ref())
    }

    fn with_cell_data_mut<F,R>(&self, f: F) -> R where F: FnOnce(&mut CellData<A>)->R {
        let data1 = self.to_cell();
        let mut data2 = (*data1.data).borrow_mut();
        let data3 = data2.borrow_mut();
        f(data3.cell_data_mut())
    }

    fn new_value_(&self) -> A {
        let value_op = self.with_cell_data_ref(|data| data.value_update.clone());
        match value_op {
            Some(value) => value,
            None => self.sample_no_trans_()
        }
    }

    fn sample(&self, sodium_ctx: &mut SodiumCtx) -> A {
        Transaction::apply(
            sodium_ctx,
            |_sodium_ctx, _trans| {
                self.sample_no_trans_()
            }
        )
    }

    fn sample_lazy(&self, sodium_ctx: &mut SodiumCtx) -> Lazy<A> {
        let me = self.to_cell();
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans|
                me.sample_lazy_(sodium_ctx, trans)
        )
    }

    fn sample_lazy_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> Lazy<A> {
        let me = self.to_cell();
        let s = LazySample::new(me.clone());
        {
            let s = s.clone();
            trans.sample(
                move || {
                    let sample_no_trans = me.sample_no_trans_();
                    let mut s = s.data.deref().borrow_mut();
                    s.value_op = Some(
                        me.data.deref().borrow()
                            .cell_data_ref()
                            .value_update.clone().unwrap_or_else(move || sample_no_trans)
                    );
                    s.cell_op = None;
                }
            );
        }
        let sodium_ctx = sodium_ctx.clone();
        Lazy::new(move || {
            let s = s.data.deref().borrow();
            match &s.value_op {
                &Some(ref value) => return value.clone(),
                &None => ()
            }
            let mut sodium_ctx = sodium_ctx.clone();
            let sodium_ctx = &mut sodium_ctx;
            s.cell_op.as_ref().unwrap().sample(sodium_ctx)
        })
    }

    fn sample_no_trans_(&self) -> A {
        self.with_cell_data_mut(|data: &mut CellData<A>| {
            data.sample_no_trans_()
        })
    }

    fn updates_(&self, _trans: &mut Transaction) -> StreamImpl<A> {
        self.with_cell_data_ref(|data| data.str.clone())
    }

    fn value_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> StreamImpl<A> {
        let s_spark = StreamWithSend::new(sodium_ctx);
        let s_spark_node = s_spark.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
        {
            let s_spark = s_spark.clone();
            trans.prioritized(
                sodium_ctx,
                s_spark_node,
                HandlerRefMut::new(
                    move |sodium_ctx, trans2| s_spark.send(sodium_ctx, trans2, &())
                )
            );
        }
        let s_initial = s_spark.snapshot_to(sodium_ctx, &self.to_cell());
        s_initial.merge(sodium_ctx, &self.updates_(trans), |_: &A, a: &A| a.clone())
    }

    fn weak(&self, sodium_ctx: &mut SodiumCtx) -> CellImpl<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                let tmp = self.sample_lazy_(sodium_ctx, trans);
                self.updates_(trans).weak(sodium_ctx).hold_lazy_(sodium_ctx, trans, tmp)
            }
        )
    }

    fn map<F,B:'static + Clone>(&self, sodium_ctx: &mut SodiumCtx, f: F) -> CellImpl<B> where F: IsLambda1<A,B> + 'static {
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans| {
                let f2 = Rc::new(f);
                let f3 = f2.clone();
                let tmp =
                    self.sample_lazy_(sodium_ctx, trans)
                        .map(move |a| f2.apply(a));
                self.updates_(trans)
                    .map(sodium_ctx, move |a: &A| f3.apply(a))
                    .hold_lazy_(
                        sodium_ctx,
                        trans,
                        tmp
                    )
            }
        )
    }

    fn lift2<B,C,CB,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, f: F) -> CellImpl<C>
        where B: Clone + 'static,
              C: Clone + 'static,
              CB: IsCellPrivate<B>,
              F: IsLambda2<A,B,C> + 'static
    {
        let f = Rc::new(f);
        let curried_f = move |a: &A| {
            let a = a.clone();
            let f = f.clone();
            Rc::new(move |b: &B| f.apply(&a, b))
        };
        let cf = self.map(sodium_ctx, curried_f);
        CellImpl::apply(sodium_ctx, &cf, cb)
    }

    fn lift3<B,C,D,CB,CC,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, f: F) -> CellImpl<D>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              F: IsLambda3<A,B,C,D> + 'static
    {
        let f = Rc::new(f);
        let slightly_curried_f = move |a: &A, b: &B| {
            let a = a.clone();
            let b = b.clone();
            let f = f.clone();
            Rc::new(move |c: &C| f.apply(&a, &b, c))
        };
        let cf = self.lift2(sodium_ctx, cb, slightly_curried_f);
        CellImpl::apply(sodium_ctx, &cf, cc)
    }

    fn lift4<B,C,D,E,CB,CC,CD,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, f: F) -> CellImpl<E>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              CD: IsCellPrivate<D>,
              F: IsLambda4<A,B,C,D,E> + 'static
    {
        let f = Rc::new(f);
        let slightly_curried_f = move |a: &A, b: &B| {
            let a = a.clone();
            let b = b.clone();
            let f = f.clone();
            Rc::new(move |c: &C, d: &D| f.apply(&a, &b, c, d))
        };
        let cf = self.lift2(sodium_ctx, cb, slightly_curried_f);
        CellImpl::apply2_(sodium_ctx, &cf, cc, cd)
    }

    fn lift5<B,C,D,E,F,CB,CC,CD,CE,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> CellImpl<F>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: Clone + 'static,
              CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              CD: IsCellPrivate<D>,
              CE: IsCellPrivate<E>,
              FN: IsLambda5<A,B,C,D,E,F> + 'static
    {
        let f = Rc::new(f);
        let slightly_curried_f = move |a: &A, b: &B, c: &C| {
            let a = a.clone();
            let b = b.clone();
            let c = c.clone();
            let f = f.clone();
            Rc::new(move |d: &D, e: &E| f.apply(&a, &b, &c, d, e))
        };
        let cf = self.lift3(sodium_ctx, cb, cc, slightly_curried_f);
        CellImpl::apply2_(sodium_ctx, &cf, cd, ce)
    }

    fn lift6<B,C,D,E,F,G,CB,CC,CD,CE,CF,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, fn_: FN) -> CellImpl<G>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: Clone + 'static,
              G: Clone + 'static,
              CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              CD: IsCellPrivate<D>,
              CE: IsCellPrivate<E>,
              CF: IsCellPrivate<F>,
              FN: IsLambda6<A,B,C,D,E,F,G> + 'static
    {
        let fn_ = Rc::new(fn_);
        let slightly_curried_f = move |a: &A, b: &B, c: &C| {
            let a = a.clone();
            let b = b.clone();
            let c = c.clone();
            let fn_ = fn_.clone();
            Rc::new(move |d: &D, e: &E, f: &F| fn_.apply(&a, &b, &c, d, e, f))
        };
        let cfn = self.lift3(sodium_ctx, cb, cc, slightly_curried_f);
        CellImpl::apply3_(sodium_ctx, &cfn, cd, ce, cf)
    }

    fn apply<CF,CA,F,B:'static + Clone>(sodium_ctx: &mut SodiumCtx, cf: &CF, ca: &CA) -> CellImpl<B>
    where
    CF: IsCellPrivate<Rc<F>>,
    CA: IsCellPrivate<A>,
    F: IsLambda1<A,B> + 'static
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                let out: StreamWithSend<Lazy<B>> = StreamWithSend::new(sodium_ctx);
                let out_target = out.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                let in_target = sodium_ctx.new_gc(GcCell::new(Node::new(sodium_ctx2, 0))).upcast(|x| x as &GcCell<HasNode>);
                let (node_target,_) = ((in_target.deref()).borrow_mut().node_mut() as &mut HasNode).link_to::<Lazy<B>>(
                    sodium_ctx,
                    out_target,
                    TransactionHandlerRef::new(
                        sodium_ctx2,
                        |_sodium_ctx: &mut SodiumCtx, _trans: &mut Transaction, _a: &Lazy<B>| {}
                    )
                );
                let h: ApplyHandler<A,B> = ApplyHandler::new(out.downgrade());
                let l1;
                {
                    let h = h.clone();
                    l1 =
                        cf
                            .value_(sodium_ctx, trans)
                            .listen_(sodium_ctx, in_target.clone(), TransactionHandlerRef::new(
                                sodium_ctx2,
                                move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, f: &Rc<F>| {
                                    (*h.data).borrow_mut().f_op = Some(f.clone());
                                    let run_it = match &(*h.data).borrow_mut().a_op {
                                        &Some(ref _a) => true,
                                        &None => false
                                    };
                                    if run_it {
                                        h.run(sodium_ctx, trans2)
                                    }
                                }
                            ));
                }
                let l2 =
                    ca
                        .value_(sodium_ctx, trans)
                        .listen_(sodium_ctx, in_target.clone(), TransactionHandlerRef::new(
                            sodium_ctx2,
                            move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                                (*h.data).borrow_mut().a_op = Some(a.clone());
                                let run_it = match &(*h.data).borrow_mut().f_op {
                                    &Some(ref _f) => true,
                                    &None => false
                                };
                                if run_it {
                                    h.run(sodium_ctx, trans2)
                                }
                            }
                        ));
                let cf: CellImpl<Rc<F>> = cf.to_cell();
                let ca: CellImpl<A> = ca.to_cell();
                out.last_firing_only_(sodium_ctx, trans)
                    .unsafe_add_cleanup(l1)
                    .unsafe_add_cleanup(l2)
                    .unsafe_add_cleanup(
                        Listener::new(
                            sodium_ctx,
                            move || {
                                (*in_target).borrow_mut().unlink_to(&node_target)
                            }
                        )
                    )
                    .map(sodium_ctx, |lazy_b: &Lazy<B>| lazy_b.get())
                    .hold_lazy(sodium_ctx, Lazy::new(move || {
                        cf.sample_no_trans_().apply(&ca.sample_no_trans_())
                    }))
            }
        )
    }

    fn apply2_<CF,CA,CB,B,C,F>(sodium_ctx: &mut SodiumCtx, cf: &CF, ca: &CA, cb: &CB) -> CellImpl<C>
        where CF: IsCellPrivate<Rc<F>>,
              CA: IsCellPrivate<A>,
              CB: IsCellPrivate<B>,
              B: Clone + 'static,
              C: Clone + 'static,
              F: Fn(&A,&B)->C + 'static
    {
        let cf2 =
            cf.map(
                sodium_ctx,
                |f: &Rc<F>| {
                    let f = f.clone();
                    Rc::new(move |a: &A| {
                        let a = a.clone();
                        let f = f.clone();
                        Rc::new(move |b: &B| f(&a, b))
                    })
                }
            );
        let cf3 = CellImpl::apply(sodium_ctx, &cf2, ca);
        CellImpl::apply(sodium_ctx, &cf3, cb)
    }

    fn apply3_<CF,CA,CB,CC,B,C,D,F>(sodium_ctx: &mut SodiumCtx, cf: &CF, ca: &CA, cb: &CB, cc: &CC) -> CellImpl<D>
        where CF: IsCellPrivate<Rc<F>>,
              CA: IsCellPrivate<A>,
              CB: IsCellPrivate<B>,
              CC: IsCellPrivate<C>,
              B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              F: Fn(&A,&B,&C)->D + 'static
    {
        let cf2 =
            cf.map(
                sodium_ctx,
                |f: &Rc<F>| {
                    let f = f.clone();
                    Rc::new(move |a: &A, b: &B| {
                        let a = a.clone();
                        let b = b.clone();
                        let f = f.clone();
                        Rc::new(move |c: &C| f(&a, &b, c))
                    })
                }
            );
        let cf3 = CellImpl::apply2_(sodium_ctx, &cf2, ca, cb);
        CellImpl::apply(sodium_ctx, &cf3, cc)
    }

    fn switch_c<CCA,CA>(sodium_ctx: &mut SodiumCtx, cca: &CCA) -> CellImpl<A>
        where CCA: IsCellPrivate<CA>,
               CA: IsCellPrivate<A> + Clone + 'static
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction| {
                let za;
                {
                    let mut sodium_ctx = sodium_ctx.clone();
                    za =
                        cca
                            .sample_lazy(&mut sodium_ctx)
                            .map(move|ca| {
                                let mut sodium_ctx = sodium_ctx.clone();
                                let sodium_ctx = &mut sodium_ctx;
                                ca.sample(sodium_ctx)
                            });
                }
                let out = StreamWithSend::new(sodium_ctx);
                let current_listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
                let h;
                {
                    let out = out.clone();
                    let current_listener = current_listener.clone();
                    h = TransactionHandlerRef::new(
                        sodium_ctx,
                        move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, ca: &CA| {
                            let out = out.clone();
                            let out_node = out.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
                            let mut current_listener = (*current_listener).borrow_mut();
                            match current_listener.as_ref() {
                                Some(current_listener) =>
                                    current_listener.unlisten(),
                                None => ()
                            }
                            let mut sodium_ctx2 = sodium_ctx.clone();
                            let sodium_ctx2 = &mut sodium_ctx2;
                            *current_listener =
                                Some(
                                    ca
                                        .value_(sodium_ctx, trans)
                                        .listen2(
                                            sodium_ctx,
                                            out_node,
                                            trans,
                                            TransactionHandlerRef::new(
                                                sodium_ctx2,
                                                move |sodium_ctx, trans2, a| {
                                                    let out = out.clone();
                                                    out.send(sodium_ctx, trans2, a);
                                                }
                                            ),
                                            false,
                                            false
                                        )
                                );
                        }
                    );
                }
                let out_node = out.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
                let l1 = cca.value_(sodium_ctx, trans).listen_(sodium_ctx, out_node, h);
                out
                    .last_firing_only_(sodium_ctx, trans)
                    .unsafe_add_cleanup(l1)
                    .unsafe_add_cleanup(
                        Listener::new(
                            sodium_ctx,
                            move || {
                                let mut current_listener = (*current_listener).borrow_mut();
                                match current_listener.as_ref() {
                                    Some(current_listener) => {
                                        current_listener.unlisten();
                                    },
                                    None => ()
                                }
                                *current_listener = None;
                            }
                        )
                    )
                    .hold_lazy(sodium_ctx, za)
            }
        )
    }

    fn switch_s<CSA,SA>(sodium_ctx: &mut SodiumCtx, csa: &CSA) -> StreamImpl<A>
        where CSA: IsCellPrivate<SA>,
              SA: IsStreamPrivate<A> + Clone + 'static
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                CellImpl::switch_s_::<CSA,SA>(sodium_ctx, trans, csa)
            }
        )
    }

    fn switch_s_<CSA,SA>(sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, csa: &CSA) -> StreamImpl<A>
        where CSA: IsCellPrivate<SA>,
              SA: IsStreamPrivate<A> + Clone + 'static
    {
        let out = StreamWithSend::new(sodium_ctx);
        let h2;
        {
            let out = out.downgrade();
            h2 = TransactionHandlerRef::new(
                sodium_ctx,
                move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                    let out = out.clone();
                    out.send(sodium_ctx, trans2, a)
                }
            );
        }
        let out_node = out.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
        let current_listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(Some(
            csa.sample_no_trans_().listen2(
                sodium_ctx,
                out_node,
                trans,
                h2.clone(),
                false,
                false
            )
        )));
        let h1;
        {
            let current_listener = current_listener.clone();
            let out = out.downgrade();
            h1 = TransactionHandlerRef::new(
                sodium_ctx,
                move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, sa: &SA| {
                    let sodium_ctx = sodium_ctx.clone();
                    let trans3 = trans2.clone();
                    let out = out.clone();
                    let h2 = h2.clone();
                    let current_listener = current_listener.clone();
                    let sa = sa.clone();
                    trans2.last(
                        move || {
                            let mut sodium_ctx = sodium_ctx.clone();
                            let sodium_ctx = &mut sodium_ctx;
                            let mut trans3 = trans3.clone();
                            let trans3 = &mut trans3;
                            let out = out.upgrade().unwrap().clone();
                            let out_node = out.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
                            let mut current_listener = (*current_listener).borrow_mut();
                            match current_listener.as_ref() {
                                Some(current_listener) =>
                                    current_listener.unlisten(),
                                None => ()
                            }
                            *current_listener =
                                Some(
                                    sa.listen2(
                                        sodium_ctx,
                                        out_node,
                                        trans3,
                                        h2.clone(),
                                        true,
                                        false
                                    )
                                );
                        }
                    )
                }
            );
        }
        let out_node = out.stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
        let l1 = csa.updates_(trans).listen2(
            sodium_ctx,
            out_node,
            trans,
            h1,
            false,
            false
        );
        out
            .unsafe_add_cleanup(l1)
            .unsafe_add_cleanup(
                Listener::new(
                    sodium_ctx,
                    move || {
                        let mut current_listener = (*current_listener).borrow_mut();
                        match current_listener.as_ref() {
                            Some(current_listener) => {
                                current_listener.unlisten();
                            },
                            None => ()
                        }
                        *current_listener = None;
                    }
                )
            )
            .to_stream()
    }

    fn listen<F>(&self, sodium_ctx: &mut SodiumCtx, action: F) -> Listener
        where F: Fn(&A) + 'static
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                self.value_(sodium_ctx, trans)
                    .listen(sodium_ctx, action)
            }
        )
    }

    fn keep_alive<X:'static>(&self, sodium_ctx: &mut SodiumCtx, object: X) -> CellImpl<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                let tmp = self.sample_lazy_(sodium_ctx, trans);
                self.updates_(trans).keep_alive(sodium_ctx, object).hold_lazy_(sodium_ctx, trans, tmp)
            }
        )
    }
}

pub struct CellImpl<A> {
    pub data: Gc<GcCell<HasCellData<A>>>
}

pub struct CellData<A> {
    pub str: StreamImpl<A>,
    pub value: Option<A>,
    pub value_update: Option<A>,
    pub cleanup: Option<Listener>,
    pub lazy_init_value: Option<Lazy<A>>
}

impl<A> Trace for CellData<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.str.trace(f);
        match &self.cleanup {
            &Some(ref cleanup) => cleanup.trace(f),
            &None => ()
        }
    }
}

pub trait HasCellDataGc<A> {
    fn cell_data(&self) -> Gc<GcCell<HasCellData<A>>>;
}

impl<A: 'static> HasCellDataGc<A> for CellImpl<A> {
    fn cell_data(&self) -> Gc<GcCell<HasCellData<A>>> {
        self.data.clone().upcast(|x| x as &GcCell<HasCellData<A>>)
    }
}

pub trait HasCellData<A> {
    fn cell_data_ref(&self) -> &CellData<A>;
    fn cell_data_mut(&mut self) -> &mut CellData<A>;

    fn sample_no_trans_(&mut self) -> A where A: Clone + 'static {
        let data = self.cell_data_mut();
        if data.value.is_none() && data.lazy_init_value.is_some() {
            data.value = Some(data.lazy_init_value.as_ref().unwrap().get());
            data.lazy_init_value = None;
        }
        data.value.clone().unwrap()
    }
}

impl<A> HasCellData<A> for CellData<A> {
    fn cell_data_ref(&self) -> &CellData<A> {
        self
    }
    fn cell_data_mut(&mut self) -> &mut CellData<A> {
        self
    }
}

impl<A> Drop for CellData<A> {
    fn drop(&mut self) {
        match &self.cleanup {
            &Some(ref cleanup) => cleanup.unlisten(),
            &None => ()
        }
    }
}

impl<A: Clone + 'static,CA: HasCellDataGc<A>> IsCellPrivate<A> for CA {
    fn to_cell(&self) -> CellImpl<A> {
        CellImpl {
            data: self.cell_data()
        }
    }
}

impl<A> Clone for CellImpl<A> {
    fn clone(&self) -> Self {
        CellImpl {
            data: self.data.clone()
        }
    }
}

impl<A:'static + Clone> CellImpl<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx, value: A) -> CellImpl<A> {
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        CellImpl {
            data: sodium_ctx.new_gc(GcCell::new(
                CellData {
                    str: StreamImpl::new(sodium_ctx2),
                    value: Some(value),
                    value_update: None,
                    cleanup: None,
                    lazy_init_value: None
                }
            )).upcast(|x| x as &GcCell<HasCellData<A>>)
        }
    }

    pub fn new_(sodium_ctx: &mut SodiumCtx, str: StreamImpl<A>, init_value: Option<A>) -> CellImpl<A> {
        let r = CellImpl {
            data: sodium_ctx.new_gc(GcCell::new(
                CellData {
                    str: str.clone(),
                    value: init_value,
                    value_update: None,
                    cleanup: None,
                    lazy_init_value: None
                }
            )).upcast(|x| x as &GcCell<HasCellData<A>>)
        };
        let self_ = r.clone();
        Transaction::run_trans(
            sodium_ctx,
            move |sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction| {
                let self__ = self_.clone();
                self_.with_cell_data_mut(move |data| {
                    let self_ = self__.cell_data().downgrade();
                    let null_node = sodium_ctx.null_node();
                    let mut sodium_ctx2 = sodium_ctx.clone();
                    let sodium_ctx2 = &mut sodium_ctx2;
                    data.cleanup = Some(data.str.listen2(
                        sodium_ctx,
                        null_node,
                        trans1,
                        TransactionHandlerRef::new(
                            sodium_ctx2,
                            move |_sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                                let self_ = CellImpl { data: self_.upgrade().unwrap() };
                                let self__ = self_.clone();
                                self_.with_cell_data_mut(move |data| {
                                    if data.value_update.is_none() {
                                        trans2.last(
                                            move || {
                                                self__.with_cell_data_mut(|data| {
                                                    data.value = data.value_update.clone();
                                                    data.lazy_init_value = None;
                                                    data.value_update = None;
                                                });
                                            }
                                        );
                                    }
                                    data.value_update = Some(a.clone());
                                });
                            }
                        ),
                        false,
                        false
                    ));
                })
            }
        );
        r
    }
}

struct LazySample<A> {
    data: Rc<RefCell<LazySampleData<A>>>
}

impl<A> Clone for LazySample<A> {
    fn clone(&self) -> Self {
        LazySample {
            data: self.data.clone()
        }
    }
}

struct LazySampleData<A> {
    cell_op: Option<CellImpl<A>>,
    value_op: Option<A>
}

impl<A> LazySample<A> {
    fn new(cell: CellImpl<A>) -> LazySample<A> {
        LazySample {
            data: Rc::new(RefCell::new(
                LazySampleData {
                    cell_op: Some(cell),
                    value_op: None
                }
            ))
        }
    }
}

struct ApplyHandler<A,B> {
    data: Rc<RefCell<ApplyHandlerData<A,B>>>
}

impl<A,B> Clone for ApplyHandler<A,B> {
    fn clone(&self) -> Self {
        ApplyHandler {
            data: self.data.clone()
        }
    }
}

struct ApplyHandlerData<A,B> {
    f_op: Option<Rc<IsLambda1<A,B>>>,
    a_op: Option<A>,
    out: WeakStreamWithSend<Lazy<B>>
}

impl<A:'static + Clone,B:'static> ApplyHandler<A,B> {
    fn new(out: WeakStreamWithSend<Lazy<B>>) -> ApplyHandler<A,B> {
        ApplyHandler {
            data: Rc::new(RefCell::new(
                ApplyHandlerData {
                    f_op: None,
                    a_op: None,
                    out: out
                }
            ))
        }
    }

    fn run(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) {
        let out = self.data.deref().borrow().out.clone();
        let out_node;
        match out.stream.upgrade() {
            Some(stream) => {
                out_node = stream.data.clone().upcast(|x| x as &GcCell<HasNode>);
            },
            None => return
        }
        let self_ = self.clone();
        trans.prioritized(
            sodium_ctx,
            out_node,
            HandlerRefMut::new(
                move |sodium_ctx, trans2| {
                    let data = self_.data.deref().borrow();
                    match &data.f_op {
                        &Some(ref f) => {
                            match &data.a_op {
                                &Some(ref a) => {
                                    let f2 = f.clone();
                                    let a2 = a.clone();
                                    let b = Lazy::new(move || (*f2).apply(&a2));
                                    data.out.send(sodium_ctx, trans2, &b);
                                },
                                &None => ()
                            }
                        },
                        &None => ()
                    }
                }
            )
        );
    }
}
