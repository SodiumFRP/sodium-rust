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
    collecting_cycles: bool
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
    pub fn to_dep(&self) -> GcDep {
        let n: &mut Node = unsafe { &mut *self.node };
        n.weak = n.weak + 1;
        GcDep {
            ctx: self.ctx.clone(),
            node: self.node
        }
    }

    pub fn strong_count(&self) -> usize {
        let node = unsafe { &*self.node };
        node.strong
    }

    pub fn weak_count(&self) -> usize {
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

impl<A: Trace> Trace for Gc<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        let self2: &A = &*self;
        self2.trace(f);
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

impl<A: Trace> Trace for Vec<A> {
    fn trace(&self, f: &mut FnMut(&GcDep)) {
        for a in self {
            a.trace(f);
        }
    }
}

impl Trace for i32 {
    fn trace(&self, f: &mut FnMut(&GcDep)) {}
}

impl Trace for &'static str {
    fn trace(&self, f: &mut FnMut(&GcDep)) {}
}

impl Trace for String {
    fn trace(&self, f: &mut FnMut(&GcDep)) {}
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
        unsafe { self.cell.into_inner() }
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
    strong: usize,
    weak: usize,
    colour: Colour,
    buffered: bool,
    trace: Box<Fn(&mut FnMut(*mut Node))>,
    cleanup: Box<Fn()>
}

impl Node {
    fn trace(&self, f: &mut FnMut(*mut Node)) {
        if self.strong > 0 {
            (self.trace)(f);
        }
    }
}

impl GcCtx {

    pub fn new() -> GcCtx {
        GcCtx {
            data: Rc::new(RefCell::new(
                GcCtxData {
                    roots: Vec::new(),
                    collecting_cycles: false
                }
            ))
        }
    }

    pub fn new_gc<A: Trace + 'static>(&mut self, value: A) -> Gc<A> {
        let value = Box::into_raw(Box::new(value));
        let value2 = value.clone();
        let r = Gc {
            ctx: self.clone(),
            value: value,
            node: Box::into_raw(Box::new(Node {
                strong: 1,
                weak: 1,
                colour: Colour::Black,
                buffered: false,
                trace: Box::new(move |f: &mut FnMut(*mut Node)| {
                    unsafe { &*value2 }.trace(&mut |dep: &GcDep| f(dep.node))
                }),
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
            self.system_free(s);
        }
    }

    fn system_free(&self, s: *mut Node) {
        self.with_data(|data| data.roots.retain(|n| !ptr::eq(*n, s)));
        let s = unsafe { &mut *s };
        debug_assert!(s.strong == 0);
        (s.cleanup)();
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

        self.with_data(|data| data.collecting_cycles = false);
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
                    self.system_free(s);
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
            self.system_free(s);
        }
    }
}
