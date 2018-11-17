use sodium::IsStream;
use sodium::Stream;
use sodium::gc::Finalize;
use sodium::gc::Trace;
use sodium::impl_;

pub struct StreamLoop<A> {
    pub impl_: impl_::StreamLoop<A>
}

impl<A: Trace + Finalize + Clone + 'static> StreamLoop<A> {

    pub fn loop_<SA:IsStream<A>>(&self, sa: SA) {
        self.impl_.loop_(sa.to_stream().impl_);
    }

    pub fn to_stream(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.to_stream()
        }
    }
}

impl<A: Trace + Finalize + Clone + 'static> Clone for StreamLoop<A> {
    fn clone(&self) -> Self {
        StreamLoop {
            impl_: self.impl_.clone()
        }
    }
}
