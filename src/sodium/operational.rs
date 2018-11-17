use sodium::IsCell;
use sodium::IsStream;
use sodium::Stream;
use sodium::gc::Finalize;
use sodium::gc::Trace;
use sodium::impl_;

pub struct Operational {}

impl Operational {
    pub fn value<A: Finalize + Trace + Clone + 'static,CA:IsCell<A>>(ca: CA) -> Stream<A> {
        Stream {
            impl_: impl_::Operational::value(ca.to_cell().impl_)
        }
    }

    pub fn updates<A: Finalize + Trace + Clone + 'static,CA:IsCell<A>>(ca: CA) -> Stream<A> {
        Stream {
            impl_: impl_::Operational::updates(ca.to_cell().impl_)
        }
    }

    pub fn defer<A: Finalize + Trace + Clone + 'static,SA:IsStream<A>>(sa: SA) -> Stream<A> {
        Stream {
            impl_: impl_::Operational::defer(sa.to_stream().impl_)
        }
    }

    pub fn split<C: Finalize + Trace + Clone + 'static,A,SC:IsStream<C>>(s: SC) -> Stream<A>
        where A: Finalize + Trace + Clone + 'static,
              C: IntoIterator<Item=A> + 'static + Clone
    {
        Stream {
            impl_: impl_::Operational::split(s.to_stream().impl_)
        }
    }
}
