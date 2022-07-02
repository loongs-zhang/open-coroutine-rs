use std::borrow::Borrow;
use std::collections::{BTreeMap, LinkedList};
use std::hash::Hash;
use std::os::raw::c_void;
use std::ptr;
use crate::coroutine::{Coroutine, Status};
use crate::timer;

pub struct Scheduler<F> {
    ready: LinkedList<Coroutine<F>>,
    running: Option<Coroutine<F>>,
    //todo 重构时间轮，只要list就可以了
    suspend: BTreeMap<u64, LinkedList<Coroutine<F>>>,
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
            running: None,
            suspend: BTreeMap::new(),
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
            for (exec_time, list) in self.suspend.iter_mut() {
                if timer::now() < *exec_time {
                    break;
                }
                //移动至"就绪"队列
                for _ in 0..list.len() {
                    match list.pop_front() {
                        Some(coroutine) => self.ready.push_front(coroutine),
                        None => {}
                    }
                }
                //todo 清理空list的entry
            }
            for _ in 0..self.ready.len() {
                match self.ready.pop_front() {
                    Some(coroutine) => {
                        let exec_time = coroutine.get_execute_time();
                        if timer::now() < exec_time {
                            //移动至"挂起"队列
                            self.suspend.entry(exec_time)
                                .or_insert(LinkedList::new())
                                .push_back(coroutine);
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
        let (time, list) = scheduler.suspend.iter().next().unwrap();
        assert_eq!(1, list.len());

        scheduler.offer(Coroutine::new(&STACK2, closure, Some(2usize as *mut c_void)));
        scheduler.schedule();

        //往下睡1s，才会轮询到
        thread::sleep(Duration::from_millis(1000));
        scheduler.schedule();
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(1, scheduler.suspend.len());
        let (time, list) = scheduler.suspend.iter().next().unwrap();
        assert_eq!(0, list.len());
    }
}