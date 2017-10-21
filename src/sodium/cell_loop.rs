use sodium::CellData;
use sodium::IsCell;
use sodium::IsStream;
use sodium::HandlerRefMut;
use sodium::HasCellData;
use sodium::HasCellDataRc;
use sodium::SodiumCtx;
use sodium::StreamLoop;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CellLoop<A> {
    data: Rc<RefCell<CellLoopData<A>>>
}

impl<A> Clone for CellLoop<A> {
    fn clone(&self) -> Self {
        CellLoop {
            data: self.data.clone()
        }
    }
}

struct CellLoopData<A> {
    cell_data: CellData<A>,
    str: StreamLoop<A>,
    is_assigned: bool
}

impl<A: Clone + 'static> HasCellDataRc<A> for CellLoop<A> {
    fn cell_data(&self) -> Rc<RefCell<HasCellData<A>>> {
        self.data.clone() as Rc<RefCell<HasCellData<A>>>
    }
}

impl<A> HasCellData<A> for CellLoopData<A> {
    fn cell_data_ref(&self) -> &CellData<A> {
        &self.cell_data
    }
    fn cell_data_mut(&mut self) -> &mut CellData<A> {
        &mut self.cell_data
    }

    fn sample_no_trans_(&mut self) -> A where A: Clone + 'static {
        if !self.is_assigned {
            panic!("CellLoop sampled before it was looped");
        }
        self.cell_data.sample_no_trans_()
    }
}

impl<A: Clone + 'static> CellLoop<A> {
    pub fn new(sodium_ctx: &mut SodiumCtx) -> CellLoop<A> {
        let str = StreamLoop::new(sodium_ctx);
        let r = CellLoop {
            data: Rc::new(RefCell::new(
                CellLoopData {
                    cell_data: CellData {
                        str: str.to_stream_ref().clone(),
                        value: None,
                        value_update: None,
                        cleanup: None,
                        lazy_init_value: None
                    },
                    str: str,
                    is_assigned: false
                }
            ))
        };
        let self_ = r.clone();
        Transaction::run_trans(
            sodium_ctx,
            move |sodium_ctx: &mut SodiumCtx, trans1: &mut Transaction| {
                let sodium_ctx = sodium_ctx.clone();
                let self__ = self_.clone();
                self_.with_cell_data_mut(move |data: &mut CellData<A>| {
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

    pub fn loop_<CA>(&self, sodium_ctx: &mut SodiumCtx, a_out: &CA) where CA: IsCell<A> {
        Transaction::apply(
            sodium_ctx,
            |sodium_ctx, trans| {
                let mut data = (*self.data).borrow_mut();
                data.str.loop_(sodium_ctx, a_out.updates_(trans));
                data.cell_data.lazy_init_value = Some(a_out.sample_lazy_(sodium_ctx, trans));
            }
        )
    }
}
