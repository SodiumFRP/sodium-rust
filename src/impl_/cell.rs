use crate::impl_::dep::Dep;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::IsLambda4;
use crate::impl_::lambda::IsLambda5;
use crate::impl_::lambda::IsLambda6;
use crate::impl_::lambda::{
    lambda1, lambda1_deps, lambda2, lambda2_deps, lambda3, lambda3_deps, lambda4_deps,
    lambda5_deps, lambda6_deps,
};
use crate::impl_::lazy::Lazy;
use crate::impl_::listener::Listener;
use crate::impl_::node::{IsNode, Node, WeakNode};
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;
use crate::impl_::stream::Stream;
use crate::impl_::stream::StreamWeakForwardRef;
use crate::impl_::stream::WeakStream;

use parking_lot::Mutex;
use parking_lot::RwLock;
use std::mem;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Weak;

use super::name::NodeName;
use super::node::IsNodeExt;

pub struct CellWeakForwardRef<A> {
    data: Arc<RwLock<Option<WeakCell<A>>>>,
}

impl<A> Clone for CellWeakForwardRef<A> {
    fn clone(&self) -> Self {
        CellWeakForwardRef {
            data: self.data.clone(),
        }
    }
}

impl<A: Send + 'static> CellWeakForwardRef<A> {
    pub fn new() -> CellWeakForwardRef<A> {
        CellWeakForwardRef {
            data: Arc::new(RwLock::new(None)),
        }
    }

    pub fn assign(&self, c: &Cell<A>) {
        let mut x = self.data.write();
        *x = Some(Cell::downgrade(c))
    }

    fn unwrap(&self) -> Cell<A> {
        let x = self.data.read();
        x.clone().unwrap().upgrade().unwrap()
    }
}

pub struct Cell<A> {
    pub data: Arc<Mutex<CellData<A>>>,
    pub node: Node,
}

pub struct WeakCell<A> {
    pub data: Weak<Mutex<CellData<A>>>,
    pub node: WeakNode,
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            data: self.data.clone(),
            node: self.node.clone(),
        }
    }
}

impl<A> Clone for WeakCell<A> {
    fn clone(&self) -> Self {
        WeakCell {
            data: self.data.clone(),
            node: self.node.clone(),
        }
    }
}

pub struct CellData<A> {
    stream: Stream<A>,
    value: Lazy<A>,
    next_value_op: Option<A>,
}

impl<A> Cell<A> {
    pub fn with_data<R, K: FnOnce(&mut CellData<A>) -> R>(&self, k: K) -> R {
        let data: &mut CellData<A> = &mut self.data.lock();
        k(data)
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        self.with_data(|data: &mut CellData<A>| data.stream.sodium_ctx())
    }

    pub fn to_dep(&self) -> Dep {
        Dep::new(self.node().gc_node.clone())
    }

    pub fn node(&self) -> &Node {
        &self.node
    }
}

impl<A: Send + 'static> Cell<A> {
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A>
    where
        A: Clone,
    {
        let cell_data = Arc::new(Mutex::new(CellData {
            stream: Stream::new(sodium_ctx),
            value: Lazy::of_value(value),
            next_value_op: None,
        }));
        Cell {
            data: cell_data,
            node: Node::new(sodium_ctx, NodeName::CELL_NEW, || {}, vec![]),
        }
    }

    pub fn _new(sodium_ctx: &SodiumCtx, stream: Stream<A>, value: Lazy<A>) -> Cell<A>
    where
        A: Clone,
    {
        sodium_ctx.transaction(|| {
            let cell_data = Arc::new(Mutex::new(CellData {
                stream: stream.clone(),
                value,
                next_value_op: None,
            }));
            let sodium_ctx = sodium_ctx.clone();
            let node: Node;
            let c_forward_ref = CellWeakForwardRef::new();
            {
                let c = c_forward_ref.clone();
                let stream_node = stream.box_clone();
                let stream_dep = stream.to_dep();
                let sodium_ctx = sodium_ctx.clone();
                let sodium_ctx2 = sodium_ctx.clone();
                node = Node::new(
                    &sodium_ctx2,
                    NodeName::CELL_HOLD,
                    move || {
                        let c = c.unwrap();
                        let firing_op = stream.with_firing_op(|firing_op| firing_op.clone());
                        if let Some(firing) = firing_op {
                            let is_first = c.with_data(|data: &mut CellData<A>| {
                                let is_first = data.next_value_op.is_none();
                                data.next_value_op = Some(firing);
                                is_first
                            });
                            if is_first {
                                sodium_ctx.post(move || {
                                    c.with_data(|data: &mut CellData<A>| {
                                        let mut next_value_op: Option<A> = None;
                                        mem::swap(&mut next_value_op, &mut data.next_value_op);
                                        if let Some(next_value) = next_value_op {
                                            data.value = Lazy::of_value(next_value);
                                        }
                                    })
                                });
                            }
                        }
                    },
                    vec![stream_node],
                );
                // Hack: Add stream gc node twice, because one is kepted in the cell_data for Cell::update() to return.
                node.add_update_dependencies(vec![stream_dep.clone(), stream_dep]);
            }
            let c = Cell {
                data: cell_data,
                node: node.clone(),
            };
            c_forward_ref.assign(&c);
            // initial update of Cell incase of stream (in Cell::hold) firing during same transaction cell is created.
            {
                let c = c.clone();
                sodium_ctx.pre_eot(move || {
                    let mut update = node.data.update.write();
                    let update: &mut Box<_> = &mut *update;
                    update();
                    // c captured, but not used so that update() will not crash here
                    c.nop();
                });
            }
            //
            c
        })
    }

    pub fn nop(&self) {
        // no operation. (NOP)
        // Purpose is for capturing inside closure to extend lifetime to atleast the lifetime of the closure,
        // but do not use it in that closure
    }

    pub fn sample(&self) -> A
    where
        A: Clone,
    {
        self.with_data(|data: &mut CellData<A>| data.value.run())
    }

    pub fn sample_lazy(&self) -> Lazy<A> {
        self.with_data(|data: &mut CellData<A>| data.value.clone())
    }

    pub fn updates(&self) -> Stream<A> {
        self.with_data(|data: &mut CellData<A>| data.stream.clone())
    }

    pub fn value(&self) -> Stream<A>
    where
        A: Clone,
    {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            let s1 = self.updates();
            let spark: Stream<Lazy<A>> = Stream::new(&sodium_ctx);
            {
                let spark = &spark;
                let self_ = &self;
                spark._send(self_.with_data(|data: &mut CellData<A>| data.value.clone()));
                let node = spark.node();
                {
                    node.data.changed.store(true, Ordering::SeqCst);
                }
                sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                    data.changed_nodes.push(node.box_clone())
                });
            }
            s1.or_else(&spark.map(|x: &Lazy<A>| x.run()))
        })
    }

    pub fn map<B: Send + 'static, FN: IsLambda1<A, B> + Send + Sync + 'static>(
        &self,
        f: FN,
    ) -> Cell<B>
    where
        A: Clone,
        B: Clone,
    {
        let self_ = self.clone();
        let f_deps = lambda1_deps(&f);
        let f = Arc::new(Mutex::new(f));
        let init;
        {
            let f = f.clone();
            init = Lazy::new(move || {
                let mut f = f.lock();
                f.call(&self_.sample())
            });
        }
        self.updates()
            .map(lambda1(
                move |a: &A| {
                    let mut f = f.lock();
                    f.call(a)
                },
                f_deps,
            ))
            .hold_lazy(init)
    }

    pub fn lift2<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        FN: IsLambda2<A, B, C> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        f: FN,
    ) -> Cell<C>
    where
        A: Clone,
    {
        let sodium_ctx = self.sodium_ctx();
        let lhs = self.sample_lazy();
        let rhs = cb.sample_lazy();
        let init: Lazy<C>;
        let f_deps = lambda2_deps(&f);
        let f = Arc::new(Mutex::new(f));
        {
            let lhs = lhs.clone();
            let rhs = rhs.clone();
            let f = f.clone();
            init = Lazy::new(move || {
                let mut f = f.lock();
                f.call(&lhs.run(), &rhs.run())
            });
        }
        let state: Arc<Mutex<(Lazy<A>, Lazy<B>)>> = Arc::new(Mutex::new((lhs, rhs)));
        let s1: Stream<()>;
        let s2: Stream<()>;
        {
            let state = state.clone();
            s1 = self.updates().map(move |a: &A| {
                let mut state2 = state.lock();
                state2.0 = Lazy::of_value(a.clone());
            });
        }
        {
            let state = state.clone();
            s2 = cb.updates().map(move |b: &B| {
                let mut state2 = state.lock();
                state2.1 = Lazy::of_value(b.clone());
            });
        }
        let s = s1.or_else(&s2).map(lambda1(
            move |_: &()| {
                let state2 = state.lock();
                let mut f = f.lock();
                f.call(&state2.0.run(), &state2.1.run())
            },
            f_deps,
        ));
        Cell::_new(&sodium_ctx, s, init)
    }

    pub fn lift3<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        FN: IsLambda3<A, B, C, D> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        mut f: FN,
    ) -> Cell<D>
    where
        A: Clone,
    {
        let f_deps = lambda3_deps(&f);
        self.lift2(cb, |a: &A, b: &B| (a.clone(), b.clone())).lift2(
            cc,
            lambda2(
                move |(ref a, ref b): &(A, B), c: &C| f.call(a, b, c),
                f_deps,
            ),
        )
    }

    pub fn lift4<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        E: Send + Clone + 'static,
        FN: IsLambda4<A, B, C, D, E> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        mut f: FN,
    ) -> Cell<E>
    where
        A: Clone,
    {
        let f_deps = lambda4_deps(&f);
        self.lift3(cb, cc, |a: &A, b: &B, c: &C| {
            (a.clone(), b.clone(), c.clone())
        })
        .lift2(
            cd,
            lambda2(
                move |(ref a, ref b, ref c): &(A, B, C), d: &D| f.call(a, b, c, d),
                f_deps,
            ),
        )
    }

    pub fn lift5<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        E: Send + Clone + 'static,
        F: Send + Clone + 'static,
        FN: IsLambda5<A, B, C, D, E, F> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        ce: &Cell<E>,
        mut f: FN,
    ) -> Cell<F>
    where
        A: Clone,
    {
        let f_deps = lambda5_deps(&f);
        self.lift3(cb, cc, |a: &A, b: &B, c: &C| {
            (a.clone(), b.clone(), c.clone())
        })
        .lift3(
            cd,
            ce,
            lambda3(
                move |(ref a, ref b, ref c): &(A, B, C), d: &D, e: &E| f.call(a, b, c, d, e),
                f_deps,
            ),
        )
    }

    pub fn lift6<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        E: Send + Clone + 'static,
        F: Send + Clone + 'static,
        G: Send + Clone + 'static,
        FN: IsLambda6<A, B, C, D, E, F, G> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        ce: &Cell<E>,
        cf: &Cell<F>,
        mut f: FN,
    ) -> Cell<G>
    where
        A: Clone,
    {
        let f_deps = lambda6_deps(&f);
        self.lift4(cb, cc, cd, |a: &A, b: &B, c: &C, d: &D| {
            (a.clone(), b.clone(), c.clone(), d.clone())
        })
        .lift3(
            ce,
            cf,
            lambda3(
                move |(ref a, ref b, ref c, ref d): &(A, B, C, D), e: &E, f2: &F| {
                    f.call(a, b, c, d, e, f2)
                },
                f_deps,
            ),
        )
    }

    pub fn switch_s(csa: &Cell<Stream<A>>) -> Stream<A>
    where
        A: Clone,
    {
        let csa = csa.clone();
        let sodium_ctx = csa.sodium_ctx();
        Stream::_new(&sodium_ctx, |sa: StreamWeakForwardRef<A>| {
            let inner_s: Arc<Mutex<WeakStream<A>>> =
                Arc::new(Mutex::new(Stream::downgrade(&Stream::new(&sodium_ctx))));
            let sa = sa;
            let node1: Node;
            {
                let inner_s = inner_s.clone();
                node1 = Node::new(
                    &sodium_ctx,
                    NodeName::CELL_SWITCH_S_INNER,
                    move || {
                        let inner_s = inner_s.lock();
                        let inner_s = inner_s.upgrade().unwrap();
                        inner_s.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                let sa = sa.unwrap();
                                sa._send(firing.clone());
                            }
                        });
                    },
                    vec![],
                );
            }
            {
                let inner_s = inner_s.clone();
                let csa = csa.clone();
                let node1 = node1.clone();
                sodium_ctx.pre_eot(move || {
                    let mut inner_s = inner_s.lock();
                    let s = csa.sample();
                    *inner_s = Stream::downgrade(&s);
                    node1.add_dependency(s);
                });
            }
            let node2: Node;
            let csa_updates = csa.updates();
            let csa_updates_node = csa_updates.box_clone();
            let csa_updates_dep = csa_updates.to_dep();
            {
                let node1: Node = node1.clone();
                let sodium_ctx = sodium_ctx.clone();
                let sodium_ctx2 = sodium_ctx.clone();
                node2 = Node::new(
                    &sodium_ctx2,
                    NodeName::CELL_SWITCH_S_OUTER,
                    move || {
                        csa_updates.with_firing_op(|firing_op: &mut Option<Stream<A>>| {
                            if let Some(ref firing) = firing_op {
                                let firing = firing.clone();
                                let node1 = node1.clone();
                                let inner_s = inner_s.clone();
                                sodium_ctx.pre_post(move || {
                                    let mut inner_s = inner_s.lock();
                                    node1.remove_dependency(&inner_s.upgrade().unwrap());
                                    node1.add_dependency(firing.clone());
                                    *inner_s = Stream::downgrade(&firing);
                                });
                            }
                        });
                    },
                    vec![csa_updates_node.box_clone()],
                );
            }
            node2.add_update_dependencies(vec![csa_updates_dep, Dep::new(node1.gc_node().clone())]);
            node1.add_dependency(node2);
            node1
        })
    }

    pub fn switch_c(cca: &Cell<Cell<A>>) -> Cell<A>
    where
        A: Clone,
    {
        let cca2 = cca.clone();
        let cca = cca.clone();
        let sodium_ctx = cca.sodium_ctx();
        Stream::_new(&sodium_ctx, |sa: StreamWeakForwardRef<A>| {
            let node1 = Node::new(
                &sodium_ctx,
                NodeName::CELL_SWITCH_C_OUTER,
                || {},
                vec![cca.updates().box_clone()],
            );
            let last_inner_s = Arc::new(Mutex::new(Stream::downgrade(&Stream::new(&sodium_ctx))));
            let node2 = Node::new(
                &sodium_ctx,
                NodeName::CELL_SWITCH_C_INNER,
                || {},
                vec![node1.box_clone()],
            );
            {
                let last_inner_s = last_inner_s.clone();
                let cca = cca.clone();
                let node2 = node2.clone();
                sodium_ctx.pre_eot(move || {
                    let mut last_inner_s = last_inner_s.lock();
                    let s = cca.sample().updates();
                    *last_inner_s = Stream::downgrade(&s);
                    node2.add_dependency(s);
                });
            }
            let node1_update;
            {
                let sodium_ctx = sodium_ctx.clone();
                let node1 = node1.clone();
                let node2 = node2.clone();
                let cca = cca.clone();
                let sa = sa.clone();
                let last_inner_s = last_inner_s.clone();
                node1_update = move || {
                    cca.updates()
                        .with_firing_op(|firing_op: &mut Option<Cell<A>>| {
                            if let Some(ref firing) = firing_op {
                                // will be overwriten by node2 firing if there is one
                                sodium_ctx.update_node(firing.updates().node());
                                let sa = sa.unwrap();
                                sa._send(firing.sample());
                                node1.data.changed.store(true, Ordering::SeqCst);
                                node2.data.changed.store(true, Ordering::SeqCst);
                                let new_inner_s = firing.updates();
                                new_inner_s.with_firing_op(|firing2_op: &mut Option<A>| {
                                    if let Some(ref firing2) = firing2_op {
                                        sa._send(firing2.clone());
                                    }
                                });
                                let mut last_inner_s = last_inner_s.lock();
                                node2.remove_dependency(last_inner_s.upgrade().unwrap().node());
                                node2.add_dependency(new_inner_s.clone());
                                node2.data.changed.store(true, Ordering::SeqCst);
                                *last_inner_s = Stream::downgrade(&new_inner_s);
                            }
                        });
                };
            }
            node1.add_update_dependencies(vec![
                Dep::new(node1.gc_node.clone()),
                Dep::new(node2.gc_node.clone()),
            ]);
            {
                let mut update = node1.data.update.write();
                *update = Box::new(node1_update);
            }
            {
                let last_inner_s = last_inner_s;
                let node2_update = move || {
                    let last_inner_s = last_inner_s.lock();
                    let last_inner_s = last_inner_s.upgrade().unwrap();
                    last_inner_s.with_firing_op(|firing_op: &mut Option<A>| {
                        if let Some(ref firing) = firing_op {
                            let sa = sa.unwrap();
                            sa._send(firing.clone());
                        }
                    });
                };
                {
                    let mut update = node2.data.update.write();
                    *update = Box::new(node2_update);
                }
            }
            node2
        })
        .hold_lazy(Lazy::new(move || cca2.sample().sample()))
    }

    pub fn listen_weak<K: FnMut(&A) + Send + Sync + 'static>(&self, k: K) -> Listener
    where
        A: Clone,
    {
        self.sodium_ctx()
            .transaction(|| self.value().listen_weak(k))
    }

    pub fn listen<K: IsLambda1<A, ()> + Send + Sync + 'static>(&self, k: K) -> Listener
    where
        A: Clone,
    {
        self.sodium_ctx().transaction(|| self.value().listen(k))
    }

    pub fn downgrade(this: &Self) -> WeakCell<A> {
        WeakCell {
            data: Arc::downgrade(&this.data),
            node: Node::downgrade2(&this.node),
        }
    }
}

impl<A> WeakCell<A> {
    pub fn upgrade(&self) -> Option<Cell<A>> {
        let data_op = self.data.upgrade();
        let node_op = self.node.upgrade2();
        if data_op.is_none() || node_op.is_none() {
            return None;
        }
        let data = data_op.unwrap();
        let node = node_op.unwrap();
        Some(Cell { data, node })
    }
}
