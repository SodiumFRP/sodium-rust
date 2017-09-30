use sodium::Cell;
use sodium::CellData;
use sodium::HasCellData;
use sodium::HasCellDataRc;
use sodium::IsStream;
use sodium::SodiumCtx;
use sodium::StreamSink;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CellSink<A> {
    cell: Cell<A>,
    str: StreamSink<A>
}

impl<A: Clone + 'static> HasCellDataRc<A> for CellSink<A> {
    fn cell_data(&self) -> Rc<RefCell<HasCellData<A>>> {
        self.cell.cell_data()
    }
}

impl<A: Clone + 'static> CellSink<A> {
    fn new(sodium_ctx: &mut SodiumCtx, init_value: A) -> CellSink<A> {
        let str = StreamSink::new(sodium_ctx);
        CellSink {
            cell: Cell::new_(sodium_ctx, str.to_stream_ref().clone(), Some(init_value)),
            str: str
        }
    }

    fn new_with_coalescer<F>(sodium_ctx: &mut SodiumCtx, init_value: A, f: F) -> CellSink<A>
        where F: Fn(&A,&A)->A + 'static
    {
        let str = StreamSink::new_with_coalescer(sodium_ctx, f);
        CellSink {
            cell: Cell::new_(sodium_ctx, str.to_stream_ref().clone(), Some(init_value)),
            str: str
        }
    }

    fn send(&self, sodium_ctx: &mut SodiumCtx, a: &A) {
        self.str.send(sodium_ctx, a)
    }
}
