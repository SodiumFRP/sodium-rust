use crate::impl_::gc_node::{GcNode, Tracer};
use crate::impl_::node::{Node, IsNode};
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::sodium_ctx::SodiumCtxData;

use std::sync::Arc;
use std::sync::Mutex;
use std::fmt;

pub struct Listener {
    pub data: Arc<Mutex<ListenerData>>,
    pub gc_node: GcNode
}

impl Clone for Listener {
    fn clone(&self) -> Self {
        self.gc_node.inc_ref();
        Listener {
            data: self.data.clone(),
            gc_node: self.gc_node.clone()
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.gc_node.dec_ref();
    } 
}

pub struct ListenerData {
    pub sodium_ctx: SodiumCtx,
    pub is_weak: bool,
    pub node_op: Option<Node>
}

impl Listener {
    pub fn new(sodium_ctx: &SodiumCtx, is_weak: bool, node: Node) -> Listener {
        let listener_data = Arc::new(Mutex::new(ListenerData {
            sodium_ctx: sodium_ctx.clone(),
            node_op: Some(node),
            is_weak
        }));
        let gc_node_desconstructor;
        {
            let listener_data = listener_data.clone();
            gc_node_desconstructor = move || {
                let mut l = listener_data.lock();
                let listener_data = l.as_mut().unwrap();
                listener_data.node_op = None;
            };
        }
        let gc_node_trace;
        {
            let listener_data = listener_data.clone();
            gc_node_trace = move |tracer: &mut Tracer| {
                let mut l = listener_data.lock();
                let listener_data = l.as_mut().unwrap();
                listener_data.node_op.iter().for_each(|node: &Node| tracer(&node.gc_node));
            };
        }
        let listener = Listener {
            data: listener_data,
            gc_node: GcNode::new(&sodium_ctx.gc_ctx(), "Listener::new", gc_node_desconstructor, gc_node_trace)
        };
        if !is_weak {
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                data.keep_alive.push(listener.clone());
            });
        }
        listener
    }

    pub fn unlisten(&self) {
        let is_weak;
        let sodium_ctx;
        {
            let mut l = self.data.lock();
            let data: &mut ListenerData = l.as_mut().unwrap();
            data.node_op = None;
            is_weak = data.is_weak;
            sodium_ctx = data.sodium_ctx.clone();
        }
        if !is_weak {
            sodium_ctx.with_data(|data: &mut SodiumCtxData| {
                data.keep_alive.retain(|l:&Listener| !Arc::ptr_eq(&l.data,&self.data))
            });
        }
    }

    pub fn node_op(&self) -> Option<Node> {
        self.with_data(|data: &mut ListenerData| data.node_op.clone())
    }

    pub fn with_data<R,K:FnOnce(&mut ListenerData)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data: &mut ListenerData = l.as_mut().unwrap();
        k(data)
    }
}

impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let node_op = self.node_op();
        write!(f, "(Listener")?;
        match node_op {
            Some(node) => {
                writeln!(f, "")?;
                writeln!(f, "{:?})", &node as &(dyn IsNode+Sync+Sync))?;
            }
            None => {
                writeln!(f, ")")?;
            }
        }
        fmt::Result::Ok(())
    }
}
