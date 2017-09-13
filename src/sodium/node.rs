use sodium::SodiumCtx;
use sodium::Transaction;
use sodium::TransactionHandlerRef;
use sodium::WeakTransactionHandlerRef;
use std::any::Any;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::cell::Ref;
use std::cell::RefMut;
use std::cmp::Ordering;
use std::cmp::PartialEq;
use std::cmp::PartialOrd;
use std::collections::HashSet;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::rc::Rc;
use std::rc::Weak;

pub trait HasNode {
    fn node_ref(&self) -> &Node;
    fn node_mut(&mut self) -> &mut Node;
}

pub struct Node {
    pub id: u32,
    pub rank: u64,
    pub listeners: Vec<Target>
}

impl Node {
    pub fn new(sodium_ctx: &mut SodiumCtx, rank: u64) -> Node {
        Node {
            id: sodium_ctx.new_id(),
            rank: rank,
            listeners: Vec::new()
        }
    }

    pub fn new_(id: u32, rank: u64) -> Node {
        Node {
            id: id,
            rank: rank,
            listeners: Vec::new()
        }
    }
}

pub struct Target {
    pub id: u32,
    pub node: Rc<RefCell<HasNode>>,
    // action here is really a strong reference to a weak reference, meaning it is still a weak
    // reference overall. This had to be done so we can use "Any" here.
    pub action: TransactionHandlerRef<Any>
}

impl Clone for Target {
    fn clone(&self) -> Self {
        Target {
            id: self.id.clone(),
            node: self.node.clone(),
            action: self.action.clone()
        }
    }
}

impl HasNode for Node {
    fn node_ref(&self) -> &Node {
        self
    }
    fn node_mut(&mut self) -> &mut Node {
        self
    }
}

impl Target {
    pub fn new<A:'static>(sodium_ctx: &mut SodiumCtx, node: Rc<RefCell<HasNode>>, action: TransactionHandlerRef<A>) -> Target {
        let action = action.downgrade();
        Target {
            id: sodium_ctx.new_id(),
            node: node,
            action: TransactionHandlerRef::new(
                move |sodium_ctx: &mut SodiumCtx, trans: &mut Transaction, a: &Any| {
                    match action.upgrade() {
                        Some(action) => {
                            match a.downcast_ref::<A>() {
                                Some(a) => action.run(sodium_ctx, trans, a),
                                None => ()
                            }
                        },
                        None => ()
                    }
                }
            )
        }
    }
}

impl HasNode {
    pub fn link_to<A:'static>(&mut self, sodium_ctx: &mut SodiumCtx, target: Rc<RefCell<HasNode>>, action: TransactionHandlerRef<A>) -> (Target, bool) {
        let changed;
        {
            let target2: &RefCell<HasNode> = target.borrow();
            let mut target3: RefMut<HasNode> = target2.borrow_mut();
            let target4: &mut HasNode = target3.deref_mut();
            changed = target4.ensure_bigger_than(self.node_ref().rank, &mut HashSet::new());
        }
        let t = Target::new(sodium_ctx, target, action);
        self.node_mut().listeners.push(t.clone());
        (t, changed)
    }

    pub fn unlink_to(&mut self, target: &Target) {
        let id = target.id.clone();
        self.node_mut().listeners.retain(
            move |target| {
                let id2 = target.id.clone();
                id != id2
            }
        )
    }

    pub fn ensure_bigger_than(&mut self, limit: u64, visited: &mut HashSet<u32>) -> bool {
        let listeners;
        let rank;
        {
            let self_ = self.node_mut();
            if self_.rank > limit || visited.contains(&self_.id) {
                return false;
            }
            visited.insert(self_.id.clone());
            self_.rank = limit + 1;
            listeners = self_.listeners.clone();
            rank = self_.rank.clone();
        }
        for target in listeners {
            let node: &RefCell<HasNode> = target.node.borrow();
            let mut node2: RefMut<HasNode> = node.borrow_mut();
            let node3: &mut HasNode = node2.deref_mut();
            node3.ensure_bigger_than(rank, visited);
        }
        return true;
    }
}

impl Ord for HasNode + 'static {
    fn cmp(&self, other: &(HasNode + 'static)) -> Ordering {
        self.node_ref().rank.cmp(&other.node_ref().rank)
    }
}

impl PartialOrd for HasNode + 'static {
    fn partial_cmp(&self, other: &(HasNode + 'static)) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for HasNode {}

impl PartialEq for HasNode + 'static {
    fn eq(&self, other: &(HasNode + 'static)) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
