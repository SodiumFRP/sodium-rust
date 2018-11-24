use sodium::impl_::Cell;
use sodium::impl_::Dep;
use sodium::impl_::IsLambda0;
use sodium::impl_::IsLambdaMut0;
use sodium::impl_::IsLambda1;
use sodium::impl_::IsLambda2;
use sodium::impl_::IsLambda3;
use sodium::impl_::IsLambda4;
use sodium::impl_::IsLambda5;
use sodium::impl_::IsLambda6;
use sodium::impl_::Lambda;
use sodium::impl_::Listener;
use sodium::impl_::MemoLazy;
use sodium::impl_::Node;
use sodium::impl_::SodiumCtx;
use sodium::impl_::StreamLoop;
use sodium::gc::Finalize;
use sodium::gc::Gc;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub struct Stream<A> {
    pub data: Gc<UnsafeCell<StreamData<A>>>
}

pub struct StreamData<A> {
    pub value: Gc<UnsafeCell<Option<MemoLazy<A>>>>,
    pub node: Node
}

impl<A: Trace> Trace for StreamData<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.value.trace(f);
        self.node.trace(f);
    }
}

impl<A: Finalize> Finalize for StreamData<A> {
    fn finalize(&mut self) {
        self.value.finalize();
        self.node.finalize();
    }
}

impl<A: Clone + Trace + Finalize + 'static> Stream<Option<A>> {
    pub fn filter_option(&self) -> Stream<A> {
        let sodium_ctx = self._node().sodium_ctx().clone();
        let sodium_ctx = &sodium_ctx;
        let self_ = self.clone();
        let sodium_ctx2 = sodium_ctx.clone();
        let update_deps = vec![self.to_dep()];
        Stream::_new(
            sodium_ctx,
            Lambda::new(
                move || {
                    let sodium_ctx = &sodium_ctx2;
                    match self_.peek_value() {
                        Some(thunk) =>
                            match thunk.get() {
                                Some(val) => {
                                    let val = val.clone();
                                    Some(sodium_ctx.new_lazy(move || val.clone()))
                                },
                                None => None
                            },
                        None => None
                    }
                },
                update_deps
            ),
            vec![self._node().clone()],
            || {},
            "Stream::filter_option"
        )
    }
}

impl<A: Clone + Trace + Finalize + 'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream::_new(
            sodium_ctx,
            || None,
            Vec::new(),
            || {},
            "Stream::new"
        )
    }

    pub fn _new<UPDATE:IsLambda0<Option<MemoLazy<A>>>+'static, CLEANUP: FnMut()+'static>(
        sodium_ctx: &SodiumCtx,
        update: UPDATE,
        deps: Vec<Node>,
        cleanup: CLEANUP,
        desc: &'static str
    ) -> Stream<A> {
        let mut gc_ctx = sodium_ctx.gc_ctx();
        let init_firing = update.apply();
        let value = gc_ctx.new_gc_with_desc(UnsafeCell::new(init_firing), String::from(desc) + "_value");
        let mut update_deps = update.deps();
        update_deps.push(Dep { gc_dep: value.to_dep() });
        let sodium_ctx2 = sodium_ctx.clone();
        Stream {
            data: gc_ctx.new_gc_with_desc(UnsafeCell::new(StreamData {
                value: value.clone(),
                node: Node::new(
                    sodium_ctx,
                    move || {
                        let sodium_ctx = &sodium_ctx2;
                        let val_op = update.apply();
                        if let Some(val) = val_op {
                            {
                                let value = unsafe { &mut *(*value).get() };
                                *value = Some(val);
                            }
                            let value = value.clone();
                            sodium_ctx.post(move || {
                                let value = unsafe { &mut *(*value).get() };
                                *value = None;
                            });
                            true
                        } else {
                            false
                        }
                    },
                    update_deps,
                    deps,
                    cleanup,
                    String::from(desc) + "_node"
                )
            }), String::from(desc))
        }
    }

    pub fn _value(&self) -> &Gc<UnsafeCell<Option<MemoLazy<A>>>> {
        let data = unsafe { &*(*self.data).get() };
        &data.value
    }

    pub fn _node(&self) -> &Node {
        let data = unsafe { &*(*self.data).get() };
        &data.node
    }

    pub fn to_dep(&self) -> Dep {
        Dep { gc_dep: self.data.to_dep() }
    }

    pub fn peek_value(&self) -> Option<MemoLazy<A>> {
        let val_op = unsafe { &*(*self._value()).get() };
        val_op.clone()
    }

    pub fn map<B: Clone + Trace + Finalize + 'static,F:IsLambda1<A,B> + 'static>(
        &self,
        f: F
    ) -> Stream<B> {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let self_ = self.clone();
        let f = Rc::new(f);
        let mut update_deps = f.deps();
        update_deps.push(self.to_dep());
        let sodium_ctx2 = sodium_ctx.clone();
        Stream::_new(
            sodium_ctx,
            Lambda::new(
                move || {
                    let sodium_ctx = &sodium_ctx2;
                    self_.peek_value().map(|thunk| {
                        let f = f.clone();
                        sodium_ctx.new_lazy(move || f.apply(thunk.get()))
                    })
                },
                update_deps
            ),
            vec![self._node().clone()],
            || {},
            "Stream::map"
        )
    }

    pub fn hold(&self, a: A) -> Cell<A> {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let self_ = self.clone();
        let deps = vec![self_._node().clone()];
        let init_value;
        if let Some(value) = self_.peek_value() {
            init_value = value;
        } else {
            init_value = sodium_ctx.new_lazy(move || a.clone());
        }
        let update_deps = vec![self.to_dep()];
        Cell::_new(
            sodium_ctx,
            init_value,
            Lambda::new(
                move || {
                    self_.peek_value()
                },
                update_deps
            ),
            deps,
            || {},
            "Stream::hold"
        )
    }

    pub fn hold_lazy(&self, a: MemoLazy<A>) -> Cell<A> {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let self_ = self.clone();
        let deps = vec![self_._node().clone()];
        let update_deps = vec![self.to_dep()];
        Cell::_new(
            sodium_ctx,
            a,
            Lambda::new(
                move || {
                    self_.peek_value()
                },
                update_deps
            ),
            deps,
            || {},
            "Stream::hold_lazy"
        )
    }

    pub fn filter<PRED:IsLambda1<A,bool> + 'static>(&self, pred: PRED) -> Stream<A> {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let self_ = self.clone();
        let pred = Rc::new(pred);
        let sodium_ctx2 = sodium_ctx.clone();
        let mut update_deps = pred.deps();
        update_deps.push(self.to_dep());
        Stream::_new(
            sodium_ctx,
            Lambda::new(
                move || {
                    let sodium_ctx = &sodium_ctx2;
                    let val_op = self_.peek_value();
                    if let Some(val) = val_op {
                        let val = val.get();
                        if pred.apply(val) {
                            let val = val.clone();
                            Some(sodium_ctx.new_lazy(move || val.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
                update_deps
            ),
            vec![self._node().clone()],
            || {},
            "Stream::filter"
        )
    }

    pub fn merge<FN:Fn(&A,&A)->A+'static>(&self, sa: Stream<A>, f: FN) -> Stream<A> {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let f = Rc::new(f);
        let node_deps = vec![self._node().clone(), sa._node().clone()];
        let self_ = self.clone();
        let sodium_ctx2 = sodium_ctx.clone();
        let mut update_deps = f.deps();
        update_deps.push(self.to_dep());
        update_deps.push(sa.to_dep());
        Stream::_new(
            sodium_ctx,
            Lambda::new(
                move || {
                    let sodium_ctx = &sodium_ctx2;
                    let lhs_op = self_.peek_value();
                    let rhs_op = sa.peek_value();
                    let f = f.clone();
                    match lhs_op {
                        Some(lhs) =>
                            match rhs_op {
                                Some(rhs) =>
                                    Some(sodium_ctx.new_lazy(move || f(lhs.get(), rhs.get()))),
                                None => Some(lhs)
                            },
                        None =>
                            match rhs_op {
                                Some(rhs) => Some(rhs),
                                None => None
                            }
                    }
                },
                update_deps
            ),
            node_deps,
            || {},
            "Stream::merge"
        )
    }

    pub fn gate(&self, ca: Cell<bool>) -> Stream<A> {
        self.filter(move |_: &A| ca.sample_no_trans())
    }

    pub fn collect_lazy<B,S,F>(&self, init_state: MemoLazy<S>, f: F) -> Stream<B>
        where B: Clone + Trace + Finalize + 'static,
              S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,(B,S)> + 'static
    {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let sodium_ctx2 = sodium_ctx.clone();
        sodium_ctx2.transaction(|| {
            let ea = self.clone();
            let es = StreamLoop::new(sodium_ctx);
            let s = es.to_stream().hold_lazy(init_state);
            let ebs = ea.snapshot2(s, f);
            let eb = ebs.map(|(ref a,ref b):&(B,S)| a.clone());
            let es_out = ebs.map(|(ref a,ref b):&(B,S)| b.clone());
            es.loop_(es_out);
            eb
        })
    }

    pub fn accum_lazy<S,F>(&self, init_state: MemoLazy<S>, f: F) -> Cell<S>
        where S: Clone + Trace + Finalize + 'static,
              F: IsLambda2<A,S,S> + 'static
    {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let sodium_ctx2 = sodium_ctx.clone();
        sodium_ctx.transaction(|| {
            let sodium_ctx = &sodium_ctx2;
            let es: StreamLoop<S> = StreamLoop::new(sodium_ctx);
            let s = es.to_stream().hold_lazy(init_state);
            let es_out = self.snapshot2(s.clone(), f);
            es.loop_(es_out);
            s
        })
    }

    pub fn once(&self) -> Stream<A> {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let sodium_ctx2 = sodium_ctx.clone();
        sodium_ctx.transaction(|| {
            let sodium_ctx = &sodium_ctx2;
            let mut gc_ctx = sodium_ctx.gc_ctx();
            let gc_ctx = &mut gc_ctx;
            let init_firing = self.peek_value();
            let init_firing_is_some = init_firing.is_some();
            let value = gc_ctx.new_gc_with_desc(UnsafeCell::new(init_firing), String::from("Stream::once_value"));
            let self_ = self.clone();
            let deps = if init_firing_is_some { Vec::new() } else { vec![self_._node().clone()] };
            let node_self: Rc<UnsafeCell<Option<Node>>> = Rc::new(UnsafeCell::new(None));
            let node;
            {
                let value = value.clone();
                let node_self = node_self.clone();
                let sodium_ctx2 = sodium_ctx.clone();
                node = Node::new(
                    &sodium_ctx,
                    move || {
                        let sodium_ctx = &sodium_ctx2;
                        {
                            let value = unsafe { &mut *(*value).get() };
                            *value = self_.peek_value();
                            if value.is_some() {
                                let node_self = unsafe { &*(*node_self).get() };
                                if let &Some(ref node_self2) = node_self {
                                    node_self2.remove_all_dependencies();
                                }
                            }
                        }
                        let value = value.clone();
                        sodium_ctx.post(move || {
                            let value = unsafe { &mut *(*value).get() };
                            *value = None;
                        });
                        true
                    },
                    Vec::new(),
                    deps,
                    || {},
                    String::from("Stream::once_node")
                );
            }
            {
                let node_self = unsafe { &mut *(*node_self).get() };
                *node_self = Some(node.clone());
            }
            node.add_update_deps(vec![node.to_dep()]);
            Stream {
                data: gc_ctx.new_gc_with_desc(UnsafeCell::new(StreamData {
                    value,
                    node
                }), String::from("Stream::once"))
            }
        })
    }

    pub fn snapshot<B>(&self, cb: Cell<B>) -> Stream<B> where B: Trace + Finalize + Clone + 'static {
        let deps = vec![cb.to_dep()];
        self.map(Lambda::new(move |_a: &A| cb.sample_no_trans(), deps))
    }

    pub fn snapshot2<B,C,FN:IsLambda2<A,B,C> + 'static>(&self, cb: Cell<B>, f: FN) -> Stream<C> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static {
        let mut deps = f.deps();
        deps.push(cb.to_dep());
        self.map(Lambda::new(move |a: &A| f.apply(a, &cb.sample_no_trans()), deps))
    }

    pub fn snapshot3<B,C,D,FN:IsLambda3<A,B,C,D> + 'static>(&self, cb: Cell<B>, cc: Cell<C>, f: FN) -> Stream<D> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static {
        let mut deps = f.deps();
        deps.push(cb.to_dep());
        deps.push(cc.to_dep());
        self.map(Lambda::new(move |a: &A| f.apply(a, &cb.sample_no_trans(), &cc.sample_no_trans()), deps))
    }

    pub fn snapshot4<B,C,D,E,FN:IsLambda4<A,B,C,D,E> + 'static>(&self, cb: Cell<B>, cc: Cell<C>, cd: Cell<D>, f: FN) -> Stream<E> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static {
        let mut deps = f.deps();
        deps.push(cb.to_dep());
        deps.push(cc.to_dep());
        deps.push(cd.to_dep());
        self.map(Lambda::new(move |a: &A| f.apply(a, &cb.sample_no_trans(), &cc.sample_no_trans(), &cd.sample_no_trans()), deps))
    }

    pub fn snapshot5<B,C,D,E,F,FN:IsLambda5<A,B,C,D,E,F> + 'static>(&self, cb: Cell<B>, cc: Cell<C>, cd: Cell<D>, ce: Cell<E>, f: FN) -> Stream<F> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, F: Trace + Finalize + Clone + 'static {
        let mut deps = f.deps();
        deps.push(cb.to_dep());
        deps.push(cc.to_dep());
        deps.push(cd.to_dep());
        deps.push(ce.to_dep());
        self.map(Lambda::new(move |a: &A| f.apply(a, &cb.sample_no_trans(), &cc.sample_no_trans(), &cd.sample_no_trans(), &ce.sample_no_trans()), deps))
    }

    pub fn snapshot6<B,C,D,E,F,G,FN:IsLambda6<A,B,C,D,E,F,G> + 'static>(&self, cb: Cell<B>, cc: Cell<C>, cd: Cell<D>, ce: Cell<E>, cf: Cell<F>, f: FN) -> Stream<G> where B: Trace + Finalize + Clone + 'static, C: Trace + Finalize + Clone + 'static, D: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, E: Trace + Finalize + Clone + 'static, F: Trace + Finalize + Clone + 'static, G: Trace + Finalize + Clone + 'static {
        let mut deps = f.deps();
        deps.push(cb.to_dep());
        deps.push(cc.to_dep());
        deps.push(cd.to_dep());
        deps.push(ce.to_dep());
        deps.push(cf.to_dep());
        self.map(Lambda::new(move |a: &A| f.apply(a, &cb.sample_no_trans(), &cc.sample_no_trans(), &cd.sample_no_trans(), &ce.sample_no_trans(), &cf.sample_no_trans()), deps))
    }

    pub fn add_cleanup<CLEANUP:IsLambdaMut0<()>+'static>(&self, cleanup: CLEANUP) {
        self._node().add_cleanup(cleanup);
    }

    pub fn listen<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self._listen(callback, false)
    }

    pub fn listen_weak<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK
    ) -> Listener {
        self._listen(callback, true)
    }

    pub fn _listen<CALLBACK:FnMut(&A)+'static>(
        &self,
        callback: CALLBACK,
        weak: bool
    ) -> Listener {
        let sodium_ctx = self._node().sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        let callback = Rc::new(UnsafeCell::new(callback));
        let self_ = self.clone();
        {
            let self_ = self_.clone();
            let callback = callback.clone();
            let value_op = self_.peek_value();
            if let Some(value) = value_op {
                sodium_ctx.pre(move || {
                    let callback = unsafe { &mut *(*callback).get() };
                    (*callback)(value.get());
                });
            }
        }
        let update_deps = vec![self.to_dep()];
        Listener::new(Node::new(
            sodium_ctx,
            move || {
                let callback = unsafe { &mut *(*callback).get() };
                let value_op = self_.peek_value();
                if let Some(value) = value_op {
                    (*callback)(value.get());
                }
                return false;
            },
            update_deps,
            vec![self._node().clone()],
            || {},
            String::from("Stream::listen_node")
        ), weak)
    }
}

impl<A:Clone + 'static> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            data: self.data.clone()
        }
    }
}

impl<A:Trace> Trace for Stream<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.data.trace(f);
    }
}

impl<A:Finalize> Finalize for Stream<A> {
    fn finalize(&mut self) {
        self.data.finalize();
    }
}