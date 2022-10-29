// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::{
    allocate,
    deallocate,
    max_size,
    min_size,
    page_size,
    protect,
};

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::{
    allocate,
    deallocate,
    max_size,
    min_size,
    page_size,
    protect,
};

pub fn default_size(protected: bool) -> usize {
    let size = self::min_size() * 8;
    let max_stack_size = self::max_size(protected);
    size.min(max_stack_size)
}
