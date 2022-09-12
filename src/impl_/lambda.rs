#![allow(clippy::many_single_char_names)]

use crate::impl_::dep::Dep;

pub struct Lambda<FN> {
    f: FN,
    deps: Vec<Dep>,
}

pub fn lambda1_deps<A, B, FN: IsLambda1<A, B>>(f: &FN) -> Vec<Dep> {
    f.deps_op().cloned().unwrap_or_default()
}

pub fn lambda2_deps<A, B, C, FN: IsLambda2<A, B, C>>(f: &FN) -> Vec<Dep> {
    f.deps_op().cloned().unwrap_or_default()
}

pub fn lambda3_deps<A, B, C, D, FN: IsLambda3<A, B, C, D>>(f: &FN) -> Vec<Dep> {
    f.deps_op().cloned().unwrap_or_default()
}

pub fn lambda4_deps<A, B, C, D, E, FN: IsLambda4<A, B, C, D, E>>(f: &FN) -> Vec<Dep> {
    f.deps_op().cloned().unwrap_or_default()
}

pub fn lambda5_deps<A, B, C, D, E, F, FN: IsLambda5<A, B, C, D, E, F>>(f: &FN) -> Vec<Dep> {
    f.deps_op().cloned().unwrap_or_default()
}

pub fn lambda6_deps<A, B, C, D, E, F, G, FN: IsLambda6<A, B, C, D, E, F, G>>(f: &FN) -> Vec<Dep> {
    f.deps_op().cloned().unwrap_or_default()
}

/// Interface for a lambda function of one argument.
pub trait IsLambda1<A, B> {
    fn call(&mut self, a: &A) -> B;
    fn deps_op(&self) -> Option<&Vec<Dep>>;
}

/// Interface for a lambda function of two arguments.
pub trait IsLambda2<A, B, C> {
    fn call(&mut self, a: &A, b: &B) -> C;
    fn deps_op(&self) -> Option<&Vec<Dep>>;
}

/// Interface for a lambda function of three arguments.
pub trait IsLambda3<A, B, C, D> {
    fn call(&mut self, a: &A, b: &B, c: &C) -> D;
    fn deps_op(&self) -> Option<&Vec<Dep>>;
}

/// Interface for a lambda function of four arguments.
pub trait IsLambda4<A, B, C, D, E> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D) -> E;
    fn deps_op(&self) -> Option<&Vec<Dep>>;
}

/// Interface for a lambda function of five arguments.
pub trait IsLambda5<A, B, C, D, E, F> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E) -> F;
    fn deps_op(&self) -> Option<&Vec<Dep>>;
}

/// Interface for a lambda function of six arguments.
pub trait IsLambda6<A, B, C, D, E, F, G> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> G;
    fn deps_op(&self) -> Option<&Vec<Dep>>;
}

impl<A, B, FN: FnMut(&A) -> B> IsLambda1<A, B> for Lambda<FN> {
    fn call(&mut self, a: &A) -> B {
        (self.f)(a)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        Some(&self.deps)
    }
}

impl<A, B, FN: FnMut(&A) -> B> IsLambda1<A, B> for FN {
    fn call(&mut self, a: &A) -> B {
        self(a)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        None
    }
}

impl<A, B, C, FN: FnMut(&A, &B) -> C> IsLambda2<A, B, C> for Lambda<FN> {
    fn call(&mut self, a: &A, b: &B) -> C {
        (self.f)(a, b)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        Some(&self.deps)
    }
}

impl<A, B, C, FN: FnMut(&A, &B) -> C> IsLambda2<A, B, C> for FN {
    fn call(&mut self, a: &A, b: &B) -> C {
        self(a, b)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        None
    }
}

impl<A, B, C, D, FN: FnMut(&A, &B, &C) -> D> IsLambda3<A, B, C, D> for Lambda<FN> {
    fn call(&mut self, a: &A, b: &B, c: &C) -> D {
        (self.f)(a, b, c)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        Some(&self.deps)
    }
}

impl<A, B, C, D, FN: FnMut(&A, &B, &C) -> D> IsLambda3<A, B, C, D> for FN {
    fn call(&mut self, a: &A, b: &B, c: &C) -> D {
        self(a, b, c)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        None
    }
}

impl<A, B, C, D, E, FN: FnMut(&A, &B, &C, &D) -> E> IsLambda4<A, B, C, D, E> for Lambda<FN> {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D) -> E {
        (self.f)(a, b, c, d)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        Some(&self.deps)
    }
}

impl<A, B, C, D, E, FN: FnMut(&A, &B, &C, &D) -> E> IsLambda4<A, B, C, D, E> for FN {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D) -> E {
        self(a, b, c, d)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        None
    }
}

impl<A, B, C, D, E, F, FN: FnMut(&A, &B, &C, &D, &E) -> F> IsLambda5<A, B, C, D, E, F>
    for Lambda<FN>
{
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E) -> F {
        (self.f)(a, b, c, d, e)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        Some(&self.deps)
    }
}

impl<A, B, C, D, E, F, FN: FnMut(&A, &B, &C, &D, &E) -> F> IsLambda5<A, B, C, D, E, F> for FN {
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E) -> F {
        self(a, b, c, d, e)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        None
    }
}

impl<A, B, C, D, E, F, G, FN: FnMut(&A, &B, &C, &D, &E, &F) -> G> IsLambda6<A, B, C, D, E, F, G>
    for Lambda<FN>
{
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> G {
        (self.f)(a, b, c, d, e, f)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        Some(&self.deps)
    }
}

impl<A, B, C, D, E, F, G, FN: FnMut(&A, &B, &C, &D, &E, &F) -> G> IsLambda6<A, B, C, D, E, F, G>
    for FN
{
    fn call(&mut self, a: &A, b: &B, c: &C, d: &D, e: &E, f: &F) -> G {
        self(a, b, c, d, e, f)
    }

    fn deps_op(&self) -> Option<&Vec<Dep>> {
        None
    }
}

pub fn lambda1<A, B, FN: FnMut(&A) -> B>(f: FN, deps: Vec<Dep>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda2<A, B, C, FN: FnMut(&A, &B) -> C>(f: FN, deps: Vec<Dep>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda3<A, B, C, D, FN: FnMut(&A, &B, &C) -> D>(f: FN, deps: Vec<Dep>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda4<A, B, C, D, E, FN: FnMut(&A, &B, &C, &D) -> E>(f: FN, deps: Vec<Dep>) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda5<A, B, C, D, E, F, FN: FnMut(&A, &B, &C, &D, &E) -> F>(
    f: FN,
    deps: Vec<Dep>,
) -> Lambda<FN> {
    Lambda { f, deps }
}

pub fn lambda6<A, B, C, D, E, F, G, FN: FnMut(&A, &B, &C, &D, &E, &F) -> G>(
    f: FN,
    deps: Vec<Dep>,
) -> Lambda<FN> {
    Lambda { f, deps }
}
