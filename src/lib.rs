#[macro_use]
extern crate lazy_static;

pub mod scheduler;

pub mod coroutine;

/// 仅限框架内部使用的context
pub(crate) mod context;

/// Provides utilities to allocate memory suitable as stack memory for `Context`.
/// todo 用bumpalo替换掉现在的实现
pub mod stack;

mod sys;