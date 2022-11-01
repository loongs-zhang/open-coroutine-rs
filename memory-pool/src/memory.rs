// Copyright 2016 coroutine-rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::os::raw::c_void;

use crate::system;

/// Error type returned by stack allocation methods.
#[derive(Debug)]
pub enum MemoryError {
    /// Contains the maximum amount of memory allowed to be allocated as stack space.
    ExceedsMaximumSize(usize),

    /// Returned if some kind of I/O error happens during allocation.
    IoError(io::Error),
}

impl Display for MemoryError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            MemoryError::ExceedsMaximumSize(size) => {
                write!(
                    fmt,
                    "Requested more than max size of {} bytes for a stack",
                    size
                )
            }
            MemoryError::IoError(ref e) => e.fmt(fmt),
        }
    }
}

impl Error for MemoryError {
    fn description(&self) -> &str {
        match *self {
            MemoryError::ExceedsMaximumSize(_) => "exceeds maximum stack size",
            MemoryError::IoError(ref e) =>
            {
                #[allow(deprecated)]
                e.description()
            }
        }
    }
    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            MemoryError::ExceedsMaximumSize(_) => None,
            MemoryError::IoError(ref e) => Some(e),
        }
    }
}

/// Represents any kind of stack memory.
///
/// `FixedSizeStack` as well as `ProtectedFixedSizeStack`
/// can be used to allocate actual stack space.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Memory {
    top: *mut c_void,
    bottom: *mut c_void,
    protected: bool,
}

unsafe impl Sync for Memory {}

impl Memory {
    /// Allocates a new stack of **at least** `size` bytes + one additional guard page.
    ///
    /// `size` is rounded up to a multiple of the size of a memory page and
    /// does not include the size of the guard page itself.
    pub fn new(size: usize) -> Result<Memory, MemoryError> {
        Memory::allocate(size, true)
    }

    /// Allocates a new stack of `size`.
    fn allocate(mut size: usize, protected: bool) -> Result<Memory, MemoryError> {
        let page_size = system::page_size();
        let min_stack_size = system::min_size();
        let max_stack_size = system::max_size(false);
        let add_shift = if protected { 1 } else { 0 };
        let add = page_size << add_shift;
        if size < min_stack_size {
            size = min_stack_size;
        }
        size = (size - 1) & !(page_size - 1);
        if let Some(size) = size.checked_add(add) {
            if size <= max_stack_size {
                let mut ret = unsafe { system::allocate(size) };
                if protected {
                    if let Ok(stack) = ret {
                        ret = unsafe { system::protect(&stack) };
                    }
                }
                return ret.map_err(MemoryError::IoError);
            }
        }
        Err(MemoryError::ExceedsMaximumSize(max_stack_size - add))
    }

    /// Creates a (non-owning) representation of some stack memory.
    ///
    /// It is unsafe because it is your reponsibility to make sure that `top` and `buttom` are valid
    /// addresses.
    #[inline]
    pub(crate) unsafe fn init(top: *mut c_void, bottom: *mut c_void, protected: bool) -> Memory {
        debug_assert!(top >= bottom);
        Memory {
            top,
            bottom,
            protected,
        }
    }

    /// Returns the top of the stack from which on it grows downwards towards bottom().
    #[inline]
    pub fn top(&self) -> *mut c_void {
        self.top
    }

    /// Returns the bottom of the stack and thus it's end.
    #[inline]
    pub fn bottom(&self) -> *mut c_void {
        self.bottom
    }

    #[inline]
    pub fn is_protected(&self) -> bool {
        self.protected
    }

    /// Returns the size of the stack between top() and bottom().
    #[inline]
    pub fn len(&self) -> usize {
        self.top as usize - self.bottom as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the minimal stack size allowed by the current platform.
    #[inline]
    pub fn min_size() -> usize {
        system::min_size()
    }

    /// Returns the maximum stack size allowed by the current platform.
    #[inline]
    pub fn max_size(&self) -> usize {
        system::max_size(self.protected)
    }

    /// Returns a implementation defined default stack size.
    ///
    /// This value can vary greatly between platforms, but is usually only a couple
    /// memory pages in size and enough for most use-cases with little recursion.
    /// It's usually a better idea to specifiy an explicit stack size instead.
    #[inline]
    pub fn default_size(&self) -> usize {
        system::default_size(self.protected)
    }

    pub fn drop(&self) {
        let mut ptr = self.bottom();
        let mut size = self.len();
        if self.protected {
            let page_size = system::page_size();
            ptr = (self.bottom() as usize - page_size) as *mut c_void;
            size = self.len() + page_size;
        }
        unsafe {
            system::deallocate(ptr, size);
        }
    }
}

unsafe impl Send for Memory {}

impl Default for Memory {
    fn default() -> Self {
        Memory::new(system::default_size(true))
            .unwrap_or_else(|err| panic!("Failed to allocate Memory with {:?}", err))
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::write_bytes;

    use super::*;
    use system;

    #[test]
    fn stack_size_too_small() {
        let stack = Memory::new(0).unwrap();
        assert_eq!(stack.len(), system::min_size());

        unsafe { write_bytes(stack.bottom() as *mut u8, 0x1d, stack.len()) };

        let stack = Memory::new(0).unwrap();
        assert_eq!(stack.len(), system::min_size());

        unsafe { write_bytes(stack.bottom() as *mut u8, 0x1d, stack.len()) };
        stack.drop();
    }

    #[test]
    fn stack_size_too_large() {
        let stack_size = system::max_size(true);
        match Memory::allocate(stack_size, true) {
            Err(MemoryError::ExceedsMaximumSize(_)) => panic!(),
            _ => {}
        }
        let stack_size = stack_size + 1;
        match Memory::allocate(stack_size, true) {
            Err(MemoryError::ExceedsMaximumSize(..)) => {}
            _ => panic!(),
        }

        let stack_size = system::max_size(false);
        match Memory::allocate(stack_size, false) {
            Err(MemoryError::ExceedsMaximumSize(_)) => panic!(),
            _ => {}
        }
        let stack_size = stack_size + 1;
        match Memory::allocate(stack_size, false) {
            Err(MemoryError::ExceedsMaximumSize(..)) => {}
            _ => panic!(),
        }
    }

    #[test]
    fn clone() {
        let size = system::min_size();
        let stack = Memory::new(size).unwrap();
        assert_eq!(stack.len(), size);
        let clone = stack.clone();
        assert_eq!(stack.len(), clone.len());
        assert_eq!(stack.is_protected(), clone.is_protected());
        assert_eq!(stack.top(), clone.top());
        assert_eq!(stack.bottom(), clone.bottom());
        assert_eq!(stack, clone);
    }
}
