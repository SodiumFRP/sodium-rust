use sodium::Cell;
use sodium::HandlerRefMut;
use sodium::HasNode;
use sodium::IsCell;
use sodium::IsStream;
use sodium::cell::IsCellPrivate;
use sodium::stream::IsStreamPrivate;
use sodium::SodiumCtx;
use sodium::Stream;
use sodium::stream::StreamImpl;
use sodium::StreamWithSend;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use sodium::cell;
use sodium::stream;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Operational {}

impl Operational {
    pub fn updates<CA,A>(c: &CA) -> Stream<A>
        where A: Clone + 'static,
              CA: IsCell<A>
    {
        let c = c.cell();
        let c = &c;
        stream::make_stream(
            cell::get_sodium_ctx(c),
            OperationalPrivate::updates(
                &mut cell::get_sodium_ctx(c),
                &cell::get_cell_impl(c)
            )
        )
    }

    pub fn value<CA,A>(c: &CA) -> Stream<A>
        where A: Clone + 'static,
              CA: IsCell<A>
    {
        let c = c.cell();
        let c = &c;
        stream::make_stream(
            cell::get_sodium_ctx(c),
            OperationalPrivate::value(
                &mut cell::get_sodium_ctx(c),
                &cell::get_cell_impl(c)
            )
        )
    }

    pub fn defer<SA,A>(s: &SA) -> Stream<A>
        where A: Clone + 'static,
              SA: IsStream<A>
    {
        let s = s.stream();
        let s = &s;
        stream::make_stream(
            stream::get_sodium_ctx(s),
            OperationalPrivate::defer(
                &mut stream::get_sodium_ctx(s),
                &stream::get_stream_impl(s)
            )
        )
    }

    pub fn split<SC,C,A>(s: &SC) -> Stream<A>
        where A: Clone + 'static,
              C: IntoIterator<Item=A> + 'static + Clone,
              SC: IsStream<Rc<C>>
    {
        let s = s.stream();
        let s = &s;
        stream::make_stream(
            stream::get_sodium_ctx(s),
            OperationalPrivate::split(
                &mut stream::get_sodium_ctx(s),
                &stream::get_stream_impl(s)
            )
        )
    }
}

struct OperationalPrivate {}

impl OperationalPrivate {
    pub fn updates<CA,A>(sodium_ctx: &mut SodiumCtx, c: &CA) -> StreamImpl<A>
        where A: Clone + 'static,
              CA: IsCellPrivate<A>
    {
        Transaction::apply(
            sodium_ctx,
            |_sodium_ctx, trans| {
                c.updates_(trans)
            }
        )
    }

    pub fn value<CA,A>(sodium_ctx: &mut SodiumCtx, c: &CA) -> StreamImpl<A>
        where A: Clone + 'static,
              CA: IsCellPrivate<A>
    {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                c.value_(sodium_ctx, trans)
            }
        )
    }

    pub fn defer<SA,A>(sodium_ctx: &mut SodiumCtx, s: &SA) -> StreamImpl<A>
        where A: Clone + 'static,
              SA: IsStreamPrivate<A>
    {
        let mut sodium_ctx2 = sodium_ctx.clone();
        let sodium_ctx2 = &mut sodium_ctx2;
        OperationalPrivate::split(
            sodium_ctx,
            &s.map(
                sodium_ctx2,
                |a: &A| {
                    Rc::new(vec![a.clone()])
                }
            )
        )
    }

    pub fn split<SC,C,A>(sodium_ctx: &mut SodiumCtx, s: &SC) -> StreamImpl<A>
        where A: Clone + 'static,
              C: IntoIterator<Item=A> + 'static + Clone,
              SC: IsStreamPrivate<Rc<C>>
    {
        let out = StreamWithSend::new(sodium_ctx);
        let l1;
        {
            let out_node = out.stream.data.clone().upcast(|x| x as &RefCell<HasNode>);
            let out = out.downgrade();
            let mut sodium_ctx2 = sodium_ctx.clone();
            let sodium_ctx2 = &mut sodium_ctx2;
            l1 = s.listen_(
                sodium_ctx,
                out_node,
                TransactionHandlerRef::new(
                    sodium_ctx2,
                    move |_sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, c: &Rc<C>| {
                        let mut child_ix = 0;
                        for a in (**c).clone() {
                            let a = a.clone();
                            let out = out.clone();
                            trans.post_(
                                child_ix,
                                HandlerRefMut::new(
                                    move |sodium_ctx: &mut SodiumCtx, trans_op: &mut Option<Transaction>| {
                                        let out = out.clone();
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
        out.unsafe_add_cleanup(l1).to_stream()
    }
}
