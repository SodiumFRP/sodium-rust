use topological_sort::TopologicalSort;
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::marker::PhantomData;

pub struct FrpContext<ENV> {
    free_cell_id: u32,
    cell_map: HashMap<u32,CellImpl<ENV,Box<Any>>>,
    cells_to_be_updated: HashSet<u32>,
    change_notifiers: Vec<Box<Fn(&mut ENV)>>,
    transaction_depth: u32
}

pub trait WithFrpContext<ENV> {
    fn with_frp_context<F>(&self, &mut ENV, k: F)
    where F: FnOnce(&mut FrpContext<ENV>);
}

impl<ENV: 'static> FrpContext<ENV> {

    pub fn new() -> FrpContext<ENV> {
        FrpContext {
            free_cell_id: 0,
            cell_map: HashMap::new(),
            cells_to_be_updated: HashSet::new(),
            change_notifiers: Vec::new(),
            transaction_depth: 0
        }
    }

    pub fn new_cell_sink<A,F>(env: &mut ENV, with_frp_context: &F, value: A) -> CellSink<ENV,A>
    where
    A:'static,
    F:WithFrpContext<ENV>
    {
        let mut cell_id: u32 = 0;
        let cell_id2: *mut u32 = &mut cell_id;
        with_frp_context.with_frp_context(
            env,
            move |frp_context| {
                let cell_id = frp_context.free_cell_id;
                frp_context.free_cell_id = frp_context.free_cell_id + 1;
                unsafe {
                    *cell_id2 = cell_id;
                }
                frp_context.cell_map.insert(
                    cell_id.clone(),
                    CellImpl {
                        id: cell_id,
                        free_observer_id: 0,
                        observer_map: HashMap::new(),
                        update_fn_op: None,
                        dependent_cells: Vec::new(),
                        value: Box::new(value) as Box<Any>
                    }
                );
            }
        );
        return CellSink::of(cell_id);
    }

    pub fn map_cell<A,B,F,F2>(env: &mut ENV, with_frp_context: &F, cell: &CellTrait<ENV,A>, f: F2) -> Cell<ENV,B>
    where
    A:'static,
    B:'static,
    F:WithFrpContext<ENV>,
    F2:Fn(&A)->B + 'static
    {
        let mut new_cell_id: u32 = 0;
        let new_cell_id2: *mut u32 = &mut new_cell_id;
        let other_cell_id = cell.id().clone();
        with_frp_context.with_frp_context(
            env,
            move |frp_context| {
                let new_cell_id = frp_context.free_cell_id;
                frp_context.free_cell_id = frp_context.free_cell_id + 1;
                unsafe {
                    *new_cell_id2 = new_cell_id;
                }
                let mut value_op: Option<B> = None;
                if let Some(other_cell) = frp_context.cell_map.get_mut(&other_cell_id) {
                    other_cell.dependent_cells.push(new_cell_id);
                    match other_cell.value.as_ref().downcast_ref::<A>() {
                        Some(other_value) => {
                            value_op = Some(f(other_value));
                        },
                        None => panic!("")
                    }
                }
                let update_fn;
                {
                    let other_cell_id2 = other_cell_id.clone();
                    update_fn = move |frp_context: &FrpContext<ENV>| {
                        if let Some(other_cell) = frp_context.cell_map.get(&other_cell_id2) {
                            match other_cell.value.as_ref().downcast_ref::<A>() {
                                Some(other_value) => {
                                    return Box::new(f(other_value)) as Box<Any>;
                                },
                                None => panic!("")
                            }
                        } else {
                            panic!("");
                        }
                    };
                }
                match value_op {
                    Some(value) => {
                        frp_context.cell_map.insert(
                            new_cell_id.clone(),
                            CellImpl::<ENV,Box<Any>> {
                                id: new_cell_id,
                                free_observer_id: 0,
                                observer_map: HashMap::new(),
                                update_fn_op: Some(Box::new(update_fn)),
                                dependent_cells: Vec::new(),
                                value: Box::new(value) as Box<Any>
                            }
                        );
                    },
                    None => ()
                }
            }
        );
        return Cell::of(new_cell_id);
    }

    pub fn transaction<F,F2>(env: &mut ENV, with_frp_context: &F, k: F2)
    where
    F:WithFrpContext<ENV>, F2: FnOnce(&mut ENV, &F),
    {
        with_frp_context.with_frp_context(
            env,
            |frp_context| {
                frp_context.transaction_depth = frp_context.transaction_depth + 1;
            }
        );
        k(env, with_frp_context);
        let mut final_transaction_depth = 0;
        with_frp_context.with_frp_context(
            env,
            |frp_context| {
                frp_context.transaction_depth = frp_context.transaction_depth - 1;
                final_transaction_depth = frp_context.transaction_depth;
            }
        );
        if final_transaction_depth == 0 {
            FrpContext::propergate(env, with_frp_context);
        }
    }

    fn propergate<F>(env: &mut ENV, with_frp_context: &F)
    where F:WithFrpContext<ENV>
    {
        let mut ts = TopologicalSort::<u32>::new();
        let mut change_notifiers: Vec<Box<Fn(&mut ENV)>> = Vec::new();
        let change_notifiers2: *mut Vec<Box<Fn(&mut ENV)>> = &mut change_notifiers;
        with_frp_context.with_frp_context(
            env,
            move |frp_context| {
                frp_context.transaction_depth = frp_context.transaction_depth + 1;
                for cell_to_be_updated in &frp_context.cells_to_be_updated {
                    ts.insert(cell_to_be_updated.clone());
                    if let &Some(cell) = &frp_context.cell_map.get(cell_to_be_updated) {
                        for dependent_cell in &cell.dependent_cells {
                            ts.add_dependency(cell.id, dependent_cell.clone());
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
                unsafe { (*change_notifiers2).append(&mut frp_context.change_notifiers) };
            }
        );
        for change_notifier in change_notifiers {
            change_notifier(env);
        }
    }

    fn update_cell(&mut self, cell_id: &u32)
    {
        let value;
        if let Some(cell) = self.cell_map.get(cell_id) {
            match &cell.update_fn_op {
                &Some(ref update_fn) => {
                    value = update_fn(self);
                },
                &None => return
            }
        } else {
            return;
        }
        let mut notifiers_to_add: Vec<Box<Fn(&mut ENV)>> = Vec::new();
        if let Some(cell) = self.cell_map.get_mut(cell_id) {
            cell.value = value;
            let cell2: *const CellImpl<ENV,Box<Any>> = cell;
            notifiers_to_add.push(Box::new(
                move |env| {
                    unsafe {
                        let ref cell3: CellImpl<ENV,Box<Any>> = *cell2;
                        for observer in cell3.observer_map.values() {
                            observer(env, &cell3.value);
                        }
                    }
                }
            ));
        }
        self.change_notifiers.append(&mut notifiers_to_add);
    }

    fn mark_all_decendent_cells_for_update(&mut self, cell_id: u32, visited: &mut HashSet<u32>) {
        visited.insert(cell_id);
        let mut dependent_cells: Vec<u32> = Vec::new();
        match self.cell_map.get(&cell_id) {
            Some(cell) => {
                for dependent_cell in &cell.dependent_cells {
                    dependent_cells.push(dependent_cell.clone());
                }
            },
            None => ()
        }
        loop {
            let dependent_cell_op = dependent_cells.pop();
            match dependent_cell_op {
                Some(dependent_cell) => {
                    if !visited.contains(&dependent_cell) {
                        self.cells_to_be_updated.insert(dependent_cell);
                        self.mark_all_decendent_cells_for_update(dependent_cell, visited);
                    }
                },
                None => break
            }
        }
    }
}

pub trait CellTrait<ENV,A> {
    fn id(&self) -> u32;
}

#[derive(Copy,Clone)]
pub struct Cell<ENV,A> {
    id: u32,
    env_phantom: PhantomData<ENV>,
    value_phantom: PhantomData<A>
}

impl<ENV,A> CellTrait<ENV,A> for Cell<ENV,A> {
    fn id(&self) -> u32 {
        self.id
    }
}

impl<ENV,A:'static> Cell<ENV,A> {
    fn of(id: u32) -> Cell<ENV,A> {
        Cell {
            id: id,
            env_phantom: PhantomData,
            value_phantom: PhantomData
        }
    }

    pub fn current_value<'a,F>(&self, env: &'a mut ENV, with_frp_context: &F) -> &'a A
    where
    F:WithFrpContext<ENV>
    {
        let mut value_op: Option<*const A> = None;
        let value_op2: *mut Option<*const A> = &mut value_op;
        let cell_id = self.id.clone();
        with_frp_context.with_frp_context(
            env,
            move |frp_context| {
                match frp_context.cell_map.get(&cell_id) {
                    Some(cell) => {
                        match cell.value.as_ref().downcast_ref::<A>() {
                            Some(value) => {
                                let value2: *const A = value;
                                unsafe { (*value_op2) = Some(value2); }
                            },
                            None => ()
                        }
                    },
                    None => ()
                }
            }
        );
        match value_op {
            Some(value) => {
                unsafe { &*value }
            },
            None => panic!("")
        }
    }

    pub fn observe<F,F2>(&self, env: &mut ENV, with_frp_context: &F, observer: F2) -> Box<FnOnce(&mut ENV, &F)>
    where
    F:WithFrpContext<ENV>,
    F2:Fn(&mut ENV,&A) + 'static
    {
        {
            let env2: *mut ENV = env;
            let value = self.current_value(unsafe { &mut *env2 }, with_frp_context);
            let value2: *const A = value;
            observer(unsafe { &mut *env2 }, unsafe { &*value2 });
        }
        let mut observer_id_op: Option<u32> = None;
        let observer_id_op2: *mut Option<u32> = &mut observer_id_op;
        let cell_id = self.id.clone();
        with_frp_context.with_frp_context(
            env,
            move |frp_context| {
                if let Some(cell) = frp_context.cell_map.get_mut(&cell_id) {
                    let observer_id = cell.free_observer_id;
                    unsafe { *observer_id_op2 = Some(observer_id); }
                    cell.free_observer_id = cell.free_observer_id + 1;
                    cell.observer_map.insert(observer_id, Box::new(
                        move |env, value| {
                            match value.as_ref().downcast_ref::<A>() {
                                Some(value) => observer(env, value),
                                None => ()
                            }
                        }
                    ));
                }
            }
        );
        let cell_id = self.id.clone();
        match observer_id_op {
            Some(observer_id) => {
                return Box::new(move |env, with_frp_context| {
                    with_frp_context.with_frp_context(
                        env,
                        move |frp_context| {
                            if let Some(cell) = frp_context.cell_map.get_mut(&cell_id) {
                                cell.observer_map.remove(&observer_id);
                            }
                        }
                    );
                });
            },
            None => Box::new(|_, _| {})
        }
    }
}

#[derive(Copy,Clone)]
pub struct CellSink<ENV,A> {
    id: u32,
    env_phantom: PhantomData<ENV>,
    value_phantom: PhantomData<A>
}

impl<ENV,A> CellTrait<ENV,A> for CellSink<ENV,A> {
    fn id(&self) -> u32 {
        self.id
    }
}

impl<ENV:'static,A:'static> CellSink<ENV,A> {
    fn of(id: u32) -> CellSink<ENV,A> {
        CellSink {
            id: id,
            env_phantom: PhantomData,
            value_phantom: PhantomData
        }
    }

    pub fn change_value<F>(&self, env: &mut ENV, with_frp_context: &F, value: A)
    where F:WithFrpContext<ENV> {
        let cell_id = self.id.clone();
        FrpContext::transaction(
            env,
            with_frp_context,
            move |env, with_frp_context| {
                with_frp_context.with_frp_context(
                    env,
                    move |frp_context| {
                        if let Some(cell) = frp_context.cell_map.get_mut(&cell_id) {
                            cell.value = Box::new(value) as Box<Any>;
                        }
                        frp_context.mark_all_decendent_cells_for_update(cell_id, &mut HashSet::new());
                    }
                );
            }
        );
    }
}

struct CellImpl<ENV,A> {
    id: u32,
    free_observer_id: u32,
    observer_map: HashMap<u32,Box<Fn(&mut ENV,&A)>>,
    update_fn_op: Option<Box<Fn(&FrpContext<ENV>)->A>>,
    dependent_cells: Vec<u32>,
    value: A
}
