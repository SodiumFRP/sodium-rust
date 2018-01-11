use sodium::Cell;
use sodium::CellLoop;
use sodium::CellSink;
use sodium::HasNode;
use sodium::Listener;
use sodium::Node;
use sodium::Stream;
use sodium::StreamLoop;
use sodium::StreamSink;
use sodium::Transaction;
use sodium::cell;
use sodium::gc::GcCtx;
use sodium::gc::Gc;
use sodium::gc::GcCell;
use sodium::gc::Trace;
use sodium::stream;
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
    pub null_node: Option<Gc<GcCell<HasNode>>>,
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
        let null_node = gc_ctx.new_gc(GcCell::new(Node::new_(0, 0))).upcast(|x| x as &GcCell<HasNode>);
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

    pub fn new_cell<A: Clone + 'static>(&mut self, a: A) -> Cell<A> {
        cell::make_cell(
            self.clone(),
            cell::CellImpl::new(self, a)
        )
    }

    pub fn new_stream<A: Clone + 'static>(&mut self) -> Stream<A> {
        stream::make_stream(
            self.clone(),
            stream::StreamImpl::new(self)
        )
    }

    pub fn new_stream_sink<A: Clone + 'static>(&mut self) -> StreamSink<A> {
        StreamSink::new(self)
    }

    pub fn new_stream_sink_with_coalescer<A: Clone + 'static, F: Fn(&A,&A)->A + 'static>(&mut self, f: F) -> StreamSink<A> {
        StreamSink::new_with_coalescer(self, f)
    }

    pub fn new_cell_sink<A: Clone + 'static>(&mut self, a: A) -> CellSink<A> {
        CellSink::new(self, a)
    }

    pub fn new_stream_loop<A: Clone + 'static>(&mut self) -> StreamLoop<A> {
        StreamLoop::new(self)
    }

    pub fn new_cell_loop<A: Clone + 'static>(&mut self) -> CellLoop<A> {
        CellLoop::new(self)
    }

    pub fn run_transaction<A, F: FnOnce()->A>(&mut self, code: F) -> A {
        Transaction::run(self, |_: &mut SodiumCtx| code())
    }

    pub fn gc(&self) {
        let gc_ctx = self.with_data_ref(|data| data.gc_ctx.clone());
        gc_ctx.collect_cycles();
    }

    pub fn null_node(&self) -> Gc<GcCell<HasNode>> {
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

    pub fn new_gc<A: Trace + 'static>(&mut self, a: A) -> Gc<A> {
        self.with_data_mut(|data| data.gc_ctx.new_gc(a))
    }
}
