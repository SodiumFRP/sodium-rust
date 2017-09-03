use sodium::HandlerRefMut;
use sodium::node::IsNode;
use sodium::IsStream;
use sodium::Lazy;
use sodium::Listener;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cell::Ref;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::Deref;

pub trait IsCell<A: Clone + 'static> {
    fn to_cell_ref(&self) -> &Cell<A>;

    fn with_cell_data_ref<F,R>(&self, f: F) -> R where F: FnOnce(&CellData<A>)->R {
        let data2 = (*self.to_cell_ref().data).borrow();
        let data3 = data2.borrow();
        f(data3)
    }

    fn with_cell_data_mut<F,R>(&self, f: F) -> R where F: FnOnce(&mut CellData<A>)->R {
        let mut data2 = (*self.to_cell_ref().data).borrow_mut();
        let data3 = data2.borrow_mut();
        f(data3)
    }

    fn new_value_(&self) -> A {
        let value_op = self.with_cell_data_ref(|data| data.value_update.clone());
        match value_op {
            Some(value) => value,
            None => self.sample_no_trans_()
        }
    }

    fn sample(&self, sodium_ctx: &mut SodiumCtx) -> A {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                self.sample_no_trans_()
            }
        )
    }

    fn sample_lazy(&self, sodium_ctx: &mut SodiumCtx) -> Lazy<A> {
        let me = self.to_cell_ref().clone();
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans|
                me.sample_lazy_(sodium_ctx, trans)
        )
    }

    fn sample_lazy_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> Lazy<A> {
        let me = self.to_cell_ref().clone();
        let s = LazySample::new(me.clone());
        {
            let s = s.clone();
            trans.last(
                move || {
                    let mut s = s.data.deref().borrow_mut();
                    s.value_op = Some(
                        me.data.deref().borrow()
                            .value_update.clone().unwrap_or_else(|| me.sample_no_trans_())
                    );
                    s.cell_op = None;
                }
            );
        }
        let sodium_ctx = sodium_ctx.clone();
        Lazy::new(move || {
            let s = s.data.deref().borrow();
            match &s.value_op {
                &Some(ref value) => return value.clone(),
                &None => ()
            }
            let mut sodium_ctx = sodium_ctx.clone();
            let sodium_ctx = &mut sodium_ctx;
            s.cell_op.as_ref().unwrap().sample(sodium_ctx)
        })
    }

    fn sample_no_trans_(&self) -> A {
        self.with_cell_data_ref(|data| data.value.clone())
    }

    fn updates_(&self, trans: &mut Transaction) -> Stream<A> {
        self.with_cell_data_ref(|data| data.str.clone())
    }

    fn value_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> Stream<A> {
        let s_spark = StreamWithSend::new(sodium_ctx);
        let s_spark_node = s_spark.stream.data.deref().borrow().node.clone();
        {
            let s_spark = s_spark.clone();
            trans.prioritized(
                sodium_ctx,
                s_spark_node,
                HandlerRefMut::new(
                    move |sodium_ctx, trans2| s_spark.send(sodium_ctx, trans2, &())
                )
            );
        }
        let s_initial = s_spark.snapshot(self.to_cell_ref());
        s_initial.merge(sodium_ctx, &self.updates_(trans), |_,a| a.clone())
    }

    fn map<F,B:'static + Clone>(&self, sodium_ctx: &mut SodiumCtx, f: F) -> Cell<B> where F: Fn(&A)->B + 'static {
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans| {
                let f2 = Rc::new(f);
                let f3 = f2.clone();
                let tmp =
                    self.sample_lazy_(sodium_ctx, trans)
                        .map(move |a| f2(a));
                self.updates_(trans)
                    .map(sodium_ctx, move |a| f3(a))
                    .hold_lazy_(
                        trans,
                        tmp
                    )
            }
        )
    }
}

pub struct Cell<A> {
    data: Rc<RefCell<CellData<A>>>
}

pub struct CellData<A> {
    str: Stream<A>,
    value: A,
    value_update: Option<A>,
    cleanup: Option<Listener>,
    lazy_init_value: Option<Lazy<A>>
}

impl<A: Clone + 'static> IsCell<A> for Cell<A> {
    fn to_cell_ref(&self) -> &Cell<A> {
        self
    }
}

impl<A> Clone for Cell<A> {
    fn clone(&self) -> Self {
        Cell {
            data: self.data.clone()
        }
    }
}

impl<A:'static + Clone> Cell<A> {
    pub fn constant(sodium_ctx: &mut SodiumCtx, value: A) -> Cell<A> {
        Cell {
            data: Rc::new(RefCell::new(
                CellData {
                    str: Stream::new(sodium_ctx),
                    value: value,
                    value_update: None,
                    cleanup: None,
                    lazy_init_value: None
                }
            ))
        }
    }

    pub fn new_(sodium_ctx: &mut SodiumCtx, str: Stream<A>, init_value: A) -> Cell<A> {
        let r = Cell {
            data: Rc::new(RefCell::new(
                CellData {
                    str: str,
                    value: init_value,
                    value_update: None,
                    cleanup: None,
                    lazy_init_value: None
                }
            ))
        };
        let self_ = r.clone();
        Transaction::run_trans(
            sodium_ctx,
            HandlerRefMut::new(
                move |sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction| {
                    let sodium_ctx = sodium_ctx.clone();
                    let self__ = self_.clone();
                    self_.with_cell_data_mut(move |data| {
                        let mut sodium_ctx2 = sodium_ctx.clone();
                        let self__ = self__.clone();
                        data.cleanup = Some(data.str.listen2(
                            &mut sodium_ctx2,
                            sodium_ctx.null_node(),
                            trans1,
                            TransactionHandlerRef::new(
                                move |sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                                    let self___ = self__.clone();
                                    self__.with_cell_data_mut(move |data| {
                                        if data.value_update.is_none() {
                                            trans2.last(
                                                move || {
                                                    self___.with_cell_data_mut(|data| {
                                                        data.value = data.value_update.clone().unwrap();
                                                        data.lazy_init_value = None;
                                                        data.value_update = None;
                                                    });
                                                }
                                            );
                                        }
                                        data.value_update = Some(a.clone());
                                    });
                                }
                            ),
                            false
                        ));
                    })
                }
            )
        );
        r
    }
}

struct LazySample<A> {
    data: Rc<RefCell<LazySampleData<A>>>
}

impl<A> Clone for LazySample<A> {
    fn clone(&self) -> Self {
        LazySample {
            data: self.data.clone()
        }
    }
}

struct LazySampleData<A> {
    cell_op: Option<Cell<A>>,
    value_op: Option<A>
}

impl<A> LazySample<A> {
    fn new(cell: Cell<A>) -> LazySample<A> {
        LazySample {
            data: Rc::new(RefCell::new(
                LazySampleData {
                    cell_op: Some(cell),
                    value_op: None
                }
            ))
        }
    }
}

/*
package nz.sodium;

import java.util.Optional;

/**
 * Represents a value of type A that changes over time.
 */
public class Cell<A> {
	final Stream<A> str;
	A value;
	A valueUpdate;
	private Listener cleanup;
    Lazy<A> lazyInitValue;  // Used by LazyCell

	/**
	 * A cell with a constant value.
	 */
    public Cell(A value)
    {
    	this.str = new Stream<A>();
    	this.value = value;
    }

    Cell(final Stream<A> str, A initValue)
    {
    	this.str = str;
    	this.value = initValue;
    	Transaction.run(new Handler<Transaction>() {
    		public void run(Transaction trans1) {
	    		Cell.this.cleanup = str.listen(Node.NULL, trans1, new TransactionHandler<A>() {
	    			public void run(Transaction trans2, A a) {
			    		if (Cell.this.valueUpdate == null) {
			    			trans2.last(new Runnable() {
			    				public void run() {
				    				Cell.this.value = Cell.this.valueUpdate;
				    				Cell.this.lazyInitValue = null;
				    				Cell.this.valueUpdate = null;
				    			}
			    			});
			    		}
			    		Cell.this.valueUpdate = a;
			    	}
	    		}, false);
    		}
    	});
    }

    /**
     * @return The value including any updates that have happened in this transaction.
     */
    final A newValue()
    {
    	return valueUpdate == null ? sampleNoTrans() :  valueUpdate;
    }

    /**
     * Sample the cell's current value.
     * <p>
     * It may be used inside the functions passed to primitives that apply them to {@link Stream}s,
     * including {@link Stream#map(Lambda1)} in which case it is equivalent to snapshotting the cell,
     * {@link Stream#snapshot(Cell, Lambda2)}, {@link Stream#filter(Lambda1)} and
     * {@link Stream#merge(Stream, Lambda2)}.
     * It should generally be avoided in favour of {@link listen(Handler)} so you don't
     * miss any updates, but in many circumstances it makes sense.
     */
    public final A sample()
    {
        return Transaction.apply(new Lambda1<Transaction, A>() {
        	public A apply(Transaction trans) {
        		return sampleNoTrans();
        	}
        });
    }

    private static class LazySample<A> {
        LazySample(Cell<A> cell) {
            this.cell = cell;
        }
        Cell<A> cell;
        boolean hasValue;
        A value;
    }

    /**
     * A variant of {@link sample()} that works with {@link CellLoop}s when they haven't been looped yet.
     * It should be used in any code that's general enough that it could be passed a {@link CellLoop}.
     * @see Stream#holdLazy(Lazy) Stream.holdLazy()
     */
    public final Lazy<A> sampleLazy() {
        final Cell<A> me = this;
        return Transaction.apply(new Lambda1<Transaction, Lazy<A>>() {
        	public Lazy<A> apply(Transaction trans) {
        	    return me.sampleLazy(trans);
            }
        });
    }

    final Lazy<A> sampleLazy(Transaction trans) {
        final Cell<A> me = this;
        final LazySample<A> s = new LazySample<A>(me);
        trans.last(new Runnable() {
            public void run() {
                s.value = me.valueUpdate != null ? me.valueUpdate : me.sampleNoTrans();
                s.hasValue = true;
                s.cell = null;
            }
        });
        return new Lazy<A>(new Lambda0<A>() {
            public A apply() {
                if (s.hasValue)
                    return s.value;
                else
                    return s.cell.sample();
            }
        });
    }

    A sampleNoTrans()
    {
        return value;
    }

    final Stream<A> updates(Transaction trans)
    {
        return str;
    }

    final Stream<A> value(Transaction trans1)
    {
    	final StreamWithSend<Unit> sSpark = new StreamWithSend<Unit>();
        trans1.prioritized(sSpark.node, new Handler<Transaction>() {
            public void run(Transaction trans2) {
                sSpark.send(trans2, Unit.UNIT);
            }
        });
    	Stream<A> sInitial = sSpark.<A>snapshot(this);
        return sInitial.merge(updates(trans1), new Lambda2<A,A,A>() {
            public A apply(A left, A right) { return right; }
        });
    }

    /**
     * Transform the cell's value according to the supplied function, so the returned Cell
     * always reflects the value of the function applied to the input Cell's value.
     * @param f Function to apply to convert the values. It must be <em>referentially transparent</em>.
     */
	public final <B> Cell<B> map(final Lambda1<A,B> f)
	{
		return Transaction.apply(new Lambda1<Transaction, Cell<B>>() {
			public Cell<B> apply(Transaction trans) {
                return updates(trans).map(f).holdLazy(trans, sampleLazy(trans).map(f));
            }
        });
	}

	/**
	 * Lift a binary function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 */
	public final <B,C> Cell<C> lift(Cell<B> b, final Lambda2<A,B,C> fn)
	{
		Lambda1<A, Lambda1<B,C>> ffa = new Lambda1<A, Lambda1<B,C>>() {
			public Lambda1<B,C> apply(final A aa) {
				return new Lambda1<B,C>() {
					public C apply(B bb) {
						return fn.apply(aa,bb);
					}
				};
			}
		};
		Cell<Lambda1<B,C>> bf = this.map(ffa);
		return this.apply(bf, b);
	}

	/**
	 * Lift a ternary function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 */
	public final <B,C,D> Cell<D> lift(Cell<B> b, Cell<C> c, final Lambda3<A,B,C,D> fn)
	{
		Lambda1<A, Lambda1<B, Lambda1<C,D>>> ffa = new Lambda1<A, Lambda1<B, Lambda1<C,D>>>() {
			public Lambda1<B, Lambda1<C,D>> apply(final A aa) {
				return new Lambda1<B, Lambda1<C,D>>() {
					public Lambda1<C,D> apply(final B bb) {
						return new Lambda1<C,D>() {
							public D apply(C cc) {
								return fn.apply(aa,bb,cc);
							}
						};
					}
				};
			}
		};
		Cell<Lambda1<B, Lambda1<C, D>>> bf = this.map(ffa);
		return this.apply(apply(bf, b), c);
	}

	/**
	 * Lift a quaternary function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 */
	public final <B,C,D,E> Cell<E> lift(Cell<B> b, Cell<C> c, Cell<D> d, final Lambda4<A,B,C,D,E> fn)
	{
		Lambda1<A, Lambda1<B, Lambda1<C, Lambda1<D,E>>>> ffa = new Lambda1<A, Lambda1<B, Lambda1<C, Lambda1<D,E>>>>() {
			public Lambda1<B, Lambda1<C, Lambda1<D,E>>> apply(final A aa) {
				return new Lambda1<B, Lambda1<C, Lambda1<D,E>>>() {
					public Lambda1<C, Lambda1<D, E>> apply(final B bb) {
						return new Lambda1<C, Lambda1<D,E>>() {
							public Lambda1<D,E> apply(final C cc) {
                                return new Lambda1<D, E>() {
                                    public E apply(D dd) {
                                        return fn.apply(aa,bb,cc,dd);
                                    }
                                };
							}
						};
					}
				};
			}
		};
		Cell<Lambda1<B, Lambda1<C, Lambda1<D, E>>>> bf = this.map(ffa);
		return this.apply(apply(apply(bf, b), c), d);
	}

	/**
	 * Lift a 5-argument function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 */
	public final <B,C,D,E,F> Cell<F> lift(Cell<B> b, Cell<C> c, Cell<D> d, Cell<E> e, final Lambda5<A,B,C,D,E,F> fn)
	{
		Lambda1<A, Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, F>>>>> ffa = new Lambda1<A, Lambda1<B, Lambda1<C, Lambda1<D,Lambda1<E, F>>>>>() {
			public Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, F>>>> apply(final A aa) {
				return new Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, F>>>>() {
					public Lambda1<C, Lambda1<D, Lambda1<E, F>>> apply(final B bb) {
						return new Lambda1<C, Lambda1<D, Lambda1<E, F>>>() {
							public Lambda1<D, Lambda1<E, F>> apply(final C cc) {
                                return new Lambda1<D, Lambda1<E, F>>() {
                                    public Lambda1<E, F> apply(final D dd) {
                                        return new Lambda1<E, F>() {
                                            public F apply(E ee) {
                                                return fn.apply(aa,bb,cc,dd,ee);
                                            }
                                        };
                                    }
                                };
							}
						};
					}
				};
			}
		};
		Cell<Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, F>>>>> bf = this.map(ffa);
		return this.apply(apply(apply(apply(bf, b), c), d), e);
	}

	/**
	 * Lift a 6-argument function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 */
	public final <B,C,D,E,F,G> Cell<G> lift(Cell<B> b, Cell<C> c, Cell<D> d, Cell<E> e, Cell<F> f, final Lambda6<A,B,C,D,E,F,G> fn)
	{
		Lambda1<A, Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, Lambda1<F, G>>>>>> ffa = new Lambda1<A, Lambda1<B, Lambda1<C, Lambda1<D,Lambda1<E, Lambda1<F, G>>>>>>() {
			public Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, Lambda1<F, G>>>>> apply(final A aa) {
				return new Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, Lambda1<F, G>>>>>() {
					public Lambda1<C, Lambda1<D, Lambda1<E, Lambda1<F, G>>>> apply(final B bb) {
						return new Lambda1<C, Lambda1<D, Lambda1<E, Lambda1<F, G>>>>() {
							public Lambda1<D, Lambda1<E, Lambda1<F, G>>> apply(final C cc) {
                                return new Lambda1<D, Lambda1<E, Lambda1<F, G>>>() {
                                    public Lambda1<E, Lambda1<F, G>> apply(final D dd) {
                                        return new Lambda1<E, Lambda1<F, G>>() {
                                            public Lambda1<F, G> apply(final E ee) {
                                                return new Lambda1<F, G>() {
                                                    public G apply(final F ff) {
                                                        return fn.apply(aa,bb,cc,dd,ee,ff);
                                                    }
                                                };
                                            }
                                        };
                                    }
                                };
							}
						};
					}
				};
			}
		};
		Cell<Lambda1<B, Lambda1<C, Lambda1<D, Lambda1<E, Lambda1<F, G>>>>>> bf = this.map(ffa);
		return this.apply(apply(apply(apply(apply(bf, b), c), d), e), f);
	}

	/**
	 * Apply a value inside a cell to a function inside a cell. This is the
	 * primitive for all function lifting.
	 */
	public static <A,B> Cell<B> apply(final Cell<Lambda1<A,B>> bf, final Cell<A> ba)
	{
    	return Transaction.apply(new Lambda1<Transaction, Cell<B>>() {
    		public Cell<B> apply(Transaction trans0) {
                final StreamWithSend<B> out = new StreamWithSend<B>();

                class ApplyHandler implements Handler<Transaction> {
                    ApplyHandler(Transaction trans0) {
                    }
                    Lambda1<A,B> f = null;
                    boolean f_present = false;
                    A a = null;
                    boolean a_present = false;
                    @Override
                    public void run(Transaction trans1) {
                        trans1.prioritized(out.node, new Handler<Transaction>() {
                            public void run(Transaction trans2) {
                                out.send(trans2, f.apply(a));
                            }
                        });
                    }
                }

                Node out_target = out.node;
                final Node in_target = new Node(0);
                Node.Target[] node_target_ = new Node.Target[1];
                in_target.linkTo(null, out_target, node_target_);
                final Node.Target node_target = node_target_[0];
                final ApplyHandler h = new ApplyHandler(trans0);
                Listener l1 = bf.value(trans0).listen_(in_target, new TransactionHandler<Lambda1<A,B>>() {
                    public void run(Transaction trans1, Lambda1<A,B> f) {
                        h.f = f;
                        h.f_present = true;
                        if (h.a_present)
                            h.run(trans1);
                    }
                });
                Listener l2 = ba.value(trans0).listen_(in_target, new TransactionHandler<A>() {
                    public void run(Transaction trans1, A a) {
                        h.a = a;
                        h.a_present = true;
                        if (h.f_present)
                            h.run(trans1);
                    }
                });
                return out.lastFiringOnly(trans0).unsafeAddCleanup(l1).unsafeAddCleanup(l2).unsafeAddCleanup(
                    new Listener() {
                        public void unlisten() {
                            in_target.unlinkTo(node_target);
                        }
                    }).holdLazy(new Lazy<B>(new Lambda0<B>() {
                        public B apply() {
                            return bf.sampleNoTrans().apply(ba.sampleNoTrans());
                        }
                    }));
            }
        });
	}

	/**
	 * Unwrap a cell inside another cell to give a time-varying cell implementation.
	 */
	public static <A> Cell<A> switchC(final Cell<Cell<A>> bba)
	{
	    return Transaction.apply(new Lambda1<Transaction, Cell<A>>() {
	        public Cell<A> apply(Transaction trans0) {
                Lazy<A> za = bba.sampleLazy().map(new Lambda1<Cell<A>, A>() {
                    public A apply(Cell<A> ba) {
                        return ba.sample();
                    }
                });
                final StreamWithSend<A> out = new StreamWithSend<A>();
                TransactionHandler<Cell<A>> h = new TransactionHandler<Cell<A>>() {
                    private Listener currentListener;
                    @Override
                    public void run(Transaction trans2, Cell<A> ba) {
                        // Note: If any switch takes place during a transaction, then the
                        // lastFiringOnly() below will always cause a sample to be fetched
                        // from the one we just switched to. So anything from the old input cell
                        // that might have happened during this transaction will be suppressed.
                        if (currentListener != null)
                            currentListener.unlisten();
                        currentListener = ba.value(trans2).listen(out.node, trans2, new TransactionHandler<A>() {
                            public void run(Transaction trans3, A a) {
                                out.send(trans3, a);
                            }
                        }, false);
                    }

                    @Override
                    protected void finalize() throws Throwable {
                        if (currentListener != null)
                            currentListener.unlisten();
                    }
                };
                Listener l1 = bba.value(trans0).listen_(out.node, h);
                return out.lastFiringOnly(trans0).unsafeAddCleanup(l1).holdLazy(za);
            }
        });
	}

	/**
	 * Unwrap a stream inside a cell to give a time-varying stream implementation.
	 */
	public static <A> Stream<A> switchS(final Cell<Stream<A>> bea)
	{
        return Transaction.apply(new Lambda1<Transaction, Stream<A>>() {
        	public Stream<A> apply(final Transaction trans) {
                return switchS(trans, bea);
        	}
        });
    }

	private static <A> Stream<A> switchS(final Transaction trans1, final Cell<Stream<A>> bea)
	{
        final StreamWithSend<A> out = new StreamWithSend<A>();
        final TransactionHandler<A> h2 = new TransactionHandler<A>() {
        	public void run(Transaction trans2, A a) {
	            out.send(trans2, a);
	        }
        };
        TransactionHandler<Stream<A>> h1 = new TransactionHandler<Stream<A>>() {
            private Listener currentListener = bea.sampleNoTrans().listen(out.node, trans1, h2, false);

            @Override
            public void run(final Transaction trans2, final Stream<A> ea) {
                trans2.last(new Runnable() {
                	public void run() {
	                    if (currentListener != null)
	                        currentListener.unlisten();
	                    currentListener = ea.listen(out.node, trans2, h2, true);
	                }
                });
            }

            @Override
            protected void finalize() throws Throwable {
                if (currentListener != null)
                    currentListener.unlisten();
            }
        };
        Listener l1 = bea.updates(trans1).listen(out.node, trans1, h1, false);
        return out.unsafeAddCleanup(l1);
	}

	@Override
	protected void finalize() throws Throwable {
	    if (cleanup != null)
            cleanup.unlisten();
	}

	/**
	 * Listen for updates to the value of this cell. This is the observer pattern. The
	 * returned {@link Listener} has a {@link Listener#unlisten()} method to cause the
	 * listener to be removed. This is an OPERATIONAL mechanism is for interfacing between
	 * the world of I/O and for FRP.
	 * @param action The handler to execute when there's a new value.
	 *   You should make no assumptions about what thread you are called on, and the
	 *   handler should not block. You are not allowed to use {@link CellSink#send(Object)}
	 *   or {@link StreamSink#send(Object)} in the handler.
	 *   An exception will be thrown, because you are not meant to use this to create
	 *   your own primitives.
     */
	public final Listener listen(final Handler<A> action) {
        return Transaction.apply(new Lambda1<Transaction, Listener>() {
        	public Listener apply(final Transaction trans) {
                return value(trans).listen(action);
			}
		});
	}

	/**
	 * Lift a binary function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 * @deprecated As of release 1.1.0, replaced by {@link lift(Cell,Lambda2)}
	 */
    @Deprecated
	public static final <A,B,C> Cell<C> lift(final Lambda2<A,B,C> fn, Cell<A> a, Cell<B> b)
	{
		return a.lift(b, fn);
	}

	/**
	 * Lift a ternary function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 * @deprecated As of release 1.1.0, replaced by {@link lift(Cell,Cell,Lambda3)}
	 */
    @Deprecated
	public static final <A,B,C,D> Cell<D> lift(final Lambda3<A,B,C,D> fn, Cell<A> a, Cell<B> b, Cell<C> c)
	{
		return a.lift(b, c, fn);
	}

	/**
	 * Lift a quaternary function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 * @deprecated As of release 1.1.0, replaced by {@link lift(Cell,Cell,Cell,Lambda4)}
	 */
    @Deprecated
	public static final <A,B,C,D,E> Cell<E> lift(final Lambda4<A,B,C,D,E> fn, Cell<A> a, Cell<B> b, Cell<C> c, Cell<D> d)
	{
		return a.lift(b, c, d, fn);
	}

	/**
	 * Lift a 5-argument function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 * @deprecated As of release 1.1.0, replaced by {@link lift(Cell,Cell,Cell,Cell,Lambda5)}
	 */
    @Deprecated
	public static final <A,B,C,D,E,F> Cell<F> lift(final Lambda5<A,B,C,D,E,F> fn, Cell<A> a, Cell<B> b, Cell<C> c, Cell<D> d, Cell<E> e)
	{
		return a.lift(b, c, d, e, fn);
	}

	/**
	 * Lift a 6-argument function into cells, so the returned Cell always reflects the specified
	 * function applied to the input cells' values.
	 * @param fn Function to apply. It must be <em>referentially transparent</em>.
	 * @deprecated As of release 1.1.0, replaced by {@link lift(Cell,Cell,Cell,Cell,Cell,Lambda6)}
	 */
    @Deprecated
	public static final <A,B,C,D,E,F,G> Cell<G> lift(final Lambda6<A,B,C,D,E,F,G> fn, Cell<A> a, Cell<B> b, Cell<C> c, Cell<D> d, Cell<E> e, Cell<F> f)
	{
		return a.lift(b, c, d, e, f, fn);
	}
}
*/
