use sodium::impl_::Dep;
use sodium::impl_::Latch;
use sodium::impl_::MemoLazy;
use sodium::impl_::Node;
use sodium::impl_::SodiumCtx;
use sodium::impl_::Stream;
use sodium::impl_::gc::Finalize;
use sodium::impl_::gc::Gc;
use sodium::impl_::gc::Trace;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub struct StreamLoop<A> {
    stream: Stream<A>,
    looped: Rc<UnsafeCell<bool>>
}

impl<A: Trace + Finalize + Clone + 'static> StreamLoop<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamLoop<A> {
        StreamLoop {
            stream: Stream::new(sodium_ctx),
            looped: Rc::new(UnsafeCell::new(false))
        }
    }

    pub fn loop_(&self, sa: Stream<A>) {
        let looped = unsafe { &mut *(*self.looped).get() };
        if *looped {
            panic!("StreamLoop looped more than once.");
        }
        let value = self.stream._value().clone();
        let update_deps = vec![sa.to_dep(), Dep { gc_dep: value.to_dep() }];
        let sa_node = sa._node().clone();
        let node = self.stream._node().clone();
        let sodium_ctx = sa_node.sodium_ctx().clone();
        node.set_update(
            move || {
                let sodium_ctx = &sodium_ctx;
                {
                    let value = unsafe { &mut *(*value).get() };
                    *value = sa.peek_value();
                }
                let value = value.clone();
                sodium_ctx.post(move || {
                    let value = unsafe { &mut *(*value).get() };
                    *value = None;
                });
                return true;
            },
            update_deps
        );
        node.ensure_bigger_than(sa_node.rank());
        node.add_dependencies(vec![sa_node]);
        *looped = true;
    }

    pub fn to_stream(&self) -> Stream<A> {
        self.stream.clone()
    }
}

impl<A: Trace + Finalize + Clone + 'static> Clone for StreamLoop<A> {
    fn clone(&self) -> Self {
        StreamLoop {
            stream: self.stream.clone(),
            looped: self.looped.clone()
        }
    }
}