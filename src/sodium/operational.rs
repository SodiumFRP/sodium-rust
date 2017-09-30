use sodium::Cell;
use sodium::HandlerRefMut;
use sodium::HasNode;
use sodium::IsCell;
use sodium::IsStream;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use std::cell::RefCell;
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
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        Operational::split(
            sodium_ctx,
            &s.map(
                sodium_ctx2,
                |a: &A| {
                    Rc::new(vec![a.clone()])
                }
            )
        )
    }

    fn split<SC,C,A>(sodium_ctx: &mut SodiumCtx, s: &SC) -> Stream<A>
        where A: Clone + 'static,
              C: IntoIterator<Item=A> + 'static + Clone,
              SC: IsStream<Rc<C>>
    {
        let out = StreamWithSend::new(sodium_ctx);
        let l1;
        {
            let out = out.clone();
            let out_node = out.stream.data.clone() as Rc<RefCell<HasNode>>;
            l1 = s.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, c: &Rc<C>| {
                        let mut child_ix = 0;
                        for a in (**c).clone() {
                            let a = a.clone();
                            let out = out.clone();
                            trans.post_(
                                child_ix,
                                HandlerRefMut::new(
                                    move |sodium_ctx: &mut SodiumCtx, trans_op: &mut Option<Transaction>| {
                                        let mut out = out.clone();
                                        match trans_op {
                                            &mut Some(ref mut trans) => {
                                                out.send(sodium_ctx, trans, &a);
                                            },
                                            &mut None => ()
                                        }
                                    }
                                )
                            );
                            child_ix = child_ix + 1;
                        }
                    }
                )
            );
        }
        out.unsafe_add_cleanup(l1)
    }
}
