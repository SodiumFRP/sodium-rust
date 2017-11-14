use sodium::Dep;
use std::ops::Deref;
use std::ops::DerefMut;
use std::convert::From;

pub struct Value<A: ?Sized> {
    deps: Vec<Dep>,
    value: A
}

macro_rules! value {
    ($a:expr) => {{
        Value::new($a, Vec::new())
    }};
    ($a:expr, $($dep:expr),*) => {{
        Value::new($a, vec![$(deps,)*])
    }}
}

impl<A> Value<A> {
    pub fn new(value: A, deps: Vec<Dep>) -> Value<A> {
        Value {
            deps: deps,
            value: value
        }
    }
}

impl<A: ?Sized> Value<A> {
    pub fn deps(&self) -> &Vec<Dep> {
        &self.deps
    }
}

impl<A> From<A> for Value<A> {
    fn from(a: A) -> Value<A> {
        value!(a)
    }
}

impl<A: ?Sized> Deref for Value<A> {
    type Target = A;

    fn deref(&self) -> &A {
        &self.value
    }
}

impl<A: ?Sized> DerefMut for Value<A> {
    fn deref_mut(&mut self) -> &mut A {
        &mut self.value
    }
}
