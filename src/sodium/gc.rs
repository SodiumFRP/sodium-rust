/*
 * A Pure Reference Counting Garbage Collector
 * DAVID F. BACON, CLEMENT R. ATTANASIO, V.T. RAJAN, STEPHEN E. SMITH
 */

use std::marker::PhantomData;
use std::ptr;
use std::ops::Deref;
use std::any::Any;

pub struct GcCtx {
    roots: Vec<*mut Node>
}

pub struct Gc<A: ?Sized> {
    ctx: *mut GcCtx,
    node: *mut Node,
    phantom: PhantomData<A>
}

impl<A: ?Sized> Clone for Gc<A> {
    fn clone(&self) -> Self {
        let ctx = unsafe { &mut *self.ctx };
        ctx.increment(self.node);
        Gc {
            ctx: self.ctx,
            node: self.node,
            phantom: PhantomData
        }
    }
}

impl<A: ?Sized> Drop for Gc<A> {
    fn drop(&mut self) {
        let ctx = unsafe { &mut *self.ctx };
        ctx.decrement(self.node);
    }
}

impl<A: ?Sized + 'static> Deref for Gc<A> {
    type Target = A;

    fn deref(&self) -> &A {
        let node = unsafe { &mut *self.node };
        let data: &Any = unsafe { &*node.data };
        let value: &Box<A> = match data.downcast_ref::<Box<A>>() {
            Some(value2) => value2,
            None => panic!()
        };
        value
    }
}

impl<A: ?Sized> Gc<A> {
    pub fn add_child<B>(&mut self, child: &Gc<B>) {
        let node = unsafe { &mut *self.node };
        let child_node = child.node;
        if !node.children.contains(&child_node) {
            node.children.push(child_node);
        }
    }

    pub fn remove_child<B>(&mut self, child: &Gc<B>) {
        let node = unsafe { &mut *self.node };
        let child_node = child.node;
        node.children.retain(|c| !ptr::eq(*c, child_node));
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
    count: u32,
    colour: Colour,
    buffered: bool,
    children: Vec<*mut Node>,
    data: Box<Any>
}

impl GcCtx {

    pub fn new() -> GcCtx {
        GcCtx {
            roots: Vec::new()
        }
    }

    pub fn new_gc<A: 'static>(&mut self, value: A) -> Gc<A> {
        let ctx: *mut GcCtx = self;
        Gc {
            ctx: ctx,
            node: Box::into_raw(Box::new(Node {
                count: 1,
                colour: Colour::Black,
                buffered: false,
                children: Vec::new(),
                data: Box::new(value) as Box<Any>
            })),
            phantom: PhantomData
        }
    }

    fn increment(&mut self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.count = s.count + 1;
        s.colour = Colour::Black;
    }

    fn decrement(&mut self, s: *mut Node) {
        let s = unsafe { &mut *s };
        s.count = s.count - 1;
        if s.count == 0 {
            self.release(s);
        } else {
            self.possible_root(s);
        }
    }

    fn release(&mut self, s: *mut Node) {
        let s = unsafe { &mut *s };
        for child in &s.children {
            self.decrement(*child);
        }
        s.colour = Colour::Black;
        if !s.buffered {
            self.system_free(s);
        }
    }

    fn system_free(&mut self, s: *mut Node) {
        let s = unsafe { &mut *s };
        unsafe {
            Box::from_raw(s);
        }
    }

    fn possible_root(&mut self, s: *mut Node) {
        let s = unsafe { &mut *s };
        if s.colour != Colour::Purple {
            s.colour = Colour::Purple;
            if !s.buffered {
                s.buffered = true;
                self.roots.push(s);
            }
        }
    }

    fn collect_cycles(&mut self) {
        self.mark_roots();
        self.scan_roots();
        self.collect_roots();
    }

    fn mark_roots(&mut self) {
        let roots = self.roots.clone();
        for s in roots {
            let s = unsafe { &mut *s };
            if s.colour == Colour::Purple && s.count > 0 {
                self.mark_gray(s);
            } else {
                s.buffered = false;
                self.roots.retain(|s2| !ptr::eq(s, *s2));
                if s.colour == Colour::Black && s.count == 0 {
                    self.system_free(s);
                }
            }
        }
    }

    fn scan_roots(&mut self) {
        let roots = self.roots.clone();
        for s in roots {
            self.scan(s);
        }
    }

    fn collect_roots(&mut self) {
        let roots = self.roots.clone();
        self.roots.clear();
        for s in roots {
            let s = unsafe { &mut *s };
            s.buffered = false;
            self.collect_white(s);
        }
    }

    fn mark_gray(&mut self, s: *mut Node) {
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

    fn scan(&mut self, s: *mut Node) {
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

    fn scan_black(&mut self, s: *mut Node) {
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

    fn collect_white(&mut self, s: *mut Node) {
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
