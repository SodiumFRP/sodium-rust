use crate::impl_::gc_node::GcNode;

#[derive(Clone)]
pub struct Dep {
    gc_node: GcNode
}

impl Dep {
    pub fn new(gc_node: GcNode) -> Dep {
        Dep {
            gc_node
        }
    }

    pub fn gc_node(&self) -> &GcNode {
        &self.gc_node
    }
}
