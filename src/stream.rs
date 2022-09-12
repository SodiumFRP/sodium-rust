use crate::cell::Cell;
use crate::impl_::dep::Dep;
use crate::impl_::lambda::{lambda1, lambda2};
use crate::impl_::lambda::{IsLambda1, IsLambda2, IsLambda3, IsLambda4, IsLambda5, IsLambda6};
use crate::impl_::stream::Stream as StreamImpl;
use crate::listener::Listener;
use crate::sodium_ctx::SodiumCtx;
use crate::Lazy;

/// Represents a stream of discrete events/firings containing values
/// of type `A`.
///
/// Also known in other FRP systems as an _event_ (which would contain
/// _event occurrences_), an _event stream_, an _observable_, or a
/// _signal_.
pub struct Stream<A> {
    pub impl_: StreamImpl<A>,
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            impl_: self.impl_.clone(),
        }
    }
}

impl<A: Clone + Send + 'static> Stream<Option<A>> {
    /// Return a `Stream` that only outputs events that have present
    /// values, removing the `Option` wrapper and discarding empty
    /// values.
    pub fn filter_option(&self) -> Stream<A> {
        self.filter(|a: &Option<A>| a.is_some())
            .map(|a: &Option<A>| a.clone().unwrap())
    }
}

impl<
        'a,
        A: Clone + Send + Sync + 'static,
        COLLECTION: IntoIterator<Item = A> + Clone + Send + 'static,
    > Stream<COLLECTION>
{
    /// Flatten a `Stream` of a collection of `A` into a `Stream` of
    /// single `A`s.
    pub fn split(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.split(),
        }
    }
}

impl<A: Clone + Send + 'static> Stream<A> {
    /// Create a `Stream` that will never fire.
    pub fn new(sodium_ctx: &SodiumCtx) -> Stream<A> {
        Stream {
            impl_: StreamImpl::new(&sodium_ctx.impl_),
        }
    }

    #[doc(hidden)]
    // use as dependency to lambda1, lambda2, etc.
    pub fn to_dep(&self) -> Dep {
        self.impl_.to_dep()
    }

    /// Return a stream whose events are the result of the combination
    /// of the event value and the current value of the cell using the
    /// specified function.
    ///
    /// Note that there is an implicit delay: state updates caused by
    /// event firings being held with [`Stream::hold`] don't become
    /// visible as the cell's current value until the following
    /// transaction. To put this another way, `snapshot` always sees
    /// the value of a cell as it wass before any state changes from
    /// the current transaction.
    pub fn snapshot<
        B: Clone + Send + 'static,
        C: Clone + Send + 'static,
        FN: IsLambda2<A, B, C> + Send + Sync + 'static,
    >(
        &self,
        cb: &Cell<B>,
        f: FN,
    ) -> Stream<C> {
        Stream {
            impl_: self.impl_.snapshot(&cb.impl_, f),
        }
    }

    /// A variant of [`snapshot`][Stream::snapshot] that captures the
    /// cell's value at the time of the event firing, ignoring the
    /// stream's value.
    pub fn snapshot1<B: Send + Clone + 'static>(&self, cb: &Cell<B>) -> Stream<B> {
        self.snapshot(cb, |_a: &A, b: &B| b.clone())
    }

    /// A variant of [`snapshot`][Stream::snapshot] that captures the
    /// value of two cells.
    pub fn snapshot3<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        FN: IsLambda3<A, B, C, D> + Send + Sync + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        mut f: FN,
    ) -> Stream<D> {
        let mut deps = if let Some(deps2) = f.deps_op() {
            deps2.clone()
        } else {
            Vec::new()
        };
        let cc = cc.clone();
        deps.push(cc.to_dep());
        self.snapshot(
            cb,
            lambda2(move |a: &A, b: &B| f.call(a, b, &cc.sample()), deps),
        )
    }

    /// A variant of [`snapshot`][Stream::snapshot] that captures the
    /// value of three cells.
    pub fn snapshot4<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        E: Send + Clone + 'static,
        FN: IsLambda4<A, B, C, D, E> + Send + Sync + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        mut f: FN,
    ) -> Stream<E> {
        let mut deps = if let Some(deps2) = f.deps_op() {
            deps2.clone()
        } else {
            Vec::new()
        };
        let cc = cc.clone();
        let cd = cd.clone();
        deps.push(cc.to_dep());
        deps.push(cd.to_dep());
        self.snapshot(
            cb,
            lambda2(
                move |a: &A, b: &B| f.call(a, b, &cc.sample(), &cd.sample()),
                deps,
            ),
        )
    }

    /// A variant of [`snapshot`][Stream::snapshot] that captures the
    /// value of four cells.
    pub fn snapshot5<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        E: Send + Clone + 'static,
        F: Send + Clone + 'static,
        FN: IsLambda5<A, B, C, D, E, F> + Send + Sync + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        ce: &Cell<E>,
        mut f: FN,
    ) -> Stream<F> {
        let mut deps = if let Some(deps2) = f.deps_op() {
            deps2.clone()
        } else {
            Vec::new()
        };
        let cc = cc.clone();
        let cd = cd.clone();
        let ce = ce.clone();
        deps.push(cc.to_dep());
        deps.push(cd.to_dep());
        deps.push(ce.to_dep());
        self.snapshot(
            cb,
            lambda2(
                move |a: &A, b: &B| f.call(a, b, &cc.sample(), &cd.sample(), &ce.sample()),
                deps,
            ),
        )
    }

    /// A variant of [`snapshot`][Stream::snapshot] that captures the
    /// value of five cells.
    pub fn snapshot6<
        B: Send + Clone + 'static,
        C: Send + Clone + 'static,
        D: Send + Clone + 'static,
        E: Send + Clone + 'static,
        F: Send + Clone + 'static,
        G: Send + Clone + 'static,
        FN: IsLambda6<A, B, C, D, E, F, G> + Send + Sync + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        ce: &Cell<E>,
        cf: &Cell<F>,
        mut f: FN,
    ) -> Stream<G> {
        let mut deps = if let Some(deps2) = f.deps_op() {
            deps2.clone()
        } else {
            Vec::new()
        };
        let cc = cc.clone();
        let cd = cd.clone();
        let ce = ce.clone();
        let cf = cf.clone();
        deps.push(cc.to_dep());
        deps.push(cd.to_dep());
        deps.push(ce.to_dep());
        deps.push(cf.to_dep());
        self.snapshot(
            cb,
            lambda2(
                move |a: &A, b: &B| {
                    f.call(a, b, &cc.sample(), &cd.sample(), &ce.sample(), &cf.sample())
                },
                deps,
            ),
        )
    }

    /// Transform this `Stream`'s event values with the supplied
    /// function.
    ///
    /// The supplied function may construct FRP logic or use
    /// [`Cell::sample`], in which case it's equivalent to
    /// [`snapshot`][Stream::snapshot]ing the cell. In addition, the
    /// function must be referentially transparent.
    pub fn map<B: Send + Clone + 'static, FN: IsLambda1<A, B> + Send + Sync + 'static>(
        &self,
        f: FN,
    ) -> Stream<B> {
        Stream {
            impl_: self.impl_.map(f),
        }
    }

    /// Transform this `Stream`'s event values into the specified constant value.
    pub fn map_to<B: Send + Sync + Clone + 'static>(&self, b: B) -> Stream<B> {
        self.map(move |_: &A| b.clone())
    }

    /// Return a `Stream` that only outputs events for which the predicate returns `true`.
    pub fn filter<PRED: IsLambda1<A, bool> + Send + Sync + 'static>(
        &self,
        pred: PRED,
    ) -> Stream<A> {
        Stream {
            impl_: self.impl_.filter(pred),
        }
    }

    /// Variant of [`merge`][Stream::merge] that merges two streams.
    ///
    /// In the case where two events are simultaneous (both in the
    /// same transaction), the event taken from `self` takes
    /// precedenc, and the event from `s2` will be dropped.
    ///
    /// If you want to specify your own combining function use
    /// [`merge`][Stream::merge]. This function is equivalent to
    /// `s1.merge(s2, |l, _r| l)`. The name `or_else` is used instead
    /// of `merge` to make it clear that care should be taken because
    /// events can be dropped.
    pub fn or_else(&self, s2: &Stream<A>) -> Stream<A> {
        self.merge(s2, |lhs: &A, _rhs: &A| lhs.clone())
    }

    /// Merge two streams of the same type into one, so that events on
    /// either input appear on the returned stream.
    ///
    /// If the events are simultaneous (that is, one event from `self`
    /// and one from `s2` occur in the same transaction), combine them
    /// into one using the specified combining function so that the
    /// returned stream is guaranteed only ever to have one event per
    /// transaction. The event from `self` will appear at the left
    /// input of the combining function, and the event from `s2` will
    /// appear at the right.
    pub fn merge<FN: IsLambda2<A, A, A> + Send + Sync + 'static>(
        &self,
        s2: &Stream<A>,
        f: FN,
    ) -> Stream<A> {
        Stream {
            impl_: self.impl_.merge(&s2.impl_, f),
        }
    }

    /// Returns a cell with the specified initial value, which is
    /// updated by this stream's event values.
    pub fn hold(&self, a: A) -> Cell<A> {
        Cell {
            impl_: self.impl_.hold(a),
        }
    }

    /// A variant of [`hold`][Stream::hold] that uses an initial value
    /// returned by [`Cell::sample_lazy`].
    pub fn hold_lazy(&self, a: Lazy<A>) -> Cell<A> {
        Cell {
            impl_: self.impl_.hold_lazy(a),
        }
    }

    /// Return a stream that only outputs events from the input stream
    /// when the specified cell's value is true.
    pub fn gate(&self, cpred: &Cell<bool>) -> Stream<A> {
        let cpred = cpred.clone();
        let cpred_dep = cpred.to_dep();
        self.filter(lambda1(move |_: &A| cpred.sample(), vec![cpred_dep]))
    }

    /// Return a stream that outputs only one value, which is the next
    /// event of the input stream, starting from the transaction in
    /// `once` was invoked.
    pub fn once(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.once(),
        }
    }

    /// Transform an event with a generalized state loop (a Mealy
    /// machine). The function is passed the input and the old state
    /// and returns the new state and output value.
    pub fn collect<B, S, F>(&self, init_state: S, f: F) -> Stream<B>
    where
        B: Send + Clone + 'static,
        S: Send + Clone + 'static,
        F: IsLambda2<A, S, (B, S)> + Send + Sync + 'static,
    {
        self.collect_lazy(Lazy::new(move || init_state.clone()), f)
    }

    /// A variant of [`collect`][Stream::collect] that takes an
    /// initial state that is returned by [`Cell::sample_lazy`].
    pub fn collect_lazy<B, S, F>(&self, init_state: Lazy<S>, f: F) -> Stream<B>
    where
        B: Send + Clone + 'static,
        S: Send + Clone + 'static,
        F: IsLambda2<A, S, (B, S)> + Send + Sync + 'static,
    {
        Stream {
            impl_: self.impl_.collect_lazy(init_state, f),
        }
    }

    /// Accumulate on an input event, outputting the new state each time.
    ///
    /// As each event is received, the accumulating function `f` is
    /// called with the current state and the new event value. The
    /// accumulating function may construct FRP logic or use
    /// [`Cell::sample`], in which case it's equivalent to
    /// [`snapshot`][Stream::snapshot]ing the cell. In additon, the
    /// function must be referentially transparent.
    pub fn accum<S, F>(&self, init_state: S, f: F) -> Cell<S>
    where
        S: Send + Clone + 'static,
        F: IsLambda2<A, S, S> + Send + Sync + 'static,
    {
        self.accum_lazy(Lazy::new(move || init_state.clone()), f)
    }

    /// A variant of [`accum`][Stream::accum] that takes an initial
    /// state returned by [`Cell::sample_lazy`].
    pub fn accum_lazy<S, F>(&self, init_state: Lazy<S>, f: F) -> Cell<S>
    where
        S: Send + Clone + 'static,
        F: IsLambda2<A, S, S> + Send + Sync + 'static,
    {
        Cell {
            impl_: self.impl_.accum_lazy(init_state, f),
        }
    }

    /// A variant of [`listen`][Stream::listen] that will deregister
    /// the listener automatically if the listener is
    /// garbage-collected.
    ///
    /// With [`listen`][Stream::listen] the listener is only
    /// deregistered if [`Listener::unlisten`] is called explicitly.
    pub fn listen_weak<K: IsLambda1<A, ()> + Send + Sync + 'static>(&self, k: K) -> Listener {
        Listener {
            impl_: self.impl_.listen_weak(k),
        }
    }

    /// Listen for events/firings on this stream.
    ///
    /// This is the observer pattern. The returned [`Listener`] has an
    /// [`unlisten`][Listener::unlisten] method to cause the listener
    /// to be removed. This is an operational mechanism for
    /// interfacing between the world of I/O and FRP.
    ///
    /// The handler function for this listener should make no
    /// assumptions about what thread it will be called on, and the
    /// handler should not block. It also is not allowed to use
    /// [`CellSink::send`][crate::CellSink::send] or
    /// [`StreamSink::send`][crate::StreamSink::send] in the handler.
    pub fn listen<K: IsLambda1<A, ()> + Send + Sync + 'static>(&self, k: K) -> Listener {
        Listener {
            impl_: self.impl_.listen(k),
        }
    }
}
