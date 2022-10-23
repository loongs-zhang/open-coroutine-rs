use core::fmt;
use std::fmt::{Debug, Formatter};
use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use memory_pool::memory::Memory;

/// A `Context` stores a `ContextFn`'s state of execution, for it to be resumed later.
///
/// If we have 2 or more `Context` instances, we can thus easily "freeze" the
/// current state of execution and explicitely switch to another `Context`.
/// This `Context` is then resumed exactly where it left of and
/// can in turn "freeze" and switch to another `Context`.
///
/// # Examples
///
/// See [examples/basic.rs](https://github.com/zonyitoo/context-rs/blob/master/examples/basic.rs)
// The reference is using 'static because we can't possibly imply the
// lifetime of the Context instances returned by resume() anyways.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Context(pub(crate) &'static c_void);

impl Debug for Context {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Context({:p})", self.0)
    }
}

// NOTE: Rustc is kinda dumb and introduces a overhead of up to 500% compared to the asm methods
//       if we don't explicitely inline them or use LTO (e.g.: 3ns/iter VS. 18ns/iter on i7 3770).
impl Context {
    /// Creates a new `Context` prepared to execute `f` at the beginning of `stack`.
    ///
    /// `f` is not executed until the first call to `resume()`.
    ///
    /// It is unsafe because it only takes a reference of `Stack`. You have to make sure the
    /// `Stack` lives longer than the generated `Context`.
    #[inline(always)]
    pub(crate) fn new(stack: ManuallyDrop<Memory>, f: ContextFn) -> Context {
        Context(unsafe { make_fcontext(stack.top(), stack.len(), f) })
    }

    /// Yields the execution to another `Context`.
    ///
    /// The exact behaviour of this method is implementation defined, but the general mechanism is:
    /// The current state of execution is preserved somewhere and the previously saved state
    /// in the `Context` pointed to by `self` is restored and executed next.
    ///
    /// This behaviour is similiar in spirit to regular function calls with the difference
    /// that the call to `resume()` only returns when someone resumes the caller in turn.
    ///
    /// The returned `Transfer` struct contains the previously active `Context` and
    /// the `data` argument used to resume the current one.
    ///
    /// It is unsafe because it is your responsibility to make sure that all data that constructed in
    /// this context have to be dropped properly when the last context is dropped.
    #[inline(always)]
    pub(crate) fn resume(self, data: *mut c_void) -> Transfer {
        unsafe { jump_fcontext(self.0, data) }
    }

    #[inline(always)]
    pub(crate) fn switch(to: &Context, param: *mut c_void) -> Transfer {
        unsafe { jump_fcontext(to.0, param) }
    }
}

/// Contains the previously active `Context` and the `data` passed to resume the current one and
/// is used as the return value by `Context::resume()` and `Context::switch()`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Transfer {
    /// The previously executed `Context` which yielded to resume the current one.
    pub context: Context,

    /// The `data` which was passed to `Context::resume()` or
    /// `Context::resume_ontop()` to resume the current `Context`.
    pub data: *mut c_void,
}

impl Transfer {
    /// Returns a new `Transfer` struct with the members set to their respective arguments.
    #[inline(always)]
    pub fn new(context: Context, data: *mut c_void) -> Transfer {
        Transfer {
            context,
            data,
        }
    }

    pub fn resume(self, data: *mut c_void) -> Transfer {
        self.context.resume(data)
    }

    pub fn switch(to: &Transfer) -> Transfer {
        Context::switch(&to.context, to.data)
    }
}

/// Functions of this signature are used as the entry point for a new `Context`.
pub type ContextFn = extern "C" fn(t: Transfer);

extern "C" {
    /// Creates a new `Context` ontop of some stack.
    ///
    /// # Arguments
    /// * `sp`   - A pointer to the bottom of the stack.
    /// * `size` - The size of the stack.
    /// * `f`    - A function to be invoked on the first call to jump_fcontext(this, _).
    #[inline(never)]
    #[allow(unused)]
    fn make_fcontext(sp: *mut c_void, size: usize, f: ContextFn) -> &'static c_void;

    /// Yields the execution to another `Context`.
    ///
    /// # Arguments
    /// * `to` - A pointer to the `Context` with whom we swap execution.
    /// * `param`  - An arbitrary argument that will be set as the `data` field
    ///          of the `Transfer` object passed to the other Context.
    #[inline(never)]
    #[allow(unused)]
    fn jump_fcontext(to: &'static c_void, param: *mut c_void) -> Transfer;
}

#[cfg(test)]
mod tests {
    use std::mem::ManuallyDrop;
    use std::os::raw::c_void;
    use memory_pool::memory::Memory;
    use crate::context::{Context, Transfer};

    // This method will always `resume()` immediately back to the
    // previous `Context` with a `data` value of the next number in the fibonacci sequence.
    // You could thus describe this method as a "fibonacci sequence generator".
    extern "C" fn context_function(mut t: Transfer) {
        let mut a = 0usize;
        let mut b = 1usize;

        loop {
            print!("Yielding {} => ", a);
            t.data = a as *mut c_void;
            t = Transfer::switch(&t);

            let next = a + b;
            a = b;
            b = next;
        }
    }

    #[test]
    fn test() {
        println!("inner context test started !");
        // Allocate some stack.
        let stack = Memory::default();

        let context = Context::new(ManuallyDrop::new(stack), context_function);
        // Allocate a Context on the stack.
        let mut t = Transfer::new(context, 0 as *mut c_void);

        // Yield 10 times to `context_function()`.
        for _ in 0..10 {
            // Yield to the "frozen" state of `context_function()`.
            // The `data` value is not used in this example and is left at 0.
            // The first and every other call will return references to the actual `Context` data.
            print!("Resuming => ");
            t.data = 0 as *mut c_void;
            t = Transfer::switch(&t);

            println!("Got {}", t.data as usize);
        }

        println!("inner context test finished!");
    }
}