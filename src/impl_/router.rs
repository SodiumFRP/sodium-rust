use crate::impl_::node::{IsNode, IsWeakNode, Node, WeakNode};
use crate::impl_::sodium_ctx::{SodiumCtx, SodiumCtxData};
use crate::impl_::stream::{Stream, WeakStream};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

use super::name::NodeName;

pub struct Router<A, K> {
    sodium_ctx: SodiumCtx,
    table: Arc<RwLock<HashMap<K, WeakStream<A>>>>,
    node: Node,
}

pub struct WeakRouter<A, K> {
    sodium_ctx: SodiumCtx,
    table: Arc<RwLock<HashMap<K, WeakStream<A>>>>,
    node: WeakNode,
}

impl<A, K> Clone for Router<A, K> {
    fn clone(&self) -> Self {
        Router {
            sodium_ctx: self.sodium_ctx.clone(),
            table: self.table.clone(),
            node: self.node.clone(),
        }
    }
}

impl<A, K> Clone for WeakRouter<A, K> {
    fn clone(&self) -> Self {
        WeakRouter {
            sodium_ctx: self.sodium_ctx.clone(),
            table: self.table.clone(),
            node: self.node.clone(),
        }
    }
}

impl<A: Send + 'static, K: Send + Sync + 'static> IsNode for Router<A, K> {
    fn node(&self) -> &Node {
        &self.node
    }

    fn box_clone(&self) -> Box<dyn IsNode + Send + Sync> {
        Box::new(self.clone())
    }

    fn downgrade(&self) -> Box<dyn IsWeakNode + Send + Sync> {
        Box::new(WeakRouter {
            sodium_ctx: self.sodium_ctx.clone(),
            table: self.table.clone(),
            node: Node::downgrade2(&self.node),
        })
    }
}

impl<A: Send + 'static, K: Send + Sync + 'static> IsWeakNode for WeakRouter<A, K> {
    fn node(&self) -> &WeakNode {
        &self.node
    }

    fn box_clone(&self) -> Box<dyn IsWeakNode + Send + Sync> {
        Box::new(self.clone())
    }

    fn upgrade(&self) -> Option<Box<dyn IsNode + Send + Sync>> {
        if let Some(node) = self.node.upgrade2() {
            Some(Box::new(Router {
                sodium_ctx: self.sodium_ctx.clone(),
                table: self.table.clone(),
                node,
            }))
        } else {
            None
        }
    }
}

impl<A, K> Router<A, K> {
    pub fn new(
        sodium_ctx: &SodiumCtx,
        in_stream: &Stream<A>,
        selector: impl Fn(&A) -> Vec<K> + Send + Sync + 'static,
    ) -> Router<A, K>
    where
        A: Clone + Send + 'static,
        K: Send + Sync + Eq + Hash + 'static,
    {
        let node;
        let table = Arc::new(RwLock::new(HashMap::<K, WeakStream<A>>::new()));
        {
            let sodium_ctx2 = sodium_ctx.clone();
            let table = table.clone();
            let in_stream2 = in_stream.clone();
            node = Node::new(
                sodium_ctx,
                NodeName::Router,
                move || {
                    let sodium_ctx = &sodium_ctx2;
                    let keys_firing_op = in_stream2.with_firing_op(|firing_op: &mut Option<A>| {
                        firing_op
                            .as_ref()
                            .map(|firing| (selector(firing), firing.clone()))
                    });
                    if let Some((keys, firing)) = keys_firing_op {
                        for key in keys {
                            let mut table = table.write().unwrap();
                            let mut remove_it = false;
                            if let Some(weak_stream) = table.get(&key) {
                                if let Some(stream) = weak_stream.upgrade() {
                                    stream._send(firing.clone());
                                    sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                                        data.changed_nodes.push(stream.box_clone());
                                    });
                                } else {
                                    remove_it = true;
                                }
                            }
                            if remove_it {
                                table.remove(&key);
                            }
                        }
                    }
                },
                vec![in_stream.box_clone()],
            );
            <dyn IsNode>::add_update_dependencies(&node, vec![in_stream.to_dep()]);
        }
        Router {
            sodium_ctx: sodium_ctx.clone(),
            table,
            node,
        }
    }

    pub fn filter_matches(&self, k: &K) -> Stream<A>
    where
        A: Clone + Send + 'static,
        K: Clone + Send + Sync + Eq + Hash + 'static,
    {
        let mut table = self.table.write().unwrap();
        let existing_op;
        if let Some(weak_stream) = table.get(k) {
            if let Some(stream) = weak_stream.upgrade() {
                existing_op = Some(stream);
            } else {
                existing_op = None;
            }
        } else {
            existing_op = None;
        }
        if let Some(existing) = existing_op {
            existing
        } else {
            let s = Stream::new(&self.sodium_ctx);
            s.node()
                .data()
                .dependencies
                .write()
                .push(self.box_clone());
            table.insert(k.clone(), Stream::downgrade(&s));
            {
                let table = self.table.clone();
                let k = k.clone();
                let weak_s = Stream::downgrade(&s);
                s.node()
                    .data()
                    .cleanups
                    .write()
                    .push(Box::new(move || {
                        let _ = &weak_s;
                        let mut table = table.write().unwrap();
                        let mut remove_it = false;
                        if let Some(weak_stream) = table.get(&k) {
                            if weak_stream.data.ptr_eq(&weak_s.data) {
                                remove_it = true;
                            }
                        }
                        if remove_it {
                            table.remove(&k);
                        }
                    }))
            }
            s
        }
    }
}
