use sodium::HandlerRefMut;
use sodium::IsNode;
use sodium::Node;
use sodium::SodiumCtx;
use std::borrow::BorrowMut;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::cmp::Ordering;
use std::cmp::PartialEq;
use std::cmp::PartialOrd;
use std::collections::BinaryHeap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::mem::swap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;

pub struct Entry {
    pub rank: Rc<RefCell<IsNode>>,
    pub action: HandlerRefMut<Transaction>,
    pub seq: u64
}

impl Entry {
    fn new(sodium_ctx: &mut SodiumCtx, rank: Rc<RefCell<IsNode>>, action: HandlerRefMut<Transaction>) -> Entry {
        Entry {
            rank: rank,
            action: action,
            seq: sodium_ctx.new_seq()
        }
    }
}

impl Clone for Entry {
    fn clone(&self) -> Entry {
        Entry {
            rank: self.rank.clone(),
            action: self.action.clone(),
            seq: self.seq.clone()
        }
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Entry) -> Ordering {
        let c = (*self.rank.borrow()).downcast_to_node_ref().rank.cmp(
            &((*other.rank.borrow()).downcast_to_node_ref().rank)
        );
        match c {
            Ordering::Equal => self.seq.cmp(&other.seq),
            _ => c
        }
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Entry) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Entry {}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

pub struct Transaction {
    pub to_regen: bool,
    pub prioritized_q: BinaryHeap<Entry>,
    pub entries: BTreeSet<Entry>,
    pub last_q: Vec<Box<Fn()>>,
    pub post_q: HashMap<i32,HandlerRefMut<Option<Transaction>>>
}

impl Transaction {

    pub fn new() -> Transaction {
        Transaction {
            to_regen: false,
            prioritized_q: BinaryHeap::new(),
            entries: BTreeSet::new(),
            last_q: Vec::new(),
            post_q: HashMap::new()
        }
    }

    pub fn run<F,A>(sodium_ctx: &mut SodiumCtx, code: F) -> A where F: Fn(&mut SodiumCtx)->A {
        let trans_was = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
        Transaction::start_if_necessary(sodium_ctx);
        let r = code(sodium_ctx);
        if trans_was.deref().borrow().is_none() {
            let trans =
                sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            trans.deref().borrow_mut().deref_mut().as_mut().unwrap().close(sodium_ctx);
        }
        sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_was);
        r
    }

    pub fn run_trans(sodium_ctx: &mut SodiumCtx, code: HandlerRefMut<Transaction>) {
        let trans_was = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
        Transaction::start_if_necessary(sodium_ctx);
        {
            let trans: Rc<RefCell<Option<Transaction>>> = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            let trans2: &RefCell<Option<Transaction>> = trans.deref();
            let mut trans3: RefMut<Option<Transaction>> = trans2.borrow_mut();
            let trans4: &mut Option<Transaction> = trans3.deref_mut();
            match trans4 {
                &mut Some(ref mut trans5) => {
                    let trans6: &mut Transaction = trans5;
                    code.run(sodium_ctx, trans6);
                },
                &mut None => ()
            }
        }
        if trans_was.deref().borrow().is_none() {
            let trans =
                sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            trans.deref().borrow_mut().deref_mut().as_mut().unwrap().close(sodium_ctx);
        }
        sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_was);
    }

    pub fn on_start<F>(sodium_ctx: &mut SodiumCtx, r: F) where F: Fn() + 'static {
        sodium_ctx.with_data_mut(|ctx| ctx.on_start_hooks.push(Box::new(r)));
    }

    pub fn apply<F,A>(sodium_ctx: &mut SodiumCtx, code: F) -> A where F: FnOnce(&mut SodiumCtx, &mut Transaction)->A {
        let trans_was = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
        Transaction::start_if_necessary(sodium_ctx);
        let r;
        {
            let trans: Rc<RefCell<Option<Transaction>>> = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            let trans2: &RefCell<Option<Transaction>> = trans.deref();
            let mut trans3: RefMut<Option<Transaction>> = trans2.borrow_mut();
            let trans4: &mut Option<Transaction> = trans3.deref_mut();
            r = code(sodium_ctx, trans4.as_mut().unwrap());
        }
        if trans_was.deref().borrow().is_none() {
            let trans =
                sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            trans.deref().borrow_mut().deref_mut().as_mut().unwrap().close(sodium_ctx);
        }
        sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_was);
        r
    }

    pub fn start_if_necessary(sodium_ctx: &mut SodiumCtx) {
        if sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.deref().borrow().is_none()) {
            if !sodium_ctx.with_data_ref(|ctx| ctx.running_on_start_hooks) {
                sodium_ctx.with_data_mut(|ctx| ctx.running_on_start_hooks = true);
                let mut on_start_hooks = Vec::new();
                sodium_ctx.with_data_mut(|ctx| swap(&mut ctx.on_start_hooks, &mut on_start_hooks));
                for start_hook in &on_start_hooks {
                    start_hook();
                }
                sodium_ctx.with_data_mut(|ctx| swap(&mut ctx.on_start_hooks, &mut on_start_hooks));
                sodium_ctx.with_data_mut(|ctx| ctx.running_on_start_hooks = false);
            }
            sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = Rc::new(RefCell::new(
                Some(Transaction::new())
            )));
        }
    }

    pub fn prioritized(&mut self, sodium_ctx: &mut SodiumCtx, rank: Rc<RefCell<IsNode>>, action: HandlerRefMut<Transaction>) {
        let e = Entry::new(sodium_ctx, rank, action);
        self.prioritized_q.push(e.clone());
        self.entries.insert(e);
    }

    pub fn last<F>(&mut self, action: F) where F: Fn() + 'static {
        self.last_q.push(Box::new(action));
    }

    pub fn post_(&mut self, child_ix: i32, action: HandlerRefMut<Option<Transaction>>) {
        let existing_op = self.post_q.remove(&child_ix);
        let neu = match existing_op {
            Some(existing) => HandlerRefMut::new(
                move |sodium_ctx, transaction_op| {
                    existing.run(sodium_ctx, transaction_op);
                    action.run(sodium_ctx, transaction_op);
                }
            ),
            None => action
        };
        self.post_q.insert(child_ix, neu);
    }

    pub fn post<F>(&mut self, sodium_ctx: &mut SodiumCtx, action: F) where F: Fn() + 'static {
        let action2 = Rc::new(action);
        Transaction::run_trans(
            sodium_ctx,
            HandlerRefMut::new(
                move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction| {
                    let action3 = action2.clone();
                    trans.post_(
                        -1,
                        HandlerRefMut::new(
                            move |_, _| {
                                action3();
                            }
                        )
                    );
                }
            )
        );
    }

    pub fn check_regen(&mut self) {
        if self.to_regen {
            self.to_regen = false;
            self.prioritized_q.clear();
            for e in &self.entries {
                self.prioritized_q.push(e.clone());
            }
        }
    }

    pub fn close(&mut self, sodium_ctx: &mut SodiumCtx) {
        loop {
            self.check_regen();
            match self.prioritized_q.pop() {
                Some(e) => {
                    self.entries.remove(&e);
                    e.action.run(sodium_ctx, self);
                },
                None => break
            }
        }
        for action in &self.last_q {
            action();
        }
        self.last_q.clear();
        while !self.post_q.is_empty() {
            for (ix, h) in self.post_q.drain() {
                let parent = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
                if ix >= 0 {
                    let trans_op = Rc::new(RefCell::new(Some(Transaction::new())));
                    sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_op.clone());
                    {
                        let tmp1: &RefCell<Option<Transaction>> = trans_op.deref();
                        let mut tmp2: RefMut<Option<Transaction>> = tmp1.borrow_mut();
                        let tmp3: &mut Option<Transaction> = tmp2.borrow_mut();
                        h.run(sodium_ctx, tmp3);
                        match tmp3 {
                            &mut Some(ref mut trans) => trans.close(sodium_ctx),
                            &mut None => ()
                        }
                    }
                } else {
                    sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = Rc::new(RefCell::new(None)));
                    h.run(sodium_ctx, &mut None);
                }
                sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = parent);
            }
        }
    }
}
