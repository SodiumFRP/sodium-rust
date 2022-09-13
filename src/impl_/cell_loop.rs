use crate::impl_::cell::Cell;
use crate::impl_::lazy::Lazy;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream_loop::StreamLoop;

use std::mem;
use std::sync::Arc;
use parking_lot::Mutex;

pub struct CellLoop<A> {
    pub init_value_op: Arc<Mutex<Option<Lazy<A>>>>,
    pub stream_loop: StreamLoop<A>,
    pub cell: Cell<A>,
}

impl<A> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            init_value_op: self.init_value_op.clone(),
            stream_loop: self.stream_loop.clone(),
            cell: self.cell.clone(),
        }
    }
}

impl<A: Send + Clone + 'static> CellLoop<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> CellLoop<A> {
        let init_value_op: Arc<Mutex<Option<Lazy<A>>>> = Arc::new(Mutex::new(None));
        let init_value: Lazy<A>;
        {
            let init_value_op = init_value_op.clone();
            init_value = Lazy::new(move || {
                let mut init_value_op = init_value_op.lock();
                let mut result_op: Option<Lazy<A>> = None;
                mem::swap(&mut result_op, &mut init_value_op);
                if let Some(init_value) = result_op {
                    return init_value.run();
                }
                panic!("CellLoop sampled before looped.");
            });
        }
        let stream_loop = StreamLoop::new(sodium_ctx);
        let stream = stream_loop.stream();
        CellLoop {
            init_value_op,
            stream_loop,
            cell: stream.hold_lazy(init_value),
        }
    }

    pub fn cell(&self) -> Cell<A> {
        self.cell.clone()
    }

    pub fn loop_(&self, ca: &Cell<A>) {
        self.stream_loop.loop_(&ca.updates());
        let mut init_value_op = self.init_value_op.lock();
        *init_value_op = Some(ca.sample_lazy());
    }
}
