/*
 * A Pure Reference Counting Garbage Collector
 * DAVID F. BACON, CLEMENT R. ATTANASIO, V.T. RAJAN, STEPHEN E. SMITH
 */

use std::ptr;
use std::ops::Deref;
use std::mem::transmute;
use std::mem::swap;
use std::cell::RefCell;
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
    to_be_freed: Vec<*mut Node>,
    collecting_cycles: bool,
    auto_collect_cycles_on_decrement: bool
}

pub struct GcDep {
    ctx: GcCtx,
    node: *mut Node
}

impl Drop for GcDep {
    fn drop(&mut self) {
        self.ctx.decrement(self.node);
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
        if self.ctx.with_data(|data| data.auto_collect_cycles_on_decrement) {
            self.ctx.collect_cycles();
        }
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
        self.ctx.increment(self.node);
        GcDep {
            ctx: self.ctx.clone(),
            node: self.node
        }
    }

    pub fn set_deps(&self, deps: Vec<GcDep>) {
        let node = unsafe { &mut *self.node };
        node.children.clear();
        for dep in deps {
            if !node.children.contains(&dep.node) {
                node.children.push(dep.node);
            }
        }
    }

    pub fn add_deps(&self, deps: Vec<GcDep>) {
        let node = unsafe { &mut *self.node };
        for dep in deps {
            if !node.children.contains(&dep.node) {
                node.children.push(dep.node);
            }
        }
    }

    pub fn remove_deps(&self, deps: Vec<GcDep>) {
        let node = unsafe { &mut *self.node };
        let dep_nodes: Vec<*mut Node> = deps.iter().map(|dep| dep.node).collect();
        node.children.retain(|node| !dep_nodes.contains(node));
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
                unsafe { Box::from_raw(node) };
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
    children: Vec<*mut Node>,
    cleanup: Box<Fn()>
}

impl GcCtx {

    pub fn new() -> GcCtx {
        GcCtx {
            data: Rc::new(RefCell::new(
                GcCtxData {
                    roots: Vec::new(),
                    to_be_freed: Vec::new(),
                    collecting_cycles: false,
                    auto_collect_cycles_on_decrement: true
                }
            ))
        }
    }

    pub fn new_gc<A: 'static>(&mut self, value: A) -> Gc<A> {
        let value = Box::into_raw(Box::new(value));
        Gc {
            ctx: self.clone(),
            value: value,
            node: Box::into_raw(Box::new(Node {
                strong: 1,
                weak: 1,
                colour: Colour::Black,
                buffered: false,
                children: Vec::new(),
                cleanup: Box::new(move || { unsafe { Box::from_raw(value); } })
            }))
        }
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
        s.weak = s.weak - 1;
        (s.cleanup)();
        if s.weak == 0 {
            self.with_data(|data| data.to_be_freed.push(s));
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

        let mut to_be_freed = Vec::new();
        self.with_data(|data| swap(&mut to_be_freed, &mut data.to_be_freed));
        for s in to_be_freed {
            //unsafe { Box::from_raw(s) };
        }

        self.with_data(|data| data.collecting_cycles = false);
    }

    fn mark_roots(&self) {
        let roots = self.with_data(|data| data.roots.clone());
        for s in roots {
            let s = unsafe { &mut *s };
            if s.colour == Colour::Purple && s.strong > 0 {
                self.mark_gray(s);
            } else {
                s.buffered = false;
                self.with_data(|data| data.roots.retain(|s2| !ptr::eq(s, *s2)));
                if s.colour == Colour::Black && s.strong == 0 {
                    self.system_free(s);
                }
            }
        }
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
            for t in &s.children {
                let t = unsafe { &mut **t };
                t.strong = t.strong - 1;
                self.mark_gray(t);
            }
        }
    }

    fn scan(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour == Colour::Gray {
            if s.strong > 0 {
                self.scan_black(s);
            } else {
                s.colour = Colour::White;
                for t in &s.children {
                    let t = unsafe { &mut **t };
                    self.scan(t);
                }
            }
        }
    }

    fn scan_black(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.colour = Colour::Black;
        for t in &s.children {
            let t = unsafe { &mut **t };
            t.strong = t.strong + 1;
            if t.colour != Colour::Black {
                self.scan_black(t);
            }
        }
    }

    fn collect_white(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour == Colour::White && !s.buffered {
            s.colour = Colour::Black;
            for t in &s.children {
                self.collect_white(*t);
            }
            self.system_free(s);
        }
    }
}
