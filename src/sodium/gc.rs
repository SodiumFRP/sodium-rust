/*
 * A Pure Reference Counting Garbage Collector
 * DAVID F. BACON, CLEMENT R. ATTANASIO, V.T. RAJAN, STEPHEN E. SMITH
 */

use std::marker::PhantomData;
use std::ptr;

pub struct GcCtx {
    roots: Vec<*mut Node>
}

pub struct Gc<A> {
    node: *mut Node,
    phantom: PhantomData<A>
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
    free_data: Box<Fn(*mut ())>,
    data: *mut ()
}

impl GcCtx {

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
        (s.free_data)(s.data);
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
        unimplemented!();
    }

    fn scan(&mut self, s: *mut Node) {
        unimplemented!();
    }

    fn collect_white(&mut self, s: *mut Node) {
        unimplemented!();
    }
}
