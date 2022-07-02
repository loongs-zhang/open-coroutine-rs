use std::collections::LinkedList;
use std::os::raw::c_void;
use std::ptr;
use crate::coroutine::{Coroutine, Status};

pub struct Scheduler<F> {
    ready: LinkedList<Coroutine<F>>,
    suspend: LinkedList<Coroutine<F>>,
    //not support for now
    system_call: LinkedList<Coroutine<F>>,
    //not support for now
    copy_stack: LinkedList<Coroutine<F>>,
    finished: LinkedList<Coroutine<F>>,
}

impl<F> Scheduler<F>
    where F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void>
{
    pub fn new() -> Self {
        Scheduler {
            ready: LinkedList::new(),
            suspend: LinkedList::new(),
            system_call: LinkedList::new(),
            copy_stack: LinkedList::new(),
            finished: LinkedList::new(),
        }
    }

    pub fn offer(&mut self, mut coroutine: Coroutine<F>) {
        coroutine.set_status(Status::Ready);
        self.ready.push_back(coroutine);
    }

    pub fn schedule(&mut self) {
        unsafe {
            for _ in 0..self.ready.len() {
                match self.ready.pop_front() {
                    Some(coroutine) => {
                        coroutine.resume();
                        //移动至"已完成"队列
                        unsafe { self.finished.push_front(coroutine); }
                    }
                    None => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::os::raw::c_void;
    use crate::coroutine::Coroutine;
    use crate::scheduler::Scheduler;
    use crate::stack::ProtectedFixedSizeStack;

    lazy_static! {
        static ref STACK1: ProtectedFixedSizeStack = ProtectedFixedSizeStack::new(2048).expect("allocate stack failed !");
        static ref STACK2: ProtectedFixedSizeStack = ProtectedFixedSizeStack::new(2048).expect("allocate stack failed !");
    }

    #[test]
    fn test() {
        let mut scheduler = Scheduler::new();
        let closure = |param| {
            match param {
                Some(param) => {
                    println!("user_function {}", param as usize);
                }
                None => {
                    println!("user_function no param");
                }
            }
            param
        };
        scheduler.offer(Coroutine::new(&STACK1, closure, Some(1usize as *mut c_void)));
        scheduler.schedule();
        scheduler.offer(Coroutine::new(&STACK2, closure, Some(2usize as *mut c_void)));
        scheduler.schedule();
    }
}