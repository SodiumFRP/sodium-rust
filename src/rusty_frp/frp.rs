use topological_sort::TopologicalSort;
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;

pub struct FrpContext<ENV> {
    free_id: u32,
    cell_map: HashMap<u32,CellImpl<ENV,Box<Any>>>,
    cells_to_be_updated: HashSet<u32>,
    change_notifiers: Vec<Box<Fn(&mut ENV)>>,
    transaction_depth: u32
}

impl<ENV> FrpContext<ENV> {
    pub fn new() -> FrpContext<ENV> {
        FrpContext {
            free_id: 0,
            cell_map: HashMap::new(),
            cells_to_be_updated: HashSet::new(),
            change_notifiers: Vec::new(),
            transaction_depth: 0
        }
    }

    pub fn transaction<F,F2>(env: &mut ENV, with_frp_context: &F, k: &F2)
    where F:Fn(&mut ENV, &FnMut(&mut FrpContext<ENV>)), F2: Fn(&mut ENV, &F)
    {
        with_frp_context(
            env,
            &|frp_context| {
                frp_context.transaction_depth = frp_context.transaction_depth + 1;
            }
        );
        k(env, with_frp_context);
        let mut final_transaction_depth = 0;
        with_frp_context(
            env,
            &mut |frp_context| {
                frp_context.transaction_depth = frp_context.transaction_depth - 1;
                final_transaction_depth = frp_context.transaction_depth;
            }
        );
        if final_transaction_depth == 0 {
            FrpContext::propergate(env, with_frp_context);
        }
    }

    fn propergate<F>(env: &mut ENV, with_frp_context: &F)
    where F:Fn(&mut ENV, &FnMut(&mut FrpContext<ENV>))
    {
        let mut ts = TopologicalSort::<u32>::new();
        with_frp_context(
            env,
            &|frp_context| {
                frp_context.transaction_depth = frp_context.transaction_depth + 1;
                for cell_to_be_updated in &frp_context.cells_to_be_updated {
                    if let &Some(cell) = &frp_context.cell_map.get(cell_to_be_updated) {
                        for dependent_cell in &cell.dependent_cells {
                            if frp_context.cells_to_be_updated.contains(dependent_cell) {
                                ts.add_dependency(cell.id, dependent_cell.clone());
                            }
                        }
                    }
                }
                loop {
                    let next_op = ts.pop();
                    match next_op {
                        Some(cell_id) => {
                            frp_context.update_cell(&cell_id);
                        },
                        None => break
                    }
                }
                frp_context.transaction_depth = frp_context.transaction_depth - 1;
            }
            /* TODO: Make this commented out code work
            loop {
                let change_notifier_op = frp_context.change_notifiers.pop();
                match change_notifier_op {
                    Some(change_notifier) => {
                        change_notifier(env);
                    },
                    None => break
                }
            }*/
        );
    }

    fn update_cell(&mut self, cell_id: &u32) {
        let value;
        if let Some(cell) = self.cell_map.get(cell_id) {
            let update_fn = &cell.update_fn;
            value = update_fn(self);
        } else {
            return;
        }
        if let Some(cell) = self.cell_map.get_mut(cell_id) {
            cell.value = Box::new(value);
        }
    }
}

pub trait Cell<ENV,A> {
    fn current_value<'a>(&'a self) -> &'a A;
}

pub trait CellSink<ENV,A>: Cell<ENV,A> {
    fn change_value(&mut self, value: A);
}

struct CellImpl<ENV,A> {
    id: u32,
    value: A,
    observers: Vec<Box<Fn(&mut ENV,&A)>>,
    update_fn: Box<Fn(&FrpContext<ENV>)->A>,
    dependent_cells: Vec<u32>
}

impl<ENV,A> Cell<ENV,A> for CellImpl<ENV,A> {
    fn current_value<'a>(&'a self) -> &'a A {
        &self.value
    }
}

impl<ENV,A> CellSink<ENV,A> for CellImpl<ENV,A> {
    fn change_value(&mut self, value: A) {
        self.value = value;
    }
}
