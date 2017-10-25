/*
 * A Pure Reference Counting Garbage Collector
 * DAVID F. BACON, CLEMENT R. ATTANASIO, V.T. RAJAN, STEPHEN E. SMITH
 */

use std::marker::PhantomData;

pub struct GcCtx {
    roots: Vec<*mut Node>
}

pub struct Gc<A> {
    node: Node,
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
                unimplemented!();
            }
        }
        unimplemented!();
    }
}
