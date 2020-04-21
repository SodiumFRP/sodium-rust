use crate::impl_::cell::Cell;
use crate::impl_::sodium_ctx::SodiumCtx;
use crate::impl_::stream_sink::StreamSink;

pub struct CellSink<A> {
    cell: Cell<A>,
    stream_sink: StreamSink<A>
}

impl<A> Clone for CellSink<A> {
    fn clone(&self) -> Self {
        CellSink {
            cell: self.cell.clone(),
            stream_sink: self.stream_sink.clone()
        }
    }
}

impl<A:Send+Clone+'static> CellSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx, a: A) -> CellSink<A> {
        let stream_sink = StreamSink::new(sodium_ctx);
        CellSink {
            cell: stream_sink.stream().hold(a),
            stream_sink
        }
    }

    pub fn cell(&self) -> Cell<A> {
        self.cell.clone()
    }

    pub fn send(&self, a: A) {
        self.stream_sink.send(a);
    }
}
