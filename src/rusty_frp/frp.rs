use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::rc::Rc;
use std::rc::Weak;
use std::cell::RefCell;
use std::cell::Ref;
use std::cell::RefMut;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cmp::max;
use std::iter::FromIterator;

pub struct Cell<ENV,A> {
    node: Rc<RefCell<Node<ENV,Any>>>,
    phantom_a: PhantomData<A>
}

impl<ENV,A> Clone for Cell<ENV,A> {
    fn clone(&self) -> Self {
        Cell::of(self.node.clone())
    }
}

impl<ENV,A> Cell<ENV,A> {
    fn of(node: Rc<RefCell<Node<ENV,Any>>>) -> Cell<ENV,A> {
        Cell {
            node: node,
            phantom_a: PhantomData
        }
    }

    fn from_stream<SA>(sa: &SA) -> Cell<ENV,Option<A>>
    where SA: IsStream<ENV,A>
    {
        Cell::of(sa.node().clone())
    }
}

impl<ENV:'static,A:'static> Cell<ENV,Option<A>> {
    fn as_stream(&self) -> Stream<ENV,A> {
        Stream::of(self.node().clone())
    }
}

impl<ENV:'static,A:'static> IsCell<ENV,A> for Cell<ENV,A> {
    fn node<'r>(&'r self) -> &'r Rc<RefCell<Node<ENV,Any>>> {
        &self.node
    }
}

pub struct CellSink<ENV,A> {
    node: Rc<RefCell<Node<ENV,Any>>>,
    phantom_a: PhantomData<A>
}

impl<ENV,A> Clone for CellSink<ENV,A> {
    fn clone(&self) -> Self {
        CellSink::of(self.node.clone())
    }
}

impl<ENV,A> CellSink<ENV,A> {
    fn of(node: Rc<RefCell<Node<ENV,Any>>>) -> CellSink<ENV,A> {
        CellSink {
            node: node,
            phantom_a: PhantomData
        }
    }

    pub fn send(&self, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, value: A)
    where
    ENV: 'static,
    A: 'static
    {
        let node_id = self.node_id();
        FrpContext::transaction(
            env,
            with_frp_context,
            move |env, with_frp_context| {
                let frp_context = with_frp_context.with_frp_context(env);
                frp_context.unsafe_with_node_as_mut_by_node_id(
                    &node_id,
                    move |_,n| {
                        n.value = Value::Direct(Box::new(value));
                    }
                );
                frp_context.mark_all_decendent_nodes_for_update(&node_id);
            }
        )
    }
}

impl<ENV:'static,A:'static> IsCell<ENV,A> for CellSink<ENV,A> {
    fn node<'r>(&'r self) -> &'r Rc<RefCell<Node<ENV,Any>>> {
        &self.node
    }
}

pub struct Stream<ENV,A> {
    node: Rc<RefCell<Node<ENV,Any>>>,
    phantom_a: PhantomData<A>
}

impl<ENV:'static,A:'static> Clone for Stream<ENV,A> {
    fn clone(&self) -> Self {
        Stream::of(self.node.clone())
    }
}

impl<ENV:'static,A:'static> Stream<ENV,A> {
    fn of(node: Rc<RefCell<Node<ENV,Any>>>) -> Stream<ENV,A> {
        Stream {
            node: node,
            phantom_a: PhantomData
        }
    }
}

impl<ENV:'static,A:'static> IsStream<ENV,A> for Stream<ENV,A> {
    fn node<'r>(&'r self) -> &'r Rc<RefCell<Node<ENV,Any>>> {
        &self.node
    }
}

pub struct StreamSink<ENV,A> {
    node: Rc<RefCell<Node<ENV,Any>>>,
    phantom_a: PhantomData<A>
}

impl<ENV:'static,A:'static> StreamSink<ENV,A> {
    fn of(node: Rc<RefCell<Node<ENV,Any>>>) -> StreamSink<ENV,A> {
        StreamSink {
            node: node,
            phantom_a: PhantomData
        }
    }

    pub fn send(&self, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, value: A) {
        let cs: CellSink<ENV,Option<A>> = CellSink::of(self.node().clone());
        cs.send(env, with_frp_context, Some(value));
    }
}


impl<ENV:'static,A:'static> Clone for StreamSink<ENV,A> {
    fn clone(&self) -> Self {
        StreamSink::of(self.node.clone())
    }
}

impl<ENV:'static,A:'static> IsStream<ENV,A> for StreamSink<ENV,A> {
    fn node<'r>(&'r self) -> &'r Rc<RefCell<Node<ENV,Any>>> {
        &self.node
    }
}

macro_rules! lift_c {
    ($frp_context:expr, $f:expr, $($cell:expr),*) => {{
        let frp_context = $frp_context;
        let f2 = Box::new($f);
        let node_id = frp_context.next_cell_id();
        let depends_on_node_ids = vec![
            $($cell.node_id(),)*
        ];
        let depends_on_nodes = vec![
            $($cell.node().clone(),)*
        ];
        let mut rank: u32 = 0;
        $(
            rank = max(rank, $cell.with_node_as_ref(|n| n.rank.clone()));
        )*
        rank = rank + 1;
        let calc = move |frp_context: &FrpContext<_>| { f2( $($cell.sample(frp_context),)* ) };
        let initial_value = calc(frp_context);
        let update_fn: Box<Fn(&mut FrpContext<_>)->bool + 'static> = Box::new(
            move |frp_context| {
                let value = calc(frp_context);
                frp_context.unsafe_with_node_as_mut_by_node_id(
                    &node_id,
                    |_,n| {
                        n.value = Value::Direct(Box::new(value));
                    }
                );
                return true;
            }
        );
        let result_node = Cell::of(frp_context.insert_node(
            Node {
                id: node_id.clone(),
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes: depends_on_nodes,
                dependent_nodes: Vec::new(),
                update_fn_op: Some(update_fn),
                reset_value_after_propergate_op: None,
                delayed_value_op: None,
                is_delayed: false,
                rank: rank,
                value: Value::Direct(Box::new(initial_value))
            }
        ));
        {
            for depends_on_node_id in depends_on_node_ids {
                let result_node = result_node.node().clone();
                frp_context.unsafe_with_node_as_mut_by_node_id(
                    &depends_on_node_id,
                    |_,n| {
                        n.dependent_nodes.push(Rc::downgrade(&result_node));
                    }
                )
            }
        }
        result_node
    }}
}

pub trait IsCell<ENV,A> {
    fn node<'r>(&'r self) -> &'r Rc<RefCell<Node<ENV,Any>>>;

    fn as_cell(&self) -> Cell<ENV,A> {
        Cell::of(self.node().clone())
    }

    fn with_node_as_ref<F,R>(&self, k:F) -> R
    where
    ENV:'static,
    A:'static,
    F:FnOnce(&Node<ENV,Any>)->R
    {
        let tmp: &RefCell<Node<ENV,Any>> = self.node().borrow();
        let tmp2: Ref<Node<ENV,Any>> = tmp.borrow();
        k(tmp2.borrow())
    }

    fn with_node_as_mut<F,R>(&self, k:F) -> R
    where
    ENV:'static,
    A:'static,
    F:FnOnce(&mut Node<ENV,Any>)->R
    {
        let tmp: &RefCell<Node<ENV,Any>> = self.node().borrow();
        let mut tmp2: RefMut<Node<ENV,Any>> = tmp.borrow_mut();
        k(tmp2.borrow_mut())
    }

    fn node_id(&self) -> NodeID
    where
    ENV:'static,
    A:'static
    {
        self.with_node_as_ref(|n| n.id.clone())
    }

    fn observe<F>(&self, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, observer: F) -> Box<FnOnce(&mut FrpContext<ENV>)>
    where
    ENV: 'static,
    A: 'static,
    F:Fn(&mut ENV,&A) + 'static
    {
        {
            let value: *const Any;
            {
                let frp_context = with_frp_context.with_frp_context(env);
                value = self.sample(frp_context);
            }
            match (unsafe { &*value }).downcast_ref::<A>() {
                Some(value2) => observer(env, value2),
                None => ()
            }
        }
        let observer2: Box<Fn(&mut ENV, &Any)> = Box::new(
            move |env, a| {
                match a.downcast_ref::<A>() {
                    Some(a2) => observer(env, a2),
                    None => ()
                }
            }
        );
        let observer_id;
        {
            let tmp: &RefCell<Node<ENV,Any>> = self.node().borrow();
            let mut tmp2: RefMut<Node<ENV,Any>> = tmp.borrow_mut();
            let tmp3: &mut Node<ENV,Any> = tmp2.borrow_mut();
            observer_id = tmp3.free_observer_id.clone();
            tmp3.free_observer_id = tmp3.free_observer_id + 1;
            tmp3.observer_map.insert(observer_id, observer2);
        }
        let node_id = self.node_id();
        Box::new(move |frp_context| {
            frp_context.unsafe_with_node_as_mut_by_node_id(
                &node_id,
                move |_,n| {
                    n.observer_map.remove(&observer_id);
                }
            )
        })
    }

    fn sample<'r>(&self, frp_context: &'r FrpContext<ENV>) -> &'r A
    where
    ENV:'static,
    A:'static
    {
        let mut result: Option<*const A> = None;
        frp_context.unsafe_sample(&self.node_id(), |_, a| result = Some(a));
        match result {
            Some(a) => unsafe { &*a },
            None => panic!("")
        }
    }

    fn map<B,F>(&self, frp_context: &mut FrpContext<ENV>, f: F) -> Cell<ENV,B>
    where
    ENV:'static,
    A:'static,
    B:'static,
    F: Fn(&A)->B + 'static
    {
        let a_node_id = self.node_id();
        let node_id = frp_context.next_cell_id();
        let initial_value = f(self.sample(frp_context));
        let result_node: Cell<ENV,B> = Cell::of(frp_context.insert_node(
            Node {
                id: node_id.clone(),
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes: vec![self.node().clone()],
                dependent_nodes: Vec::new(),
                update_fn_op: Some(Box::new(move |frp_context| {
                    let b = frp_context.unsafe_sample(
                        &a_node_id,
                        |_: &FrpContext<ENV>, a: &A| f(a)
                    );
                    frp_context.unsafe_with_node_as_mut_by_node_id(
                        &node_id,
                        |_: &FrpContext<ENV>, n: &mut Node<ENV,Any>| {
                            match &mut n.value {
                                &mut Value::Direct(ref mut v) => {
                                    match v.downcast_mut::<B>() {
                                        Some(v2) => {
                                            *v2 = b;
                                        },
                                        None => panic!("Node has wrong type")
                                    }
                                },
                                &mut Value::InDirect(_) => panic!("Expected direct value")
                            }
                        }
                    );
                    return true;
                })),
                reset_value_after_propergate_op: None,
                delayed_value_op: None,
                is_delayed: false,
                rank: self.with_node_as_ref(|n| n.rank.clone() + 1),
                value: Value::Direct(Box::new(initial_value))
            }
        ));
        {
            let result_node = result_node.node().clone();
            self.with_node_as_mut(move |n| { n.dependent_nodes.push(Rc::downgrade(&result_node)); });
        }
        result_node
    }

    fn apply<B,CF>(&self, frp_context: &mut FrpContext<ENV>, cf: &CF) -> Cell<ENV,B>
    where
    ENV: 'static,
    A: 'static,
    B: 'static,
    CF: IsCell<ENV,Box<Fn(&A)->B>>
    {
        let a_node_id = self.node_id();
        let f_node_id = cf.node_id();
        let node_id = frp_context.next_cell_id();
        let initial_value = cf.sample(frp_context)(self.sample(frp_context));
        let result_node: Cell<ENV,B> = Cell::of(frp_context.insert_node(
            Node {
                id: node_id.clone(),
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes:
                    vec![
                        self.node().clone(),
                        cf.node().clone()
                    ],
                dependent_nodes: Vec::new(),
                update_fn_op: Some(Box::new(move |frp_context| {
                    let b = frp_context.unsafe_sample(
                        &a_node_id,
                        |frp_context: &FrpContext<ENV>, a: &A| {
                            frp_context.unsafe_sample(
                                &f_node_id,
                                |frp_context: &FrpContext<ENV>, f: &Box<Fn(&A)->B>| {
                                    f(a)
                                }
                            )
                        }
                    );
                    frp_context.unsafe_with_node_as_mut_by_node_id(
                        &node_id,
                        |_: &FrpContext<ENV>, n: &mut Node<ENV,Any>| {
                            match &mut n.value {
                                &mut Value::Direct(ref mut v) => {
                                    match v.downcast_mut::<B>() {
                                        Some(v2) => {
                                            *v2 = b
                                        },
                                        None => panic!("Node has wrong type")
                                    }
                                },
                                &mut Value::InDirect(_) => panic!("Expected direct value")
                            }
                        }
                    );
                    return true;
                })),
                reset_value_after_propergate_op: None,
                delayed_value_op: None,
                is_delayed: false,
                rank: max(self.with_node_as_ref(|n| n.rank.clone()), cf.with_node_as_ref(|n| n.rank.clone())) + 1,
                value: Value::Direct(Box::new(initial_value))
            }
        ));
        {
            let result_node = result_node.node().clone();
            self.with_node_as_mut(move |n| { n.dependent_nodes.push(Rc::downgrade(&result_node)); });
        }
        {
            let result_node = result_node.node().clone();
            cf.with_node_as_mut(move |n| { n.dependent_nodes.push(Rc::downgrade(&result_node)); });
        }
        result_node
    }

    fn lift2<B,C,CB,F>(&self, frp_context: &mut FrpContext<ENV>, cb: &CB, f: F) -> Cell<ENV,C>
    where
    ENV: 'static,
    A: 'static,
    B: 'static,
    C: 'static,
    CB: IsCell<ENV,B>,
    F: Fn(&A,&B)->C + 'static
    {
        let ca: Cell<ENV,A> = self.as_cell();
        let cb: Cell<ENV,B> = cb.as_cell();
        let result: Cell<ENV,C> = lift_c!(frp_context, f, ca, cb);
        return result;
    }

    fn lift3<B,C,D,CB,CC,F>(&self, frp_context: &mut FrpContext<ENV>, cb: &CB, cc: &CC, f: F) -> Cell<ENV,D>
    where
    ENV: 'static,
    A: 'static,
    B: 'static,
    C: 'static,
    D: 'static,
    CB: IsCell<ENV,B>,
    CC: IsCell<ENV,C>,
    F: Fn(&A,&B,&C)->D + 'static
    {
        let ca: Cell<ENV,A> = self.as_cell();
        let cb: Cell<ENV,B> = cb.as_cell();
        let cc: Cell<ENV,C> = cc.as_cell();
        let result: Cell<ENV,D> = lift_c!(frp_context, f, ca, cb, cc);
        return result;
    }

    fn lift4<B,C,D,E,CB,CC,CD,F>(&self, frp_context: &mut FrpContext<ENV>, cb: &CB, cc: &CC, cd: &CD, f: F) -> Cell<ENV,E>
    where
    ENV: 'static,
    A: 'static,
    B: 'static,
    C: 'static,
    D: 'static,
    E: 'static,
    CB: IsCell<ENV,B>,
    CC: IsCell<ENV,C>,
    CD: IsCell<ENV,D>,
    F: Fn(&A,&B,&C,&D)->E + 'static
    {
        let ca: Cell<ENV,A> = self.as_cell();
        let cb: Cell<ENV,B> = cb.as_cell();
        let cc: Cell<ENV,C> = cc.as_cell();
        let cd: Cell<ENV,D> = cd.as_cell();
        let result: Cell<ENV,E> = lift_c!(frp_context, f, ca, cb, cc, cd);
        return result;
    }

    fn values(&self, frp_context: &mut FrpContext<ENV>) -> Stream<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone
    {
        self.map(frp_context, |a| Some(a.clone())).as_stream()
    }

    fn updates(&self, frp_context: &mut FrpContext<ENV>) -> Stream<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone
    {
        let ca: Cell<ENV,Option<A>> = self.map(frp_context, |a| Some(a.clone()));
        let tmp1: &Rc<RefCell<Node<ENV,Any>>> = ca.node();
        let tmp2: &RefCell<Node<ENV,Any>> = tmp1.borrow();
        let mut tmp3: RefMut<Node<ENV,Any>> = tmp2.borrow_mut();
        let mut tmp4: &mut Node<ENV,Any> = tmp3.borrow_mut();
        tmp4.value = Value::Direct(Box::new(None as Option<A>));
        ca.as_stream()
    }
}

pub trait IsStream<ENV,A> {
    fn node<'r>(&'r self) -> &'r Rc<RefCell<Node<ENV,Any>>>;

    fn as_cell(&self) -> Cell<ENV,Option<A>> {
        Cell::of(self.node().clone())
    }

    fn with_node_as_ref<F,R>(&self, k:F) -> R
    where
    ENV:'static,
    A:'static,
    F:FnOnce(&Node<ENV,Any>)->R
    {
        let tmp: &RefCell<Node<ENV,Any>> = self.node().borrow();
        let tmp2: Ref<Node<ENV,Any>> = tmp.borrow();
        k(tmp2.borrow())
    }

    fn node_id(&self) -> NodeID
    where
    ENV:'static,
    A:'static
    {
        self.with_node_as_ref(|n| n.id.clone())
    }

    fn map<B,F>(&self, frp_context: &mut FrpContext<ENV>, f: F) -> Stream<ENV,B>
    where
    ENV: 'static,
    A: 'static,
    B: 'static,
    F: Fn(&A)->B + 'static
    {
        self.as_cell()
            .map(
                frp_context,
                move |a| {
                    match a {
                        &Some(ref a2) => Some(f(a2)),
                        &None => None
                    }
                }
            )
            .as_stream()
    }

    fn filter<F>(&self, frp_context: &mut FrpContext<ENV>, f: F) -> Stream<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone,
    F: Fn(&A)->bool + 'static
    {
        self.as_cell()
            .map(
                frp_context,
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
            )
            .as_stream()
    }

    fn snapshot<B,C,CB,F>(&self, frp_context: &mut FrpContext<ENV>, cb: &CB, f: F) -> Stream<ENV,C>
    where
    ENV: 'static,
    A: 'static,
    B: 'static,
    C: 'static,
    CB: IsCell<ENV,B>,
    F: Fn(&A,&B)->C + 'static
    {
        self.as_cell()
            .lift2(
                frp_context,
                &cb.as_cell(),
                move |a_op,b| {
                    match a_op {
                        &Some(ref a) => Some(f(a,b)),
                        &None => None
                    }
                }
            )
            .as_stream()
    }

    fn merge<F,SA>(&self, frp_context: &mut FrpContext<ENV>, sa: &SA, f: F) -> Stream<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone,
    SA: IsStream<ENV,A>,
    F: Fn(&A,&A)->A + 'static
    {
        self.as_cell()
            .lift2(
                frp_context,
                &sa.as_cell(),
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
                }
            )
            .as_stream()
    }

    fn or_else<SA>(&self, frp_context: &mut FrpContext<ENV>, sa: &SA) -> Stream<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone,
    SA: IsStream<ENV,A>,
    {
        self.merge(frp_context, sa, |a,b| a.clone())
    }

    fn observe<F>(&self, env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, k: F) -> Box<FnOnce(&mut FrpContext<ENV>)>
    where
    ENV: 'static,
    A: 'static,
    F: Fn(&mut ENV, &A) + 'static
    {
        self.as_cell()
            .observe(
                env,
                with_frp_context,
                move |env, a| {
                    match a {
                        &Some(ref a2) => k(env, a2),
                        &None => ()
                    }
                }
            )
    }

    fn hold(&self, frp_context: &mut FrpContext<ENV>, value: A) -> Cell<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone
    {
        let delayed_self = self.delay(frp_context);
        let delayed_self_id = delayed_self.node_id().clone();
        let node_id = frp_context.next_cell_id();
        let result: Cell<ENV,A> = Cell::of(frp_context.insert_node(
            Node {
                id: node_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes: vec![delayed_self.node().clone()],
                dependent_nodes: Vec::new(),
                update_fn_op: Some(Box::new(
                    move |frp_context: &mut FrpContext<ENV>| {
                        return frp_context.unsafe_with_node_as_mut_by_node_id(
                            &node_id,
                            |frp_context: &FrpContext<ENV>, n| {
                                let v_op = frp_context.unsafe_sample(
                                    &delayed_self_id,
                                    |_: &FrpContext<ENV>, v: &Option<A>| v.clone()
                                );
                                match v_op {
                                    Some(v) => {
                                        n.value = Value::Direct(Box::new(v) as Box<Any>);
                                        return true;
                                    },
                                    None => return false
                                }
                            }
                        );
                    }
                )),
                reset_value_after_propergate_op: None,
                delayed_value_op: None,
                is_delayed: false,
                rank: self.with_node_as_ref(|n| n.rank.clone()) + 1,
                value: Value::Direct(Box::new(value))
            }
        ));
        frp_context.unsafe_with_node_as_mut_by_node_id(
            &delayed_self.node_id(),
            |_: &FrpContext<ENV>, n| {
                n.dependent_nodes.push(Rc::downgrade(result.node()));
            }
        );
        result
    }

    fn delay(&self, frp_context: &mut FrpContext<ENV>) -> Stream<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone
    {
        let node_id = self.node_id().clone();
        let result_node_id = frp_context.next_cell_id();
        let update_fn: Box<Fn(&mut FrpContext<ENV>)->bool> = Box::new(
            move |frp_context| {
                let changed = frp_context.unsafe_with_node_as_mut_by_node_id(
                    &result_node_id,
                    move |frp_context: &FrpContext<ENV>, n| {
                        let mut changed = false;
                        let value = frp_context.unsafe_sample(
                            &node_id,
                            |_: &FrpContext<ENV>, v_op: &Option<A>| {
                                v_op.clone()
                            }
                        );
                        match value {
                            Some(value2) => {
                                n.delayed_value_op = Some(Box::new(
                                    move |v: &mut Any| {
                                        match v.downcast_mut::<Option<A>>() {
                                            Some(v2) => {
                                                *v2 = Some(value2.clone())
                                            }
                                            None => ()
                                        }
                                    }
                                ));
                            },
                            None => {
                                n.delayed_value_op = None;
                            }
                        }
                        return changed;
                    }
                );
                return changed;
            }
        );
        let initial_value: Option<A> = None;
        let result: Stream<ENV,A> = Stream::of(frp_context.insert_node(
            Node::<ENV,Option<A>> {
                id: result_node_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes: vec![self.node().clone()],
                dependent_nodes: Vec::new(),
                update_fn_op: Some(update_fn),
                reset_value_after_propergate_op: Some(Box::new(
                    |a: &mut Option<A>| {
                        *a = None;
                    }
                )),
                delayed_value_op: None,
                is_delayed: true,
                rank: self.with_node_as_ref(|n| n.rank.clone()) + 1,
                value: Value::Direct(Box::new(initial_value))
            }
        ));
        frp_context.unsafe_with_node_as_mut_by_node_id(
            &self.node_id(),
            |_: &FrpContext<ENV>, n| {
                n.dependent_nodes.push(Rc::downgrade(result.node()));
            }
        );
        result
    }
}

type NodeID = usize;

type Time = u32;

struct Node<ENV,A:?Sized> {
    id: NodeID,
    free_observer_id: u32,
    observer_map: HashMap<u32,Box<Fn(&mut ENV,&A)>>,
    depends_on_nodes: Vec<Rc<RefCell<Node<ENV,Any>>>>,
    dependent_nodes: Vec<Weak<RefCell<Node<ENV,Any>>>>,
    update_fn_op: Option<Box<Fn(&mut FrpContext<ENV>)->bool>>,
    reset_value_after_propergate_op: Option<Box<Fn(&mut A)>>,
    delayed_value_op: Option<Box<Fn(&mut A)>>,
    is_delayed: bool,
    rank: u32,
    value: Value<A>
}

enum Value<A:?Sized> {
    Direct(Box<A>),
    InDirect(NodeID)
}

impl<ENV,A> Node<ENV,A> {
    fn to_raw(mut self) -> Node<ENV,Any>
    where
    ENV:'static,
    A:'static
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
            Value::InDirect(x) => Value::InDirect(x)
        };
        Node {
            id: self.id,
            free_observer_id: self.free_observer_id,
            observer_map: observer_map,
            depends_on_nodes: self.depends_on_nodes,
            dependent_nodes: self.dependent_nodes,
            update_fn_op: self.update_fn_op,
            reset_value_after_propergate_op: reset_value_after_propergate_op,
            delayed_value_op: match self.delayed_value_op {
                Some(k) => Some(Box::new(
                    move |v: &mut Any| {
                        match v.downcast_mut::<A>() {
                            Some(v2) => k(&mut *v2),
                            None => ()
                        }
                    }
                )),
                None => None
            },
            is_delayed: self.is_delayed,
            rank: self.rank,
            value: value
        }
    }
}

pub struct FrpContext<ENV> {
    graph: Vec<Weak<RefCell<Node<ENV,Any>>>>,
    nodes_to_be_updated: HashSet<NodeID>,
    transaction_depth: u32
}

impl<ENV:'static> FrpContext<ENV> {
    pub fn new() -> FrpContext<ENV> {
        FrpContext {
            graph: Vec::new(),
            nodes_to_be_updated: HashSet::new(),
            transaction_depth: 0
        }
    }

    pub fn cell_loop<A,F>(&mut self, k: F) -> Cell<ENV,A>
    where
    ENV: 'static,
    A: 'static + Clone,
    F: FnOnce(&mut FrpContext<ENV>,&Cell<ENV,Box<Fn()->A + 'static>>)->Cell<ENV,A>
    {
        let cell: CellSink<ENV,Box<Fn()->A + 'static>> = self.new_cell_sink(
            Box::new(
                || panic!("cell_loop: value observed before loop was constructed")
            )
        );
        let cell2 = k(self,&cell.as_cell());
        let cell3: Cell<ENV,Box<Fn()->A + 'static>> = cell2.map(
            self,
            |a| {
                let a2 = a.clone();
                let r: Box<Fn()->A> = Box::new(move || a2.clone());
                r
            }
        );
        {
            let tmp1: &Rc<RefCell<Node<ENV,Any>>> = cell.node();
            let tmp2: &RefCell<Node<ENV,Any>> = tmp1.borrow();
            let mut tmp3: RefMut<Node<ENV,Any>> = tmp2.borrow_mut();
            let tmp4: &mut Node<ENV,Any> = tmp3.borrow_mut();
            tmp4.value = Value::InDirect(cell3.node_id());
        }
        let cell4: Cell<ENV,A> = cell3.map(
            self,
            |thunk| thunk()
        );
        cell4
    }

    pub fn filter_option<A,SA_OP>(&mut self, sa_op: &SA_OP) -> Stream<ENV,A>
    where
    A: 'static + Clone,
    SA_OP: IsStream<ENV,Option<A>>
    {
        sa_op
            .as_cell()
            .map(
                self,
                |a_op_op| {
                    match a_op_op {
                        &Some(ref a_op) => {
                            match a_op {
                                &Some(ref a) => Some(a.clone()),
                                &None => None
                            }
                        },
                        &None => None
                    }
                }
            )
            .as_stream()
    }

    pub fn transaction<F,R>(env: &mut ENV, with_frp_context: &WithFrpContext<ENV>, k: F) -> R
    where F: FnOnce(&mut ENV, &WithFrpContext<ENV>) -> R
    {
        {
            let frp_context = with_frp_context.with_frp_context(env);
            frp_context.transaction_depth = frp_context.transaction_depth + 1;
        }
        let result = k(env, with_frp_context);
        let final_depth;
        {
            let frp_context = with_frp_context.with_frp_context(env);
            frp_context.transaction_depth = frp_context.transaction_depth - 1;
            final_depth = frp_context.transaction_depth;
        }
        if final_depth == 0 {
            FrpContext::propergate(env, with_frp_context);
        }
        result
    }

    fn propergate(env: &mut ENV, with_frp_context: &WithFrpContext<ENV>) {
        let mut limit = 50;
        loop {
            limit = limit - 1;
            if limit == 0 {
                panic!("Propergation limit exceeded.");
            }
            let node_ids_in_update_order: Vec<NodeID>;
            {
                let mut node_id_rank_list: Vec<(NodeID,u32)> = Vec::new();
                let frp_context = with_frp_context.with_frp_context(env);
                for node_id in &frp_context.nodes_to_be_updated {
                    let rank = frp_context.unsafe_with_node_as_ref_by_node_id(
                        node_id,
                        |_: &FrpContext<ENV>, n| {
                            n.rank.clone()
                        }
                    );
                    node_id_rank_list.push((node_id.clone(), rank));
                }
                node_id_rank_list.sort_by(|&(_,ref rank1),&(_,ref rank2)| rank1.cmp(rank2));
                node_ids_in_update_order = Vec::from_iter(node_id_rank_list.iter().map(|&(ref node_id,_)| node_id.clone()));
            }
            {
                let frp_context = with_frp_context.with_frp_context(env);
                frp_context.nodes_to_be_updated.clear();
            }
            {
                let frp_context = with_frp_context.with_frp_context(env);
                for node_id in &node_ids_in_update_order {
                    frp_context.update_node(node_id);
                }
            }
            {
                let env2: *mut ENV = env;
                let frp_context = with_frp_context.with_frp_context(env);
                for node_id in &node_ids_in_update_order {
                    let is_delayed = frp_context.unsafe_with_node_as_ref_by_node_id(node_id, |_:&FrpContext<ENV>,n| n.is_delayed);
                    if !is_delayed {
                        let mut value_op: Option<*const Any> = None;
                        frp_context.unsafe_sample2(
                            node_id,
                            |_: &FrpContext<ENV>, value| {
                                value_op = Some(value);
                            }
                        );
                        match value_op {
                            Some(value) => {
                                frp_context.unsafe_with_node_as_ref_by_node_id(
                                    node_id,
                                    |_, n| {
                                        for (_,observer) in &n.observer_map {
                                            observer(unsafe { &mut *env2 }, unsafe { &*value });
                                        }
                                    }
                                );
                            },
                            None => ()
                        }
                    }
                }
            }
            for node_id in &node_ids_in_update_order {
                let frp_context = with_frp_context.with_frp_context(env);
                frp_context.unsafe_with_node_as_mut_by_node_id(
                    node_id,
                    |_, n| {
                        match &n.reset_value_after_propergate_op {
                            &Some(ref reset_value_after_propergate) => {
                                match &mut n.value {
                                    &mut Value::Direct(ref mut x) => {
                                        reset_value_after_propergate(x.as_mut());
                                    },
                                    &mut Value::InDirect(_) => ()
                                }
                            },
                            &None => ()
                        }
                    }
                );
                let mark_it = frp_context.unsafe_with_node_as_mut_by_node_id(
                    node_id,
                    |_: &FrpContext<ENV>, n| {
                        match &n.delayed_value_op {
                            &Some(ref delayed_value) => {
                                match &mut n.value {
                                    &mut Value::Direct(ref mut v) => {
                                        delayed_value(v.as_mut());
                                        true
                                    },
                                    &mut Value::InDirect(_) => false
                                }
                            },
                            &None => false
                        }
                    }
                );
                if mark_it {
                    let mut decendent_ids: Vec<NodeID> = Vec::new();
                    frp_context.unsafe_with_node_as_ref_by_node_id(
                        node_id,
                        |frp_context:&FrpContext<ENV>,n| {
                            for dependent_node in &n.dependent_nodes {
                                match dependent_node.upgrade() {
                                    Some(tmp) => {
                                        let tmp2: &RefCell<Node<ENV,Any>> = tmp.borrow();
                                        let tmp3: Ref<Node<ENV,Any>> = tmp2.borrow();
                                        let tmp4: &Node<ENV,Any> = tmp3.borrow();
                                        decendent_ids.push(tmp4.id.clone());
                                    },
                                    None => ()
                                }
                            }
                        }
                    );
                    for decendent_id in decendent_ids {
                        frp_context.mark_all_decendent_nodes_for_update(&decendent_id);
                    }
                }
            }
            {
                let env2: *mut ENV = env;
                let frp_context = with_frp_context.with_frp_context(env);
                for node_id in &node_ids_in_update_order {
                    let is_delayed = frp_context.unsafe_with_node_as_ref_by_node_id(node_id, |_:&FrpContext<ENV>,n| n.is_delayed);
                    if is_delayed {
                        let mut value_op: Option<*const Any> = None;
                        frp_context.unsafe_sample2(
                            node_id,
                            |_: &FrpContext<ENV>, value| {
                                value_op = Some(value);
                            }
                        );
                        match value_op {
                            Some(value) => {
                                frp_context.unsafe_with_node_as_ref_by_node_id(
                                    node_id,
                                    |_, n| {
                                        for (_,observer) in &n.observer_map {
                                            observer(unsafe { &mut *env2 }, unsafe { &*value });
                                        }
                                    }
                                );
                            },
                            None => ()
                        }
                    }
                }
            }
            let again: bool;
            {
                let frp_context = with_frp_context.with_frp_context(env);
                again = frp_context.nodes_to_be_updated.len() > 0;
            }
            if !again {
                break;
            }
        }
    }

    fn update_node(&mut self, node_id: &NodeID)->bool {
        let mut update_fn_op: Option<*const Fn(&mut FrpContext<ENV>)->bool> = None;
        self.unsafe_with_node_as_ref_by_node_id(
            node_id,
            |_,n| {
                match &n.update_fn_op {
                    &Some(ref update_fn) => update_fn_op = Some(update_fn.as_ref()),
                    &None => ()
                }
            }
        );
        match update_fn_op {
            Some(update_fn) => {
                return unsafe { (*update_fn)(self) };
            },
            None => ()
        }
        return true;
    }

    fn mark_all_decendent_nodes_for_update(&mut self, node_id: &NodeID) {
        self.mark_all_decendent_nodes_for_update2(node_id, &mut HashSet::new());
    }

    fn mark_all_decendent_nodes_for_update2(&mut self, node_id: &NodeID, visited: &mut HashSet<NodeID>) {
        if visited.contains(node_id) {
            return;
        }
        let mut dependent_node_ids: Vec<NodeID> = Vec::new();
        self.nodes_to_be_updated.insert(node_id.clone());
        visited.insert(node_id.clone());
        if self.unsafe_with_node_as_ref_by_node_id(&node_id, |_:&FrpContext<ENV>, n| n.is_delayed.clone()) {
            return;
        }
        self.unsafe_with_node_as_ref_by_node_id(
            &node_id,
            |_: &FrpContext<ENV>, n: &Node<ENV,Any>| {
                for dependent_node in &n.dependent_nodes {
                    match dependent_node.upgrade() {
                        Some(tmp1) => {
                            let tmp2: &RefCell<Node<ENV,Any>> = tmp1.borrow();
                            let tmp3: Ref<Node<ENV,Any>> = tmp2.borrow();
                            let tmp4: &Node<ENV,Any> = tmp3.borrow();
                            dependent_node_ids.push(tmp4.id.clone());
                        },
                        None => ()
                    }
                }
            }
        );
        for dependent_node_id in dependent_node_ids {
            self.mark_all_decendent_nodes_for_update2(&dependent_node_id, visited);
        }
    }

    fn next_cell_id(&self) -> NodeID {
        let len = self.graph.len();
        for i in 0..len {
            match self.graph.get(i) {
                Some(x) => {
                    match x.upgrade() {
                        Some(_) => (),
                        None => return i
                    }
                },
                None => ()
            }
        }
        return self.graph.len();
    }

    fn with_raw_node_as_ref<F,R>(&self, node_id: &NodeID, k: F) -> R
    where
    F: FnOnce(&Node<ENV,Any>)->R,
    {
        let do_panic;
        {
            let node_id = node_id.clone();
            do_panic = move || {
                panic!("Node with ID {} did not live long enough", node_id);
            };
        }
        match self.graph.get(node_id.clone()) {
            Some(x) => {
                match x.upgrade() {
                    Some(x3) => {
                        let x4: &RefCell<Node<ENV,Any>> = x3.borrow();
                        let x5: Ref<Node<ENV,Any>> = x4.borrow();
                        k(x5.borrow())
                    },
                    None => do_panic()
                }
            },
            None => do_panic()
        }
    }

    fn unsafe_node_id_into_raw_node(&self, node_id: &NodeID) -> Rc<RefCell<Node<ENV,Any>>> {
        match self.graph[node_id.clone()].upgrade() {
            Some(n2) => n2,
            None => panic!("No node exists with node id {}", node_id)
        }
    }

    fn unsafe_with_node_as_ref_by_node_id<F,R>(&self, node_id: &NodeID, k:F) -> R
    where
    ENV:'static,
    F:FnOnce(&FrpContext<ENV>,&Node<ENV,Any>)->R
    {
        let tmp = self.unsafe_node_id_into_raw_node(node_id);
        let tmp2: &RefCell<Node<ENV,Any>> = tmp.borrow();
        let tmp3: Ref<Node<ENV,Any>> = tmp2.borrow();
        k(self, tmp3.borrow())
    }

    fn unsafe_with_node_as_mut_by_node_id<F,R>(&self, node_id: &NodeID, k:F) -> R
    where
    ENV:'static,
    F:FnOnce(&FrpContext<ENV>,&mut Node<ENV,Any>)->R
    {
        let tmp = self.unsafe_node_id_into_raw_node(node_id);
        let tmp2: &RefCell<Node<ENV,Any>> = tmp.borrow();
        let mut tmp3: RefMut<Node<ENV,Any>> = tmp2.borrow_mut();
        k(self, tmp3.borrow_mut())
    }

    fn unsafe_sample2<F,R>(&self, node_id: &NodeID, k: F) -> R
    where
    ENV:'static,
    F: FnOnce(&Self,&Any) -> R
    {
        self.unsafe_with_node_as_ref_by_node_id(
            node_id,
            move |frp_context, node| {
                match &node.value {
                    &Value::Direct(ref x) => {
                        let x2 = x.as_ref();
                        k(frp_context, x2)
                    },
                    &Value::InDirect(ref node_id2) => frp_context.unsafe_sample2(node_id2, k)
                }
            }
        )
    }

    fn unsafe_sample<A,F,R>(&self, node_id: &NodeID, k: F) -> R
    where
    ENV:'static,
    A:'static,
    F: FnOnce(&Self,&A) -> R
    {
        self.unsafe_with_node_as_ref_by_node_id(
            node_id,
            move |frp_context, node| {
                match &node.value {
                    &Value::Direct(ref x) => {
                        let x2 = x.as_ref();
                        match x.downcast_ref::<A>() {
                            Some(x2) => k(frp_context, x2),
                            None => panic!("Node of wrong type")
                        }
                    },
                    &Value::InDirect(ref node_id2) => frp_context.unsafe_sample(node_id2, k)
                }
            }
        )
    }

    fn insert_node<A>(&mut self, node: Node<ENV,A>) -> Rc<RefCell<Node<ENV,Any>>>
    where
    A:'static
    {
        let node_id = node.id.clone();
        let rc = Rc::new(RefCell::new(node.to_raw()));
        let w = Rc::downgrade(&rc);
        let len = self.graph.len();
        if node_id == len {
            self.graph.push(w);
        } else {
            self.graph[node_id] = w;
        }
        return rc;
    }

    pub fn never<A>(&mut self) -> Stream<ENV,A>
    where
    A:'static
    {
        let ss: StreamSink<ENV,A> = self.new_stream_sink();
        Stream::of(ss.node().clone())
    }

    pub fn constant<A>(&mut self, value: A) -> Cell<ENV,A>
    where
    A: 'static
    {
        let cs: CellSink<ENV,A> = self.new_cell_sink(value);
        Cell::of(cs.node().clone())
    }

    pub fn new_cell_sink<A>(&mut self, value: A) -> CellSink<ENV,A>
    where
    A:'static
    {
        let node_id = self.next_cell_id();
        CellSink::of(self.insert_node(
            Node {
                id: node_id,
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes: Vec::new(),
                dependent_nodes: Vec::new(),
                update_fn_op: None,
                reset_value_after_propergate_op: None,
                delayed_value_op: None,
                is_delayed: false,
                rank: 0,
                value: Value::Direct(Box::new(value))
            }
        ))
    }

    pub fn new_stream_sink<A>(&mut self) -> StreamSink<ENV,A>
    where
    A:'static
    {
        let ca: CellSink<ENV,Option<A>> = self.new_cell_sink(None);
        let tmp1: &Rc<RefCell<Node<ENV,Any>>> = ca.node();
        let tmp2: &RefCell<Node<ENV,Any>> = tmp1.borrow();
        let mut tmp3: RefMut<Node<ENV,Any>> = tmp2.borrow_mut();
        let tmp4: &mut Node<ENV,Any> = tmp3.borrow_mut();
        tmp4.reset_value_after_propergate_op = Some(
            Box::new(|v: &mut Any| {
                match v.downcast_mut::<Option<A>>() {
                    Some(v2) => *v2 = None,
                    None => ()
                }
            }
        ));
        StreamSink::of(ca.node().clone())
    }

    pub fn switch_s<A,CSA>(&mut self, cell_thunk_stream_a: &CSA) -> Stream<ENV,A>
    where
    A:'static,
    CSA:IsCell<ENV,Box<Fn(&mut FrpContext<ENV>)->Stream<ENV,A>>>
    {
        let ccaop: Cell<ENV,Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,Option<A>>>> = cell_thunk_stream_a.map(
            self,
            |k: &Box<Fn(&mut FrpContext<ENV>)->Stream<ENV,A>>| {
                let k2: *const Fn(&mut FrpContext<ENV>)->Stream<ENV,A> = k.as_ref();
                let k3: Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,Option<A>>>;
                k3 = Box::new(move |frp_context| { unsafe { (*k2)(frp_context).as_cell() } });
                k3
            }
        );
        let c: Cell<ENV,Option<A>> = self.switch_c(&ccaop);
        c.as_stream()
    }

    pub fn switch_c<A,CCA>(&mut self, cell_thunk_cell_a: &CCA) -> Cell<ENV,A>
    where
    A:'static,
    CCA:IsCell<ENV,Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,A>>>
    {
        let initial_inner_cell_fn: *const Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,A>> = cell_thunk_cell_a.sample(self);
        let initial_inner_cell = unsafe { (*initial_inner_cell_fn)(self) };
        let node_id = self.next_cell_id();
        let cell_thunk_cell_a = cell_thunk_cell_a.as_cell();
        let cell_thunk_cell_a2 = cell_thunk_cell_a.clone();
        let c: Cell<ENV,A> = Cell::of(self.insert_node(
            Node::<ENV,A> {
                id: node_id.clone(),
                free_observer_id: 0,
                observer_map: HashMap::new(),
                depends_on_nodes: vec![cell_thunk_cell_a.node().clone(), initial_inner_cell.node().clone()],
                dependent_nodes: Vec::new(),
                update_fn_op: Some(
                    Box::new(move |frp_context: &mut FrpContext<ENV>| {
                        let inner_cell_fn: *const Box<Fn(&mut FrpContext<ENV>)->Cell<ENV,A>> = cell_thunk_cell_a.sample(frp_context);
                        let inner_cell = unsafe { (*initial_inner_cell_fn)(frp_context) };
                        let last_inner_node_id_op = frp_context.unsafe_with_node_as_ref_by_node_id(
                            &node_id,
                            |_: &FrpContext<ENV>, n: &Node<ENV,Any>| {
                                match &n.value {
                                    &Value::Direct(_) => None,
                                    &Value::InDirect(ref n2) => Some(n2.clone())
                                }
                            }
                        );
                        match last_inner_node_id_op {
                            Some(last_inner_node_id) => {
                                frp_context.unsafe_with_node_as_mut_by_node_id(
                                    &last_inner_node_id,
                                    |_: &FrpContext<ENV>, n| {
                                        n.dependent_nodes.retain(|n2| {
                                            match n2.upgrade() {
                                                Some(tmp1) => {
                                                    let tmp2: &RefCell<Node<ENV,Any>> = tmp1.borrow();
                                                    let tmp3: Ref<Node<ENV,Any>> = tmp2.borrow();
                                                    let tmp4: &Node<ENV,Any> = tmp3.borrow();
                                                    tmp4.id != node_id.clone()
                                                },
                                                None => true
                                            }
                                        });
                                    }
                                );
                            },
                            None => ()
                        }
                        frp_context.unsafe_with_node_as_mut_by_node_id(
                            &inner_cell.node_id(),
                            |_: &FrpContext<ENV>, n| {
                                n.dependent_nodes.push(frp_context.graph[node_id].clone());
                            }
                        );
                        let cell_thunk_cell_a2 = cell_thunk_cell_a.clone();
                        frp_context.unsafe_with_node_as_mut_by_node_id(
                            &node_id,
                            move |_: &FrpContext<ENV>, n: &mut Node<ENV,Any>| {
                                n.value = Value::InDirect(inner_cell.node_id());
                                n.rank = max(cell_thunk_cell_a2.with_node_as_ref(|n| n.rank.clone()), inner_cell.with_node_as_ref(|n| n.rank.clone())) + 1;
                            }
                        );
                        return true;
                    })
                ),
                reset_value_after_propergate_op: None,
                delayed_value_op: None,
                is_delayed: false,
                rank: max(cell_thunk_cell_a2.with_node_as_ref(|n| n.rank.clone()), initial_inner_cell.with_node_as_ref(|n| n.rank.clone())) + 1,
                value: Value::InDirect(initial_inner_cell.node_id())
            }
        ));
        self.unsafe_with_node_as_mut_by_node_id(
            &cell_thunk_cell_a2.node_id(),
            |_: &FrpContext<ENV>, n| {
                n.dependent_nodes.push(Rc::downgrade(c.node()));
            }
        );
        self.unsafe_with_node_as_mut_by_node_id(
            &initial_inner_cell.node_id(),
            |_: &FrpContext<ENV>, n| {
                n.dependent_nodes.push(Rc::downgrade(c.node()));
            }
        );
        c
    }
}

pub trait WithFrpContext<ENV> {
    fn with_frp_context<'r>(&self, &'r mut ENV) -> &'r mut FrpContext<ENV>;
}
