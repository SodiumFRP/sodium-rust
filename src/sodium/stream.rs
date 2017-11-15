use sodium::Cell;
use sodium::CoalesceHandler;
use sodium::Dep;
use sodium::HandlerRefMut;
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

pub struct Stream<A> {
    pub data: Gc<RefCell<StreamData<A>>>
}

pub struct WeakStream<A> {
    pub data: GcWeak<RefCell<StreamData<A>>>
}

pub trait IsStream<A: Clone + 'static> {
    fn to_stream_ref(&self) -> &Stream<A>;

    fn to_stream(&self) -> Stream<A> {
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

    fn weak(&self, sodium_ctx: &mut SodiumCtx) -> Stream<A> {
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

    fn map<B:'static + Clone,F>(&self, sodium_ctx: &mut SodiumCtx, f: F) -> Stream<B>
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

    fn map_to<B:'static + Clone>(&self, sodium_ctx: &mut SodiumCtx, b: B) -> Stream<B> {
        return self.map(sodium_ctx, move |_: &A| b.clone());
    }

    fn hold(&self, sodium_ctx: &mut SodiumCtx, init_value: A) -> Cell<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, _trans| Cell::new_(sodium_ctx, self.to_stream_ref().clone(), Some(init_value))
        )
    }

    fn hold_lazy(&self, sodium_ctx: &mut SodiumCtx, initial_value: Lazy<A>) -> Cell<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans|
                self.hold_lazy_(sodium_ctx, trans, initial_value)
        )
    }

    fn hold_lazy_(&self, sodium_ctx: &mut SodiumCtx, _trans: &mut Transaction, initial_value: Lazy<A>) -> Cell<A> {
        LazyCell::new(sodium_ctx, self.to_stream_ref().clone(), initial_value).to_cell()
    }

    fn snapshot_to<CB,B>(&self, sodium_ctx: &mut SodiumCtx, c: &CB) -> Stream<B> where CB: IsCell<B>, B: Clone + 'static {
        self.snapshot(sodium_ctx, c, |_a: &A, b: &B| b.clone())
    }

    fn snapshot<CB,B,C,F>(&self, sodium_ctx: &mut SodiumCtx, c: &CB, f: F) -> Stream<C>
        where CB: IsCell<B>,
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

    fn snapshot2<CB,CC,B,C,D,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, f: F) -> Stream<D>
        where CB: IsCell<B>,
              CC: IsCell<C>,
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

    fn snapshot3<CB,CC,CD,B,C,D,E,F>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, f: F) -> Stream<E>
        where CB: IsCell<B>,
              CC: IsCell<C>,
              CD: IsCell<D>,
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

    fn snapshot4<CB,CC,CD,CE,B,C,D,E,F,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, f: FN) -> Stream<F>
        where CB: IsCell<B>,
              CC: IsCell<C>,
              CD: IsCell<D>,
              CE: IsCell<E>,
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

    fn snapshot5<CB,CC,CD,CE,CF,B,C,D,E,F,G,FN>(&self, sodium_ctx: &mut SodiumCtx, cb: &CB, cc: &CC, cd: &CD, ce: &CE, cf: &CF, f: FN) -> Stream<G>
        where CB: IsCell<B>,
              CC: IsCell<C>,
              CD: IsCell<D>,
              CE: IsCell<E>,
              CF: IsCell<F>,
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

    fn or_else<SA>(&self, sodium_ctx: &mut SodiumCtx, s: &SA) -> Stream<A> where SA: IsStream<A> {
        self.merge(sodium_ctx, s, |a,_| a.clone())
    }

    fn merge_<SA>(&self, sodium_ctx: &mut SodiumCtx, s: &SA) -> Stream<A> where SA: IsStream<A> {
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

    fn merge<SA,F>(&self, sodium_ctx: &mut SodiumCtx, s: &SA, f: F) -> Stream<A> where SA: IsStream<A>, F: Fn(&A,&A)->A + 'static {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction| {
                self.merge_(sodium_ctx, s).coalesce_(sodium_ctx,trans, f)
            }
        )
    }

    fn coalesce_<F>(&self, sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction, f: F) -> Stream<A> where F: Fn(&A,&A)->A + 'static {
        let out = StreamWithSend::new(sodium_ctx);
        let h = CoalesceHandler::new(f, out.downgrade());
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        let l = self.listen2(
            sodium_ctx,
            out.to_stream_ref().data.clone().upcast(|x| x as &RefCell<HasNode>),
            trans1,
            h.to_transaction_handler(sodium_ctx2),
            false,
            false
        );
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn last_firing_only_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> Stream<A> {
        self.coalesce_(sodium_ctx, trans, |_,a| a.clone())
    }

    fn filter<F>(&self, sodium_ctx: &mut SodiumCtx, predicate: F) -> Stream<A> where F: Fn(&A)->bool + 'static {
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
                        if predicate(a) {
                            out.send(sodium_ctx, trans, a);
                        }
                    }
                )
            );
        }
        out.unsafe_add_cleanup(l).to_stream()
    }

    fn filter_option<S>(sodium_ctx: &mut SodiumCtx, self_: &S) -> Stream<A> where S: IsStream<Option<A>> {
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

    fn gate<CB>(&self, sodium_ctx: &mut SodiumCtx, c: &CB) -> Stream<A> where CB: IsCell<bool> {
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
        Stream::filter_option(sodium_ctx, &s)
    }

    fn collect<B,S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: S, f: F) -> Stream<B>
        where B: Clone + 'static,
              S: Clone + 'static,
              F: Fn(&A,&S)->(B,S) + 'static
    {
        self.collect_lazy(sodium_ctx, Lazy::new(move || init_state.clone()), f)
    }

    fn collect_lazy<B,S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: Lazy<S>, f: F) -> Stream<B>
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
                let s = es.hold_lazy(sodium_ctx, init_state.clone());
                let f = f.clone();
                let f2 = move |a: &A, s: &S| f(a,s);
                let ebs = ea.snapshot(sodium_ctx, &s, f2);
                let eb = ebs.map(sodium_ctx, |&(ref b,ref _s): &(B,S)| b.clone());
                let es_out = ebs.map(sodium_ctx, |&(ref _b,ref s): &(B,S)| s.clone());
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                es.loop_(sodium_ctx, es_out.weak(sodium_ctx2));
                eb.keep_alive(sodium_ctx, es_out)
            }
        )
    }

    fn accum<S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: S, f: F) -> Cell<S>
        where S: Clone + 'static,
              F: Fn(&A,&S)->S + 'static
    {
        self.accum_lazy(sodium_ctx, Lazy::new(move || init_state.clone()), f)
    }

    fn accum_lazy<S,F>(&self, sodium_ctx: &mut SodiumCtx, init_state: Lazy<S>, f: F) -> Cell<S>
        where S: Clone + 'static,
              F: Fn(&A,&S)->S + 'static
    {
        let ea = self.to_stream_ref().clone();
        let f = Rc::new(f);
        Transaction::run(
            sodium_ctx,
            move |sodium_ctx| {
                let mut es = StreamLoop::new(sodium_ctx);
                let s = es.hold_lazy(sodium_ctx, init_state.clone());
                let f = f.clone();
                let f2 = move |a: &A,s: &S| f(a,s);
                let es_out = ea.snapshot(sodium_ctx, &s, f2);
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                es.loop_(sodium_ctx, es_out.weak(sodium_ctx2));
                es_out.hold_lazy(sodium_ctx, init_state.clone())
            }
        )
    }

    fn once(&self, sodium_ctx: &mut SodiumCtx) -> Stream<A> {
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

    fn keep_alive<X:'static>(&self, sodium_ctx: &mut SodiumCtx, object: X) -> Stream<A> {
        let l = Listener::new(sodium_ctx, move || { let _object2 = &object; });
        self.add_cleanup(sodium_ctx, l)
    }

    fn unsafe_add_cleanup(&self, listener: Listener) -> &Self {
        let mut data = self.to_stream_ref().data.borrow_mut();
        let data_: &mut StreamData<A> = &mut *data;
        data_.finalizers.push(listener);
        self
    }

    fn add_cleanup(&self, sodium_ctx: &mut SodiumCtx, cleanup: Listener) -> Stream<A> {
        self
            .map(sodium_ctx, |a: &A| a.clone())
            .unsafe_add_cleanup(cleanup)
            .to_stream()
    }
}

impl<A: 'static + Clone> IsStream<A> for Stream<A> {
    fn to_stream_ref(&self) -> &Stream<A> {
        self
    }
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            data: self.data.clone()
        }
    }
}

impl<A> Clone for WeakStream<A> {
    fn clone(&self) -> Self {
        WeakStream {
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
    WeakS(WeakStream<A>),
    StrongS(Stream<A>)
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

impl<A: Clone + 'static> Stream<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> Stream<A> {
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        Stream {
            data: sodium_ctx.new_gc(RefCell::new(
                StreamData {
                    node: Node::new(sodium_ctx2, 0),
                    finalizers: Vec::new(),
                    firings: Vec::new()
                }
            ))
        }
    }

    pub fn downgrade(&self) -> WeakStream<A> {
        WeakStream {
            data: self.data.downgrade()
        }
    }

    pub fn or_else<IT>(sodium_ctx: &mut SodiumCtx, ss: IT) -> Stream<A>
        where IT: Iterator<Item=Stream<A>>
    {
        Stream::merge(
            sodium_ctx,
            ss,
            |left, _right|
                left.clone()
        )
    }

    pub fn merge<IT,F>(sodium_ctx: &mut SodiumCtx, ss: IT, f: F) -> Stream<A>
        where IT: Iterator<Item=Stream<A>>,
              F: Fn(&A,&A)->A + 'static
    {
        let ss_vec: Vec<Stream<A>> = ss.collect();
        let ss_vec_len = ss_vec.len();
        Stream::merge_(sodium_ctx, &ss_vec, 0, ss_vec_len, f)
    }

    fn merge_<F>(sodium_ctx: &mut SodiumCtx, ss: &Vec<Stream<A>>, start: usize, end: usize, f: F) -> Stream<A>
        where F: Fn(&A,&A)->A + 'static
    {
        Stream::merge__(sodium_ctx, ss, start, end, &Rc::new(f))
    }

    fn merge__<F>(sodium_ctx: &mut SodiumCtx, ss: &Vec<Stream<A>>, start: usize, end: usize, f: &Rc<F>) -> Stream<A>
        where F: Fn(&A,&A)->A + 'static
    {
        let len = end - start;
        if len == 0 {
            Stream::new(sodium_ctx)
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
                Stream::merge__(
                    sodium_ctx,
                    &ss,
                    start,
                    mid,
                    f
                );
            let s2 =
                Stream::merge__(
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

impl<A: Clone + 'static> WeakStream<A> {
    pub fn upgrade(&self) -> Option<Stream<A>> {
        self.data.upgrade().map(|data| {
            Stream {
                data: data
            }
        })
    }
}
