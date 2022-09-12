//! Sodium is a library for doing Functional Reactive Programming
//! (FRP) in Rust.
#[macro_use]
extern crate log;

mod cell;
mod cell_loop;
mod cell_sink;
mod impl_;
mod listener;
mod operational;
mod router;
mod sodium_ctx;
mod stream;
mod stream_loop;
mod stream_sink;
mod transaction;

pub use self::cell::Cell;
pub use self::cell_loop::CellLoop;
pub use self::cell_sink::CellSink;
#[doc(hidden)]
pub use self::impl_::dep::Dep;
#[doc(hidden)]
pub use self::impl_::lambda::lambda1;
#[doc(hidden)]
pub use self::impl_::lambda::lambda2;
#[doc(hidden)]
pub use self::impl_::lambda::lambda3;
#[doc(hidden)]
pub use self::impl_::lambda::lambda4;
#[doc(hidden)]
pub use self::impl_::lambda::lambda5;
#[doc(hidden)]
pub use self::impl_::lambda::lambda6;
pub use self::impl_::lambda::IsLambda1;
pub use self::impl_::lambda::IsLambda2;
pub use self::impl_::lambda::IsLambda3;
pub use self::impl_::lambda::IsLambda4;
pub use self::impl_::lambda::IsLambda5;
pub use self::impl_::lambda::IsLambda6;
#[doc(hidden)]
pub use self::impl_::lambda::Lambda;
pub use self::impl_::lazy::Lazy;
#[doc(hidden)]
pub use self::impl_::node::Node;
pub use self::listener::Listener;
pub use self::operational::Operational;
pub use self::router::Router;
pub use self::sodium_ctx::SodiumCtx;
pub use self::stream::Stream;
pub use self::stream_loop::StreamLoop;
pub use self::stream_sink::StreamSink;
pub use self::transaction::Transaction;

#[cfg(test)]
mod tests;
