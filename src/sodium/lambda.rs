use sodium::Dep;

pub struct Lambda<FN: ?Sized> {
    apply: Box<FN>,
    deps: Vec<Dep>
}

impl<FN: ?Sized> Lambda<FN> {
    pub fn new(apply: Box<FN>, deps: Vec<Dep>) -> Lambda<FN> {
        Lambda {
            apply: apply,
            deps: deps
        }
    }

    pub fn add_deps(&mut self, mut deps: Vec<Dep>) {
        self.deps.append(&mut deps);
    }

    pub fn add_deps_tunneled(mut self, deps: Vec<Dep>) -> Lambda<FN> {
        self.add_deps(deps);
        self
    }
}

macro_rules! lambda {
    ($f:expr) => {{
        Lambda::new(Box::new($f), Vec::new())
    }};
    ($f:expr, $($deps:expr),*) => {{
        Lambda::new(Box::new($f), vec![$($deps,)*])
    }}
}

pub trait IsLambda1<A,R> {
    fn apply(&self, a: &A) -> R;
    fn deps(&self) -> Vec<Dep>;
}

pub trait IsLambda2<A,B,R> {
    fn apply(&self, a: &A, b: &B) -> R;
    fn deps(&self) -> Vec<Dep>;
}

pub trait IsLambda3<A,B,C,R> {
    fn apply(&self, a: &A, b: &B, c: &C) -> R;
    fn deps(&self) -> Vec<Dep>;
}

pub trait IsLambda4<A,B,C,D,R> {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D) -> R;
    fn deps(&self) -> Vec<Dep>;
}

pub trait IsLambda5<A,B,C,D,E,R> {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D, e: &E) -> R;
    fn deps(&self) -> Vec<Dep>;
}

pub trait IsLambda6<A,B,C,D,E,F,R> {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> R;
    fn deps(&self) -> Vec<Dep>;
}

impl<A,R,FN:Fn(&A)->R> IsLambda1<A,R> for FN {
    fn apply(&self, a: &A) -> R {
        self(a)
    }
    fn deps(&self) -> Vec<Dep> {
        Vec::new()
    }
}

impl<A,B,R,FN:Fn(&A,&B)->R> IsLambda2<A,B,R> for FN {
    fn apply(&self, a: &A, b: &B) -> R {
        self(a, b)
    }
    fn deps(&self) -> Vec<Dep> {
        Vec::new()
    }
}

impl<A,B,C,R,FN:Fn(&A,&B,&C)->R> IsLambda3<A,B,C,R> for FN {
    fn apply(&self, a: &A, b: &B, c: &C) -> R {
        self(a, b, c)
    }
    fn deps(&self) -> Vec<Dep> {
        Vec::new()
    }
}

impl<A,B,C,D,R,FN:Fn(&A,&B,&C,&D)->R> IsLambda4<A,B,C,D,R> for FN {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D) -> R {
        self(a, b, c, d)
    }
    fn deps(&self) -> Vec<Dep> {
        Vec::new()
    }
}

impl<A,B,C,D,E,R,FN:Fn(&A,&B,&C,&D,&E)->R> IsLambda5<A,B,C,D,E,R> for FN {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D, e: &E) -> R {
        self(a, b, c, d, e)
    }
    fn deps(&self) -> Vec<Dep> {
        Vec::new()
    }
}

impl<A,B,C,D,E,F,R,FN:Fn(&A,&B,&C,&D,&E,&F)->R> IsLambda6<A,B,C,D,E,F,R> for FN {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> R {
        self(a, b, c, d, e, f)
    }
    fn deps(&self) -> Vec<Dep> {
        Vec::new()
    }
}

impl<A,R,FN:Fn(&A)->R> IsLambda1<A,R> for Lambda<FN> {
    fn apply(&self, a: &A) -> R {
        (self.apply)(a)
    }
    fn deps(&self) -> Vec<Dep> {
        self.deps.clone()
    }
}

impl<A,B,R,FN:Fn(&A,&B)->R> IsLambda2<A,B,R> for Lambda<FN> {
    fn apply(&self, a: &A, b: &B) -> R {
        (self.apply)(a, b)
    }
    fn deps(&self) -> Vec<Dep> {
        self.deps.clone()
    }
}

impl<A,B,C,R,FN:Fn(&A,&B,&C)->R> IsLambda3<A,B,C,R> for Lambda<FN> {
    fn apply(&self, a: &A, b: &B, c: &C) -> R {
        (self.apply)(a, b, c)
    }
    fn deps(&self) -> Vec<Dep> {
        self.deps.clone()
    }
}

impl<A,B,C,D,R,FN:Fn(&A,&B,&C,&D)->R> IsLambda4<A,B,C,D,R> for Lambda<FN> {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D) -> R {
        (self.apply)(a, b, c, d)
    }
    fn deps(&self) -> Vec<Dep> {
        self.deps.clone()
    }
}

impl<A,B,C,D,E,R,FN:Fn(&A,&B,&C,&D,&E)->R> IsLambda5<A,B,C,D,E,R> for Lambda<FN> {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D, e: &E) -> R {
        (self.apply)(a, b, c, d, e)
    }
    fn deps(&self) -> Vec<Dep> {
        self.deps.clone()
    }
}

impl<A,B,C,D,E,F,R,FN:Fn(&A,&B,&C,&D,&E,&F)->R> IsLambda6<A,B,C,D,E,F,R> for Lambda<FN> {
    fn apply(&self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> R {
        (self.apply)(a, b, c, d, e, f)
    }
    fn deps(&self) -> Vec<Dep> {
        self.deps.clone()
    }
}
