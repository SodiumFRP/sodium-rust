use crate::impl_::router::Router as RouterImpl;
use crate::SodiumCtx;
use crate::Stream;
use std::hash::Hash;

pub struct Router<A, K> {
    impl_: RouterImpl<A, K>,
}

impl<A, K> Router<A, K> {
    pub fn new(
        sodium_ctx: &SodiumCtx,
        in_stream: &Stream<A>,
        selector: impl Fn(&A) -> Vec<K> + Send + Sync + 'static,
    ) -> Router<A, K>
    where
        A: Clone + Send + 'static,
        K: Send + Sync + Eq + Hash + 'static,
    {
        Router {
            impl_: RouterImpl::new(&sodium_ctx.impl_, &in_stream.impl_, selector),
        }
    }

    pub fn filter_matches(&self, k: &K) -> Stream<A>
    where
        A: Clone + Send + 'static,
        K: Clone + Send + Sync + Eq + Hash + 'static,
    {
        Stream {
            impl_: self.impl_.filter_matches(k),
        }
    }
}
