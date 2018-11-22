/*
 * A Pure Reference Counting Garbage Collector
 * DAVID F. BACON, CLEMENT R. ATTANASIO, V.T. RAJAN, STEPHEN E. SMITH
 */

use std::ptr;
use std::ops::Deref;
use std::ops::DerefMut;
use std::mem::transmute;
use std::mem::swap;
use std::cell::Cell;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::rc::Rc;
use std::hash::Hash;
use std::collections::{BinaryHeap, BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque};

pub struct GcCtx {
    data: Rc<RefCell<GcCtxData>>
}

impl Clone for GcCtx {
    fn clone(&self) -> Self {
        GcCtx {
            data: self.data.clone()
        }
    }
}

struct GcCtxData {
    roots: Vec<*mut Node>,
    collecting_cycles: bool,
    to_be_freed: Vec<*mut Node>
}

pub struct GcDep {
    ctx: GcCtx,
    node: *mut Node
}

impl Clone for GcDep {
    fn clone(&self) -> Self {
        let n: &mut Node = unsafe { &mut *self.node };
        n.weak = n.weak + 1;
        GcDep {
            ctx: self.ctx.clone(),
            node: self.node
        }
    }
}

impl Drop for GcDep {
    fn drop(&mut self) {
        let n: &mut Node = unsafe { &mut *self.node };
        n.weak = n.weak - 1;
    }
}

pub struct Gc<A: ?Sized> {
    ctx: GcCtx,
    value: *mut A,
    node: *mut Node
}

impl<A: ?Sized> Clone for Gc<A> {
    fn clone(&self) -> Self {
        self.ctx.increment(self.node);
        Gc {
            ctx: self.ctx.clone(),
            value: self.value,
            node: self.node
        }
    }
}

impl<A: ?Sized> Drop for Gc<A> {
    fn drop(&mut self) {
        self.ctx.decrement(self.node);
        self.ctx.collect_cycles();
    }
}

impl<A: ?Sized> Deref for Gc<A> {
    type Target = A;

    fn deref(&self) -> &A {
        unsafe { &*self.value }
    }
}

impl<A: ?Sized> Gc<A> {
    pub fn debug(&self) {
        let node = unsafe { &*self.node };
        node.debug();
    }

    pub fn to_dep(&self) -> GcDep {
        let n: &mut Node = unsafe { &mut *self.node };
        n.weak = n.weak + 1;
        GcDep {
            ctx: self.ctx.clone(),
            node: self.node
        }
    }

    pub fn strong_count(&self) -> i32 {
        let node = unsafe { &*self.node };
        node.strong
    }

    pub fn weak_count(&self) -> i32 {
        let node = unsafe { &*self.node };
        node.weak
    }

    pub fn downgrade(&self) -> GcWeak<A> {
        let node = unsafe { &mut *self.node };
        node.weak = node.weak + 1;
        GcWeak {
            ctx: self.ctx.clone(),
            node: self.node,
            value: self.value
        }
    }

    pub fn upcast<F,B:?Sized>(&self, f: F) -> Gc<B> where F: FnOnce(&A)->&B {
        let s = unsafe { &mut *self.node };
        s.strong = s.strong + 1;
        Gc {
            ctx: self.ctx.clone(),
            value: unsafe { transmute(f(&mut *self.value)) },
            node: self.node
        }
    }
}

pub struct GcWeak<A: ?Sized> {
    ctx: GcCtx,
    node: *mut Node,
    value: *mut A
}

impl<A: ?Sized> Clone for GcWeak<A> {
    fn clone(&self) -> Self {
        let node = unsafe { &mut *self.node };
        node.weak = node.weak + 1;
        GcWeak {
            ctx: self.ctx.clone(),
            node: self.node,
            value: self.value
        }
    }
}

impl<A: ?Sized> Drop for GcWeak<A> {
    fn drop(&mut self) {
        let node = unsafe { &mut *self.node };
        if node.weak > 0 {
            node.weak = node.weak - 1;
            if node.weak == 0 {
                unsafe { Box::from_raw(node); }
            }
        }
    }
}

impl<A: ?Sized> GcWeak<A> {
    pub fn upgrade(&self) -> Option<Gc<A>> {
        let node = unsafe { &mut *self.node };
        if node.strong == 0 {
            None
        } else {
            node.strong = node.strong + 1;
            Some(
                Gc {
                    ctx: self.ctx.clone(),
                    node: self.node,
                    value: self.value
                }
            )
        }
    }
}

pub trait Trace {
    fn trace(&self, f: &mut FnMut(&GcDep));
}

pub trait Finalize {
    fn finalize(&mut self) {
        // default impl: Do nothing.
    }
}

#[cfg(feature = "nightly")]
default impl<T> Trace for T {
    #[inline]
    default fn trace(&self, _tracer: &mut FnMut(&GcDep)) {
        // default impl: Do nothing.
    }
}

#[cfg(feature = "nightly")]
default impl<T> Finalize for T {
    #[inline]
    default fn finalize(&mut self) {
        // default impl: Do nothing.
    }
}

macro_rules! mk_empty_finalize_trace {
    ($($T:ty),*) => {
        $(
            impl Finalize for $T {}
            impl Trace for $T {
                fn trace(&self, _tracer: &mut FnMut(&GcDep)) {
                }
            }
        )*
    }
}

mk_empty_finalize_trace![(), isize, usize, bool, i8, u8, i16, u16, i32,
    u32, i64, u64, f32, f64, char, String];

#[cfg(feature = "nightly")]
mk_empty_finalize_trace![i128, u128];

impl<A: Trace> Trace for Gc<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        f(&self.to_dep());
    }
}

impl<A: Trace> Trace for GcCell<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let self2: &A = unsafe { &*self.cell.get() };
        self2.trace(f);
    }
}

impl<A: Trace> Trace for Option<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        match self {
            &Some(ref a) => a.trace(f),
            &None => ()
        }
    }
}

impl<A: Trace, E: Trace> Trace for Result<A,E> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        match self {
            &Ok(ref x) => x.trace(tracer),
            &Err(ref x) => x.trace(tracer)
        }
    }
}

impl<A: Trace> Trace for Box<A> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        (**self).trace(tracer);
    }
}

impl<A: Trace> Trace for Vec<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        for a in self {
            a.trace(f);
        }
    }
}

impl <A: Ord + Trace> Trace for BinaryHeap<A> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for v in self.into_iter() {
            v.trace(tracer);
        }
    }
}

impl<K: Trace, V: Trace> Trace for BTreeMap<K,V> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for (k,v) in self {
            k.trace(tracer);
            v.trace(tracer);
        }
    }
}

impl<A: Trace> Trace for BTreeSet<A> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for v in self {
            v.trace(tracer);
        }
    }
}

impl<K: Eq + Hash + Trace, V: Trace> Trace for HashMap<K,V> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for (k,v) in self.iter() {
            k.trace(tracer);
            v.trace(tracer);
        }
    }
}

impl<A: Eq + Hash + Trace> Trace for HashSet<A> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for v in self.iter() {
            v.trace(tracer);
        }
    }
}

impl<A: Trace> Trace for LinkedList<A> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for v in self.iter() {
            v.trace(tracer);
        }
    }
}

impl<A: Trace> Trace for VecDeque<A> {
    fn trace(&self, tracer: &mut FnMut(&GcDep)) {
        for v in self.iter() {
            v.trace(tracer);
        }
    }
}

impl<A:Trace,B:Trace> Trace for (A,B) {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let &(ref a, ref b) = self;
        a.trace(f);
        b.trace(f);
    }
}

impl<A:Trace,B:Trace,C:Trace> Trace for (A,B,C) {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let &(ref a, ref b, ref c) = self;
        a.trace(f);
        b.trace(f);
        c.trace(f);
    }
}

impl<A:Trace,B:Trace,C:Trace,D:Trace> Trace for (A,B,C,D) {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let &(ref a, ref b, ref c, ref d) = self;
        a.trace(f);
        b.trace(f);
        c.trace(f);
        d.trace(f);
    }
}

impl<A:Trace,B:Trace,C:Trace,D:Trace,E:Trace> Trace for (A,B,C,D,E) {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let &(ref a, ref b, ref c, ref d, ref e) = self;
        a.trace(f);
        b.trace(f);
        c.trace(f);
        d.trace(f);
        e.trace(f);
    }
}

impl<A:Trace,B:Trace,C:Trace,D:Trace,E:Trace,F:Trace> Trace for (A,B,C,D,E,F) {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let &(ref a, ref b, ref c, ref d, ref e, ref f2) = self;
        a.trace(f);
        b.trace(f);
        c.trace(f);
        d.trace(f);
        e.trace(f);
        f2.trace(f);
    }
}

impl Trace for &'static str {
    fn trace(&self, _f: &mut FnMut(&GcDep)) {}
}

impl<A:Trace> Trace for UnsafeCell<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let val = unsafe { &*self.get() };
        val.trace(f);
    }
}

impl<A: Finalize> Finalize for Gc<A> {
    fn finalize(&mut self) {
        // Already handled by the finalize for A when Gc is finished,
        // do nothing here.
    }
}

impl<A: Finalize> Finalize for GcCell<A> {
    fn finalize(&mut self) {
        let self2: &mut A = unsafe { &mut *self.cell.get() };
        self2.finalize();
    }
}

impl<A: Finalize> Finalize for Option<A> {
    fn finalize(&mut self) {
        match self {
            &mut Some(ref mut a) => a.finalize(),
            &mut None => ()
        }
    }
}

impl<A: Finalize, E: Finalize> Finalize for Result<A,E> {
    fn finalize(&mut self) {
        match self {
            &mut Ok(ref mut a) => a.finalize(),
            &mut Err(ref mut a) => a.finalize()
        }
    }
}

impl<A: Finalize> Finalize for Box<A> {
    fn finalize(&mut self) {
        (**self).finalize();
    }
}

impl<A: Finalize> Finalize for Vec<A> {
    fn finalize(&mut self) {
        for a in self {
            a.finalize();
        }
    }
}

impl<A: Ord + Finalize> Finalize for BinaryHeap<A> {
    fn finalize(&mut self) {
        for mut a in self.drain() {
            a.finalize();
        }
    }
}

impl<K: Clone + Finalize, V: Finalize> Finalize for BTreeMap<K,V> {
    fn finalize(&mut self) {
        for (k, v) in self.iter_mut() {
            k.clone().finalize();
            v.finalize();
        }
    }
}

impl<A: Clone + Finalize> Finalize for BTreeSet<A> {
    fn finalize(&mut self) {
        for v in self.iter() {
            v.clone().finalize();
        }
    }
}

impl<K: Eq + Hash + Finalize + Clone, V: Finalize> Finalize for HashMap<K,V> {
    fn finalize(&mut self) {
        for (k,v) in self.iter_mut() {
            k.clone().finalize();
            v.finalize();
        }
    }
}

impl<A: Eq + Hash + Finalize + Clone> Finalize for HashSet<A> {
    fn finalize(&mut self) {
        for v in self.iter() {
            v.clone().finalize();
        }
    }
}

impl<A: Finalize> Finalize for LinkedList<A> {
    fn finalize(&mut self) {
        for v in self {
            v.finalize();
        }
    }
}

impl<A: Finalize> Finalize for VecDeque<A> {
    fn finalize(&mut self) {
        for v in self {
            v.finalize();
        }
    }
}

impl<A:Finalize,B:Finalize> Finalize for (A,B) {
    fn finalize(&mut self) {
        let &mut (ref mut a, ref mut b) = self;
        a.finalize();
        b.finalize();
    }
}

impl<A:Finalize,B:Finalize,C:Finalize> Finalize for (A,B,C) {
    fn finalize(&mut self) {
        let &mut (ref mut a, ref mut b, ref mut c) = self;
        a.finalize();
        b.finalize();
        c.finalize();
    }
}

impl<A:Finalize,B:Finalize,C:Finalize,D:Finalize> Finalize for (A,B,C,D) {
    fn finalize(&mut self) {
        let &mut (ref mut a, ref mut b, ref mut c, ref mut d) = self;
        a.finalize();
        b.finalize();
        c.finalize();
        d.finalize();
    }
}

impl<A:Finalize,B:Finalize,C:Finalize,D:Finalize,E:Finalize> Finalize for (A,B,C,D,E) {
    fn finalize(&mut self) {
        let &mut (ref mut a, ref mut b, ref mut c, ref mut d, ref mut e) = self;
        a.finalize();
        b.finalize();
        c.finalize();
        d.finalize();
        e.finalize();
    }
}

impl<A:Finalize,B:Finalize,C:Finalize,D:Finalize,E:Finalize,F:Finalize> Finalize for (A,B,C,D,E,F) {
    fn finalize(&mut self) {
        let &mut (ref mut a, ref mut b, ref mut c, ref mut d, ref mut e, ref mut f) = self;
        a.finalize();
        b.finalize();
        c.finalize();
        d.finalize();
        e.finalize();
        f.finalize();
    }
}

impl Finalize for &'static str {}

impl<A:Finalize> Finalize for UnsafeCell<A> {
    fn finalize(&mut self) {
        let self_ = unsafe { &mut *self.get() };
        self_.finalize();
    }
}

#[derive(Clone,Copy)]
enum GcCellFlags {
    Unused,
    Reading(u32),
    Writing
}

impl GcCellFlags {
    fn is_reading(&self) -> bool {
        match self {
            &GcCellFlags::Reading(_) => true,
            _ => false
        }
    }

    fn add_reading(&self) -> GcCellFlags {
        match self {
            &GcCellFlags::Reading(ref count) => GcCellFlags::Reading(*count + 1),
            &GcCellFlags::Unused => GcCellFlags::Reading(1),
            _ => self.clone(),
        }
    }

    fn sub_reading(&self) -> GcCellFlags {
        match self {
            &GcCellFlags::Reading(ref count) => {
                let count2 = *count - 1;
                if count2 == 0 {
                    GcCellFlags::Unused
                } else {
                    GcCellFlags::Reading(count2)
                }
            },
            _ => self.clone(),
        }
    }

    fn is_writing(&self) -> bool {
        match self {
            &GcCellFlags::Writing => true,
            _ => false
        }
    }

    fn set_writing(&self) -> GcCellFlags {
        GcCellFlags::Writing
    }

    fn is_unused(&self) -> bool {
        match self {
            &GcCellFlags::Unused => true,
            _ => false
        }
    }

    fn set_unused(&self) -> GcCellFlags {
        GcCellFlags::Unused
    }
}

pub struct GcCell<A: ?Sized> {
    flags: Cell<GcCellFlags>,
    cell: UnsafeCell<A>
}

impl<A> GcCell<A> {
    pub fn new(a: A) -> Self {
        GcCell {
            flags: Cell::new(GcCellFlags::Unused),
            cell: UnsafeCell::new(a)
        }
    }

    pub fn into_inner(self) -> A {
        self.cell.into_inner()
    }
}

impl<A: ?Sized> GcCell<A> {
    pub fn borrow(&self) -> GcCellRef<A> {
        if self.flags.get().is_writing() {
            panic!("GcCell<T> already mutably borrowed");
        }
        self.flags.set(self.flags.get().add_reading());
        unsafe {
            GcCellRef {
                flags: &self.flags,
                value: &*self.cell.get(),
            }
        }
    }

    pub fn borrow_mut(&self) -> GcCellRefMut<A> {
        if !self.flags.get().is_unused() {
            panic!("GcCell<T> already borrowed");
        }
        self.flags.set(self.flags.get().set_writing());
        unsafe {
            GcCellRefMut {
                flags: &self.flags,
                value: &mut *self.cell.get(),
            }
        }
    }
}

pub struct GcCellRef<'a, A: ?Sized + 'static> {
    flags: &'a Cell<GcCellFlags>,
    value: &'a A,
}

impl<'a, A: ?Sized> Deref for GcCellRef<'a, A> {
    type Target = A;

    fn deref(&self) -> &A {
        self.value
    }
}

impl<'a, A: ?Sized> Drop for GcCellRef<'a, A> {
    fn drop(&mut self) {
        debug_assert!(self.flags.get().is_reading());
        self.flags.set(self.flags.get().sub_reading());
    }
}

pub struct GcCellRefMut<'a, A: ?Sized + 'static> {
    flags: &'a Cell<GcCellFlags>,
    value: &'a mut A,
}

impl<'a, A: ?Sized> Deref for GcCellRefMut<'a, A> {
    type Target = A;

    fn deref(&self) -> &A {
        self.value
    }
}

impl<'a, A: ?Sized> DerefMut for GcCellRefMut<'a, A> {
    fn deref_mut(&mut self) -> &mut A {
        self.value
    }
}

impl<'a, T: ?Sized> Drop for GcCellRefMut<'a, T> {
    fn drop(&mut self) {
        debug_assert!(self.flags.get().is_writing());
        self.flags.set(self.flags.get().set_unused());
    }
}

#[derive(PartialEq)]
enum Colour {
    Black,
    Purple,
    White,
    Gray
}

struct Node {
    desc_op: Option<String>,
    strong: i32,
    weak: i32,
    colour: Colour,
    buffered: bool,
    trace: Box<Fn(&mut FnMut(*mut Node))>,
    finalize: Box<Fn()>,
    freed: bool,
    cleanup: Box<Fn()>
}

impl Node {
    fn trace(&self, f: &mut FnMut(*mut Node)) {
        if !self.freed {
            (self.trace)(f);
        }
    }

    fn debug(&self) {
        self.debug2(&mut HashSet::new(), &mut HashMap::new(), &mut 1);
    }

    fn node_id(&self, id_map: &mut HashMap<*const Node,u32>, next_id: &mut u32) -> u32 {
        {
            let id_op = id_map.get(&(self as *const Node));
            if let Some(id) = id_op {
                return id.clone();
            }
        }
        let id = *next_id;
        id_map.insert(self as *const Node, id);
        *next_id = *next_id + 1;
        id
    }

    fn debug2(&self, visited: &mut HashSet<*const Node>, id_map: &mut HashMap<*const Node,u32>, next_id: &mut u32) {
        if visited.contains(&(self as *const Node)) {
            return;
        }
        visited.insert(self as *const Node);
        let id = self.node_id(id_map, next_id);
        print!("node");
        if let &Some(ref desc) = &self.desc_op {
            print!("({})", desc);
        }
        print!(" {}/{} {}: ", self.strong, self.weak, id);
        self.trace(&mut |node:*mut Node| {
            let node = unsafe { &*node };
            let id = node.node_id(id_map, next_id);
            print!(" {}", id);
        });
        println!();
        self.trace(&mut |node:*mut Node| {
            let node = unsafe { &*node };
            node.debug2(visited, id_map, next_id);
        });
    }
}

impl GcCtx {

    pub fn new() -> GcCtx {
        GcCtx {
            data: Rc::new(RefCell::new(
                GcCtxData {
                    roots: Vec::new(),
                    collecting_cycles: false,
                    to_be_freed: Vec::new()
                }
            ))
        }
    }

    pub fn new_gc<A: Trace + Finalize + 'static>(&mut self, value: A) -> Gc<A> {
        self._new_gc(value, None)
    }

    pub fn new_gc_with_desc<A: Trace + Finalize + 'static>(&mut self, value: A, desc: String) -> Gc<A> {
        self._new_gc(value, Some(desc))
    }

    fn _new_gc<A: Trace + Finalize + 'static>(&mut self, value: A, desc_op: Option<String>) -> Gc<A> {
        let value = Box::into_raw(Box::new(value));
        let value2 = value.clone();
        let value3 = value.clone();
        let r = Gc {
            ctx: self.clone(),
            value: value,
            node: Box::into_raw(Box::new(Node {
                desc_op: desc_op,
                strong: 1,
                weak: 1,
                colour: Colour::Black,
                buffered: false,
                trace: Box::new(move |f: &mut FnMut(*mut Node)| {
                    unsafe { &*value2 }.trace(&mut |dep: &GcDep| f(dep.node))
                }),
                finalize: Box::new(move || {
                    unsafe { &mut *value3 }.finalize()
                }),
                freed: false,
                cleanup: Box::new(move || {
                    unsafe { Box::from_raw(value); }
                })
            }))
        };
        r
    }

    fn with_data<F,A>(&self, f: F)->A where F: FnOnce(&mut GcCtxData)->A {
        f(&mut self.data.borrow_mut())
    }

    fn increment(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.strong = s.strong + 1;
        s.colour = Colour::Black;
    }

    fn decrement(&self, s: *mut Node) {
        let node = unsafe { &mut *s };
        if node.strong > 0 {
            node.strong = node.strong - 1;
            if node.strong == 0 {
                self.release(node);
            } else {
                self.possible_root(node);
            }
        }
    }

    fn release(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        debug_assert!(s.strong == 0);
        s.colour = Colour::Black;
        if !s.buffered {
            self.finalize_and_mark_to_be_freed(s);
        }
    }

    fn system_free(&self, s: *mut Node) {
        self.with_data(|data| data.roots.retain(|n| !ptr::eq(*n, s)));
        let s = unsafe { &mut *s };
        debug_assert!(s.strong == 0);
        (s.cleanup)();
        s.freed = true;
        if s.weak > 0 {
            s.weak = s.weak - 1;
            if s.weak == 0 {
                unsafe { Box::from_raw(s); }
            }
        }
    }

    fn possible_root(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        debug_assert!(s.strong > 0);
        if s.colour != Colour::Purple {
            s.colour = Colour::Purple;
            if !s.buffered {
                s.buffered = true;
                self.with_data(|data| {
                    let s2: *mut Node = s;
                    if !data.roots.contains(&s2) {
                        data.roots.push(s2);
                    }
                });
            }
        }
    }

    pub fn collect_cycles(&self) {
        if self.with_data(|data| data.collecting_cycles) {
            return;
        }
        self.with_data(|data| data.collecting_cycles = true);

        self.mark_roots();
        self.scan_roots();
        self.collect_roots();

        let again = self.with_data(|data| data.to_be_freed.len() != 0);

        self.free_to_be_freed();

        self.with_data(|data| data.collecting_cycles = false);

        if again {
            self.collect_cycles();
        }
    }

    fn mark_roots(&self) {
        let roots = self.with_data(|data| data.roots.clone());
        let mut new_roots = Vec::new();
        for s in roots {
            let s2 = s;
            let s = unsafe { &mut *s };
            if s.colour == Colour::Purple && s.strong > 0 {
                self.mark_gray(s);
                new_roots.push(s2);
            } else {
                s.buffered = false;
                self.with_data(|data| data.roots.retain(|s2| !ptr::eq(s, *s2)));
                if s.colour == Colour::Black && s.strong == 0 {
                    self.finalize_and_mark_to_be_freed(s);
                }
            }
        }
        self.with_data(|data| {
            data.roots = new_roots;
        });
    }

    fn scan_roots(&self) {
        let roots = self.with_data(|data| data.roots.clone());
        for s in roots {
            self.scan(s);
        }
    }

    fn collect_roots(&self) {
        let roots = self.with_data(|data| {
            let roots = data.roots.clone();
            data.roots.clear();
            roots
        });
        for s in roots {
            let s = unsafe { &mut *s };
            s.buffered = false;
            self.collect_white(s);
        }
    }

    fn mark_gray(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour != Colour::Gray {
            s.colour = Colour::Gray;
            s.trace(&mut |t| {
                let t = unsafe { &mut *t };
                t.strong = t.strong - 1;
                self.mark_gray(t);
            });
        }
    }

    fn scan(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour == Colour::Gray {
            if s.strong > 0 {
                self.scan_black(s);
            } else {
                s.colour = Colour::White;
                s.trace(&mut |t| {
                    let t = unsafe { &mut *t };
                    self.scan(t);
                });
            }
        }
    }

    fn scan_black(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.colour = Colour::Black;
        s.trace(&mut |t| {
            let t = unsafe { &mut *t };
            t.strong = t.strong + 1;
            if t.colour != Colour::Black {
                self.scan_black(t);
            }
        });
    }

    fn collect_white(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour == Colour::White && !s.buffered {
            s.colour = Colour::Black;
            s.trace(&mut |t| {
                self.collect_white(t);
            });
            self.finalize_and_mark_to_be_freed(s);
        }
    }

    fn finalize_and_mark_to_be_freed(&self, s: *mut Node) {
        unsafe { ((*s).finalize)() };
        self.with_data(|data| data.to_be_freed.push(s));
    }

    fn free_to_be_freed(&self) {
        let mut to_be_freed = Vec::new();
        self.with_data(|data| swap(&mut data.to_be_freed, &mut to_be_freed));
        for node in to_be_freed {
            self.system_free(node);
        }
    }
}
