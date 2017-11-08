/*
 * A Pure Reference Counting Garbage Collector
 * DAVID F. BACON, CLEMENT R. ATTANASIO, V.T. RAJAN, STEPHEN E. SMITH
 */

use std::ptr;
use std::ops::Deref;
use std::mem::transmute;
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

    pub fn downgrade(&self) -> GcWeak<A> {
        let weak_node = Box::into_raw(Box::new(
            WeakNode {
                node: Some(self.node)
            }
        ));
        unsafe {
            (*self.node).weak_nodes.push(weak_node);
        }
        GcWeak {
            ctx: self.ctx.clone(),
            weak_node: weak_node,
            value: self.value
        }
    }

    pub fn upcast<F,B:?Sized>(&self, f: F) -> Gc<B> where F: FnOnce(&A)->&B {
        let s = unsafe { &mut *self.node };
        s.count = s.count + 1;
        Gc {
            ctx: self.ctx.clone(),
            value: unsafe { transmute(f(&mut *self.value)) },
            node: self.node
        }
    }
}

pub struct GcWeak<A: ?Sized> {
    ctx: GcCtx,
    weak_node: *mut WeakNode,
    value: *mut A
}

impl<A: ?Sized> Clone for GcWeak<A> {
    fn clone(&self) -> Self {
        match self.upgrade() {
            Some(gc) => gc.downgrade(),
            None =>
                GcWeak {
                    ctx: self.ctx.clone(),
                    weak_node: Box::into_raw(Box::new(WeakNode { node: None })),
                    value: self.value
                }
        }
    }
}

impl<A: ?Sized> Drop for GcWeak<A> {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.weak_node); }
    }
}

impl<A: ?Sized> GcWeak<A> {
    pub fn upgrade(&self) -> Option<Gc<A>> {
        let weak_node = unsafe { &*self.weak_node };
        weak_node.node.map(|node| {
            self.ctx.increment(node);
            Gc {
                ctx: self.ctx.clone(),
                value: self.value,
                node: node
            }
        })
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
    count: i32,
    colour: Colour,
    buffered: bool,
    children: Vec<*mut Node>,
    weak_nodes: Vec<*mut WeakNode>,
    cleanup: Box<Fn()>
}

impl Drop for Node {
    fn drop(&mut self) {
        for weak_node in &self.weak_nodes {
            let weak_node = unsafe { &mut **weak_node };
            weak_node.node = None;
        }
        (self.cleanup)();
    }
}

struct WeakNode {
    node: Option<*mut Node>
}

impl Drop for WeakNode {
    fn drop(&mut self) {
        match &self.node {
            &Some(ref node) => {
                let node = unsafe { &mut **node };
                node.weak_nodes.retain(|weak_node| !ptr::eq(*weak_node, self));
            },
            &None => ()
        }
    }
}

impl GcCtx {

    pub fn new() -> GcCtx {
        GcCtx {
            data: Rc::new(RefCell::new(
                GcCtxData {
                    roots: Vec::new(),
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
                count: 1,
                colour: Colour::Black,
                buffered: false,
                children: Vec::new(),
                weak_nodes: Vec::new(),
                cleanup: Box::new(move || { unsafe { Box::from_raw(value); } })
            }))
        }
    }

    fn with_data<F,A>(&self, f: F)->A where F: FnOnce(&mut GcCtxData)->A {
        f(&mut self.data.borrow_mut())
    }

    fn increment(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.count = s.count + 1;
        s.colour = Colour::Black;
    }

    fn decrement(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.count = s.count - 1;
        if s.count == 0 {
            self.release(s);
        } else {
            self.possible_root(s);
        }
    }

    fn release(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        /*
        The destructor of A inside Gc<A> will do this for us.

        for child in &s.children {
            self.decrement(*child);
        }
        */
        s.colour = Colour::Black;
        if !s.buffered {
            self.system_free(s);
        }
    }

    fn system_free(&self, s: *mut Node) {
        self.with_data(|data| data.roots.retain(|n| !ptr::eq(*n, s)));
        let s = unsafe { &mut *s };
        unsafe {
            Box::from_raw(s);
        }
    }

    fn possible_root(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour != Colour::Purple {
            s.colour = Colour::Purple;
            if !s.buffered {
                s.buffered = true;
                self.with_data(|data| {
                    let s2: *mut Node = s;
                    if !data.roots.contains(&s2) {
                        data.roots.push(s);
                    }
                });
            }
        }
    }

    pub fn collect_cycles(&self) {
        self.mark_roots();
        self.scan_roots();
        self.collect_roots();
    }

    fn mark_roots(&self) {
        let roots = self.with_data(|data| data.roots.clone());
        for s in roots {
            let s = unsafe { &mut *s };
            if s.colour == Colour::Purple && s.count > 0 {
                self.mark_gray(s);
            } else {
                s.buffered = false;
                self.with_data(|data| data.roots.retain(|s2| !ptr::eq(s, *s2)));
                if s.colour == Colour::Black && s.count == 0 {
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
                t.count = t.count - 1;
                self.mark_gray(t);
            }
        }
    }

    fn scan(&self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour == Colour::Gray {
            if s.count > 0 {
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
            t.count = t.count + 1;
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
