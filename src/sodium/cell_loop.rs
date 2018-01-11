use sodium::Cell;
use sodium::cell::HasCell;
use sodium::HasCellDataGc;
use sodium::IsCell;
use sodium::SodiumCtx;
use sodium::StreamLoop;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use sodium::cell;
use sodium::cell::CellData;
use sodium::cell::IsCellPrivate;
use sodium::stream;
use sodium::stream::IsStreamPrivate;
use sodium::gc::Gc;
use sodium::gc::GcCell;
use std::cell::RefCell;

pub struct CellLoop<A: Clone + 'static> {
    sodium_ctx: SodiumCtx,
    cell: cell::CellImpl<A>,
    str: StreamLoop<A>,
    is_assigned: bool
}

impl<A: Clone + 'static> HasCell<A> for CellLoop<A> {
    fn cell(&self) -> Cell<A> {
        cell::make_cell(
            self.sodium_ctx.clone(),
            self.cell.clone()
        )
    }
}

impl<A: Clone + 'static> CellLoop<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> CellLoop<A> {
        let str = StreamLoop::new(sodium_ctx);
        let r = CellLoop {
            sodium_ctx: sodium_ctx.clone(),
            cell: cell::CellImpl {
                data: sodium_ctx.new_gc(GcCell::new(
                    cell::CellData {
                        str: str.to_stream_ref().clone(),
                        value: None,
                        value_update: None,
                        cleanup: None,
                        lazy_init_value: None
                    },
                )).upcast(|x| x as &GcCell<cell::HasCellData<A>>)
            },
            str: str,
            is_assigned: false
        };
        let self_ = r.cell.clone();
        Transaction::run_trans(
            sodium_ctx,
            move |sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction| {
                let sodium_ctx = sodium_ctx.clone();
                let self__ = self_.clone();
                self_.with_cell_data_mut(move |data: &mut CellData<A>| {
                    let mut sodium_ctx2 = sodium_ctx.clone();
                    let mut sodium_ctx3 = sodium_ctx.clone();
                    let self__ = self__.data.downgrade();
                    data.cleanup = Some(data.str.listen2(
                        &mut sodium_ctx2,
                        sodium_ctx.null_node(),
                        trans1,
                        TransactionHandlerRef::new(
                            &mut sodium_ctx3,
                            move |_sodium_ctx: &mut SodiumCtx, trans2: &mut Transaction, a: &A| {
                                let self___ = cell::CellImpl { data: self__.upgrade().unwrap() };
                                self___.clone().with_cell_data_mut(move |data| {
                                    if data.value_update.is_none() {
                                        trans2.last(
                                            move || {
                                                self___.with_cell_data_mut(|data| {
                                                    data.value = data.value_update.clone();
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
                        false,
                        false
                    ));
                })
            }
        );
        r
    }

    pub fn loop_<CA>(&mut self, a_out: &CA) where CA: IsCell<A> {
        let mut sodium_ctx = cell::get_sodium_ctx(&a_out.cell());
        let sodium_ctx = &mut sodium_ctx;
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                let a_out2 = cell::get_cell_impl(&a_out.cell());
                let mut sodium_ctx2 = sodium_ctx.clone();
                let sodium_ctx2 = &mut sodium_ctx2;
                self.str.loop_(
                    stream::make_stream(
                        sodium_ctx2.clone(),
                        a_out2.updates_(trans).weak(sodium_ctx2)
                    )
                );
                self.cell.data.borrow_mut().cell_data_mut().lazy_init_value = Some(a_out2.sample_lazy_(sodium_ctx, trans));
            }
        )
    }
}
