use sodium::gc::Finalize;
use sodium::gc::GcCtx;
use sodium::gc::Trace;
use sodium::impl_::IsLambda0;
use sodium::impl_::Listener;
use sodium::impl_::MemoLazy;
use sodium::impl_::Node;
use std::cell::UnsafeCell;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::mem::swap;
use std::rc::Rc;
use std::rc::Weak;

pub struct SodiumCtx {
    pub data: Rc<UnsafeCell<SodiumCtxData>>
}

pub struct WeakSodiumCtx {
    pub data: Weak<UnsafeCell<SodiumCtxData>>
}

pub struct SodiumCtxData {
    pub gc_ctx: GcCtx,
    pub next_id: u32,
    pub transaction_depth: u32,
    pub to_be_updated: BinaryHeap<Node>,
    pub to_be_updated_set: HashSet<Node>,
    pub resort_required: bool,
    pub pre_trans: Vec<Box<FnMut()>>,
    pub post_trans: Vec<Box<FnMut()>>,
    pub node_count: u32,
    pub keep_alive: HashSet<Node>
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            data: Rc::new(UnsafeCell::new(SodiumCtxData {
                gc_ctx: GcCtx::new(),
                next_id: 0,
                transaction_depth: 0,
                to_be_updated: BinaryHeap::new(),
                to_be_updated_set: HashSet::new(),
                resort_required: false,
                pre_trans: Vec::new(),
                post_trans: Vec::new(),
                node_count: 0,
                keep_alive: HashSet::new()
            }))
        }
    }

    pub fn gc_ctx(&self) -> GcCtx {
        let self_ = unsafe { &*(*self.data).get() };
        self_.gc_ctx.clone()
    }

    pub fn downgrade(&self) -> WeakSodiumCtx {
        WeakSodiumCtx {
            data: Rc::downgrade(&self.data)
        }
    }

    pub fn new_lazy<A: Trace + Finalize + Clone + 'static, THUNK: IsLambda0<A> + 'static>(&self, thunk: THUNK) -> MemoLazy<A> {
        let mut gc_ctx = self.gc_ctx();
        let gc_ctx = &mut gc_ctx;
        MemoLazy::new(gc_ctx, thunk)
    }

    pub fn new_id(&self) -> u32 {
        let self_ = unsafe { &mut *(*self.data).get() };
        let id = self_.next_id;
        self_.next_id = self_.next_id + 1;
        id
    }

    pub fn add_keep_alive(&self, node: Node) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.keep_alive.insert(node);
    }

    pub fn remove_keep_alive(&self, node: &Node) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.keep_alive.remove(node);
    }

    pub fn inc_node_count(&self) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.node_count = self_.node_count + 1;
    }

    pub fn dec_node_count(&self) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.node_count = self_.node_count - 1;
    }

    pub fn node_count(&self) -> u32 {
        let self_ = unsafe { &*(*self.data).get() };
        self_.node_count
    }

    pub fn pre<F: FnMut() + 'static>(&self, f: F) {
        self.transaction(|| {
            let self_ = unsafe { &mut *(*self.data).get() };
            self_.pre_trans.push(Box::new(f));
        });
    }

    pub fn post<F: FnMut() + 'static>(&self, f: F) {
        self.transaction(|| {
            let self_ = unsafe { &mut *(*self.data).get() };
            self_.post_trans.push(Box::new(f));
        });
    }

    pub fn transaction<A,CODE:FnOnce()->A>(&self, code: CODE)->A {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.transaction_depth = self_.transaction_depth + 1;
        let result = code();
        self_.transaction_depth = self_.transaction_depth - 1;
        if self_.transaction_depth == 0 {
            self.propergate();
        }
        result
    }

    pub fn schedule_update_sort(&self) {
        let self_ = unsafe { &mut *(*self.data).get() };
        self_.resort_required = true;
    }

    fn propergate(&self) {
        let self_ = unsafe { &mut *(*self.data).get() };
        if self_.resort_required {
            self_.to_be_updated.clear();
            for node in &self_.to_be_updated_set {
                self_.to_be_updated.push(node.clone());
            }
            self_.resort_required = false;
        }
        loop {
            let mut pre_trans = Vec::new();
            swap(&mut self_.pre_trans, &mut pre_trans);
            for mut f in pre_trans {
                f();
            }
            if self_.pre_trans.is_empty() {
                break;
            }
        }
        self_.transaction_depth = self_.transaction_depth + 1;
        loop {
            let node_op = self_.to_be_updated.pop();
            match node_op {
                Some(node) => {
                    self_.to_be_updated_set.remove(&node);
                    let mark_dependents_dirty = node.update();
                    if mark_dependents_dirty {
                        node.mark_dependents_dirty();
                    }
                },
                None => break
            }
        }
        self_.transaction_depth = self_.transaction_depth - 1;
        loop {
            let mut post_trans = Vec::new();
            swap(&mut self_.post_trans, &mut post_trans);
            for mut f in post_trans {
                f();
            }
            if self_.post_trans.is_empty() {
                break;
            }
        }
    }
}

impl WeakSodiumCtx {
    pub fn upgrade(&self) -> Option<SodiumCtx> {
        self.data.upgrade().map(|data| SodiumCtx { data })
    }
}

impl Clone for SodiumCtx {
    fn clone(&self) -> Self {
        SodiumCtx {
            data: self.data.clone()
        }
    }
}

impl Clone for WeakSodiumCtx {
    fn clone(&self) -> Self {
        WeakSodiumCtx {
            data: self.data.clone()
        }
    }
}
