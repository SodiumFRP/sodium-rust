use sodium::Listener;
use sodium::Node;
use sodium::Transaction;
use std::borrow::Borrow;
use std::cell::Ref;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
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
    pub null_node: Rc<RefCell<Node>>,
    pub next_id: u32,
    pub next_seq: u64,
    pub current_transaction_op: Option<Transaction>,
    pub in_callback: u32,
    pub on_start_hooks: Vec<Box<Fn()>>,
    pub running_on_start_hooks: bool,
    pub keep_listeners_alive: HashMap<u32,Listener>,
    pub num_nodes: u32
}

impl SodiumCtx {
    pub fn new() -> SodiumCtx {
        SodiumCtx {
            data: Rc::new(RefCell::new(SodiumCtxData {
                null_node: Rc::new(RefCell::new(Node::new_(0, 0))),
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

    pub fn null_node(&self) -> Rc<RefCell<Node>> {
        let self_: &RefCell<SodiumCtxData> = self.data.borrow();
        let self__: Ref<SodiumCtxData> = self_.borrow();
        let self___: &SodiumCtxData = &*self__;
        self___.null_node.clone()
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
}
