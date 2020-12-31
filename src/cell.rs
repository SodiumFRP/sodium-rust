use crate::impl_::cell::Cell as CellImpl;
use crate::impl_::lambda::IsLambda1;
use crate::impl_::lambda::IsLambda2;
use crate::impl_::lambda::IsLambda3;
use crate::impl_::lambda::IsLambda4;
use crate::impl_::lambda::IsLambda5;
use crate::impl_::lambda::IsLambda6;
use crate::impl_::lazy::Lazy;
use crate::listener::Listener;
use crate::sodium_ctx::SodiumCtx;
use crate::stream::Stream;
use crate::Dep;

/// Represents a value of type `A` that changes over time.
///
/// In other Functional Reactive Programming (FRP) systems this is
/// also called a _behavior_, _property_, or a _signal_. A `Cell`
/// should be used for modeling any pieces of mutable state in an FRP
/// application.
pub struct Cell<A> {
    pub impl_: CellImpl<A>,
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            impl_: self.impl_.clone(),
        }
    }
}

impl<A: Clone + Send + 'static> Cell<A> {
    /// Create a `Cell` with a constant value.
    pub fn new(sodium_ctx: &SodiumCtx, value: A) -> Cell<A> {
        Cell {
            impl_: CellImpl::new(&sodium_ctx.impl_, value),
        }
    }

    /// Sample the `Cell`'s current value.
    ///
    /// `Cell::sample` may be used in the functions passed to
    /// primitives that apply them to [`Stream`]s, including
    /// [`Stream::map`], [`Stream::snapshot`], [`Stream::filter`], and
    /// [`Stream::merge`].
    ///
    /// When called within a function passed to [`Stream::map`] using
    /// `sample` is equivalent to [snapshotting][Stream::snapshot]
    /// this `Cell` with that [`Stream`].
    pub fn sample(&self) -> A {
        self.impl_.sample()
    }

    /// Sample the `Cell`'s current value lazily.
    ///
    /// When it is necessary to use `sample` while implementing more
    /// general abstractions, `sample_lazy` should be preferred in
    /// case a [`CellLoop`][crate::CellLoop] is passed rather than a
    /// `Cell`.
    ///
    /// See [`Cell::sample`] for more details.
    pub fn sample_lazy(&self) -> Lazy<A> {
        self.impl_.sample_lazy()
    }

    // use as dependency to lambda1, lambda2, etc.
    #[doc(hidden)]
    pub fn to_dep(&self) -> Dep {
        self.impl_.to_dep()
    }

    /// Return a [`Stream`] that gives the updates/steps for a `Cell`.
    ///
    /// ## Important
    ///
    /// This is an operational primitive, which isn't part of the main
    /// Sodium API. It breaks the property of non-detectability of
    /// cell updates/steps. The rule with this primitive is that you
    /// should only use it in functions that don't allow the caller to
    /// detect the `Cell` updates.
    pub fn updates(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.updates(),
        }
    }

    /// Return a [`Stream`] that is guaranteed to fire at least once.
    ///
    /// When `value` is called, the returned `Stream` will fire once
    /// in the current transaction with the current value of this
    /// `Cell` and thereafter behaves like [`Cell::updates`].
    ///
    /// ## Important
    ///
    /// This is an operational primitive, which isn't part of the main
    /// Sodium API. It breaks the property of non-detectability of
    /// cell updates/steps. The rule with this primitive is that you
    /// should only use it in functions that don't allow the caller to
    /// detect the `Cell` updates.
    pub fn value(&self) -> Stream<A> {
        Stream {
            impl_: self.impl_.value(),
        }
    }

    /// Transform the `Cell`s value with the supplied function.
    ///
    /// The returned `Cell` always reflects the value produced by the
    /// function applied to the input `Cell`s value. The given
    /// function _must_ be referentially transparent.
    pub fn map<B: Clone + Send + 'static, FN: IsLambda1<A, B> + Send + Sync + 'static>(
        &self,
        f: FN,
    ) -> Cell<B> {
        Cell {
            impl_: self.impl_.map(f),
        }
    }

    /// Lift a binary function into cells so the returned [`Cell`]
    /// always reflects the specified function applied to the input
    /// cells' values.
    pub fn lift2<
        B: Clone + Send + 'static,
        C: Clone + Send + 'static,
        FN: IsLambda2<A, B, C> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        f: FN,
    ) -> Cell<C> {
        Cell {
            impl_: self.impl_.lift2(&cb.impl_, f),
        }
    }

    /// Lift a ternary function into cells so the returned [`Cell`]
    /// always reflects the specified function applied to the input
    /// cells' values.
    pub fn lift3<
        B: Clone + Send + 'static,
        C: Clone + Send + 'static,
        D: Clone + Send + 'static,
        FN: IsLambda3<A, B, C, D> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        f: FN,
    ) -> Cell<D> {
        Cell {
            impl_: self.impl_.lift3(&cb.impl_, &cc.impl_, f),
        }
    }

    /// Lift a quaternary function into cells so the returned [`Cell`]
    /// always reflects the specified function applied to the input
    /// cells' values.
    pub fn lift4<
        B: Clone + Send + 'static,
        C: Clone + Send + 'static,
        D: Clone + Send + 'static,
        E: Clone + Send + 'static,
        FN: IsLambda4<A, B, C, D, E> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        f: FN,
    ) -> Cell<E> {
        Cell {
            impl_: self.impl_.lift4(&cb.impl_, &cc.impl_, &cd.impl_, f),
        }
    }

    /// Lift a five-argument function into cells so the returned
    /// [`Cell`] always reflects the specified function applied to the
    /// input cells' values.
    pub fn lift5<
        B: Clone + Send + 'static,
        C: Clone + Send + 'static,
        D: Clone + Send + 'static,
        E: Clone + Send + 'static,
        F: Clone + Send + 'static,
        FN: IsLambda5<A, B, C, D, E, F> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        ce: &Cell<E>,
        f: FN,
    ) -> Cell<F> {
        Cell {
            impl_: self
                .impl_
                .lift5(&cb.impl_, &cc.impl_, &cd.impl_, &ce.impl_, f),
        }
    }

    /// Lift a six argument function into cells so the returned
    /// [`Cell`] always reflects the specified function applied to the
    /// input cells' values.
    pub fn lift6<
        B: Clone + Send + 'static,
        C: Clone + Send + 'static,
        D: Clone + Send + 'static,
        E: Clone + Send + 'static,
        F: Clone + Send + 'static,
        G: Clone + Send + 'static,
        FN: IsLambda6<A, B, C, D, E, F, G> + Send + 'static,
    >(
        &self,
        cb: &Cell<B>,
        cc: &Cell<C>,
        cd: &Cell<D>,
        ce: &Cell<E>,
        cf: &Cell<F>,
        f: FN,
    ) -> Cell<G> {
        Cell {
            impl_: self
                .impl_
                .lift6(&cb.impl_, &cc.impl_, &cd.impl_, &ce.impl_, &cf.impl_, f),
        }
    }

    /// Unwrap a [`Stream`] in a `Cell` to give a time-varying stream implementation.
    pub fn switch_s(csa: &Cell<Stream<A>>) -> Stream<A> {
        Stream {
            impl_: CellImpl::switch_s(&csa.map(|sa: &Stream<A>| sa.impl_.clone()).impl_),
        }
    }

    /// Unwrap a `Cell` in another `Cell` to give a time-varying cell implementation.
    pub fn switch_c(cca: &Cell<Cell<A>>) -> Cell<A> {
        Cell {
            impl_: CellImpl::switch_c(&cca.map(|ca: &Cell<A>| ca.impl_.clone()).impl_),
        }
    }

    /// A variant of [`listen`][Cell::listen] that will deregister the
    /// listener automatically if the listener is garbage-collected.
    pub fn listen_weak<K: FnMut(&A) + Send + Sync + 'static>(&self, k: K) -> Listener {
        Listener {
            impl_: self.impl_.listen_weak(k),
        }
    }

    /// Listen for updates to the value of this `Cell`.
    ///
    /// This is the observer pattern. The returned [`Listener`] has an
    /// [`unlisten`][Listener::unlisten] method to cause the listener
    /// to be removed.
    ///
    /// This is an operational mechanism for interfacing between the
    /// world of I/O and FRP.
    pub fn listen<K: IsLambda1<A, ()> + Send + Sync + 'static>(&self, k: K) -> Listener {
        Listener {
            impl_: self.impl_.listen(k),
        }
    }
}
