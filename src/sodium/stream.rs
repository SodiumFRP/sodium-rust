use sodium::Cell;
use sodium::CoalesceHandler;
use sodium::HandlerRef;
use sodium::HandlerRefMut;
use sodium::IsCell;
use sodium::Lazy;
use sodium::Listener;
use sodium::Node;
use sodium::HasNode;
use sodium::SodiumCtx;
use sodium::StreamWithSend;
use sodium::Target;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use std::cell::RefCell;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;
use std::rc::Weak;

pub struct Stream<A> {
    pub data: Rc<RefCell<StreamData<A>>>
}

pub struct WeakStream<A> {
    pub data: Weak<RefCell<StreamData<A>>>
}

pub trait IsStream<A: Clone + 'static> {
    fn to_stream_ref(&self) -> &Stream<A>;

    fn listen<F>(&self, sodium_ctx: &mut SodiumCtx, handler: F) -> Listener where F: Fn(&A) + 'static {
        let l0 = self.listen_weak(sodium_ctx, handler);
        let mut l_id = RefCell::new(0);
        let l_id2 = l_id.clone();
        let sodium_ctx2 = sodium_ctx.clone();
        let l = Listener::new(
            sodium_ctx,
            move || {
                l0.unlisten();
                sodium_ctx2.with_data_mut(|ctx| ctx.keep_listeners_alive.remove(&*l_id2.borrow()));
            }
        );
        *l_id.get_mut() = l.id;
        sodium_ctx.with_data_mut(|ctx| ctx.keep_listeners_alive.insert(l.id.clone(), l.clone()));
        l
    }

    fn listen_once<F>(&self, sodium_ctx: &mut SodiumCtx, handler: F) -> Listener where F: Fn(&A) + 'static {
        let listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
        let listener2 = listener.clone();
        let result = self.listen(
            sodium_ctx,
            move |a: &A| {
                let mut tmp = listener2.borrow_mut();
                let tmp2 = &mut *tmp;
                match tmp2 {
                    &mut Some(ref mut tmp3) => tmp3.unlisten(),
                    &mut None => ()
                }
                handler(a);
            }
        );
        *listener.borrow_mut() = Some(result.clone());
        result
    }

    fn listen_(&self, sodium_ctx: &mut SodiumCtx, target: Rc<RefCell<HasNode>>, action: TransactionHandlerRef<A>) -> Listener {
        Transaction::apply(
            sodium_ctx,
            move |sodium_ctx, trans1| {
                self.listen2(sodium_ctx, target, trans1, action, false)
            }
        )
    }

    fn listen_weak<F>(&self, sodium_ctx: &mut SodiumCtx, action: F) -> Listener where F: Fn(&A) + 'static {
        let null_node = sodium_ctx.null_node();
        return self.listen_(
            sodium_ctx,
            null_node,
            TransactionHandlerRef::new(
                move |sodium_ctx, trans2, a| {
                    action(a)
                }
            )
        );
    }

    fn listen2(&self, sodium_ctx: &mut SodiumCtx, target: Rc<RefCell<HasNode>>, trans: &mut Transaction, action: TransactionHandlerRef<A>, suppress_earlier_firings: bool) -> Listener {
        let mut self_ = self.to_stream_ref().data.borrow_mut();
        let mut self__: &mut StreamData<A> = &mut *self_;
        let node_target;
        let regen;
        {
            let mut node = self__.node_mut();
            let node2: &mut HasNode = &mut *node;
            let (node_target2, regen2) = node2.link_to(sodium_ctx, target.clone(), action.clone());
            node_target = node_target2;
            regen = regen2;
        }
        let firings = self__.firings.clone();
        if !suppress_earlier_firings && !firings.is_empty() {
            let action = action.clone();
            trans.prioritized(
                sodium_ctx,
                target,
                HandlerRefMut::new(
                    move |sodium_ctx, trans2| {
                        for a in &firings {
                            sodium_ctx.with_data_mut(|ctx| ctx.in_callback = ctx.in_callback + 1);
                            action.run(sodium_ctx, trans2, a);
                            sodium_ctx.with_data_mut(|ctx| ctx.in_callback = ctx.in_callback - 1);
                        }
                    }
                )
            );
        }
        ListenerImpl::new(self.to_stream_ref().clone(), action, node_target)
            .into_listener(sodium_ctx)
    }

    fn map<B:'static + Clone,F>(&self, sodium_ctx: &mut SodiumCtx, f: F) -> Stream<B>
    where F: Fn(&A)->B + 'static
    {
        let out = StreamWithSend::new(sodium_ctx);
        let out2 = out.clone();
        let l = self.listen_(
            sodium_ctx,
            out.stream.data.clone() as Rc<RefCell<HasNode>>,
            TransactionHandlerRef::new(
                move |sodium_ctx, trans, a| {
                    out2.send(sodium_ctx, trans, &f(a));
                }
            )
        );
        out.unsafe_add_cleanup(l)
    }

    fn map_to<B:'static + Clone>(&self, sodium_ctx: &mut SodiumCtx, b: B) -> Stream<B> {
        return self.map(sodium_ctx, move |_| b.clone());
    }

    fn hold(&self, sodium_ctx: &mut SodiumCtx, init_value: A) -> Cell<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| Cell::new_(sodium_ctx, self.to_stream_ref().clone(), init_value)
        )
    }

    fn hold_lazy_(&self, trans: &mut Transaction, initial_value: Lazy<A>) -> Cell<A> {
        unimplemented!();
    }

    fn snapshot<CB,B>(&self, c: &CB) -> Stream<B> where CB: IsCell<B>, B: Clone + 'static {
        unimplemented!();
    }

    fn or_else<SA>(&self, sodium_ctx: &mut SodiumCtx, s: &SA) -> Stream<A> where SA: IsStream<A> {
        self.merge(sodium_ctx, s, |a,_| a.clone())
    }

    fn merge_<SA>(&self, sodium_ctx: &mut SodiumCtx, s: &SA) -> Stream<A> where SA: IsStream<A> {
        let out = StreamWithSend::<A>::new(sodium_ctx);
        let left = Rc::new(RefCell::new(Node::new(sodium_ctx, 0))) as Rc<RefCell<HasNode>>;
        let right = out.to_stream_ref().data.clone() as Rc<RefCell<HasNode>>;
        let (node_target, _) = left.borrow_mut().link_to(
            sodium_ctx,
            right.clone(),
            TransactionHandlerRef::new(
                |_: &mut SodiumCtx, _: &mut Transaction, _: &A| ()
            )
        );
        let h;
        {
            let out = out.clone();
            h = TransactionHandlerRef::new(
                move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &A| {
                    out.send(sodium_ctx, trans, a);
                }
            );
        }
        let l1 = self.listen_(sodium_ctx, left.clone(), h.clone());
        let l2 = s.listen_(sodium_ctx, right, h);
        out.unsafe_add_cleanup(l1).unsafe_add_cleanup(l2).unsafe_add_cleanup(Listener::new(
            sodium_ctx,
            move || {
                left.borrow_mut().unlink_to(&node_target);
            }
        ))
    }

    fn merge<SA,F>(&self, sodium_ctx: &mut SodiumCtx, s: &SA, f: F) -> Stream<A> where SA: IsStream<A>, F: Fn(&A,&A)->A + 'static {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction| {
                self.merge_(sodium_ctx, s).coalesce_(sodium_ctx,trans, f)
            }
        )
    }

    fn coalesce_<F>(&self, sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction, f: F) -> Stream<A> where F: Fn(&A,&A)->A + 'static {
        let out = StreamWithSend::new(sodium_ctx);
        let h = CoalesceHandler::new(f, &out);
        let l = self.listen2(
            sodium_ctx,
            out.to_stream_ref().data.clone() as Rc<RefCell<HasNode>>,
            trans1,
            h.to_transaction_handler(),
            false
        );
        out.unsafe_add_cleanup(l)
    }

    fn last_firing_only_(&self, sodium_ctx: &mut SodiumCtx, trans: &mut Transaction) -> Stream<A> {
        self.coalesce_(sodium_ctx, trans, |_,a| a.clone())
    }

    fn filter<F>(&self, sodium_ctx: &mut SodiumCtx, predicate: F) -> Stream<A> where F: Fn(&A)->bool + 'static {
        let out = StreamWithSend::new(sodium_ctx);
        let l;
        {
            let out = out.clone();
            let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
            l = self.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    move |sodium_ctx, trans, a| {
                        if predicate(a) {
                            out.send(sodium_ctx, trans, a);
                        }
                    }
                )
            );
        }
        out.unsafe_add_cleanup(l)
    }

    fn filter_option<S>(self_: &S, sodium_ctx: &mut SodiumCtx) -> Stream<A> where S: IsStream<Option<A>> {
        let out = StreamWithSend::new(sodium_ctx);
        let l;
        {
            let out = out.clone();
            let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
            l = self_.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    move |sodium_ctx, trans, oa| {
                        match oa {
                            &Some(ref a) => out.send(sodium_ctx, trans, a),
                            &None => ()
                        }
                    }
                )
            );
        }
        out.unsafe_add_cleanup(l)
    }

    fn unsafe_add_cleanup(&self, listener: Listener) -> Stream<A> {
        let mut data = self.to_stream_ref().data.borrow_mut();
        let data_: &mut StreamData<A> = &mut *data;
        data_.finalizers.push(listener);
        self.to_stream_ref().clone()
    }
}

impl<A: 'static + Clone> IsStream<A> for Stream<A> {
    fn to_stream_ref(&self) -> &Stream<A> {
        self
    }
}

impl<A> Clone for Stream<A> {
    fn clone(&self) -> Self {
        Stream {
            data: self.data.clone()
        }
    }
}

impl<A> Clone for WeakStream<A> {
    fn clone(&self) -> Self {
        WeakStream {
            data: self.data.clone()
        }
    }
}

pub struct StreamData<A> {
    pub node: Node,
    pub finalizers: Vec<Listener>,
    pub firings: Vec<A>,
}

impl<A> Drop for StreamData<A> {
    fn drop(&mut self) {
        for firing in &self.finalizers {
            firing.unlisten();
        }
    }
}

impl<A> HasNode for StreamData<A> {
    fn node_ref(&self) -> &Node {
        &self.node
    }
    fn node_mut(&mut self) -> &mut Node {
        &mut self.node
    }
}

struct ListenerImpl<A> {
    event: Stream<A>,
    action: TransactionHandlerRef<A>,
    target: Target,
    done: bool
}

impl<A:'static> ListenerImpl<A> {
    fn new(event: Stream<A>, action: TransactionHandlerRef<A>, target: Target) -> ListenerImpl<A>
    {
        ListenerImpl {
            event: event,
            action: action,
            target: target,
            done: false
        }
    }

    fn into_listener(self, sodium_ctx: &mut SodiumCtx) -> Listener {
        let self_ = RefCell::new(self);
        Listener::new(
            sodium_ctx,
            move || {
                let mut self__ = self_.borrow_mut();
                let self___ = &mut *self__;
                if !self___.done {
                    let mut stream_data = self___.event.data.borrow_mut();
                    HasNode::unlink_to(&mut *stream_data as &mut HasNode, &self___.target);
                    self___.done = true;
                }
            }
        )
    }
}

impl<A: Clone + 'static> Stream<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> Stream<A> {
        Stream {
            data: Rc::new(RefCell::new(
                StreamData {
                    node: Node::new(sodium_ctx, 0),
                    finalizers: Vec::new(),
                    firings: Vec::new()
                }
            ))
        }
    }

    fn new_(node: Node, finalizers: Vec<Listener>, firings: Vec<A>) -> Stream<A> {
        Stream {
            data: Rc::new(RefCell::new(
                StreamData {
                    node: node,
                    finalizers: finalizers,
                    firings: firings
                }
            ))
        }
    }

    pub fn downgrade(&self) -> WeakStream<A> {
        WeakStream {
            data: Rc::downgrade(&self.data)
        }
    }
}

impl<A: Clone + 'static> WeakStream<A> {
    pub fn upgrade(&self) -> Option<Stream<A>> {
        self.data.upgrade().map(|data| {
            Stream {
                data: data
            }
        })
    }
}

/*
package nz.sodium;

import java.util.ArrayList;
import java.util.List;
import java.util.HashSet;
import java.util.Optional;
import java.util.Vector;

/**
 * Represents a stream of discrete events/firings containing values of type A.
 */
public class Stream<A> {
	private static final class ListenerImplementation<A> extends Listener {
		/**
		 * It's essential that we keep the listener alive while the caller holds
		 * the Listener, so that the finalizer doesn't get triggered.
		 */
		private Stream<A> event;
		/**
		 * It's also essential that we keep the action alive, since the node uses
		 * a weak reference.
		 */
		private TransactionHandler<A> action;
		private Node.Target target;

		private ListenerImplementation(Stream<A> event, TransactionHandler<A> action, Node.Target target) {
			this.event = event;
			this.action = action;
			this.target = target;
		}

		public void unlisten() {
		    synchronized (Transaction.listenersLock) {
		        if (this.event != null) {
                    event.node.unlinkTo(target);
                    this.event = null;
                    this.action = null;
                    this.target = null;
                }
            }
		}
	}

	final Node node;
	final List<Listener> finalizers;
	final List<A> firings;

	/**
	 * A stream that never fires.
	 */
	public Stream() {
	    this.node = new Node(0L);
	    this.finalizers = new ArrayList<Listener>();
	    this.firings = new ArrayList<A>();
	}

	private Stream(Node node, List<Listener> finalizers, List<A> firings) {
	    this.node = node;
	    this.finalizers = finalizers;
        this.firings = firings;
	}

    static HashSet<Listener> keepListenersAlive = new HashSet<Listener>();

	/**
	 * Listen for events/firings on this stream. This is the observer pattern. The
	 * returned {@link Listener} has a {@link Listener#unlisten()} method to cause the
	 * listener to be removed. This is an OPERATIONAL mechanism is for interfacing between
	 * the world of I/O and for FRP.
	 * @param handler The handler to execute when there's a new value.
	 *   You should make no assumptions about what thread you are called on, and the
	 *   handler should not block. You are not allowed to use {@link CellSink#send(Object)}
	 *   or {@link StreamSink#send(Object)} in the handler.
	 *   An exception will be thrown, because you are not meant to use this to create
	 *   your own primitives.
     */
	public final Listener listen(final Handler<A> handler) {
        final Listener l0 = listenWeak(handler);
        Listener l = new Listener() {
            public void unlisten() {
                l0.unlisten();
                synchronized (keepListenersAlive) {
                    keepListenersAlive.remove(this);
                }
            }
        };
        synchronized (keepListenersAlive) {
            keepListenersAlive.add(l);
        }
        return l;
	}

    /**
     * A variant of {@link listen(Handler)} that handles the first event and then
     * automatically deregisters itself. This is useful for implementing things that
     * work like promises.
     */
    public final Listener listenOnce(final Handler<A> handler) {
        final Listener[] lRef = new Listener[1];
        lRef[0] = listen(new Handler<A>() {
            public void run(A a) {
                lRef[0].unlisten();
                handler.run(a);
            }
        });
        return lRef[0];
    }

	final Listener listen_(final Node target, final TransactionHandler<A> action) {
		return Transaction.apply(new Lambda1<Transaction, Listener>() {
			public Listener apply(Transaction trans1) {
				return listen(target, trans1, action, false);
			}
		});
	}

    /**
     * A variant of {@link listen(Handler)} that will deregister the listener automatically
     * if the listener is garbage collected. With {@link listen(Handler)}, the listener is
     * only deregistered if {@link Listener#unlisten()} is called explicitly.
     * <P>
     * This method should be used for listeners that are to be passed to {@link Stream#addCleanup(Listener)}
     * to ensure that things don't get kept alive when they shouldn't.
     */
    public final Listener listenWeak(final Handler<A> action) {
		return listen_(Node.NULL, new TransactionHandler<A>() {
			public void run(Transaction trans2, A a) {
				action.run(a);
			}
		});
    }

	@SuppressWarnings("unchecked")
	final Listener listen(Node target, Transaction trans, final TransactionHandler<A> action, boolean suppressEarlierFirings) {
	    Node.Target[] node_target_ = new Node.Target[1];
        synchronized (Transaction.listenersLock) {
            if (node.linkTo((TransactionHandler<Unit>)action, target, node_target_))
                trans.toRegen = true;
        }
        Node.Target node_target = node_target_[0];
        final List<A> firings = new ArrayList<A>(this.firings);
        if (!suppressEarlierFirings && !firings.isEmpty())
            trans.prioritized(target, new Handler<Transaction>() {
                public void run(Transaction trans2) {
                    // Anything sent already in this transaction must be sent now so that
                    // there's no order dependency between send and listen.
                    for (A a : firings) {
                        Transaction.inCallback++;
                        try {  // Don't allow transactions to interfere with Sodium
                               // internals.
                            action.run(trans2, a);
                        } catch (Throwable t) {
                            t.printStackTrace();
                        }
                        finally {
                            Transaction.inCallback--;
                        }
                    }
                }
            });
		return new ListenerImplementation<A>(this, action, node_target);
	}

    /**
     * Transform the stream's event values according to the supplied function, so the returned
     * Stream's event values reflect the value of the function applied to the input
     * Stream's event values.
     * @param f Function to apply to convert the values. It may construct FRP logic or use
     *    {@link Cell#sample()} in which case it is equivalent to {@link Stream#snapshot(Cell)}ing the
     *    cell. Apart from this the function must be <em>referentially transparent</em>.
     */
	public final <B> Stream<B> map(final Lambda1<A,B> f)
	{
	    final Stream<A> ev = this;
	    final StreamWithSend<B> out = new StreamWithSend<B>();
        Listener l = listen_(out.node, new TransactionHandler<A>() {
        	public void run(Transaction trans2, A a) {
	            out.send(trans2, f.apply(a));
	        }
        });
        return out.unsafeAddCleanup(l);
	}

    /**
     * Transform the stream's event values into the specified constant value.
     * @param b Constant value.
     */
	public final <B> Stream<B> mapTo(final B b)
	{
		return this.<B>map(new Lambda1<A, B>() {
			public B apply(A a) {
			    return b;
			}
		});
	}

	/**
	 * Create a {@link Cell} with the specified initial value, that is updated
     * by this stream's event values.
     * <p>
     * There is an implicit delay: State updates caused by event firings don't become
     * visible as the cell's current value as viewed by {@link Stream#snapshot(Cell, Lambda2)}
     * until the following transaction. To put this another way,
     * {@link Stream#snapshot(Cell, Lambda2)} always sees the value of a cell as it was before
     * any state changes from the current transaction.
     */
	public final Cell<A> hold(final A initValue) {
		return Transaction.apply(new Lambda1<Transaction, Cell<A>>() {
			public Cell<A> apply(Transaction trans) {
			    return new Cell<A>(Stream.this, initValue);
			}
		});
	}

	/**
	 * A variant of {@link hold(Object)} with an initial value captured by {@link Cell#sampleLazy()}.
	 */
	public final Cell<A> holdLazy(final Lazy<A> initValue) {
		return Transaction.apply(new Lambda1<Transaction, Cell<A>>() {
			public Cell<A> apply(Transaction trans) {
			    return holdLazy(trans, initValue);
			}
		});
	}

	final Cell<A> holdLazy(Transaction trans, final Lazy<A> initValue) {
	    return new LazyCell<A>(this, initValue);
	}

	/**
	 * Variant of {@link snapshot(Cell, Lambda2)} that captures the cell's value
	 * at the time of the event firing, ignoring the stream's value.
	 */
	public final <B> Stream<B> snapshot(Cell<B> c)
	{
	    return snapshot(c, new Lambda2<A,B,B>() {
	    	public B apply(A a, B b) {
	    		return b;
	    	}
	    });
	}

	/**
	 * Return a stream whose events are the result of the combination using the specified
	 * function of the input stream's event value and the value of the cell at that time.
     * <P>
     * There is an implicit delay: State updates caused by event firings being held with
     * {@link Stream#hold(Object)} don't become visible as the cell's current value until
     * the following transaction. To put this another way, {@link Stream#snapshot(Cell, Lambda2)}
     * always sees the value of a cell as it was before any state changes from the current
     * transaction.
     */
	public final <B,C> Stream<C> snapshot(final Cell<B> c, final Lambda2<A,B,C> f)
	{
	    final Stream<A> ev = this;
		final StreamWithSend<C> out = new StreamWithSend<C>();
        Listener l = listen_(out.node, new TransactionHandler<A>() {
        	public void run(Transaction trans2, A a) {
	            out.send(trans2, f.apply(a, c.sampleNoTrans()));
	        }
        });
        return out.unsafeAddCleanup(l);
	}

	/**
	 * Variant of {@link snapshot(Cell, Lambda2)} that captures the values of
	 * two cells.
     */
	public final <B,C,D> Stream<D> snapshot(final Cell<B> cb, final Cell<C> cc, final Lambda3<A,B,C,D> fn)
	{
		return this.snapshot(cb, new Lambda2<A,B,D>() {
	    	public D apply(A a, B b) {
	    		return fn.apply(a, b, cc.sample());
	    	}
		});
	}

	/**
	 * Variant of {@link snapshot(Cell, Lambda2)} that captures the values of
	 * three cells.
     */
	public final <B,C,D,E> Stream<E> snapshot(final Cell<B> cb, final Cell<C> cc, final Cell<D> cd, final Lambda4<A,B,C,D,E> fn)
	{
		return this.snapshot(cb, new Lambda2<A,B,E>() {
	    	public E apply(A a, B b) {
	    		return fn.apply(a, b, cc.sample(), cd.sample());
	    	}
		});
	}

	/**
	 * Variant of {@link snapshot(Cell, Lambda2)} that captures the values of
	 * four cells.
     */
	public final <B,C,D,E,F> Stream<F> snapshot(final Cell<B> cb, final Cell<C> cc, final Cell<D> cd, final Cell<E> ce, final Lambda5<A,B,C,D,E,F> fn)
	{
		return this.snapshot(cb, new Lambda2<A,B,F>() {
	    	public F apply(A a, B b) {
	    		return fn.apply(a, b, cc.sample(), cd.sample(), ce.sample());
	    	}
		});
	}

	/**
	 * Variant of {@link snapshot(Cell, Lambda2)} that captures the values of
	 * five cells.
     */
	public final <B,C,D,E,F,G> Stream<G> snapshot(final Cell<B> cb, final Cell<C> cc, final Cell<D> cd, final Cell<E> ce, final Cell<F> cf, final Lambda6<A,B,C,D,E,F,G> fn)
	{
		return this.snapshot(cb, new Lambda2<A,B,G>() {
	    	public G apply(A a, B b) {
	    		return fn.apply(a, b, cc.sample(), cd.sample(), ce.sample(), cf.sample());
	    	}
		});
	}

    /**
     * Variant of {@link Stream#merge(Stream, Lambda2)} that merges two streams and will drop an event
     * in the simultaneous case.
     * <p>
     * In the case where two events are simultaneous (i.e. both
     * within the same transaction), the event from <em>this</em> will take precedence, and
     * the event from <em>s</em> will be dropped.
     * If you want to specify your own combining function, use {@link Stream#merge(Stream, Lambda2)}.
     * s1.orElse(s2) is equivalent to s1.merge(s2, (l, r) -&gt; l).
     * <p>
     * The name orElse() is used instead of merge() to make it really clear that care should
     * be taken, because events can be dropped.
     */
	public final Stream<A> orElse(final Stream<A> s)
	{
	    return merge(s, new Lambda2<A,A,A>() {
            public A apply(A left, A right) { return left; }
        });
	}

	private static <A> Stream<A> merge(final Stream<A> ea, final Stream<A> eb)
	{
	    final StreamWithSend<A> out = new StreamWithSend<A>();
        final Node left = new Node(0);
        final Node right = out.node;
        Node.Target[] node_target_ = new Node.Target[1];
        left.linkTo(null, right, node_target_);
        final Node.Target node_target = node_target_[0];
        TransactionHandler<A> h = new TransactionHandler<A>() {
        	public void run(Transaction trans, A a) {
	            out.send(trans, a);
	        }
        };
        Listener l1 = ea.listen_(left, h);
        Listener l2 = eb.listen_(right, h);
        return out.unsafeAddCleanup(l1).unsafeAddCleanup(l2).unsafeAddCleanup(new Listener() {
            public void unlisten() {
                left.unlinkTo(node_target);
            }
        });
	}

    /**
     * Merge two streams of the same type into one, so that events on either input appear
     * on the returned stream.
     * <p>
     * If the events are simultaneous (that is, one event from this and one from <em>s</em>
     * occurring in the same transaction), combine them into one using the specified combining function
     * so that the returned stream is guaranteed only ever to have one event per transaction.
     * The event from <em>this</em> will appear at the left input of the combining function, and
     * the event from <em>s</em> will appear at the right.
     * @param f Function to combine the values. It may construct FRP logic or use
     *    {@link Cell#sample()}. Apart from this the function must be <em>referentially transparent</em>.
     */
    public final Stream<A> merge(final Stream<A> s, final Lambda2<A,A,A> f)
    {
	    return Transaction.apply(new Lambda1<Transaction, Stream<A>>() {
	    	public Stream<A> apply(Transaction trans) {
                return Stream.<A>merge(Stream.this, s).coalesce(trans, f);
	    	}
	    });
    }

    /**
     * Variant of {@link orElse(Stream)} that merges a collection of streams.
     */
    public static <A> Stream<A> orElse(Iterable<Stream<A>> ss) {
        return Stream.<A>merge(ss, new Lambda2<A,A,A>() {
            public A apply(A left, A right) { return right; }
        });
    }

    /**
     * Variant of {@link merge(Stream,Lambda2)} that merges a collection of streams.
     */
    public static <A> Stream<A> merge(Iterable<Stream<A>> ss, final Lambda2<A,A,A> f) {
        Vector<Stream<A>> v = new Vector<Stream<A>>();
        for (Stream<A> s : ss)
            v.add(s);
        return merge(v, 0, v.size(), f);
    }

    private static <A> Stream<A> merge(Vector<Stream<A>> sas, int start, int end, final Lambda2<A,A,A> f) {
        int len = end - start;
        if (len == 0) return new Stream<A>(); else
        if (len == 1) return sas.get(start); else
        if (len == 2) return sas.get(start).merge(sas.get(start+1), f); else {
            int mid = (start + end) / 2;
            return Stream.<A>merge(sas, start, mid, f).merge(Stream.<A>merge(sas, mid, end, f), f);
        }
    }

	private final Stream<A> coalesce(Transaction trans1, final Lambda2<A,A,A> f)
	{
	    final Stream<A> ev = this;
	    final StreamWithSend<A> out = new StreamWithSend<A>();
        TransactionHandler<A> h = new CoalesceHandler<A>(f, out);
        Listener l = listen(out.node, trans1, h, false);
        return out.unsafeAddCleanup(l);
    }

    /**
     * Clean up the output by discarding any firing other than the last one.
     */
    final Stream<A> lastFiringOnly(Transaction trans)
    {
        return coalesce(trans, new Lambda2<A,A,A>() {
        	public A apply(A first, A second) { return second; }
        });
    }

    /**
     * Return a stream that only outputs events for which the predicate returns true.
     */
    public final Stream<A> filter(final Lambda1<A,Boolean> predicate)
    {
        final Stream<A> ev = this;
        final StreamWithSend<A> out = new StreamWithSend<A>();
        Listener l = listen_(out.node, new TransactionHandler<A>() {
        	public void run(Transaction trans2, A a) {
	            if (predicate.apply(a)) out.send(trans2, a);
	        }
        });
        return out.unsafeAddCleanup(l);
    }

    /**
     * Return a stream that only outputs events that have present
     * values, removing the {@link java.util.Optional} wrapper, discarding empty values.
     */
    public static <A> Stream<A> filterOptional(final Stream<Optional<A>> ev)
    {
        final StreamWithSend<A> out = new StreamWithSend<A>();
        Listener l = ev.listen_(out.node, new TransactionHandler<Optional<A>>() {
        	public void run(Transaction trans2, Optional<A> oa) {
	            if (oa.isPresent()) out.send(trans2, oa.get());
	        }
        });
        return out.unsafeAddCleanup(l);
    }

    /**
     * Return a stream that only outputs events from the input stream
     * when the specified cell's value is true.
     */
    public final Stream<A> gate(Cell<Boolean> c)
    {
        return Stream.filterOptional(
            snapshot(c, new Lambda2<A,Boolean,Optional<A>>() {
                public Optional<A> apply(A a, Boolean pred) { return pred ? Optional.of(a) : Optional.<A>empty(); }
            })
        );
    }

    /**
     * Transform an event with a generalized state loop (a Mealy machine). The function
     * is passed the input and the old state and returns the new state and output value.
     * @param f Function to apply to update the state. It may construct FRP logic or use
     *    {@link Cell#sample()} in which case it is equivalent to {@link Stream#snapshot(Cell)}ing the
     *    cell. Apart from this the function must be <em>referentially transparent</em>.
     */
    public final <B,S> Stream<B> collect(final S initState, final Lambda2<A, S, Tuple2<B, S>> f)
    {
        return collectLazy(new Lazy<S>(initState), f);
    }

    /**
     * A variant of {@link collect(Object, Lambda2)} that takes an initial state returned by
     * {@link Cell#sampleLazy()}.
     */
    public final <B,S> Stream<B> collectLazy(final Lazy<S> initState, final Lambda2<A, S, Tuple2<B, S>> f)
    {
        return Transaction.<Stream<B>>run(new Lambda0<Stream<B>>() {
            public Stream<B> apply() {
                final Stream<A> ea = Stream.this;
                StreamLoop<S> es = new StreamLoop<S>();
                Cell<S> s = es.holdLazy(initState);
                Stream<Tuple2<B,S>> ebs = ea.snapshot(s, f);
                Stream<B> eb = ebs.map(new Lambda1<Tuple2<B,S>,B>() {
                    public B apply(Tuple2<B,S> bs) { return bs.a; }
                });
                Stream<S> es_out = ebs.map(new Lambda1<Tuple2<B,S>,S>() {
                    public S apply(Tuple2<B,S> bs) { return bs.b; }
                });
                es.loop(es_out);
                return eb;
            }
        });
    }

    /**
     * Accumulate on input event, outputting the new state each time.
     * @param f Function to apply to update the state. It may construct FRP logic or use
     *    {@link Cell#sample()} in which case it is equivalent to {@link Stream#snapshot(Cell)}ing the
     *    cell. Apart from this the function must be <em>referentially transparent</em>.
     */
    public final <S> Cell<S> accum(final S initState, final Lambda2<A, S, S> f)
    {
        return accumLazy(new Lazy<S>(initState), f);
    }

    /**
     * A variant of {@link accum(Object, Lambda2)} that takes an initial state returned by
     * {@link Cell#sampleLazy()}.
     */
    public final <S> Cell<S> accumLazy(final Lazy<S> initState, final Lambda2<A, S, S> f)
    {
        return Transaction.<Cell<S>>run(new Lambda0<Cell<S>>() {
            public Cell<S> apply() {
                final Stream<A> ea = Stream.this;
                StreamLoop<S> es = new StreamLoop<S>();
                Cell<S> s = es.holdLazy(initState);
                Stream<S> es_out = ea.snapshot(s, f);
                es.loop(es_out);
                return es_out.holdLazy(initState);
            }
        });
    }

    /**
     * Return a stream that outputs only one value: the next event of the
     * input stream, starting from the transaction in which once() was invoked.
     */
    public final Stream<A> once()
    {
        // This is a bit long-winded but it's efficient because it deregisters
        // the listener.
        final Stream<A> ev = this;
        final Listener[] la = new Listener[1];
        final StreamWithSend<A> out = new StreamWithSend<A>();
        la[0] = ev.listen_(out.node, new TransactionHandler<A>() {
        	public void run(Transaction trans, A a) {
	            if (la[0] != null) {
                    out.send(trans, a);
	                la[0].unlisten();
	                la[0] = null;
	            }
	        }
        });
        return out.unsafeAddCleanup(la[0]);
    }

    /**
     * This is not thread-safe, so one of these two conditions must apply:
     * 1. We are within a transaction, since in the current implementation
     *    a transaction locks out all other threads.
     * 2. The object on which this is being called was created has not yet
     *    been returned from the method where it was created, so it can't
     *    be shared between threads.
     */
    Stream<A> unsafeAddCleanup(Listener cleanup)
    {
        finalizers.add(cleanup);
        return this;
    }

    /**
     * Attach a listener to this stream so that its {@link Listener#unlisten()} is invoked
     * when this stream is garbage collected. Useful for functions that initiate I/O,
     * returning the result of it through a stream.
     * <P>
     * You must use this only with listeners returned by {@link listenWeak(Handler)} so that
     * things don't get kept alive when they shouldn't.
     */
    public Stream<A> addCleanup(final Listener cleanup) {
        return Transaction.run(new Lambda0<Stream<A>>() {
            public Stream<A> apply() {
                List<Listener> fsNew = new ArrayList<Listener>(finalizers);
                fsNew.add(cleanup);
                return new Stream<A>(node, fsNew, firings);
            }
        });
    }

	@Override
	protected void finalize() throws Throwable {
		for (Listener l : finalizers)
			l.unlisten();
	}
}
*/
