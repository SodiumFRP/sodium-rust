use sodium::impl_::Dep;
use sodium::impl_::Stream;
use sodium::impl_::StreamData;
use sodium::impl_::MemoLazy;
use sodium::impl_::Node;
use sodium::impl_::SodiumCtx;
use sodium::gc::Finalize;
use sodium::gc::Gc;
use sodium::gc::GcDep;
use sodium::gc::Trace;
use std::cell::UnsafeCell;
use std::mem::swap;
use std::rc::Rc;

pub struct StreamSink<A> {
    value: Gc<UnsafeCell<Option<MemoLazy<A>>>>,
    next_value: Gc<UnsafeCell<Option<MemoLazy<A>>>>,
    node: Node,
    will_clear: Rc<UnsafeCell<bool>>,
    coalescer_op: Option<Rc<Fn(&A,&A)->A>>
}

impl<A: Trace + Finalize + Clone + 'static> StreamSink<A> {
    pub fn new(sodium_ctx: &SodiumCtx) -> StreamSink<A> {
        StreamSink::_new(sodium_ctx, None)
    }

    pub fn new_with_coalescer<COALESCER:Fn(&A,&A)->A+'static>(sodium_ctx: &SodiumCtx, coalescer: COALESCER) -> StreamSink<A> {
        StreamSink::_new(sodium_ctx, Some(Rc::new(coalescer)))
    }

    pub fn _new(sodium_ctx: &SodiumCtx, coalescer_op: Option<Rc<Fn(&A,&A)->A>>) -> StreamSink<A> {
        let mut gc_ctx = sodium_ctx.gc_ctx();
        let value = gc_ctx.new_gc_with_desc(UnsafeCell::new(None), String::from("StreamSink_value"));
        let next_value = gc_ctx.new_gc_with_desc(UnsafeCell::new(None), String::from("StreamSink_next_value"));
        let update_deps = vec![Dep { gc_dep: value.to_dep() }, Dep { gc_dep: next_value.to_dep() }];
        StreamSink {
            value: value.clone(),
            next_value: next_value.clone(),
            node: Node::new(
                sodium_ctx,
                move || {
                    let next_value = unsafe { &mut *(*next_value).get() };
                    let mut next_value2 = None;
                    swap(next_value, &mut next_value2);
                    let value = unsafe { &mut *(*value).get() };
                    *value = next_value2.clone();
                    return true;
                },
                update_deps,
                Vec::new(),
                || {},
                String::from("StreamSink::new_node")
            ),
            will_clear: Rc::new(UnsafeCell::new(false)),
            coalescer_op: coalescer_op
        }
    }

    pub fn send(&self, value: A) {
        let sodium_ctx = self.node.sodium_ctx();
        sodium_ctx.transaction(|| {
            let will_clear = unsafe { &mut *(*self.will_clear).get() };
            if !*will_clear {
                *will_clear = true;
                let self_ = self.clone();
                sodium_ctx.post(move || {
                    let value = unsafe { &mut *(*self_.value).get() };
                    let will_clear = unsafe { &mut *(*self_.will_clear).get() };
                    *value = None;
                    *will_clear = false;
                });
            }
            let next_value = unsafe { &mut *(*self.next_value).get() };
            match &self.coalescer_op {
                &Some(ref coalescer) => {
                    let next_value2 =
                        match next_value {
                            &mut Some(ref next_value3) => {
                                let next_value4 = coalescer(next_value3.get(), &value);
                                Some(sodium_ctx.new_lazy(move || next_value4.clone()))
                            },
                            &mut None => Some(sodium_ctx.new_lazy(move || value.clone()))
                        };
                    *next_value = next_value2;
                },
                &None => {
                    *next_value = Some(sodium_ctx.new_lazy(move || value.clone()));
                }
            };
            self.node.mark_dirty();
        });
    }

    pub fn to_stream(&self) -> Stream<A> {
        let mut gc_ctx = self.node.sodium_ctx().gc_ctx();
        Stream {
            data: gc_ctx.new_gc_with_desc(UnsafeCell::new(StreamData {
                value: self.value.clone(),
                node: self.node.clone()
            }), String::from("StreamSink::to_stream"))
        }
    }
}

impl<A: Clone + Trace + Finalize + 'static> Clone for StreamSink<A> {
    fn clone(&self) -> Self {
        StreamSink {
            value: self.value.clone(),
            next_value: self.next_value.clone(),
            node: self.node.clone(),
            will_clear: self.will_clear.clone(),
            coalescer_op: self.coalescer_op.clone()
        }
    }
}

impl<A: Clone + Trace + Finalize + 'static> Finalize for StreamSink<A> {
    fn finalize(&mut self) {
        self.node.finalize();
    }
}

impl<A: Clone + Trace + Finalize + 'static> Trace for StreamSink<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.node.trace(f);
    }
}
