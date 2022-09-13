use crate::impl_::gc_node::GcCtx;
use crate::impl_::listener::Listener;
use crate::impl_::node::{
    box_clone_vec_is_node, box_clone_vec_is_weak_node, IsNode, IsWeakNode, Node,
};

use std::mem;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use parking_lot::Mutex;
use std::thread;

use super::name::NodeName;

#[derive(Clone)]
pub struct SodiumCtx {
    gc_ctx: GcCtx,
    data: Arc<Mutex<SodiumCtxData>>,
    node_count: Arc<AtomicUsize>,
    node_ref_count: Arc<AtomicUsize>,
    threaded_mode: Arc<ThreadedMode>,
}

pub struct SodiumCtxData {
    pub changed_nodes: Vec<Box<dyn IsNode>>,
    pub visited_nodes: Vec<Box<dyn IsNode>>,
    pub transaction_depth: u32,
    pub pre_eot: Vec<Box<dyn FnMut() + Send>>,
    pub pre_post: Vec<Box<dyn FnMut() + Send>>,
    pub post: Vec<Box<dyn FnMut() + Send>>,
    pub keep_alive: Vec<Listener>,
    pub collecting_cycles: bool,
    pub allow_add_roots: bool,
    pub allow_collect_cycles_counter: u32,
}

pub struct ThreadedMode {
    pub spawner: ThreadSpawner,
}

type SpawnFn = Box<dyn FnOnce() + Send>;

pub struct ThreadSpawner {
    pub spawn_fn: Box<dyn Fn(SpawnFn) -> ThreadJoiner<()> + Send + Sync>,
}

pub struct ThreadJoiner<R> {
    pub join_fn: Box<dyn FnOnce() -> R + Send>,
}

impl ThreadedMode {
    pub fn spawn<R: Send + 'static, F: FnOnce() -> R + Send + 'static>(
        &self,
        f: F,
    ) -> ThreadJoiner<R> {
        let r: Arc<Mutex<Option<R>>> = Arc::new(Mutex::new(None));
        let thread_joiner;
        {
            let r = r.clone();
            thread_joiner = (self.spawner.spawn_fn)(Box::new(move || {
                let r2 = f();
                let mut r = r.lock();
                *r = Some(r2);
            }));
        }
        ThreadJoiner {
            join_fn: Box::new(move || {
                (thread_joiner.join_fn)();
                let mut r = r.lock();
                let mut r2: Option<R> = None;
                mem::swap(&mut *r, &mut r2);
                r2.unwrap()
            }),
        }
    }
}

impl<R> ThreadJoiner<R> {
    pub fn join(self) -> R {
        (self.join_fn)()
    }
}

pub fn single_threaded_mode() -> ThreadedMode {
    ThreadedMode {
        spawner: ThreadSpawner {
            spawn_fn: Box::new(|callback| {
                callback();
                ThreadJoiner {
                    join_fn: Box::new(|| {}),
                }
            }),
        },
    }
}

#[allow(dead_code)]
pub fn simple_threaded_mode() -> ThreadedMode {
    ThreadedMode {
        spawner: ThreadSpawner {
            spawn_fn: Box::new(|callback| {
                let h = thread::spawn(callback);
                ThreadJoiner {
                    join_fn: Box::new(move || {
                        h.join().unwrap();
                    }),
                }
            }),
        },
    }
}

// TODO:
//pub fn thread_pool_threaded_mode(num_threads: usize) -> ThreadedMode

impl Default for SodiumCtx {
    fn default() -> SodiumCtx {
        SodiumCtx::new()
    }
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            gc_ctx: GcCtx::new(),
            data: Arc::new(Mutex::new(SodiumCtxData {
                changed_nodes: Vec::new(),
                visited_nodes: Vec::new(),
                transaction_depth: 0,
                pre_eot: Vec::new(),
                pre_post: Vec::new(),
                post: Vec::new(),
                keep_alive: Vec::new(),
                collecting_cycles: false,
                allow_add_roots: true,
                allow_collect_cycles_counter: 0,
            })),
            node_count: Arc::new(AtomicUsize::new(0)),
            node_ref_count: Arc::new(AtomicUsize::new(0)),
            threaded_mode: Arc::new(single_threaded_mode()),
        }
    }

    pub fn gc_ctx(&self) -> GcCtx {
        self.gc_ctx.clone()
    }

    pub fn null_node(&self) -> Node {
        Node::new(self, NodeName::NullNode, || {}, Vec::new())
    }

    pub fn transaction<R, K: FnOnce() -> R>(&self, k: K) -> R {
        self.enter_transaction();
        let result = k();
        self.leave_transaction();
        result
    }

    pub fn enter_transaction(&self) {
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth += 1;
        });
    }

    pub fn leave_transaction(&self) {
        let is_end_of_transaction = self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth -= 1;
            data.transaction_depth == 0
        });
        if is_end_of_transaction {
            self.end_of_transaction();
        }
    }

    pub fn add_dependents_to_changed_nodes(&self, node: &dyn IsNode) {
        self.with_data(|data: &mut SodiumCtxData| {
            let node_dependents = node.data().dependents.read();
            node_dependents
                .iter()
                .flat_map(|node: &Box<dyn IsWeakNode + Send + Sync + 'static>| node.upgrade())
                .for_each(|node: Box<dyn IsNode + Send + Sync>| {
                    data.changed_nodes.push(node);
                });
        });
    }

    pub fn pre_eot<K: FnMut() + Send + 'static>(&self, k: K) {
        self.with_data(|data: &mut SodiumCtxData| data.pre_eot.push(Box::new(k)));
    }

    pub fn pre_post<K: FnMut() + Send + 'static>(&self, k: K) {
        self.with_data(|data: &mut SodiumCtxData| {
            data.pre_post.push(Box::new(k));
        });
    }

    pub fn post<K: FnMut() + Send + 'static>(&self, k: K) {
        self.with_data(|data: &mut SodiumCtxData| {
            data.post.push(Box::new(k));
        });
    }

    pub fn with_data<R, K: FnOnce(&mut SodiumCtxData) -> R>(&self, k: K) -> R {
        let mut data = self.data.lock();
        k(&mut data)
    }

    pub fn node_count(&self) -> usize {
        self.node_count.load(Ordering::Relaxed)
    }

    pub fn inc_node_count(&self) {
        self.node_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_node_count(&self) {
        self.node_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn node_ref_count(&self) -> usize {
        self.node_ref_count.load(Ordering::Relaxed)
    }

    pub fn inc_node_ref_count(&self) {
        self.node_ref_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_node_ref_count(&self) {
        self.node_ref_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn end_of_transaction(&self) {
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth += 1;
            data.allow_collect_cycles_counter += 1;
        });
        // pre eot
        {
            let pre_eot = self.with_data(|data: &mut SodiumCtxData| {
                let mut pre_eot: Vec<Box<dyn FnMut() + Send>> = Vec::new();
                mem::swap(&mut pre_eot, &mut data.pre_eot);
                pre_eot
            });
            for mut k in pre_eot {
                k();
            }
        }
        //
        loop {
            let changed_nodes: Vec<Box<dyn IsNode>> = self.with_data(|data: &mut SodiumCtxData| {
                let mut changed_nodes: Vec<Box<dyn IsNode>> = Vec::new();
                mem::swap(&mut changed_nodes, &mut data.changed_nodes);
                changed_nodes
            });
            if changed_nodes.is_empty() {
                break;
            }
            for node in changed_nodes {
                self.update_node(node.node());
            }
        }
        self.with_data(|data: &mut SodiumCtxData| {
            data.transaction_depth -= 1;
        });
        // pre_post
        {
            let pre_post = self.with_data(|data: &mut SodiumCtxData| {
                let mut pre_post: Vec<Box<dyn FnMut() + Send>> = Vec::new();
                mem::swap(&mut pre_post, &mut data.pre_post);
                pre_post
            });
            for mut k in pre_post {
                k();
            }
        }
        // post
        {
            let post = self.with_data(|data: &mut SodiumCtxData| {
                let mut post: Vec<Box<dyn FnMut() + Send>> = Vec::new();
                mem::swap(&mut post, &mut data.post);
                post
            });
            for mut k in post {
                k();
            }
        }
        let allow_collect_cycles = self.with_data(|data: &mut SodiumCtxData| {
            data.allow_collect_cycles_counter -= 1;
            data.allow_collect_cycles_counter == 0
        });
        if allow_collect_cycles {
            // gc
            self.collect_cycles()
        }
    }

    pub fn update_node(&self, node: &Node) {
        let bail;
        {
            let mut visited = node.data.visited.write();
            bail = *visited;
            *visited = true;
        }
        if bail {
            return;
        }
        let dependencies: Vec<Box<dyn IsNode + Send + Sync + 'static>>;
        {
            let dependencies2 = node.data.dependencies.read();
            dependencies = box_clone_vec_is_node(&dependencies2);
        }
        {
            let node = node.clone();
            self.pre_post(move || {
                let mut visited = node.data.visited.write();
                *visited = false;
            });
        }
        // visit dependencies
        let handle;
        {
            let dependencies = box_clone_vec_is_node(&dependencies);
            let _self = self.clone();
            handle = self.threaded_mode.spawn(move || {
                for dependency in &dependencies {
                    let visit_it = !*dependency.data().visited.read();
                    if visit_it {
                        _self.update_node(dependency.node());
                    }
                }
            });
        }
        handle.join();
        // any dependencies changed?
        let any_changed =
            dependencies
                .iter()
                .any(|node: &Box<dyn IsNode + Send + Sync + 'static>| {
                    *node.node().data().changed.read()
                });
        // if dependencies changed, then execute update on current node
        if any_changed {
            let mut update = node.data.update.write();
            let update: &mut Box<_> = &mut *update;
            update();
        }
        // if self changed then update dependents
        if *node.data.changed.read() {
            let dependents = box_clone_vec_is_weak_node(&node.data().dependents.read());
            {
                let _self = self.clone();
                for dependent in dependents {
                    if let Some(dependent2) = dependent.upgrade() {
                        _self.update_node(dependent2.node());
                    }
                }
            }
        }
    }

    pub fn collect_cycles(&self) {
        self.gc_ctx.collect_cycles();
    }
}
