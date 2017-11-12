use sodium::HasNode;
use sodium::Listener;
use sodium::Node;
use sodium::Transaction;
use sodium::gc::GcCtx;
use sodium::gc::Gc;
use std::borrow::Borrow;
use std::cell::Ref;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct SodiumCtx {
    pub data: Rc<RefCell<SodiumCtxData>>
}

impl Clone for SodiumCtx {
    fn clone(&self) -> Self {
        SodiumCtx {
            data: self.data.clone()
        }
    }
}

pub struct SodiumCtxData {
    pub gc_ctx: GcCtx,
    pub null_node: Option<Gc<RefCell<HasNode>>>,
    pub next_id: u32,
    pub next_seq: u64,
    pub current_transaction_op: Option<Transaction>,
    pub in_callback: u32,
    pub on_start_hooks: Vec<Box<Fn()>>,
    pub running_on_start_hooks: bool,
    pub keep_listeners_alive: HashMap<u32,Listener>,
    pub num_nodes: u32
}

impl Drop for SodiumCtxData {
    fn drop(&mut self) {
        self.null_node = None;
    }
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        let mut gc_ctx = GcCtx::new();
        let null_node = gc_ctx.new_gc(RefCell::new(Node::new_(0, 0))).upcast(|x| x as &RefCell<HasNode>);
        SodiumCtx {
            data: Rc::new(RefCell::new(SodiumCtxData {
                gc_ctx: gc_ctx,
                null_node: Some(null_node),
                next_id: 1,
                next_seq: 1,
                current_transaction_op: None,
                in_callback: 0,
                on_start_hooks: Vec::new(),
                running_on_start_hooks: false,
                keep_listeners_alive: HashMap::new(),
                num_nodes: 0
            }))
        }
    }

    pub fn with_data_ref<F,A>(&self, f: F) -> A where F: FnOnce(&SodiumCtxData)->A {
        let self_: &RefCell<SodiumCtxData> = self.data.borrow();
        let self__: Ref<SodiumCtxData> = self_.borrow();
        let self___: &SodiumCtxData = &*self__;
        f(self___)
    }

    pub fn with_data_mut<F,A>(&self, f: F) -> A where F: FnOnce(&mut SodiumCtxData) -> A {
        f(&mut *self.data.borrow_mut())
    }

    pub fn gc(&self) {
        let gc_ctx = self.with_data_ref(|data| data.gc_ctx.clone());
        gc_ctx.collect_cycles();
    }

    pub fn null_node(&self) -> Gc<RefCell<HasNode>> {
        let self_: &RefCell<SodiumCtxData> = self.data.borrow();
        let self__: Ref<SodiumCtxData> = self_.borrow();
        let self___: &SodiumCtxData = &*self__;
        self___.null_node.clone().unwrap()
    }

    pub fn new_id(&mut self) -> u32 {
        let self_ = &mut *self.data.borrow_mut();
        let r = self_.next_id;
        self_.next_id = self_.next_id + 1;
        r
    }

    pub fn new_seq(&mut self) -> u64 {
        let self_ = &mut *self.data.borrow_mut();
        let r = self_.next_seq;
        self_.next_seq = self_.next_seq + 1;
        r
    }

    pub fn new_gc<A:'static>(&mut self, a: A) -> Gc<A> {
        self.with_data_mut(|data| data.gc_ctx.new_gc(a))
    }
}
