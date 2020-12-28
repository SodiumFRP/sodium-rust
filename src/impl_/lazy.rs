use std::sync::Arc;
use std::sync::Mutex;

/// A representation for a value that may not be available until the
/// current transaction is closed.
pub struct Lazy<A> {
    data: Arc<Mutex<LazyData<A>>>,
}

impl<A> Clone for Lazy<A> {
    fn clone(&self) -> Self {
        Lazy {
            data: self.data.clone(),
        }
    }
}

pub enum LazyData<A> {
    Thunk(Box<dyn FnMut() -> A + Send>),
    Value(A),
}

impl<A: Send + Clone + 'static> Lazy<A> {
    pub fn new<THUNK: FnMut() -> A + Send + 'static>(thunk: THUNK) -> Lazy<A> {
        Lazy {
            data: Arc::new(Mutex::new(LazyData::Thunk(Box::new(thunk)))),
        }
    }

    pub fn of_value(value: A) -> Lazy<A> {
        Lazy {
            data: Arc::new(Mutex::new(LazyData::Value(value))),
        }
    }

    pub fn run(&self) -> A {
        let mut l = self.data.lock();
        let data: &mut LazyData<A> = l.as_mut().unwrap();
        let next_op: Option<LazyData<A>>;
        let result: A;
        match data {
            LazyData::Thunk(ref mut k) => {
                result = k();
                next_op = Some(LazyData::Value(result.clone()));
            }
            LazyData::Value(ref x) => {
                result = x.clone();
                next_op = None;
            }
        }
        if let Some(next) = next_op {
            *data = next;
        }
        result
    }
}
