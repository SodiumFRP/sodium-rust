use sodium::HandlerRefMut;
use sodium::IsNode;
use sodium::Node;
use sodium::SodiumCtx;
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
use std::mem::replace;
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
    pub data: Rc<RefCell<TransactionData>>
}

impl Clone for Transaction {
    fn clone(&self) -> Self {
        Transaction {
            data: self.data.clone()
        }
    }
}

pub struct TransactionData {
    pub to_regen: bool,
    pub prioritized_q: BinaryHeap<Entry>,
    pub entries: BTreeSet<Entry>,
    pub last_q: Vec<Box<Fn()>>,
    pub post_q: HashMap<i32,HandlerRefMut<Option<Transaction>>>
}

impl Transaction {

    pub fn with_data_ref<F,A>(&self, f: F)->A where F: FnOnce(&TransactionData)->A {
        f(&self.data.borrow())
    }

    pub fn with_data_mut<F,A>(&self, f: F)->A where F: FnOnce(&mut TransactionData)->A {
        f(&mut self.data.borrow_mut())
    }

    pub fn new() -> Transaction {
        Transaction {
            data: Rc::new(RefCell::new(TransactionData {
                to_regen: false,
                prioritized_q: BinaryHeap::new(),
                entries: BTreeSet::new(),
                last_q: Vec::new(),
                post_q: HashMap::new()
            }))
        }
    }

    pub fn run<F,A>(sodium_ctx: &mut SodiumCtx, code: F) -> A where F: Fn(&mut SodiumCtx)->A {
        let trans_was = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
        Transaction::start_if_necessary(sodium_ctx);
        let r = code(sodium_ctx);
        if trans_was.is_none() {
            let trans =
                sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            trans.unwrap().close(sodium_ctx);
        }
        sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_was);
        r
    }

    pub fn run_trans(sodium_ctx: &mut SodiumCtx, code: HandlerRefMut<Transaction>) {
        let trans_was = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
        Transaction::start_if_necessary(sodium_ctx);
        {
            let mut trans: Option<Transaction> = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            match &mut trans {
                &mut Some(ref mut trans5) => {
                    let trans6: &mut Transaction = trans5;
                    code.run(sodium_ctx, trans6);
                },
                &mut None => ()
            }
        }
        if trans_was.is_none() {
            let mut trans: Option<Transaction> = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            match &mut trans {
                &mut Some(ref mut trans5) => {
                    let trans6: &mut Transaction = trans5;
                    trans6.close(sodium_ctx);
                },
                &mut None => ()
            }
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
            let mut trans: Option<Transaction> = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            r = code(sodium_ctx, &mut trans.unwrap());
        }
        if trans_was.is_none() {
            let mut trans: Option<Transaction> = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
            match &mut trans {
                &mut Some(ref mut trans5) => {
                    let trans6: &mut Transaction = trans5;
                    trans6.close(sodium_ctx);
                },
                &mut None => ()
            }
        }
        sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_was);
        r
    }

    pub fn start_if_necessary(sodium_ctx: &mut SodiumCtx) {
        if sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.is_none()) {
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
            sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = Some(Transaction::new()));
        }
    }

    pub fn prioritized(&mut self, sodium_ctx: &mut SodiumCtx, rank: Rc<RefCell<IsNode>>, action: HandlerRefMut<Transaction>) {
        let e = Entry::new(sodium_ctx, rank, action);
        self.with_data_mut(|data| {
            data.prioritized_q.push(e.clone());
            data.entries.insert(e);
        });
    }

    pub fn last<F>(&mut self, action: F) where F: Fn() + 'static {
        self.with_data_mut(|data| {
            data.last_q.push(Box::new(action));
        });
    }

    pub fn post_(&mut self, child_ix: i32, action: HandlerRefMut<Option<Transaction>>) {
        self.with_data_mut(move |data| {
            let existing_op = data.post_q.remove(&child_ix);
            let neu = match existing_op {
                Some(existing) => HandlerRefMut::new(
                    move |sodium_ctx, transaction_op| {
                        existing.run(sodium_ctx, transaction_op);
                        action.run(sodium_ctx, transaction_op);
                    }
                ),
                None => action
            };
            data.post_q.insert(child_ix, neu);
        });
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
        self.with_data_mut(|data| {
            if data.to_regen {
                data.to_regen = false;
                data.prioritized_q.clear();
                for e in &data.entries {
                    data.prioritized_q.push(e.clone());
                }
            }
        });
    }

    pub fn close(&mut self, sodium_ctx: &mut SodiumCtx) {
        loop {
            self.check_regen();
            match self.with_data_mut(|data| data.prioritized_q.pop()) {
                Some(e) => {
                    {
                        let e = e.clone();
                        self.with_data_mut(move |data| data.entries.remove(&e));
                    }
                    e.action.run(sodium_ctx, self);
                },
                None => break
            }
        }
        let mut last_q = Vec::new();
        self.with_data_mut(|data| swap(&mut last_q, &mut data.last_q));
        for action in last_q {
            action();
        }
        while self.with_data_ref(|data| !data.post_q.is_empty()) {
            let mut post_q = HashMap::new();
            self.with_data_mut(|data| swap(&mut post_q, &mut data.post_q));
            for (ix, h) in post_q.drain() {
                let parent = sodium_ctx.with_data_ref(|ctx| ctx.current_transaction_op.clone());
                if ix >= 0 {
                    let trans_op = Some(Transaction::new());
                    sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = trans_op.clone());
                    {
                        let mut tmp1: Option<Transaction> = trans_op.clone();
                        h.run(sodium_ctx, &mut tmp1);
                        match &mut tmp1 {
                            &mut Some(ref mut trans) => trans.close(sodium_ctx),
                            &mut None => ()
                        }
                    }
                } else {
                    sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = None);
                    h.run(sodium_ctx, &mut None);
                }
                sodium_ctx.with_data_mut(|ctx| ctx.current_transaction_op = parent);
            }
        }
    }
}
