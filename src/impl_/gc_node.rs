use std::cell::Cell;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

pub type Tracer<'a> = dyn FnMut(&GcNode) + 'a;

pub type Trace = dyn Fn(&mut Tracer) + Send + Sync;

#[derive(PartialEq,Eq,Clone,Copy)]
enum Color {
    Black,
    Gray,
    Purple,
    White
}

#[derive(Clone)]
pub struct GcNode {
    id: u32,
    name: String,
    gc_ctx: GcCtx,
    data: Arc<GcNodeData>
}

struct GcNodeData {
    freed: Cell<bool>,
    ref_count: Cell<u32>,
    ref_count_adj: Cell<u32>,
    visited: Cell<bool>,
    color: Cell<Color>,
    buffered: Cell<bool>,
    deconstructor: RwLock<Box<dyn Fn()+Send+Sync>>,
    trace: RwLock<Box<Trace>>
}

unsafe impl Send for GcNodeData {}
unsafe impl Sync for GcNodeData {}

#[derive(Clone)]
pub struct GcCtx {
    data: Arc<Mutex<GcCtxData>>
}

struct GcCtxData {
    next_id: u32,
    roots: Vec<GcNode>,
    to_be_freed: Vec<GcNode>
}

impl Default for GcCtx {
    fn default() -> GcCtx {
        GcCtx::new()
    }
}

impl GcCtx {
    pub fn new() -> GcCtx {
        GcCtx {
            data: Arc::new(Mutex::new(GcCtxData {
                next_id: 0,
                roots: Vec::new(),
                to_be_freed: Vec::new()
            }))
        }
    }

    fn with_data<R,K:FnOnce(&mut GcCtxData)->R>(&self, k: K) -> R {
        let mut l = self.data.lock();
        let data = l.as_mut().unwrap();
        k(data)
    }

    pub fn make_id(&self) -> u32 {
        self.with_data(|data: &mut GcCtxData| {
            let id = data.next_id;
            data.next_id = data.next_id + 1;
            id
        })
    }

    pub fn add_possible_root(&self, node: GcNode) {
        self.with_data(|data: &mut GcCtxData| data.roots.push(node));
    }

    pub fn collect_cycles(&self) {
        loop {
            trace!("start: collect_cycles");
            self.mark_roots();
            self.scan_roots();
            self.collect_roots();
            trace!("end: collect_cycles");
            let bail = self.with_data(|data: &mut GcCtxData| data.roots.is_empty() && data.to_be_freed.is_empty());
            if bail {
                break;
            }
        }
    }

    fn mark_roots(&self) {
        trace!("start: mark_roots");
        let mut old_roots: Vec<GcNode> = Vec::new();
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut old_roots, &mut data.roots)
        );
        self.display_graph(&old_roots);
        let mut new_roots: Vec<GcNode> = Vec::new();
        for root in &old_roots {
            self.reset_ref_count_adj(root);
        }
        for root in old_roots {
            let color = root.data.color.get();
            if color == Color::Purple {
                self.mark_gray(&root);
                new_roots.push(root);
            } else {
                root.data.buffered.set(false);
                if root.data.color.get() == Color::Black && root.data.ref_count.get() == 0 && !root.data.freed.get() {
                    self.with_data(
                        |data: &mut GcCtxData|
                            data.to_be_freed.push(root)
                    );
                }
            }
        }
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut new_roots, &mut data.roots)
        );
        trace!("end: mark_roots");
    }

    fn display_graph(&self, roots: &Vec<GcNode>) {
        let mut stack = Vec::new();
        let mut visited: HashSet<*const GcNodeData> = HashSet::new();
        let mut show_names_for = Vec::new();
        for root in roots {
            stack.push(root.clone());
        }
        trace!("-- start of graph drawing --");
        loop {
            let next_op = stack.pop();
            if next_op.is_none() {
                break;
            }
            let next = next_op.unwrap();
            {
                let next_ptr: &GcNodeData = &next.data;
                let next_ptr: *const GcNodeData = next_ptr;
                if visited.contains(&next_ptr) {
                    continue;
                }
                visited.insert(next_ptr);
            }
            show_names_for.push(next.clone());
            let mut line: String = format!("id {}, ref_count {}: ", next.id, next.data.ref_count.get());
            let mut first: bool = true;
            next.trace(|t| {
                if first {
                    first = false;
                } else {
                    line.push(',');
                }
                line.push_str(&format!("{}", t.id));
                stack.push(t.clone());
            });
            trace!("{}", line);
        }
        trace!("node names:");
        for next in show_names_for {
            trace!("{}: {}", next.id, next.name);
        }
        trace!("-- end of graph drawing --");
    }

    fn mark_gray(&self, s: &GcNode) {
        if s.data.color.get() == Color::Gray {
            return;
        }
        s.data.color.set(Color::Gray);

        s.trace(&mut |t: &GcNode| {
            trace!("mark_gray: gc node {} dec ref count", t.id);
            t.data.ref_count_adj.set(t.data.ref_count_adj.get() + 1);
            if t.data.ref_count_adj.get() > t.data.ref_count.get() {
                panic!("ref count adj was larger than ref count for node {} ({}) (ref adj {}) (ref cnt {})", t.id, t.name, t.data.ref_count_adj.get(), t.data.ref_count.get());
            }
            self.mark_gray(t);
        });
    }

    fn scan_roots(&self) {
        trace!("start: scan_roots");
        let mut roots = Vec::new();
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut roots, &mut data.roots)
        );
        for root in &roots {
            self.scan(root);
        }
        for root in &roots {
            self.reset_ref_count_adj(root);
        }
        self.with_data(
            |data: &mut GcCtxData|
                std::mem::swap(&mut roots, &mut data.roots)
        );
        trace!("end: scan_roots");
    }

    fn scan(&self, s: &GcNode) {
        if s.data.color.get() != Color::Gray {
            return;
        }
        if s.data.ref_count_adj.get() == s.data.ref_count.get() {
            s.data.color.set(Color::White);
            trace!("scan: gc node {} became white", s.id);
            s.trace(|t| {
                self.scan(t);
            });
        } else {
            self.scan_black(s);
        }
    }

    fn reset_ref_count_adj(&self, s: &GcNode) {
        if s.data.visited.get() {
            return;
        }
        s.data.visited.set(true);
        s.data.ref_count_adj.set(0);
        s.trace(|t| {
            self.reset_ref_count_adj(t);
        });
        s.data.visited.set(false);
    }

    fn scan_black(&self, s: &GcNode) {
        s.data.color.set(Color::Black);
        trace!("scan: gc node {} became black", s.id);
        let this = self.clone();
        s.trace(|t| {
            if t.data.color.get() != Color::Black {
                this.scan_black(t);
            }
        });
    }

    fn collect_roots(&self) {
        let mut white = Vec::new();
        let mut roots = Vec::new();
        self.with_data(|data: &mut GcCtxData| roots.append(&mut data.roots));
        for root in &roots {
            root.data.buffered.set(false);
            self.collect_white(root, &mut white);
        }
        for i in &white {
            if !i.data.freed.get() {
                trace!("collect_roots: freeing white node {} ({})", i.id, i.name);
                i.free();
                self.with_data(|data: &mut GcCtxData| data.roots.retain(|root: &GcNode| root.id != i.id));
            }
        }
        let mut to_be_freed = Vec::new();
        self.with_data(|data: &mut GcCtxData| to_be_freed.append(&mut data.to_be_freed));
        for i in &to_be_freed {
            if !i.data.freed.get() {
                trace!("collect_roots: freeing to_be_freed node {} ({})", i.id, i.name);
                i.free();
                self.with_data(|data: &mut GcCtxData| data.roots.retain(|root: &GcNode| root.id != i.id));
            }
        }
        for i in white {
            if i.ref_count() != 0 {
                panic!("freed node ref count did not drop to zero for node {} ({})", i.id, i.name);
            }
        }
        for i in to_be_freed {
            if i.ref_count() != 0 {
                panic!("freed node ref count did not drop to zero for node {} ({})", i.id, i.name);
            }
        }
    }

    fn collect_white(&self, s: &GcNode, white: &mut Vec<GcNode>) {
        if s.data.color.get() == Color::White /*&& !s.data.buffered.get()*/ {
            s.data.color.set(Color::Black);
            s.trace(|t| {
                self.collect_white(t, white);
            });
            trace!("collect_white: gc node {} added to white list", s.id);
            white.push(s.clone());
        }
    }
}

impl GcNode {
    pub fn new<
        NAME: ToString,
        DECONSTRUCTOR: 'static + Fn() + Send + Sync,
        TRACE: 'static + Fn(&mut Tracer) + Send + Sync
    >(
        gc_ctx: &GcCtx,
        name: NAME,
        deconstructor: DECONSTRUCTOR,
        trace: TRACE
    ) -> GcNode {
        GcNode {
            id: gc_ctx.make_id(),
            name: name.to_string(),
            gc_ctx: gc_ctx.clone(),
            data: Arc::new(GcNodeData {
                freed: Cell::new(false),
                ref_count: Cell::new(1),
                ref_count_adj: Cell::new(0),
                visited: Cell::new(false),
                color: Cell::new(Color::Black),
                buffered: Cell::new(false),
                deconstructor: RwLock::new(Box::new(deconstructor)),
                trace: RwLock::new(Box::new(trace))
            })
        }
    }

    pub fn ref_count(&self) -> u32 {
        self.data.ref_count.get()
    }

    pub fn inc_ref_if_alive(&self) -> bool {
        if self.ref_count() != 0 && !self.data.freed.get() {
            self.data.ref_count.set(self.data.ref_count.get() + 1);
            self.data.color.set(Color::Black);
            true
        } else {
            false
        }
    }

    pub fn inc_ref(&self) {
        if self.data.freed.get() {
            panic!("gc_node {} inc_ref on freed node ({})", self.id, self.name);
        }
        self.data.ref_count.set(self.data.ref_count.get() + 1);
        self.data.color.set(Color::Black);
    }

    pub fn dec_ref(&self) {
        if self.data.ref_count.get() == 0 {
            return;
        }
        self.data.ref_count.set(self.data.ref_count.get() - 1);
        if self.data.ref_count.get() == 0 {
            self.release();
        } else {
            self.possible_root();
        }
    }

    pub fn release(&self) {
        self.data.color.set(Color::Black);
        if !self.data.buffered.get() {
            trace!("release: freeing gc_node {} ({})", self.id, self.name);
            self.free();
        }
    }

    pub fn possible_root(&self) {
        if self.data.color.get() != Color::Purple {
            self.data.color.set(Color::Purple);
            if !self.data.buffered.get() {
                self.data.buffered.set(true);
                self.gc_ctx.add_possible_root(self.clone());
            }
        }
    }

    pub fn free(&self) {
        self.data.freed.set(true);
        let mut tmp: Box<dyn Fn() + Send + Sync + 'static> = Box::new(|| {});
        {
            let mut deconstructor = self.data.deconstructor.write().unwrap();
            std::mem::swap(&mut *deconstructor, &mut tmp);
        }
        tmp();
        let mut trace = self.data.trace.write().unwrap();
        *trace = Box::new(|_tracer: &mut Tracer| {});
    }

    pub fn trace<TRACER: FnMut(&GcNode)>(&self, mut tracer: TRACER) {
        let trace = self.data.trace.read().unwrap();
        trace(&mut tracer);
    }
}
