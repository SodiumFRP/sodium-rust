use sodium::impl_::Node;
use sodium::gc::Finalize;
use sodium::gc::Gc;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use std::cell::UnsafeCell;

pub struct Listener {
    node_op: Gc<UnsafeCell<Option<Node>>>
}

impl Listener {
    pub fn debug(&self) {
        self.node_op.debug();
    }

    pub fn new(node: Node) -> Listener {
        let sodium_ctx = node.sodium_ctx();
        let mut gc_ctx = sodium_ctx.gc_ctx();
        Listener {
            node_op: gc_ctx.new_gc_with_desc(UnsafeCell::new(Some(node)), String::from("Listener::new"))
        }
    }

    pub fn unlisten(&self) {
        let node_op = unsafe { &mut *(*self.node_op).get() };
        if let &mut Some(ref mut node) = node_op {
            node.remove_all_dependencies();
        }
        *node_op = None;
    }
}

impl Finalize for Listener {
    fn finalize(&mut self) {
        self.node_op.finalize();
    }
}

impl Trace for Listener {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.node_op.trace(f);
    }
}