use topological_sort::TopologicalSort;
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::marker::PhantomData;

pub struct FrpContext<ENV> {
    free_cell_id: u32,
    cell_map: HashMap<u32,CellImpl<ENV,Any>>,
    cell_loop_map: HashMap<u32,u32>,
    cells_to_be_updated: HashSet<u32>,
    change_notifiers: Vec<Box<Fn(&mut ENV)>>,

    // when executing inside cell_switch, this will hold that cell's id
    inside_cell_switch_id_op: Option<u32>,

    transaction_depth: u32
}

pub trait WithFrpContext<ENV> {
    fn with_frp_context<'r>(&self, &'r mut ENV) -> &'r mut FrpContext<ENV>;
}

impl<ENV: 'static> FrpContext<ENV> {

    pub fn new() -> FrpContext<ENV> {
        FrpContext {
            free_cell_id: 0,
            cell_map: HashMap::new(),
            cell_loop_map: HashMap::new(),
            cells_to_be_updated: HashSet::new(),
            change_notifiers: Vec::new(),
            inside_cell_switch_id_op: None,
            transaction_depth: 0
        }
    }

    pub fn never<A: 'static>(&mut self) -> Stream<ENV,A> {
        let ss: StreamSink<ENV,A> = self.new_stream_sink();
        Stream::of(ss.id)
    }

    pub fn constant<A: 'static>(&mut self, a: A) -> Cell<ENV,A> {
        let cs: CellSink<ENV,A> = self.new_cell_sink(a);
        Cell::of(cs.id)
    }

    pub fn hold<A,SA>(&mut self, a: A, sa: &SA) -> Cell<ENV,A>
    where
    A:'static + Clone,
    SA:StreamTrait<ENV,A>
    {
        let sa2 = Stream::of(sa.id());
        self.loop_c(
            a,
            move |frp_context, ca| {
                frp_context.lift2_c(
                    |old_a: &A, new_a_op: &Option<A>| {
                        match new_a_op {
                            &Some(ref new_a) => new_a.clone(),
                            &None => old_a.clone()
                        }
                    },
                    ca,
                    &sa2.as_cell()
                )
            }
        )
    }

    pub fn loop_c<A,F>(&mut self, time0_value: A, k:F) -> Cell<ENV,A>
    where
    A:'static,
    F:Fn(&mut FrpContext<ENV>,&Cell<ENV,A>)->Cell<ENV,A>
    {
        let cell = self.new_cell_sink(time0_value);
        let cell2 = k(self,&Cell::of(cell.id));
        self.cell_loop_map.insert(cell.id, cell2.id);
        return Cell::of(cell2.id);
    }

    pub fn switch_s<F,A,CSA>(&mut self, cell_thunk_stream_a: &CSA) -> Stream<ENV,A>
    where
    A:'static,
    F:Fn(&mut FrpContext<ENV>)->Stream<ENV,A> + 'static + Clone,
    CSA:CellTrait<ENV,Box<F>>
    {
        let cell_thunk_cell_a: Cell<ENV,Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,Option<A>> + 'static>> = self.map_c(
            cell_thunk_stream_a,
            |k| {
                let k3 = k.clone();
                let k2: Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,Option<A>> + 'static> = Box::new(move |frp_context| {
                    k3(frp_context).as_cell()
                });
                k2
            }
        );
        self.switch_c(&cell_thunk_cell_a).as_stream()
    }

    pub fn switch_c<A,CCA>(&mut self, cell_thunk_cell_a: &CCA) -> Cell<ENV,A>
    where
    A:'static,
    CCA:CellTrait<ENV,Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,A>>>
    {
        let initial_value_thunk = cell_current_value_via_context(cell_thunk_cell_a, self);
        let outer_cell_id = cell_thunk_cell_a.id().clone();
        let new_cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        {
            let tmp: Cell<ENV,A> = Cell::of(0);
            let x: CellImpl<ENV,A> = CellImpl {
                id: new_cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: None,
                dependent_cells: Vec::new(),
                depends_on_cells: vec![outer_cell_id.clone()],
                reset_value_after_propergate_op: None,
                child_cells: Vec::new(),
                value: Value::AnotherCell(tmp)
            };
            self.insert_cell(x);
        }
        self.inside_cell_switch_id_op = Some(new_cell_id.clone());
        let initial_inner_cell = initial_value_thunk(self);
        let initial_inner_cell_id = initial_inner_cell.id().clone();
        self.inside_cell_switch_id_op = None;
        if let Some(cell) = self.cell_map.get_mut(&new_cell_id) {
            cell.dependent_cells.push(initial_inner_cell_id.clone());
            cell.value = Value::AnotherCell(Cell::of(initial_inner_cell_id.clone()));
        }
        if let Some(cell) = self.cell_map.get_mut(&outer_cell_id) {
            cell.dependent_cells.push(new_cell_id);
        }
        if let Some(cell) = self.cell_map.get_mut(&initial_inner_cell_id) {
            cell.dependent_cells.push(new_cell_id);
        }
        let update_fn: Box<FnMut(&mut ENV, &WithFrpContext<ENV>, &mut Any) + 'static>;
        {
            let new_cell_id = new_cell_id.clone();
            let cell_thunk_cell_a: Cell<ENV,Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,A>>> = Cell::of(cell_thunk_cell_a.id());
            update_fn = Box::new(
                move |env, with_frp_context, value| {
                    let frp_context = with_frp_context.with_frp_context(env);
                    let mut child_cells: Vec<u32> = Vec::new();
                    let mut disconnect_from_inner_id_op = None;
                    if let Some(cell) = frp_context.cell_map.get_mut(&new_cell_id) {
                        child_cells.append(&mut cell.child_cells);
                        match &cell.value {
                            &Value::Direct(_) => {},
                            &Value::AnotherCell(ref c) => {
                                disconnect_from_inner_id_op = Some(c.id.clone());
                            }
                        }
                    }
                    match disconnect_from_inner_id_op {
                        Some(disconnect_from_inner_id) => {
                            if let Some(cell) = frp_context.cell_map.get_mut(&disconnect_from_inner_id) {
                                cell.dependent_cells.retain(|id| id.clone() != new_cell_id.clone());
                            }
                        },
                        None => ()
                    }
                    for child_cell in child_cells {
                        frp_context.free_cell(&child_cell);
                    }
                    let value_thunk = cell_current_value_via_context(&cell_thunk_cell_a, frp_context);
                    frp_context.inside_cell_switch_id_op = Some(new_cell_id.clone());
                    let inner_cell = value_thunk(frp_context);
                    let inner_cell_id = inner_cell.id().clone();
                    frp_context.inside_cell_switch_id_op = None;
                    if let Some(cell) = frp_context.cell_map.get_mut(&inner_cell_id) {
                        cell.dependent_cells.push(new_cell_id);
                    }
                    if let Some(cell) = frp_context.cell_map.get_mut(&new_cell_id) {
                        cell.value = Value::AnotherCell(Cell::of(inner_cell_id));
                    }
                }
            );
        }
        if let Some(cell) = self.cell_map.get_mut(&new_cell_id) {
            cell.update_fn_op = Some(update_fn);
        }
        Cell::of(new_cell_id)
    }

    pub fn value<A,CA>(&mut self, ca: &CA) -> Stream<ENV,A>
    where
    A:'static + Clone,
    CA:CellTrait<ENV,A>
    {
        let sa = self.map_c(
            ca,
            |a| { Some(a.clone()) }
        ).as_stream();
        if let Some(cell) = self.cell_map.get_mut(&sa.id) {
            cell.reset_value_after_propergate_op = Some(Box::new(
                |a| {
                    match a.downcast_mut::<Option<A>>() {
                        Some(a2) => {
                            *a2 = None;
                        },
                        None => ()
                    }
                }
            ));
        }
        return sa;
    }

    pub fn updates<A,CA>(&mut self, ca: &CA) -> Stream<ENV,A>
    where
    A:'static + Clone,
    CA:CellTrait<ENV,A>
    {
        let sa = self.value(ca);
        if let Some(cell) = self.cell_map.get_mut(&sa.id) {
            let none: Option<A> = None;
            cell.value = Value::Direct(Box::new(none) as Box<Any>);
        }
        return sa;
    }

    pub fn new_stream_sink<A>(&mut self) -> StreamSink<ENV,A>
    where
    A:'static
    {
        let cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        let initial_value: Option<A> = None;
        self.insert_cell(
            CellImpl {
                id: cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: None,
                dependent_cells: Vec::new(),
                depends_on_cells: Vec::new(),
                reset_value_after_propergate_op: Some(Box::new(
                    |a| {
                        *a = None;
                    }
                )),
                child_cells: Vec::new(),
                value: Value::Direct(Box::new(initial_value) as Box<Option<A>>)
            }
        );
        StreamSink::of(cell_id)
    }

    pub fn new_cell_sink<A>(&mut self, value: A) -> CellSink<ENV,A>
    where
    A:'static
    {
        let cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        self.insert_cell(
            CellImpl {
                id: cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: None,
                dependent_cells: Vec::new(),
                depends_on_cells: Vec::new(),
                reset_value_after_propergate_op: None,
                child_cells: Vec::new(),
                value: Value::Direct(Box::new(value))
            }
        );
        CellSink::of(cell_id)
    }

    pub fn map_s<A,B,SA,F>(&mut self, sa: &SA, f: F) -> Stream<ENV,B>
    where
    A:'static,
    B:Any + 'static,
    SA:StreamTrait<ENV,A>,
    F:Fn(&A)->B + 'static
    {
        let f2 = Box::new(f);
        let c = self.map_c(
            &sa.as_cell(),
            move |a| {
                match a {
                    &Some(ref a2) => Some(f2(&a2)),
                    &None => None
                }
            }
        );
        let cell_id = c.id.clone();
        if let Some(cell) = self.cell_map.get_mut(&cell_id) {
            cell.reset_value_after_propergate_op = Some(Box::new(|a| {
                match a.downcast_mut::<Option<A>>() {
                    Some(a2) => {
                        *a2 = None;
                    },
                    None => ()
                }
            }));
        }
        Stream::of(cell_id)
    }

    pub fn map_c<A,B,CA,F>(&mut self, cell: &CA, f: F) -> Cell<ENV,B>
    where
    A:'static,
    B:Any + 'static,
    CA:CellTrait<ENV,A>,
    F:Fn(&A)->B + 'static
    {
        let initial_value = f(cell_current_value_via_context(cell, self));
        let cell = Cell::of(cell.id().clone());
        let new_cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        if let Some(cell_impl) = self.cell_map.get_mut(&cell.id) {
            cell_impl.dependent_cells.push(new_cell_id);
        }
        let update_fn = move |env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, result: &mut B| {
            let frp_context = with_frp_context.with_frp_context(env);
            *result = f(cell_current_value_via_context(&cell, frp_context));
        };
        self.insert_cell(
            CellImpl {
                id: new_cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: Some(Box::new(update_fn)),
                dependent_cells: Vec::new(),
                depends_on_cells: vec!(cell.id.clone()),
                reset_value_after_propergate_op: None,
                child_cells: Vec::new(),
                value: Value::Direct(Box::new(initial_value))
            }
        );
        return Cell::of(new_cell_id);
    }

    pub fn snapshot<A,B,C,SA,CB,F>(&mut self, f: F, sa: &SA, cb: &CB) -> Stream<ENV,C>
    where
    A: 'static,
    B: 'static,
    C: 'static,
    F: Fn(&A,&B)->C + 'static,
    SA: StreamTrait<ENV,A>,
    CB: CellTrait<ENV,B>
    {
        let c: Cell<ENV,Option<C>> = self.lift2_c(
            move |a_op, b| {
                match a_op {
                    &Some(ref a) => Some(f(a,b)),
                    &None => None
                }
            },
            &sa.as_cell(),
            cb
        );
        let s: Stream<ENV,C> = Stream::of(c.id());
        s
    }

    pub fn or_else<A,SA>(&mut self, sa1: &SA, sa2: &SA) -> Stream<ENV,A>
    where
    A: 'static + Clone, // <-- Clone is unfortunate here (But can be avoided using Value::AnotherCell trick)
    SA: StreamTrait<ENV,A>
    {
        self.merge(sa1, sa2, |a1, a2| a1.clone())
    }

    pub fn merge<A,SA,F>(&mut self, sa1: &SA, sa2: &SA, f: F) -> Stream<ENV,A>
    where
    A: 'static + Clone, // <-- Clone is unfortunate here (But can be avoided using Value::AnotherCell trick)
    SA: StreamTrait<ENV,A>,
    F: Fn(&A,&A)->A + 'static
    {
        let c: Cell<ENV,Option<A>> = self.lift2_c(
            move |a1_op, a2_op| {
                match a1_op {
                    &Some(ref a1) => {
                        match a2_op {
                            &Some(ref a2) => Some(f(a1,a2)),
                            &None => Some(a1.clone())
                        }
                    },
                    &None => {
                        match a2_op {
                            &Some(ref a2) => Some(a2.clone()),
                            &None => None
                        }
                    }
                }
            },
            &sa1.as_cell(),
            &sa2.as_cell()
        );
        let s: Stream<ENV,A> = Stream::of(c.id());
        s
    }

    pub fn gate<A,SA,C>(&mut self, sa: &SA, c_pred: &C) -> Stream<ENV,A>
    where
    A: 'static + Clone,
    SA: StreamTrait<ENV,A>,
    C: CellTrait<ENV,bool>
    {
        let s1 = self.snapshot(
            |a, pred| (a.clone(), pred.clone()),
            sa,
            c_pred
        );
        let s2 = self.filter(
            |&(ref a, ref pred)| pred.clone(),
            &s1
        );
        self.map_s(
            &s2,
            |&(ref a, ref pred)| a.clone()
        )
    }

    pub fn filter_some<A,SA_OP>(&mut self, sa_op: &SA_OP) -> Stream<ENV,A>
    where
    A: 'static + Clone,
    SA_OP: StreamTrait<ENV,Option<A>>,
    {
        let s = self.filter(
            |a_op| {
                match a_op {
                    &Some(_) => true,
                    &None => false
                }
            },
            sa_op
        );
        self.map_s(
            &s,
            |a_op| {
                match a_op {
                    &Some(ref a) => a.clone(),
                    &None => panic!("")
                }
            }
        )
    }

    pub fn filter<A,SA,F>(&mut self, f: F, sa: &SA) -> Stream<ENV,A>
    where
    A: 'static + Clone,
    SA: StreamTrait<ENV,A>,
    F: Fn(&A)->bool + 'static
    {
        self.map_c(
            &sa.as_cell(),
            move |a_op| {
                match a_op {
                    &Some(ref a) => {
                        if f(a) {
                            Some(a.clone())
                        } else {
                            None
                        }
                    },
                    &None => None
                }
            }
        ).as_stream()
    }

    pub fn apply<A,B,CAB,CA>(&mut self, cab: &CAB, ca: &CA) -> Cell<ENV,B>
    where
    A:'static,
    B:'static,
    CAB:CellTrait<ENV,Box<Fn(&A)->B + 'static>>,
    CA:CellTrait<ENV,A>
    {
        self.lift2_c(
            |ab, a| { ab(a) },
            cab,
            ca
        )
    }

    pub fn lift2_c<A,B,C,CA,CB,F>(&mut self, f: F, cell_a: &CA, cell_b: &CB) -> Cell<ENV,C>
    where
    A:'static,
    B:'static,
    C:'static,
    CA: CellTrait<ENV,A>,
    CB: CellTrait<ENV,B>,
    F:Fn(&A,&B)->C + 'static
    {
        let cell_a = Cell::of(cell_a.id().clone());
        let cell_b = Cell::of(cell_b.id().clone());
        let initial_value;
        {
            let value_a = cell_current_value_via_context(&cell_a, self);
            let value_b = cell_current_value_via_context(&cell_b, self);
            initial_value =
                f(
                    value_a, value_b
                );
        }
        let new_cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        if let Some(cell_a_impl) = self.cell_map.get_mut(&cell_a.id) {
            cell_a_impl.dependent_cells.push(new_cell_id);
        }
        if let Some(cell_b_impl) = self.cell_map.get_mut(&cell_b.id) {
            cell_b_impl.dependent_cells.push(new_cell_id);
        }
        let update_fn = move |env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, result: &mut C| {
            let frp_context = with_frp_context.with_frp_context(env);
            *result = f(
                cell_current_value_via_context(&cell_a, frp_context),
                cell_current_value_via_context(&cell_b, frp_context)
            );
        };
        self.insert_cell(
            CellImpl {
                id: new_cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: Some(Box::new(update_fn)),
                dependent_cells: Vec::new(),
                depends_on_cells: vec!(cell_a.id.clone(), cell_b.id.clone()),
                reset_value_after_propergate_op: None,
                child_cells: Vec::new(),
                value: Value::Direct(Box::new(initial_value))
            }
        );
        return Cell::of(new_cell_id);
    }

    pub fn lift3_c<A,B,C,D,CA,CB,CC,F>(&mut self, f: F, cell_a: &CA, cell_b: &CB, cell_c: &CC) -> Cell<ENV,D>
    where
    A:'static,
    B:'static,
    C:'static,
    D:'static,
    CA:CellTrait<ENV,A>,
    CB:CellTrait<ENV,B>,
    CC:CellTrait<ENV,C>,
    F:Fn(&A,&B,&C)->D + 'static
    {
        let cell_a = Cell::of(cell_a.id().clone());
        let cell_b = Cell::of(cell_b.id().clone());
        let cell_c = Cell::of(cell_c.id().clone());
        let initial_value;
        {
            let value_a = cell_current_value_via_context(&cell_a, self);
            let value_b = cell_current_value_via_context(&cell_b, self);
            let value_c = cell_current_value_via_context(&cell_c, self);
            initial_value =
                f(
                    value_a, value_b, value_c
                );
        }
        let new_cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        if let Some(cell_a_impl) = self.cell_map.get_mut(&cell_a.id) {
            cell_a_impl.dependent_cells.push(new_cell_id);
        }
        if let Some(cell_b_impl) = self.cell_map.get_mut(&cell_b.id) {
            cell_b_impl.dependent_cells.push(new_cell_id);
        }
        if let Some(cell_c_impl) = self.cell_map.get_mut(&cell_c.id) {
            cell_c_impl.dependent_cells.push(new_cell_id);
        }
        let update_fn = move |env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, result: &mut D| {
            let frp_context = with_frp_context.with_frp_context(env);
            *result = f(
                cell_current_value_via_context(&cell_a, frp_context),
                cell_current_value_via_context(&cell_b, frp_context),
                cell_current_value_via_context(&cell_c, frp_context)
            )
        };
        self.insert_cell(
            CellImpl {
                id: new_cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: Some(Box::new(update_fn)),
                dependent_cells: Vec::new(),
                depends_on_cells: vec!(cell_a.id.clone(), cell_b.id.clone(), cell_c.id.clone()),
                reset_value_after_propergate_op: None,
                child_cells: Vec::new(),
                value: Value::Direct(Box::new(initial_value))
            }
        );
        return Cell::of(new_cell_id);
    }

    pub fn lift4_c<A,B,C,D,E,CA,CB,CC,CD,F>(&mut self, f: F, cell_a: &CA, cell_b: &CB, cell_c: &CC, cell_d: &CD) -> Cell<ENV,E>
    where
    A:'static,
    B:'static,
    C:'static,
    D:'static,
    E:'static,
    CA: CellTrait<ENV,A>,
    CB: CellTrait<ENV,B>,
    CC: CellTrait<ENV,C>,
    CD: CellTrait<ENV,D>,
    F:Fn(&A,&B,&C,&D)->E + 'static
    {
        let cell_a = Cell::of(cell_a.id().clone());
        let cell_b = Cell::of(cell_b.id().clone());
        let cell_c = Cell::of(cell_c.id().clone());
        let cell_d = Cell::of(cell_d.id().clone());
        let initial_value;
        {
            let value_a = cell_current_value_via_context(&cell_a, self);
            let value_b = cell_current_value_via_context(&cell_b, self);
            let value_c = cell_current_value_via_context(&cell_c, self);
            let value_d = cell_current_value_via_context(&cell_d, self);
            initial_value =
                f(
                    value_a, value_b, value_c, value_d
                );
        }
        let new_cell_id = self.free_cell_id;
        self.free_cell_id = self.free_cell_id + 1;
        if let Some(cell_a_impl) = self.cell_map.get_mut(&cell_a.id) {
            cell_a_impl.dependent_cells.push(new_cell_id);
        }
        if let Some(cell_b_impl) = self.cell_map.get_mut(&cell_b.id) {
            cell_b_impl.dependent_cells.push(new_cell_id);
        }
        if let Some(cell_c_impl) = self.cell_map.get_mut(&cell_c.id) {
            cell_c_impl.dependent_cells.push(new_cell_id);
        }
        if let Some(cell_d_impl) = self.cell_map.get_mut(&cell_d.id) {
            cell_d_impl.dependent_cells.push(new_cell_id);
        }
        let update_fn = move |env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, result: &mut E| {
            let frp_context = with_frp_context.with_frp_context(env);
            *result = f(
                cell_current_value_via_context(&cell_a, frp_context),
                cell_current_value_via_context(&cell_b, frp_context),
                cell_current_value_via_context(&cell_c, frp_context),
                cell_current_value_via_context(&cell_d, frp_context)
            );
        };
        self.insert_cell(
            CellImpl {
                id: new_cell_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                update_fn_op: Some(Box::new(update_fn)),
                dependent_cells: Vec::new(),
                depends_on_cells: vec!(cell_a.id.clone(), cell_b.id.clone(), cell_c.id.clone(), cell_d.id.clone()),
                reset_value_after_propergate_op: None,
                child_cells: Vec::new(),
                value: Value::Direct(Box::new(initial_value))
            }
        );
        return Cell::of(new_cell_id);
    }

    pub fn transaction<F>(env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, k: F)
    where
    F: FnOnce(&mut ENV, &WithFrpContext<ENV>),
    {
        {
            let frp_context = with_frp_context.with_frp_context(env);
            frp_context.transaction_depth = frp_context.transaction_depth + 1;
        }
        k(env, with_frp_context);
        let final_transaction_depth;
        {
            let frp_context = with_frp_context.with_frp_context(env);
            frp_context.transaction_depth = frp_context.transaction_depth - 1;
            final_transaction_depth = frp_context.transaction_depth;
        }
        if final_transaction_depth == 0 {
            FrpContext::propergate(env, with_frp_context);
        }
    }

    fn propergate(env: &mut ENV, with_frp_context: &WithFrpContext<ENV>)
    {
        let mut ts = TopologicalSort::<u32>::new();
        let mut change_notifiers: Vec<Box<Fn(&mut ENV)>> = Vec::new();
        let change_notifiers2: *mut Vec<Box<Fn(&mut ENV)>> = &mut change_notifiers;

        {
            let frp_context = with_frp_context.with_frp_context(env);
            frp_context.transaction_depth = frp_context.transaction_depth + 1;
            for cell_to_be_updated in &frp_context.cells_to_be_updated {
                ts.insert(cell_to_be_updated.clone());
                if let &Some(cell) = &frp_context.cell_map.get(cell_to_be_updated) {
                    for dependent_cell in &cell.dependent_cells {
                        ts.add_dependency(cell.id, dependent_cell.clone());
                    }
                }
            }
        }
        loop {
            let next_op = ts.pop();
            match next_op {
                Some(cell_id) => {
                    FrpContext::update_cell(env, with_frp_context, &cell_id);
                },
                None => break
            }
        }
        {
            let frp_context = with_frp_context.with_frp_context(env);
            frp_context.transaction_depth = frp_context.transaction_depth - 1;
            unsafe { (*change_notifiers2).append(&mut frp_context.change_notifiers) };
        }
        for change_notifier in change_notifiers {
            change_notifier(env);
        }
        {
            let frp_context = with_frp_context.with_frp_context(env);
            let cells_to_be_updated = frp_context.cells_to_be_updated.clone();
            for cell_to_be_updated in cells_to_be_updated {
                if let Some(cell) = frp_context.cell_map.get_mut(&cell_to_be_updated) {
                    match &cell.reset_value_after_propergate_op {
                        &Some(ref reset_value_after_propergate) => {
                            match &mut cell.value {
                                &mut Value::Direct(ref mut v) => {
                                    let v2 = v.as_mut();
                                    let v3: *mut Any = v2;
                                    reset_value_after_propergate(unsafe { &mut *v3 });
                                }
                                &mut Value::AnotherCell(_) => ()
                            }
                        },
                        &None => ()
                    }
                }
            }
            frp_context.cells_to_be_updated.clear();
        }
    }

    fn insert_cell<A: Any + 'static>(&mut self, cell: CellImpl<ENV,A>) {
        let cell_id = cell.id.clone();
        let cell2 = cell.into_any();
        self.cell_map.insert(cell_id, cell2);
        let inside_cell_switch_id_op = self.inside_cell_switch_id_op.clone();
        if let Some(inside_cell_switch_id) = inside_cell_switch_id_op {
            if let Some(inside_cell_switch) = self.cell_map.get_mut(&inside_cell_switch_id) {
                inside_cell_switch.child_cells.push(cell_id);
            }
        }
    }

    fn free_cell(&mut self, cell_id: &u32) {
        let mut depends_on_cells: Vec<u32> = Vec::new();
        let mut child_cells: Vec<u32> = Vec::new();
        if let Some(cell) = self.cell_map.get_mut(cell_id) {
            for depends_on_cell in &cell.depends_on_cells {
                depends_on_cells.push(depends_on_cell.clone());
            }
            child_cells.append(&mut cell.child_cells);
        }
        for depends_on_cell in depends_on_cells {
            if let Some(cell) = self.cell_map.get_mut(cell_id) {
                cell.dependent_cells.retain(|id| { id != cell_id });
            }
        }
        for child_cell in child_cells.drain(..) {
            self.cell_map.remove(&child_cell);
        }
        self.cell_map.remove(cell_id);
    }

    fn update_cell(env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, cell_id: &u32)
    {
        let mut notifiers_to_add: Vec<Box<Fn(&mut ENV)>> = Vec::new();
        let mut update_fn_op: Option<*mut FnMut(&mut ENV, &WithFrpContext<ENV>, &mut Any)> = None;
        {
            let frp_context = with_frp_context.with_frp_context(env);
            if let Some(cell) = frp_context.cell_map.get_mut(cell_id) {
                match &mut cell.update_fn_op {
                    &mut Some(ref mut update_fn) => {
                        update_fn_op = Some(update_fn.as_mut());
                    },
                    &mut None => ()
                }
            }
        }
        match update_fn_op {
            Some(x) => {
                let update_fn: &mut FnMut(&mut ENV, &WithFrpContext<ENV>, &mut Any);
                update_fn = unsafe { &mut *x };
                let value: *mut Any;
                {
                    let frp_context = with_frp_context.with_frp_context(env);
                    if let Some(cell) = frp_context.cell_map.get_mut(cell_id) {
                        match &mut cell.value {
                            &mut Value::Direct(ref mut x) => {
                                value = x.as_mut();
                            },
                            &mut Value::AnotherCell(_) => return
                        }
                    } else {
                        return;
                    }
                }
                update_fn(env, with_frp_context, unsafe { &mut *value });
            },
            None => ()
        }
        let frp_context = with_frp_context.with_frp_context(env);
        if let Some(cell) = frp_context.cell_map.get_mut(cell_id) {
            let cell2: *const CellImpl<ENV,Any> = cell;
            notifiers_to_add.push(Box::new(
                move |env| {
                    unsafe {
                        let ref cell3: CellImpl<ENV,Any> = *cell2;
                        for observer in cell3.observer_map.values() {
                            match &cell3.value {
                                &Value::Direct(ref x) => {
                                    observer(env, x.as_ref());
                                },
                                &Value::AnotherCell(_) => ()
                            }
                        }
                    }
                }
            ));
        }
        frp_context.change_notifiers.append(&mut notifiers_to_add);
    }

    fn mark_all_decendent_cells_for_update(&mut self, cell_id: u32, visited: &mut HashSet<u32>) {
        self.cells_to_be_updated.insert(cell_id);
        visited.insert(cell_id);
        let mut dependent_cells: Vec<u32> = Vec::new();
        dependent_cells.push(cell_id);
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
                        self.mark_all_decendent_cells_for_update(dependent_cell, visited);
                    }
                },
                None => break
            }
        }
    }
}

pub trait StreamTrait<ENV:'static,A:'static>: Sized {
    fn id(&self) -> u32;

    fn as_cell(&self) -> Cell<ENV,Option<A>> {
        Cell::of(self.id())
    }

    fn observe<F,F2>(&self, env: &mut ENV, with_frp_context: &F, observer: F2) -> Box<FnOnce(&mut ENV, &F)>
    where
    F:WithFrpContext<ENV>,
    F2:Fn(&mut ENV,&A) + 'static
    {
        let observer2 = Box::new(observer);
        let c: Cell<ENV,Option<A>> = Cell::of(self.id());
        c.observe(
            env,
            with_frp_context,
            move |env, a| {
                match a {
                    &Some(ref a2) => observer2(env, &a2),
                    &None => ()
                }
            }
        )
    }
}

pub trait CellTrait<ENV:'static,A:'static>: Sized {
    fn id(&self) -> u32;

    fn current_value<'a,F>(&self, env: &'a mut ENV, with_frp_context: &F) -> &'a A
    where
    F:WithFrpContext<ENV>
    {
        cell_current_value(self, env, with_frp_context)
    }

    fn observe<F,F2>(&self, env: &mut ENV, with_frp_context: &F, observer: F2) -> Box<FnOnce(&mut ENV, &F)>
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
        let cell_id = self.id().clone();
        {
            let frp_context = with_frp_context.with_frp_context(env);
            if let Some(cell) = frp_context.cell_map.get_mut(&cell_id) {
                let observer_id = cell.free_observer_id;
                unsafe { *observer_id_op2 = Some(observer_id); }
                cell.free_observer_id = cell.free_observer_id + 1;
                cell.observer_map.insert(observer_id, Box::new(
                    move |env, value| {
                        match value.downcast_ref::<A>() {
                            Some(value) => observer(env, value),
                            None => ()
                        }
                    }
                ));
            }
        }
        let cell_id = self.id().clone();
        match observer_id_op {
            Some(observer_id) => {
                return Box::new(move |env, with_frp_context| {
                    let frp_context = with_frp_context.with_frp_context(env);
                    if let Some(cell) = frp_context.cell_map.get_mut(&cell_id) {
                        cell.observer_map.remove(&observer_id);
                    }
                });
            },
            None => Box::new(|_, _| {})
        }
    }
}

// NOTE: Not safe for API use. Internal use only!
fn cell_current_value<ENV:'static,A:'static,C>(cell: &C, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>) -> &'static A
where
C: CellTrait<ENV,A>
{
    let mut value_op: Option<*const A> = None;
    let value_op2: *mut Option<*const A> = &mut value_op;
    {
        let frp_context = with_frp_context.with_frp_context(env);
        let value = cell_current_value_via_context(cell, frp_context);
        unsafe { (*value_op2) = Some(value); }
    }
    match value_op {
        Some(value) => {
            unsafe { &*value }
        },
        None => panic!("")
    }
}

// NOTE: Not safe for API use. Internal use only!
fn cell_current_value_via_context<ENV:'static,A:'static,C>(cell: &C, frp_context: &FrpContext<ENV>) -> &'static A
where
C: CellTrait<ENV,A>
{
    if let Some(loop_id) = frp_context.cell_loop_map.get(&cell.id()) {
        return cell_current_value_via_context(&Cell::of(loop_id.clone()), frp_context);
    }
    let result: *const A;
    match frp_context.cell_map.get(&cell.id()) {
        Some(cell) => {
            match &cell.value {
                &Value::Direct(ref x) => {
                    match x.as_ref().downcast_ref::<A>() {
                        Some(value) => result = value,
                        None => panic!("paniced on id: {}", cell.id)
                    }
                },
                &Value::AnotherCell(ref x) => {
                    let cell2: Cell<ENV,A> = Cell::of(x.id.clone());
                    result = cell_current_value_via_context(&cell2, frp_context);
                }
            }
        },
        None => panic!("")
    }
    return unsafe { &*result };
}

pub struct Cell<ENV,A:?Sized> {
    id: u32,
    env_phantom: PhantomData<ENV>,
    value_phantom: PhantomData<A>
}

impl<ENV:'static,A:'static> Clone for Cell<ENV,A> {
    fn clone(&self) -> Self {
        Cell::of(self.id.clone())
    }
}

impl<ENV:'static,A:'static> Copy for Cell<ENV,A> {}

impl<ENV:'static,A:'static> CellTrait<ENV,A> for Cell<ENV,A> {
    fn id(&self) -> u32 {
        self.id
    }
}

impl<ENV:'static,A:'static> Cell<ENV,Option<A>> {
    fn as_stream(&self) -> Stream<ENV,A> {
        Stream::of(self.id)
    }
}

impl<ENV,A:?Sized> Cell<ENV,A> {
    fn of(id: u32) -> Cell<ENV,A> {
        Cell {
            id: id,
            env_phantom: PhantomData,
            value_phantom: PhantomData
        }
    }
}

pub struct CellSink<ENV,A:?Sized> {
    id: u32,
    env_phantom: PhantomData<ENV>,
    value_phantom: PhantomData<A>
}

impl<ENV:'static,A:'static> Clone for CellSink<ENV,A> {
    fn clone(&self) -> Self {
        CellSink::of(self.id.clone())
    }
}

impl<ENV:'static,A:'static> Copy for CellSink<ENV,A> {}

impl<ENV:'static,A:'static> CellTrait<ENV,A> for CellSink<ENV,A> {
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

    pub fn change_value(&self, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, value: A)
    {
        let cell_id = self.id.clone();
        FrpContext::transaction(
            env,
            with_frp_context,
            move |env, with_frp_context| {
                let frp_context = with_frp_context.with_frp_context(env);
                if let Some(cell) = frp_context.cell_map.get_mut(&cell_id) {
                    cell.value = Value::Direct(Box::new(value) as Box<Any>);
                }
                frp_context.mark_all_decendent_cells_for_update(cell_id, &mut HashSet::new());
            }
        );
    }
}

pub struct Stream<ENV,A> {
    id: u32,
    env_phantom: PhantomData<ENV>,
    value_phantom: PhantomData<A>
}

impl<ENV:'static,A:'static> Stream<ENV,A> {
    fn of(id: u32) -> Stream<ENV,A> {
        Stream {
            id: id,
            env_phantom: PhantomData,
            value_phantom: PhantomData
        }
    }
}

impl<ENV:'static,A:'static> StreamTrait<ENV,A> for Stream<ENV,A> {
    fn id(&self) -> u32 {
        return self.id.clone();
    }
}

pub struct StreamSink<ENV,A> {
    id: u32,
    env_phantom: PhantomData<ENV>,
    value_phantom: PhantomData<A>
}

impl<ENV:'static,A:'static> StreamSink<ENV,A> {
    fn of(id: u32) -> StreamSink<ENV,A> {
        StreamSink {
            id: id,
            env_phantom: PhantomData,
            value_phantom: PhantomData
        }
    }

    pub fn send(&self, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, value: A)
    {
        CellSink::of(self.id).change_value(env, with_frp_context, Some(value));
    }
}


impl<ENV:'static,A:'static> StreamTrait<ENV,A> for StreamSink<ENV,A> {
    fn id(&self) -> u32 {
        return self.id.clone();
    }
}

struct CellImpl<ENV,A:?Sized> {
    id: u32,
    free_observer_id: u32,
    observer_map: HashMap<u32,Box<Fn(&mut ENV,&A)>>,
    update_fn_op: Option<Box<FnMut(&mut ENV, &WithFrpContext<ENV>, &mut A)>>,
    dependent_cells: Vec<u32>,
    depends_on_cells: Vec<u32>,

    reset_value_after_propergate_op: Option<Box<Fn(&mut A)>>,

    // When a cell gets freed, these child cells get freed also. It gets used in cell_switch(...).
    child_cells: Vec<u32>,

    value: Value<ENV,A>
}

enum Value<ENV,A:?Sized> {
    Direct(Box<A>),
    AnotherCell(Cell<ENV,A>)
}

impl<ENV:'static,A:?Sized> CellImpl<ENV,A> {
    fn into_any(mut self) -> CellImpl<ENV,Any>
    where A:Sized + 'static
    {
        let mut observer_map: HashMap<u32,Box<Fn(&mut ENV,&Any)>> = HashMap::new();
        for (k,v) in self.observer_map.drain() {
            observer_map.insert(k, Box::new(
                move |env, a| {
                    match a.downcast_ref::<A>() {
                        Some(a2) => v(env, a2),
                        None => ()
                    }
                }
            ));
        }
        let old_update_fn_op = self.update_fn_op;
        let mut update_fn_op: Option<Box<FnMut(&mut ENV, &WithFrpContext<ENV>, &mut Any) + 'static>>;
        match old_update_fn_op {
            Some(mut update_fn) => {
                let update_fn2 = move |env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, a: &mut Any| {
                    match a.downcast_mut::<A>() {
                        Some(a2) => update_fn.as_mut()(env, with_frp_context, a2),
                        None => ()
                    }
                };
                update_fn_op = Some(Box::new(update_fn2));
            },
            None => {
                update_fn_op = None;
            }
        }
        let reset_value_after_propergate_op: Option<Box<Fn(&mut Any)>>;
        match self.reset_value_after_propergate_op {
            Some(reset_value_after_propergate) => {
                reset_value_after_propergate_op = Some(
                    Box::new(move |a: &mut Any| {
                       match a.downcast_mut::<A>() {
                           Some(a2) => {
                               reset_value_after_propergate(a2);
                           },
                           None => {
                               println!("reset failed");
                           }
                       };
                   })
                );
            },
            None => {
                reset_value_after_propergate_op = None;
            }
        }
        let value = match self.value {
            Value::Direct(x) => Value::Direct(x as Box<Any>),
            Value::AnotherCell(x) => {
                let cell: Cell<ENV,Any> = Cell::of(x.id);
                Value::AnotherCell(cell)
            }
        };
        CellImpl {
            id: self.id,
            free_observer_id: self.free_observer_id,
            observer_map: observer_map,
            update_fn_op: update_fn_op,
            dependent_cells: self.dependent_cells,
            depends_on_cells: self.depends_on_cells,
            reset_value_after_propergate_op: reset_value_after_propergate_op,
            child_cells: self.child_cells,
            value: value
        }
    }
}
