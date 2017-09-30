use sodium::Cell;
use sodium::IsCell;
use sodium::IsStream;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::Transaction;
use std::rc::Rc;

pub struct Operational {}

impl Operational {
    fn updates<CA,A>(sodium_ctx: &mut SodiumCtx, c: &CA) -> Stream<A>
        where A: Clone + 'static,
              CA: IsCell<A>
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                c.updates_(trans)
            }
        )
    }

    fn value<CA,A>(sodium_ctx: &mut SodiumCtx, c: &CA) -> Stream<A>
        where A: Clone + 'static,
              CA: IsCell<A>
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                c.value_(sodium_ctx, trans)
            }
        )
    }

    fn defer<SA,A>(sodium_ctx: &mut SodiumCtx, s: &SA) -> Stream<A>
        where A: Clone + 'static,
              SA: IsStream<A>
    {
        /*
        Operational::split(
            sodium_ctx,
            &s.map(
                sodium_ctx,
                |a| {
                    Rc::new(vec![a].iter())
                }
            )
        )*/
        unimplemented!();
    }

    fn split<SC,C,A>(sodium_ctx: &mut SodiumCtx, s: &SC) -> Stream<A>
        where A: Clone + 'static,
              C: Iterator<Item=A> + Clone + 'static,
              SC: IsStream<Rc<C>>
    {
        unimplemented!();
    }
}

/*
	public static <A> Stream<A> defer(Stream<A> s)
	{
	    return split(s.map(new Lambda1<A,Iterable<A>>() {
	        public Iterable<A> apply(A a) {
                LinkedList<A> l = new LinkedList<A>();
                l.add(a);
                return l;
            }
        }));
	}
    public static <A, C extends Iterable<A>> Stream<A> split(Stream<C> s) {
	    final StreamWithSend<A> out = new StreamWithSend<A>();
	    Listener l1 = s.listen_(out.node, new TransactionHandler<C>() {
	        public void run(Transaction trans, C as) {
	            int childIx = 0;
                for (final A a : as) {
                    trans.post_(childIx, new Handler<Transaction>() {
                        public void run(Transaction trans) {
                            out.send(trans, a);
                        }
                    });
                    childIx++;
                }
	        }
	    });
	    return out.unsafeAddCleanup(l1);
    }
}
*/
