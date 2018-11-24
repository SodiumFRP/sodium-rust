use sodium::impl_::Dep;
use sodium::impl_::SodiumCtx;
use sodium::impl_::WeakSodiumCtx;
use sodium::gc::Finalize;
use sodium::gc::Gc;
use sodium::gc::GcDep;
use sodium::gc::GcWeak;
use sodium::gc::Trace;
use std::boxed::Box;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::cmp::PartialEq;
use std::cmp::PartialOrd;
use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;
use std::rc::Rc;
use std::vec::Vec;

pub struct Node {
    data: Gc<UnsafeCell<NodeData>>
}

pub struct WeakNode {
    data: GcWeak<UnsafeCell<NodeData>>
}

pub struct NodeData {
    id: u32,
    rank: u32,
    update: Box<FnMut()->bool>,
    update_dependencies: Vec<Dep>,
    dependencies: Vec<Node>,
    dependents: Vec<WeakNode>,
    cleanup: Box<FnMut()>,
    sodium_ctx: SodiumCtx
}

impl Node {
    pub fn new<UPDATE: FnMut()->bool + 'static, CLEANUP: FnMut() + 'static>(
        sodium_ctx: &SodiumCtx,
        mut update: UPDATE,
        update_dependencies: Vec<Dep>,
        dependencies: Vec<Node>,
        mut cleanup: CLEANUP,
        desc: String
    ) -> Node {
        let id = sodium_ctx.new_id();
        sodium_ctx.inc_node_count();
        let mut rank = 0;
        for dependency in &dependencies {
            let dependency = unsafe { &*(*dependency.data).get() };
            if rank <= dependency.rank {
                rank = dependency.rank + 1;
            }
        }
        let update2;
        {
            let sodium_ctx = sodium_ctx.clone();
            update2 = move || {
                sodium_ctx.inc_callback_depth();
                let result = update();
                sodium_ctx.dec_callback_depth();
                result
            };
        }
        let self_: Rc<UnsafeCell<Option<WeakNode>>> = Rc::new(UnsafeCell::new(None));
        let cleanup2;
        {
            let self_ = self_.clone();
            cleanup2 = move || {
                cleanup();
                let self_ = unsafe { &mut *(*self_).get() };
                match self_ {
                    Some(ref mut self_) => {
                        match self_.upgrade() {
                            Some(self_2) => {
                                let self_ = unsafe { &*(*self_2.data).get() };
                                self_.dependencies.iter().for_each(|dependency| {
                                    let dependency = unsafe { &mut *(*dependency.data).get() };
                                    dependency.dependents.retain(|dependent| {
                                        match dependent.upgrade() {
                                            Some(dependent) => {
                                                let dependent = unsafe { &*(*dependent.data).get() };
                                                dependent.id != self_.id
                                            },
                                            None => false
                                        }
                                    });
                                });
                            },
                            None => ()
                        }
                    },
                    None => ()
                }
            };
        }
        let mut gc_ctx = sodium_ctx.gc_ctx();
        let node = Node {
            data: gc_ctx.new_gc_with_desc(UnsafeCell::new(
                NodeData {
                    id,
                    rank,
                    update: Box::new(update2),
                    update_dependencies,
                    dependencies: dependencies.clone(),
                    dependents: Vec::new(),
                    cleanup: Box::new(cleanup2),
                    sodium_ctx: sodium_ctx.clone()
                }
            ), desc)
        };
        unsafe {
            *(*self_).get() = Some(node.downgrade());
        };
        let weak_node = node.downgrade();
        for dependency in &dependencies {
            let dependency = unsafe { &mut *(*dependency.data).get() };
            dependency.dependents.push(weak_node.clone());
        }
        node
    }

    pub fn set_update<UPDATE: FnMut()->bool + 'static>(&self, update: UPDATE, update_deps: Vec<Dep>) {
        let data = unsafe { &mut *(*self.data).get() };
        data.update = Box::new(update);
        data.update_dependencies = update_deps;
    }

    pub fn add_update_deps(&self, update_deps: Vec<Dep>) {
        let data = unsafe { &mut *(*self.data).get() };
        for dep in update_deps {
            data.update_dependencies.push(dep);
        }
    }

    pub fn remove_all_dependencies(&self) {
        let data = unsafe { &mut *(*self.data).get() };
        let self_id = data.id.clone();
        for dependency in &data.dependencies {
            {
                let dependency = unsafe { &mut *(*dependency.data).get() };
                dependency.dependents.retain(|weak_node| {
                    match weak_node.upgrade() {
                        Some(node2) => {
                            let node2_data = unsafe { &mut *(*node2.data).get() };
                            node2_data.id != self_id
                        },
                        None => false
                    }
                });
            }
        }
        data.dependencies.clear();
    }

    pub fn add_dependencies(&self, dependencies: Vec<Node>) {
        let data = unsafe { &mut *(*self.data).get() };
        let weak_node = self.downgrade();
        for dependency in dependencies {
            {
                let dependency = unsafe { &mut *(*dependency.data).get() };
                dependency.dependents.push(weak_node.clone());
            }
            data.dependencies.push(dependency);
        }
    }

    pub fn to_dep(&self) -> Dep {
        Dep {
            gc_dep: self.data.to_dep()
        }
    }

    pub fn mark_dependents_dirty(&self) {
        let self_ = unsafe { &*(*self).data.get() };
        self_.dependents.iter().for_each(|dependent| {
            dependent.upgrade().iter().for_each(|dependent| {
                dependent.mark_dirty();
            });
        });
    }

    pub fn mark_dirty(&self) {
        let self_ = unsafe { &*(*self).data.get() };
        let sodium_ctx = unsafe { &mut *(*self_.sodium_ctx.data).get() };
        if !sodium_ctx.to_be_updated_set.contains(self) {
            sodium_ctx.to_be_updated.push(self.clone());
            sodium_ctx.to_be_updated_set.insert(self.clone());
        }
    }

    pub fn undirty(&self) {
        let self_ = unsafe { &*(*self).data.get() };
        let sodium_ctx = unsafe { &mut *(*self_.sodium_ctx.data).get() };
        sodium_ctx.to_be_updated_set.remove(self);
        sodium_ctx.to_be_updated.clear();
        for node in &sodium_ctx.to_be_updated_set {
            sodium_ctx.to_be_updated.push(node.clone());
        }
    }

    pub fn is_dirty(&self) -> bool {
        let self_ = unsafe { &*(*self).data.get() };
        let sodium_ctx = unsafe { &mut *(*self_.sodium_ctx.data).get() };
        sodium_ctx.to_be_updated_set.contains(self)
    }

    pub fn rank(&self) -> u32 {
        let data = unsafe { &*(*self.data).get() };
        data.rank.clone()
    }

    pub fn ensure_bigger_than(&self, rank: u32) {
        let sodium_ctx = self.sodium_ctx();
        let sodium_ctx = &sodium_ctx;
        sodium_ctx.schedule_update_sort();
        self.ensure_bigger_than2(rank, &mut HashSet::new());
    }

    fn ensure_bigger_than2(&self, rank: u32, visited: &mut HashSet<u32>) {
        let self_ = unsafe { &mut *(*self).data.get() };
        if visited.contains(&self_.id) {
            return;
        }
        visited.insert(self_.id);
        if self_.rank <= rank {
            return
        }
        let rank2 = rank + 1;
        self_.rank = rank2;
        self_.dependents.iter().for_each(|dependent| {
            dependent.upgrade().iter().for_each(|dependent| {
                dependent.ensure_bigger_than2(rank2, visited);
            });
        })
    }

    pub fn update(&self)->bool {
        let self_ = unsafe { &mut *(*self.data).get() };
        (self_.update)()
    }

    pub fn sodium_ctx(&self) -> SodiumCtx {
        let self_ = unsafe { &*(*self.data).get() };
        self_.sodium_ctx.clone()
    }

    pub fn downgrade(&self) -> WeakNode {
        WeakNode {
            data: self.data.downgrade()
        }
    }
}

impl Drop for NodeData {
    fn drop(&mut self) {
        self.sodium_ctx.dec_node_count();
    }
}

impl WeakNode {
    pub fn upgrade(&self) -> Option<Node> {
        self.data.upgrade().map(|data| Node { data } )
    }
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Node {
            data: self.data.clone()
        }
    }
}

impl Clone for WeakNode {
    fn clone(&self) -> Self {
        WeakNode {
            data: self.data.clone()
        }
    }
}

impl Trace for Node {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        f(&self.data.to_dep());
    }
}

impl Finalize for Node {
    fn finalize(&mut self) {
    }
}

impl Trace for NodeData {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        self.dependencies.trace(f);
        self.update_dependencies.iter().for_each(|update_dep| f(&update_dep.gc_dep));
    }
}

impl Finalize for NodeData {
    fn finalize(&mut self) {
        (self.cleanup)();
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Node) -> Ordering {
        let self_ = unsafe { &*(*self).data.get() };
        let other = unsafe { &*(*other).data.get() };
        self_.rank.cmp(&other.rank).reverse()
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Node) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Node) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let data = unsafe { &*(*self.data).get() };
        data.id.hash(state);
    }
}
