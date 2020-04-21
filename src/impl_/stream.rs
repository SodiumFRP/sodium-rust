use crate::impl_::cell::Cell;
use crate::impl_::dep::Dep;
use crate::impl_::node::{Node, WeakNode, IsNode, IsWeakNode, box_clone_vec_is_node};
use crate::impl_::lazy::Lazy;
use crate::impl_::listener::Listener;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream_loop::StreamLoop;
use crate::impl_::stream_sink::StreamSink;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::{lambda1, lambda1_deps, lambda2_deps};

use std::sync::Arc;
use std::sync::RwLock;
use std::sync::Mutex;
use std::sync::Weak;

pub struct StreamWeakForwardRef<A> {
    data: Arc<RwLock<Option<WeakStream<A>>>>
}

impl<A> Clone for StreamWeakForwardRef<A> {
    fn clone(&self) -> Self {
        StreamWeakForwardRef {
            data: self.data.clone()
        }
    }
}

impl<A:Send+'static> StreamWeakForwardRef<A> {
    pub fn new() -> StreamWeakForwardRef<A> {
        StreamWeakForwardRef {
            data: Arc::new(RwLock::new(None))
        }
    }

    pub fn assign(&self, s: &Stream<A>) {
        let mut x = self.data.write().unwrap();
        *x = Some(Stream::downgrade(s))
    }

    pub fn unwrap(&self) -> Stream<A> {
        let x = self.data.read().unwrap();
        (&*x).clone().unwrap().upgrade().unwrap()
    }
}

pub struct Stream<A> {
    pub data: Arc<Mutex<StreamData<A>>>,
    pub node: Node
}

pub struct WeakStream<A> {
    pub data: Weak<Mutex<StreamData<A>>>,
    pub node: WeakNode
}

impl<A:Send+'static> IsNode for Stream<A> {
    fn node(&self) -> &Node {
        &self.node
    }

    fn box_clone(&self) -> Box<dyn IsNode + Send + Sync> {
        Box::new(self.clone())
    }

    fn downgrade(&self) -> Box<dyn IsWeakNode + Send + Sync> {
        Box::new(Stream::downgrade(self))
    }
}

impl<A:Send+'static> IsWeakNode for WeakStream<A> {
    fn node(&self) -> &WeakNode {
        &self.node
    }

    fn box_clone(&self) -> Box<dyn IsWeakNode + Send + Sync> {
        Box::new(self.clone())
    }

    fn upgrade(&self) -> Option<Box<dyn IsNode + Send + Sync>> {
        WeakStream::upgrade(self).map(|x| Box::new(x) as Box<dyn IsNode + Send + Sync>)
    }
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            data: self.data.clone(),
            node: self.node.clone()
        }
    }
}

impl<A> Clone for WeakStream<A> {
    fn clone(&self) -> Self {
        WeakStream {
            data: self.data.clone(),
            node: self.node.clone()
        }
    }
}

pub struct StreamData<A> {
    pub firing_op: Option<A>,
    pub sodium_ctx: SodiumCtx,
    pub coalescer_op: Option<Box<dyn FnMut(&A,&A)->A+Send>>
}

impl<A> Stream<A> {
    pub fn with_data<R,K:FnOnce(&mut StreamData<A>)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut StreamData<A> = l.as_mut().unwrap();
        k(data)
    }

    pub fn with_firing_op<R,K:FnOnce(&mut Option<A>)->R>(&self, k: K) -> R {
        self.with_data(|data: &mut StreamData<A>| k(&mut data.firing_op))
    }

    pub fn to_dep(&self) -> Dep {
        Dep::new(self.node().gc_node.clone())
    }

    pub fn node(&self) -> &Node {
        &self.node
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        self.with_data(|data: &mut StreamData<A>| data.sodium_ctx.clone())
    }

}

impl<A:Send+'static> Stream<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream::_new(
            sodium_ctx,
            |_s: StreamWeakForwardRef<A>| {
                Node::new(
                    sodium_ctx,
                    "Stream::new",
                    || {},
                    Vec::new()
                )
            }
        )
    }

    // for purpose of capturing stream in lambda
    pub fn nop(&self) {}

    pub fn _new_with_coalescer<COALESCER:FnMut(&A,&A)->A+Send+'static>(sodium_ctx: &SodiumCtx, coalescer: COALESCER) -> Stream<A> {
        Stream {
            data: Arc::new(Mutex::new(StreamData {
                firing_op: None,
                sodium_ctx: sodium_ctx.clone(),
                coalescer_op: Some(Box::new(coalescer))
            })),
            node: Node::new(
                &sodium_ctx,
                "Stream::_new_with_coalescer",
                || {},
                vec![]
            )
        }
    }

    pub fn _new<MkNode:FnOnce(StreamWeakForwardRef<A>)->Node>(sodium_ctx: &SodiumCtx, mk_node: MkNode) -> Stream<A> {
        let result_forward_ref: StreamWeakForwardRef<A> = StreamWeakForwardRef::new();
        let s;
        let node = mk_node(result_forward_ref.clone());
        {
            s = Stream {
                data: Arc::new(Mutex::new(StreamData {
                    firing_op: None,
                    sodium_ctx: sodium_ctx.clone(),
                    coalescer_op: None
                })),
                node: node.clone()
            };
        }
        result_forward_ref.assign(&s);
        {
            let mut update = node.data.update.write().unwrap();
            let update: &mut Box<_> = &mut update;
            update();
        }
        let is_firing =
            s.with_data(|data: &mut StreamData<A>| data.firing_op.is_some());
        if is_firing {
            {
                let s_node = s.node();
                let mut changed = s_node.data.changed.write().unwrap();
                *changed = true;
            }
            let s = s.clone();
            sodium_ctx.pre_post(move || {
                s.with_data(|data: &mut StreamData<A>| {
                    data.firing_op = None;
                    let s_node = s.node();
                    let mut changed = s_node.data.changed.write().unwrap();
                    *changed = true;
                });
            });
        }
        s
    }

    pub fn snapshot<B:Send+Clone+'static,C:Send+'static,FN:IsLambda2<A,B,C>+Send+Sync+'static>(&self, cb: &Cell<B>, mut f: FN) -> Stream<C> {
        let cb = cb.clone();
        let mut f_deps = lambda2_deps(&f);
        f_deps.push(Dep::new(cb.node().gc_node().clone()));
        self.map(lambda1(move |a: &A| f.call(a, &cb.sample()), f_deps))
    }

    pub fn snapshot1<B:Send+Clone+'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    pub fn map<B:Send+'static,FN:IsLambda1<A,B>+Send+Sync+'static>(&self, mut f: FN) -> Stream<B> {
        let self_ = self.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: StreamWeakForwardRef<B>| {
                let f_deps = lambda1_deps(&f);
                let node = Node::new(
                    &sodium_ctx,
                    "Stream::map",
                    move || {
                        self_.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                s.unwrap()._send(f.call(firing));
                            }
                        })
                    },
                    vec![self.box_clone()]
                );
                IsNode::add_update_dependencies(&node, f_deps);
                IsNode::add_update_dependencies(&node, vec![self.to_dep()]);
                node
            }
        )
    }

    pub fn filter<PRED:IsLambda1<A,bool>+Send+Sync+'static>(&self, mut pred: PRED) -> Stream<A> where A: Clone {
        let self_ = self.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: StreamWeakForwardRef<A>| {
                let pred_deps = lambda1_deps(&pred);
                let node = Node::new(
                    &sodium_ctx,
                    "Stream::filter",
                    move || {
                        self_.with_firing_op(|firing_op: &mut Option<A>| {
                            let firing_op2 = firing_op.clone().filter(|firing| pred.call(firing));
                            if let Some(firing) = firing_op2 {
                                s.unwrap()._send(firing);
                            }
                        });
                    },
                    vec![self.box_clone()]
                );
                IsNode::add_update_dependencies(&node, pred_deps);
                IsNode::add_update_dependencies(&node, vec![self.to_dep()]);
                node
            }
        )
    }

    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> where A: Clone {
        self.merge(s2, |lhs:&A, _rhs:&A| lhs.clone())
    }

    pub fn merge<FN:IsLambda2<A,A,A>+Send+Sync+'static>(&self, s2: &Stream<A>, mut f: FN) -> Stream<A> where A: Clone {
        let self_ = self.clone();
        let s2 = s2.clone();
        let s2_node = s2.box_clone();
        let s2_dep = s2.to_dep();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: StreamWeakForwardRef<A>| {
                let f_deps = lambda2_deps(&f);
                let node = Node::new(
                    &sodium_ctx,
                    "Stream::merge",
                    move || {
                        self_.with_firing_op(|firing1_op: &mut Option<A>| {
                            s2.with_firing_op(|firing2_op: &mut Option<A>| {
                                if let Some(ref firing1) = firing1_op {
                                    if let Some(ref firing2) = firing2_op {
                                        s.unwrap()._send(f.call(firing1, firing2));
                                    } else {
                                        s.unwrap()._send(firing1.clone());
                                    }
                                } else {
                                    if let Some(ref firing2) = firing2_op {
                                        s.unwrap()._send(firing2.clone());
                                    }
                                }
                            })
                        })
                    },
                    vec![self.box_clone(), s2_node]
                );
                IsNode::add_update_dependencies(&node, f_deps);
                IsNode::add_update_dependencies(&node, vec![self.to_dep(), s2_dep]);
                node
            }
        )
    }

    pub fn hold(&self, a: A) -> Cell<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            Cell::_new(&sodium_ctx, self.clone(), Lazy::of_value(a))
        })
    }

    pub fn hold_lazy(&self, a: Lazy<A>) -> Cell<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            Cell::_new(&sodium_ctx, self.clone(), a)
        })
    }

    pub fn collect_lazy<B,S,F>(&self, init_state: Lazy<S>, f: F) -> Stream<B>
        where B: Send + Clone + 'static,
              S: Send + Clone + 'static,
              F: IsLambda2<A,S,(B,S)> + Send + Sync + 'static
    {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            let ea = self.clone();
            let es = StreamLoop::new(&sodium_ctx);
            let s = es.stream().hold_lazy(init_state);
            let ebs = ea.snapshot(&s, f);
            let eb = ebs.map(|(ref a,ref _b):&(B,S)| a.clone());
            let es_out = ebs.map(|(ref _a,ref b):&(B,S)| b.clone());
            es.loop_(&es_out);
            eb
        })
    }

    pub fn accum_lazy<S,F>(&self, init_state: Lazy<S>, f: F) -> Cell<S>
        where S: Send + Clone + 'static,
              F: IsLambda2<A,S,S> + Send + Sync + 'static
    {
        let sodium_ctx = self.sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        sodium_ctx.transaction(|| {
            let es: StreamLoop<S> = StreamLoop::new(sodium_ctx);
            let s = es.stream().hold_lazy(init_state);
            let es_out = self.snapshot(&s, f);
            es.loop_(&es_out);
            s
        })
    }

    pub fn defer(&self) -> Stream<A> where A: Clone {
        let sodium_ctx = self.sodium_ctx();
        sodium_ctx.transaction(|| {
            let ss = StreamSink::new(&sodium_ctx);
            let s = ss.stream();
            let sodium_ctx = sodium_ctx.clone();
            let ss = StreamSink::downgrade(&ss);
            let listener = self.listen_weak(move |a:&A| {
                let ss = ss.upgrade().unwrap();
                let a = a.clone();
                sodium_ctx.post(move || ss.send(a.clone()))
            });
            IsNode::add_keep_alive(&s, &listener.gc_node);
            return s;
        })
    }

    pub fn once(&self) -> Stream<A> where A: Clone {
        let self_ = self.clone();
        let sodium_ctx = self.sodium_ctx().clone();
        Stream::_new(
            &sodium_ctx,
            |s: StreamWeakForwardRef<A>| {
                let sodium_ctx = sodium_ctx.clone();
                let sodium_ctx2 = sodium_ctx.clone();
                let node = Node::new(
                    &sodium_ctx2,
                    "Stream::once",
                    move || {
                        self_.with_firing_op(|firing_op: &mut Option<A>| {
                            if let Some(ref firing) = firing_op {
                                let s = s.unwrap();
                                s._send(firing.clone());
                                let node = s.box_clone();
                                sodium_ctx.post(move || {
                                    let deps;
                                    {
                                        let dependencies = node.data().dependencies.read().unwrap();
                                        deps = box_clone_vec_is_node(&(*dependencies));
                                    }
                                    for dep in deps {
                                        IsNode::remove_dependency(node.node(), dep.node());
                                    }
                                });
                            }
                        })
                    },
                    vec![self.box_clone()]
                );
                IsNode::add_update_dependencies(&node, vec![self.to_dep()]);
                node
            }
        )
    }

    pub fn _listen<K:IsLambda1<A,()>+Send+Sync+'static>(&self, mut k: K, weak: bool) -> Listener {
        let self_ = self.clone();
        let node =
            Node::new(
                &self.sodium_ctx(),
                "Stream::listen",
                move || {
                    self_.with_data(|data: &mut StreamData<A>| {
                        for firing in &data.firing_op {
                            k.call(firing)
                        }
                    });
                },
                vec![self.box_clone()]
            );
        IsNode::add_update_dependencies(&node, vec![self.to_dep()]);
        Listener::new(&self.sodium_ctx(), weak, node)
    }

    pub fn listen_weak<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener {
        self._listen(k, true)
    }

    pub fn listen<K:IsLambda1<A,()>+Send+Sync+'static>(&self, k: K) -> Listener {
        self._listen(k, false)
    }

    pub fn _send(&self, a: A) {
        let sodium_ctx = self.sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        sodium_ctx.transaction(|| {
            let is_first = self.with_data(|data: &mut StreamData<A>| {
                let is_first = data.firing_op.is_none();
                if let Some(ref mut coalescer) = data.coalescer_op {
                    if let Some(ref mut firing) = data.firing_op {
                        *firing = coalescer(firing, &a);
                    }
                    if is_first {
                        data.firing_op = Some(a);
                    }
                } else {
                    data.firing_op = Some(a);
                }
                is_first
            });
            {
                let self_node = self.node();
                let mut changed = self_node.data.changed.write().unwrap();
                *changed = true;
            }
            if is_first {
                let _self = self.clone();
                let self_node = _self.box_clone();
                sodium_ctx.pre_post(move || {
                    _self.with_data(|data: &mut StreamData<A>| {
                        data.firing_op = None;
                        {
                            let mut changed = self_node.data().changed.write().unwrap();
                            *changed = false;
                        }
                    });
                });
            }
        });
    }

    pub fn downgrade(this: &Self) -> WeakStream<A> {
        WeakStream {
            data: Arc::downgrade(&this.data),
            node: Node::downgrade2(&this.node)
        }
    }
}

impl<A> WeakStream<A> {
    pub fn upgrade(&self) -> Option<Stream<A>> {
        let data_op = self.data.upgrade();
        let node_op = self.node.upgrade2();
        if data_op.is_none() || node_op.is_none() {
            return None;
        }
        let data = data_op.unwrap();
        let node = node_op.unwrap();
        Some(Stream { data, node })
    }
}
