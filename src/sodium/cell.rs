use sodium::HandlerRefMut;
use sodium::HasNode;
use sodium::Node;
use sodium::IsStream;
use sodium::Lazy;
use sodium::Listener;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use sodium::WeakStreamWithSend;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cell::Ref;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::Deref;

pub trait IsCell<A: Clone + 'static> {
    fn to_cell(&self) -> Cell<A>;

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
            |sodium_ctx, trans| {
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
            trans.last(
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

    fn updates_(&self, trans: &mut Transaction) -> Stream<A> {
        self.with_cell_data_ref(|data| data.str.clone())
    }

    fn value_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> Stream<A> {
        let s_spark = StreamWithSend::new(sodium_ctx);
        let s_spark_node = s_spark.stream.data.clone() as Rc<RefCell<HasNode>>;
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
        s_initial.merge(sodium_ctx, &self.updates_(trans), |_,a| a.clone())
    }

    fn map<F,B:'static + Clone>(&self, sodium_ctx: &mut SodiumCtx, f: F) -> Cell<B> where F: Fn(&A)->B + 'static {
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans| {
                let f2 = Rc::new(f);
                let f3 = f2.clone();
                let tmp =
                    self.sample_lazy_(sodium_ctx, trans)
                        .map(move |a| f2(a));
                self.updates_(trans)
                    .map(sodium_ctx, move |a| f3(a))
                    .hold_lazy_(
                        sodium_ctx,
                        trans,
                        tmp
                    )
            }
        )
    }

    fn lift2<B,C,CB,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, f: F) -> Cell<C>
        where B: Clone + 'static,
              C: Clone + 'static,
              CB: IsCell<B>,
              F: Fn(&A,&B)->C + 'static
    {
        let f = Rc::new(f);
        let curried_f = move |a: &A| {
            let a = a.clone();
            let f = f.clone();
            Rc::new(move |b: &B| f(&a, b))
        };
        let cf = self.map(sodium_ctx, curried_f);
        Cell::apply(sodium_ctx, &cf, cb)
    }

    fn lift3<B,C,D,CB,CC,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, f: F) -> Cell<D>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              CB: IsCell<B>,
              CC: IsCell<C>,
              F: Fn(&A,&B,&C)->D + 'static
    {
        let f = Rc::new(f);
        let slightly_curried_f = move |a: &A, b: &B| {
            let a = a.clone();
            let b = b.clone();
            let f = f.clone();
            Rc::new(move |c: &C| f(&a, &b, c))
        };
        let cf = self.lift2(sodium_ctx, cb, slightly_curried_f);
        Cell::apply(sodium_ctx, &cf, cc)
    }

    fn lift4<B,C,D,E,CB,CC,CD,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, f: F) -> Cell<E>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              CB: IsCell<B>,
              CC: IsCell<C>,
              CD: IsCell<D>,
              F: Fn(&A,&B,&C,&D)->E + 'static
    {
        let f = Rc::new(f);
        let slightly_curried_f = move |a: &A, b: &B| {
            let a = a.clone();
            let b = b.clone();
            let f = f.clone();
            Rc::new(move |c: &C, d: &D| f(&a, &b, c, d))
        };
        let cf = self.lift2(sodium_ctx, cb, slightly_curried_f);
        Cell::apply2_(sodium_ctx, &cf, cc, cd)
    }

    fn lift5<B,C,D,E,F,CB,CC,CD,CE,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> Cell<F>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: Clone + 'static,
              CB: IsCell<B>,
              CC: IsCell<C>,
              CD: IsCell<D>,
              CE: IsCell<E>,
              FN: Fn(&A,&B,&C,&D,&E)->F + 'static
    {
        let f = Rc::new(f);
        let slightly_curried_f = move |a: &A, b: &B, c: &C| {
            let a = a.clone();
            let b = b.clone();
            let c = c.clone();
            let f = f.clone();
            Rc::new(move |d: &D, e: &E| f(&a, &b, &c, d, e))
        };
        let cf = self.lift3(sodium_ctx, cb, cc, slightly_curried_f);
        Cell::apply2_(sodium_ctx, &cf, cd, ce)
    }

    fn lift6<B,C,D,E,F,G,CB,CC,CD,CE,CF,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, fn_: FN) -> Cell<G>
        where B: Clone + 'static,
              C: Clone + 'static,
              D: Clone + 'static,
              E: Clone + 'static,
              F: Clone + 'static,
              G: Clone + 'static,
              CB: IsCell<B>,
              CC: IsCell<C>,
              CD: IsCell<D>,
              CE: IsCell<E>,
              CF: IsCell<F>,
              FN: Fn(&A,&B,&C,&D,&E,&F)->G + 'static
    {
        let fn_ = Rc::new(fn_);
        let slightly_curried_f = move |a: &A, b: &B, c: &C| {
            let a = a.clone();
            let b = b.clone();
            let c = c.clone();
            let fn_ = fn_.clone();
            Rc::new(move |d: &D, e: &E, f: &F| fn_(&a, &b, &c, d, e, f))
        };
        let cfn = self.lift3(sodium_ctx, cb, cc, slightly_curried_f);
        Cell::apply3_(sodium_ctx, &cfn, cd, ce, cf)
    }

    fn apply<CF,CA,F,B:'static + Clone>(sodium_ctx: &mut SodiumCtx, cf: &CF, ca: &CA) -> Cell<B>
    where
    CF: IsCell<Rc<F>>,
    CA: IsCell<A>,
    F: Fn(&A)->B + 'static
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                let out: StreamWithSend<Lazy<B>> = StreamWithSend::new(sodium_ctx);
                let out_target = out.stream.data.clone() as Rc<RefCell<HasNode>>;
                let mut in_target = Rc::new(RefCell::new(Node::new(sodium_ctx, 0))) as Rc<RefCell<HasNode>>;
                let (node_target,_) = ((*in_target).borrow_mut().node_mut() as &mut HasNode).link_to::<Lazy<B>>(
                    sodium_ctx,
                    out_target,
                    TransactionHandlerRef::new(
                        |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &Lazy<B>| {}
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
                                move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, f: &Rc<F>| {
                                    (*h.data).borrow_mut().f_op = Some(f.clone());
                                    let run_it = match &(*h.data).borrow_mut().a_op {
                                        &Some(ref a) => true,
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
                            move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                                (*h.data).borrow_mut().a_op = Some(a.clone());
                                let run_it = match &(*h.data).borrow_mut().f_op {
                                    &Some(ref f) => true,
                                    &None => false
                                };
                                if run_it {
                                    h.run(sodium_ctx, trans2)
                                }
                            }
                        ));
                let cf: Cell<Rc<F>> = cf.to_cell();
                let ca: Cell<A> = ca.to_cell();
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
                    .map(sodium_ctx, |lazy_b| lazy_b.get())
                    .hold_lazy(sodium_ctx, Lazy::new(move || {
                        cf.sample_no_trans_()(&ca.sample_no_trans_())
                    }))
            }
        )
    }

    fn apply2_<CF,CA,CB,B,C,F>(sodium_ctx: &mut SodiumCtx, cf: &CF, ca: &CA, cb: &CB) -> Cell<C>
        where CF: IsCell<Rc<F>>,
              CA: IsCell<A>,
              CB: IsCell<B>,
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
        let cf3 = Cell::apply(sodium_ctx, &cf2, ca);
        Cell::apply(sodium_ctx, &cf3, cb)
    }

    fn apply3_<CF,CA,CB,CC,B,C,D,F>(sodium_ctx: &mut SodiumCtx, cf: &CF, ca: &CA, cb: &CB, cc: &CC) -> Cell<D>
        where CF: IsCell<Rc<F>>,
              CA: IsCell<A>,
              CB: IsCell<B>,
              CC: IsCell<C>,
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
        let cf3 = Cell::apply2_(sodium_ctx, &cf2, ca, cb);
        Cell::apply(sodium_ctx, &cf3, cc)
    }

    fn switch_c<CCA,CA>(sodium_ctx: &mut SodiumCtx, cca: &CCA) -> Cell<A>
        where CCA: IsCell<CA>,
               CA: IsCell<A> + Clone + 'static
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
                        move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, ca: &CA| {
                            let out = out.clone();
                            let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
                            let mut current_listener = (*current_listener).borrow_mut();
                            match current_listener.as_ref() {
                                Some(current_listener) =>
                                    current_listener.unlisten(),
                                None => ()
                            }
                            *current_listener =
                                Some(
                                    ca
                                        .value_(sodium_ctx, trans)
                                        .listen2(
                                            sodium_ctx,
                                            out_node,
                                            trans,
                                            TransactionHandlerRef::new(
                                                move |sodium_ctx, trans2, a| {
                                                    let mut out = out.clone();
                                                    out.send(sodium_ctx, trans2, a);
                                                }
                                            ),
                                            false
                                        )
                                );
                        }
                    );
                }
                let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
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

    fn switch_s<CSA,SA>(sodium_ctx: &mut SodiumCtx, csa: &CSA) -> Stream<A>
        where CSA: IsCell<SA>,
              SA: IsStream<A> + Clone + 'static
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                Cell::switch_s_::<CSA,SA>(sodium_ctx, trans, csa)
            }
        )
    }

    fn switch_s_<CSA,SA>(sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, csa: &CSA) -> Stream<A>
        where CSA: IsCell<SA>,
              SA: IsStream<A> + Clone + 'static
    {
        let out = StreamWithSend::new(sodium_ctx);
        let h2;
        {
            let out = out.clone();
            h2 = TransactionHandlerRef::new(
                move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                    let mut out = out.clone();
                    out.send(sodium_ctx, trans2, a)
                }
            );
        }
        let current_listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
        let h1;
        {
            let current_listener = current_listener.clone();
            let out = out.clone();
            h1 = TransactionHandlerRef::new(
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
                            let out = out.clone();
                            let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
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
                                        true
                                    )
                                );
                        }
                    )
                }
            );
        }
        let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
        let l1 = csa.updates_(trans).listen2(
            sodium_ctx,
            out_node,
            trans,
            h1,
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
}

pub struct Cell<A> {
    pub data: Rc<RefCell<HasCellData<A>>>
}

pub struct CellData<A> {
    pub str: Stream<A>,
    pub value: Option<A>,
    pub value_update: Option<A>,
    pub cleanup: Option<Listener>,
    pub lazy_init_value: Option<Lazy<A>>
}

pub trait HasCellDataRc<A> {
    fn cell_data(&self) -> Rc<RefCell<HasCellData<A>>>;
}

impl<A: 'static> HasCellDataRc<A> for Cell<A> {
    fn cell_data(&self) -> Rc<RefCell<HasCellData<A>>> {
        self.data.clone() as Rc<RefCell<HasCellData<A>>>
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

impl<A: Clone + 'static,CA: HasCellDataRc<A>> IsCell<A> for CA {
    fn to_cell(&self) -> Cell<A> {
        Cell {
            data: self.cell_data()
        }
    }
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            data: self.data.clone()
        }
    }
}

impl<A:'static + Clone> Cell<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx, value: A) -> Cell<A> {
        Cell {
            data: Rc::new(RefCell::new(
                CellData {
                    str: Stream::new(sodium_ctx),
                    value: Some(value),
                    value_update: None,
                    cleanup: None,
                    lazy_init_value: None
                }
            ))
        }
    }

    pub fn new_(sodium_ctx: &mut SodiumCtx, str: Stream<A>, init_value: Option<A>) -> Cell<A> {
        let r = Cell {
            data: Rc::new(RefCell::new(
                CellData {
                    str: str,
                    value: init_value,
                    value_update: None,
                    cleanup: None,
                    lazy_init_value: None
                }
            ))
        };
        let self_ = r.clone();
        Transaction::run_trans(
            sodium_ctx,
            move |sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction| {
                let self__ = self_.clone();
                self_.with_cell_data_mut(move |data| {
                    let self_ = Rc::downgrade(&self__.cell_data());
                    let null_node = sodium_ctx.null_node();
                    data.cleanup = Some(data.str.listen2(
                        sodium_ctx,
                        null_node,
                        trans1,
                        TransactionHandlerRef::new(
                            move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                                let self_ = Cell { data: self_.upgrade().unwrap() };
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
    cell_op: Option<Cell<A>>,
    value_op: Option<A>
}

impl<A> LazySample<A> {
    fn new(cell: Cell<A>) -> LazySample<A> {
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
    f_op: Option<Rc<Fn(&A)->B>>,
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
                out_node = stream.data.clone() as Rc<RefCell<HasNode>>;
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
                                    let b = Lazy::new(move || (*f2)(&a2));
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
