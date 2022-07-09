#[macro_use]
extern crate lazy_static;

pub mod scheduler;

pub mod timer;

pub mod coroutine;

/// 仅限框架内部使用的context
pub(crate) mod context;