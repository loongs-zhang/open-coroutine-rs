use std::borrow::Borrow;
use std::collections::{BTreeMap, VecDeque};
use std::hash::Hash;
use std::os::raw::c_void;
use std::ptr;
use crate::coroutine::{Coroutine, Status};
use crate::timer;
use crate::timer::{TimerEntry, TimerList};

/// todo 用ObjectList代替VecDeque
pub struct Scheduler<F> {
    ready: VecDeque<Coroutine<F>>,
    //正在执行的协程id
    running: Option<usize>,
    suspend: TimerList,
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

    pub fn try_schedule(&mut self) -> VecDeque<Coroutine<F>> {
        let mut queue = VecDeque::new();
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
                    Some(mut coroutine) => {
                        let exec_time = coroutine.get_execute_time();
                        if timer::now() < exec_time {
                            //移动至"挂起"队列
                            self.suspend.insert(exec_time, coroutine);
                            continue;
                        }
                        self.running = Some(coroutine.get_id());
                        let result = coroutine.resume();
                        //移动至"已完成"队列
                        queue.push_back(ptr::read(&result));
                        self.finished.push_back(result);
                    }
                    None => {}
                }
            }
            queue
        }
    }

    pub fn get_finished(&self) -> &VecDeque<Coroutine<F>> {
        &self.finished
    }
}

#[cfg(test)]
mod tests {
    use std::{mem, thread};
    use std::os::raw::c_void;
    use std::time::Duration;
    use memory_pool::memory::Memory;
    use crate::coroutine::{Coroutine, Status};
    use crate::scheduler::Scheduler;

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
        let mut coroutine = Coroutine::new(2048, closure, Some(1usize as *mut c_void));
        coroutine.set_delay(Duration::from_millis(500))
            .set_status(Status::Suspend);
        scheduler.offer(coroutine);
        scheduler.try_schedule();
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(1, scheduler.suspend.len());
        let entry = scheduler.suspend.front().unwrap();
        assert_eq!(1, entry.len());

        scheduler.offer(Coroutine::new(2048, closure, Some(2usize as *mut c_void)));
        for co in scheduler.try_schedule() {
            match co.get_result() {
                Some(data) => {
                    println!("{}", data as usize)
                }
                None => {}
            }
        }

        //往下睡500+ms，才会轮询到
        thread::sleep(Duration::from_millis(501));
        for co in scheduler.try_schedule() {
            match co.get_result() {
                Some(data) => {
                    println!("{}", data as usize)
                }
                None => {}
            }
        }
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(0, scheduler.suspend.len());
    }
}