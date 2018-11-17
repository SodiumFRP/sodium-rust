use sodium::Stream;
use sodium::gc::Finalize;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use sodium::impl_;

pub struct StreamSink<A> {
    pub impl_: impl_::StreamSink<A>
}

impl<A: Clone + Trace + Finalize + 'static> StreamSink<A> {
    pub fn send(&self, a: &A) {
        self.impl_.send(a.clone());
    }

    pub fn to_stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.to_stream()
        }
    }
}

impl<A: Clone + Trace + Finalize + 'static> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            impl_: self.impl_.clone()
        }
    }
}

impl<A: Clone + Trace + Finalize + 'static> Finalize for StreamSink<A> {
    fn finalize(&mut self) {
        self.impl_.finalize()
    }
}

impl<A: Clone + Trace + Finalize + 'static> Trace for StreamSink<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.impl_.trace(f)
    }
}
