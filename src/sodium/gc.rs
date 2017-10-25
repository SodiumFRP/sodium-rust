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

impl Node {

    fn increment(&mut self) {
        self.count = self.count + 1;
        self.colour = Colour::Black;
    }

    fn decrement(&mut self) {
        self.count = self.count - 1;
        if self.count == 0 {
            self.release();
        } else {
            self.possible_root();
        }
    }

    fn release(&mut self) {
        for child in &self.children {
            unsafe {
                (&mut **child).decrement();
            }
        }
        self.colour = Colour::Black;
        if !self.buffered {
            self.system_free();
        }
    }

    fn system_free(&mut self) {
        (self.free_data)(self.data);
        let self2: *mut Node = self;
        unsafe {
            Box::from_raw(self2);
        }
    }

    fn possible_root(&mut self) {
        if self.colour != Colour::Purple {
            self.colour = Colour::Purple;
            if !self.buffered {
                self.buffered = true;
                unimplemented!();
           }
        }
        unimplemented!();
    }
}