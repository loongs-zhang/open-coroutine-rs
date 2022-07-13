use std::borrow::Borrow;
use std::collections::{BTreeMap, VecDeque};
use std::hash::Hash;
use std::os::raw::c_void;
use std::ptr;
use object_list::ObjectList;
use crate::coroutine::{Coroutine, Status};
use crate::timer;
use crate::timer::{TimerEntry, TimerList};

pub struct Scheduler {
    ready: ObjectList,
    //正在执行的协程id
    running: Option<usize>,
    suspend: TimerList,
    //not support for now
    system_call: ObjectList,
    //not support for now
    copy_stack: ObjectList,
    finished: ObjectList,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            ready: ObjectList::new(),
            running: None,
            suspend: TimerList::new(),
            system_call: ObjectList::new(),
            copy_stack: ObjectList::new(),
            finished: ObjectList::new(),
        }
    }

    pub fn offer(&mut self, mut coroutine: Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>) {
        coroutine.set_status(Status::Ready);
        self.ready.push_back(coroutine);
    }

    pub fn try_schedule(&mut self) -> ObjectList {
        let mut queue = ObjectList::new();
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
                                    match entry.pop_front_raw() {
                                        Some(pointer) => {
                                            let mut coroutine = ptr::read(pointer as
                                                *mut Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>);
                                            coroutine.set_status(Status::Ready);
                                            self.ready.push_back(coroutine)
                                        }
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
                match self.ready.pop_front_raw() {
                    Some(mut pointer) => {
                        let mut coroutine = ptr::read(pointer as
                            *mut Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>);
                        //fixme 这里拿到的时间不对
                        let exec_time = coroutine.get_execute_time();
                        if timer::now() < exec_time {
                            //设置协程状态
                            coroutine.set_status(Status::Suspend);
                            //移动至"挂起"队列
                            self.suspend.insert(exec_time, coroutine);
                            continue;
                        }
                        self.running = Some(coroutine.get_id());
                        let result = coroutine.resume();
                        self.running = None;
                        //移动至"已完成"队列
                        queue.push_back(ptr::read(&result));
                        self.finished.push_back(result);
                        coroutine.exit();
                        //fixme 修复内存泄漏问题
                        std::mem::forget(coroutine);
                    }
                    None => {}
                }
            }
            queue
        }
    }

    pub fn get_finished(&self) -> &ObjectList {
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
    fn simple() {
        let x = 1;
        let y = 2;
        let mut scheduler = Scheduler::new();
        scheduler.offer(Coroutine::new(2048, |param| {
            println!("\nenv {}", x);
            match param {
                Some(param) => {
                    println!("coroutine1 {}", param as usize);
                }
                None => {
                    println!("coroutine1 no param");
                }
            }
            param
        }, Some(1usize as *mut c_void)));
        scheduler.offer(Coroutine::new(2048, |param| {
            println!("\nenv {}", y);
            match param {
                Some(param) => {
                    println!("coroutine2 {}", param as usize);
                }
                None => {
                    println!("coroutine2 no param");
                }
            }
            param
        }, Some(2usize as *mut c_void)));
        scheduler.try_schedule();
    }

    #[test]
    fn test() {
        let mut scheduler = Scheduler::new();
        let mut coroutine = Coroutine::new(2048, |param| {
            match param {
                Some(param) => {
                    println!("user_function {}", param as usize);
                }
                None => {
                    println!("user_function no param");
                }
            }
            param
        }, Some(1usize as *mut c_void));
        coroutine.set_delay(Duration::from_millis(500));
        scheduler.offer(coroutine);
        scheduler.try_schedule();
        assert_eq!(0, scheduler.ready.len());
        //fixme
        assert_eq!(1, scheduler.suspend.len());
        let entry = scheduler.suspend.front().unwrap();
        assert_eq!(1, entry.len());

        scheduler.offer(Coroutine::new(2048, |param| {
            match param {
                Some(param) => {
                    println!("user_function {}", param as usize);
                }
                None => {
                    println!("user_function no param");
                }
            }
            param
        }, Some(2usize as *mut c_void)));
        scheduler.try_schedule();

        //往下睡500+ms，才会轮询到
        thread::sleep(Duration::from_millis(501));
        scheduler.try_schedule();
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(0, scheduler.suspend.len());
    }
}