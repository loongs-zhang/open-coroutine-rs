use std::borrow::Borrow;
use std::collections::{BTreeMap, VecDeque};
use std::hash::Hash;
use std::os::raw::c_void;
use std::ptr;
use crate::coroutine::{Coroutine, Status};
use crate::timer;
use crate::timer::{TimerEntry, TimerList};

pub struct Scheduler<F> {
    ready: VecDeque<Coroutine<F>>,
    running: Option<Coroutine<F>>,
    suspend: TimerList<Coroutine<F>>,
    //not support for now
    system_call: VecDeque<Coroutine<F>>,
    //not support for now
    copy_stack: VecDeque<Coroutine<F>>,
    finished: VecDeque<Coroutine<F>>,
}

impl<F> Scheduler<F>
    where F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void>
{
    pub fn new() -> Self {
        Scheduler {
            ready: VecDeque::new(),
            running: None,
            suspend: TimerList::new(),
            system_call: VecDeque::new(),
            copy_stack: VecDeque::new(),
            finished: VecDeque::new(),
        }
    }

    pub fn offer(&mut self, mut coroutine: Coroutine<F>) {
        coroutine.set_status(Status::Ready);
        self.ready.push_back(coroutine);
    }

    pub fn schedule(&mut self) {
        unsafe {
            for _ in 0..self.suspend.len() {
                match self.suspend.front() {
                    Some(entry) => {
                        let exec_time = entry.get_time();
                        if timer::now() < exec_time {
                            break;
                        }
                        //移动至"就绪"队列
                        match self.suspend.pop_front() {
                            Some(mut entry) => {
                                for _ in 0..entry.len() {
                                    match entry.pop_front() {
                                        Some(coroutine) => self.ready.push_back(coroutine),
                                        None => {}
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                    None => {}
                }
            }
            for _ in 0..self.ready.len() {
                match self.ready.pop_front() {
                    Some(coroutine) => {
                        let exec_time = coroutine.get_execute_time();
                        if timer::now() < exec_time {
                            //移动至"挂起"队列
                            self.suspend.insert(exec_time,coroutine);
                            continue;
                        }
                        self.running = Some(ptr::read(&coroutine));
                        coroutine.resume();
                        //移动至"已完成"队列
                        self.finished.push_back(coroutine);
                    }
                    None => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{mem, thread};
    use std::os::raw::c_void;
    use std::time::Duration;
    use crate::coroutine::{Coroutine, Status};
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
        let mut coroutine = Coroutine::new(&STACK1, closure, Some(1usize as *mut c_void));
        coroutine.set_delay(Duration::from_millis(500))
            .set_status(Status::Suspend);
        scheduler.offer(coroutine);
        scheduler.schedule();
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(1, scheduler.suspend.len());
        let entry = scheduler.suspend.front().unwrap();
        assert_eq!(1, entry.len());

        scheduler.offer(Coroutine::new(&STACK2, closure, Some(2usize as *mut c_void)));
        scheduler.schedule();

        //往下睡1s，才会轮询到
        thread::sleep(Duration::from_millis(1000));
        scheduler.schedule();
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(0, scheduler.suspend.len());
    }
}