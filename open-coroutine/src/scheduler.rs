use crate::coroutine::{Coroutine, Status};
use id_generator::IdGenerator;
use object_list::ObjectList;
use once_cell::sync::Lazy;
use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use std::ptr;
use std::time::Duration;
use timer::TimerList;

static mut GLOBAL: Lazy<ManuallyDrop<Scheduler>> =
    Lazy::new(|| ManuallyDrop::new(Scheduler::new()));

thread_local! {
    static SCHEDULER: Box<Scheduler> = Box::new(Scheduler::new());
}

#[repr(C)]
#[derive(Debug)]
pub struct Scheduler {
    id: usize,
    ready: ObjectList,
    //正在执行的协程id
    running: Option<usize>,
    suspend: TimerList,
    //not support for now
    #[allow(unused)]
    system_call: ObjectList,
    //not support for now
    #[allow(unused)]
    copy_stack: ObjectList,
    finished: ObjectList,
}

impl PartialEq for Scheduler {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

unsafe impl Send for Scheduler {}

unsafe impl Sync for Scheduler {}

impl Scheduler {
    fn new() -> Self {
        //构造
        Scheduler {
            id: IdGenerator::next_id("scheduler"),
            ready: ObjectList::new(),
            running: None,
            suspend: TimerList::new(),
            system_call: ObjectList::new(),
            copy_stack: ObjectList::new(),
            finished: ObjectList::new(),
        }
    }

    pub fn global() -> &'static mut ManuallyDrop<Scheduler> {
        unsafe { &mut GLOBAL }
    }

    pub fn current<'a>() -> &'a mut Scheduler {
        SCHEDULER.with(|boxed| Box::leak(unsafe { ptr::read_unaligned(boxed) }))
    }

    pub fn submit(
        &mut self,
        mut coroutine: Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
    ) {
        let time = coroutine.get_execute_time();
        coroutine.set_scheduler(self);
        if timer::now() < time {
            coroutine.set_execute_time(time).set_status(Status::Suspend);
            self.suspend.insert(time, coroutine);
            return;
        }
        coroutine.set_status(Status::Ready);
        self.ready.push_back(coroutine);
    }

    pub fn execute(
        &mut self,
        mut coroutine: Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
    ) {
        let time = coroutine.get_execute_time();
        if timer::now() < time {
            self.execute_at(time, coroutine);
            return;
        }
        coroutine.set_status(Status::Ready);
        coroutine.set_scheduler(self);
        self.ready.push_back(coroutine);
    }

    pub fn delay(
        &mut self,
        delay: Duration,
        coroutine: Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
    ) {
        let time = timer::get_timeout_time(delay);
        self.execute_at(time, coroutine)
    }

    pub fn execute_at(
        &mut self,
        time: u64,
        mut coroutine: Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
    ) {
        coroutine.set_execute_time(time).set_status(Status::Suspend);
        coroutine.set_scheduler(self);
        self.suspend.insert(time, coroutine);
    }

    pub fn try_timed_schedule(&mut self, timeout: Duration) -> ObjectList {
        let timeout_time = timer::get_timeout_time(timeout);
        let mut scheduled = ObjectList::new();
        while !self.suspend.is_empty() || !self.ready.is_empty() {
            if timeout_time <= timer::now() {
                break;
            }
            let mut temp = self.try_schedule();
            while !temp.is_empty() {
                scheduled.push_back_raw(temp.pop_front_raw().unwrap());
            }
        }
        scheduled
    }

    pub fn try_schedule(&mut self) -> ObjectList {
        self.check_ready();
        self.mark_entrance();
        self.do_schedule()
    }

    fn mark_entrance(&mut self) {
        if let Some(front_pointer) = self.ready.front_mut_raw() {
            let front = unsafe {
                ptr::read_unaligned(
                    front_pointer
                        as *mut Coroutine<dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
                )
            };
            if let Some(back_pointer) = self.ready.back_mut_raw() {
                if front_pointer != back_pointer {
                    let mut back = unsafe {
                        ptr::read_unaligned(
                            back_pointer
                                as *mut Coroutine<
                                    dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>,
                                >,
                        )
                    };
                    back.set_entrance(&front);
                    std::mem::forget(back);
                }
            }
            std::mem::forget(front);
        }
    }

    fn do_schedule(&mut self) -> ObjectList {
        let mut scheduled = ObjectList::new();
        for _ in 0..self.ready.len() {
            if let Some(pointer) = self.ready.pop_front_raw() {
                let mut coroutine = unsafe {
                    ptr::read_unaligned(
                        pointer
                            as *mut Coroutine<
                                dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>,
                            >,
                    )
                };
                //fixme 这里拿到的时间不对
                let exec_time = coroutine.get_execute_time();
                //过滤未到执行时间的协程
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
                scheduled.push_back(unsafe { ptr::read_unaligned(&result) });
                self.finished.push_back(result);
                coroutine.exit();
                std::mem::forget(coroutine);
            }
        }
        scheduled
    }

    fn check_ready(&mut self) {
        for _ in 0..self.suspend.len() {
            if let Some(entry) = self.suspend.front() {
                let exec_time = entry.get_time();
                if timer::now() < exec_time {
                    break;
                }
                //移动至"就绪"队列
                if let Some(mut entry) = self.suspend.pop_front() {
                    for _ in 0..entry.len() {
                        if let Some(pointer) = entry.pop_front_raw() {
                            unsafe {
                                let mut coroutine = ptr::read_unaligned(
                                    pointer
                                        as *mut Coroutine<
                                            dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>,
                                        >,
                                );
                                coroutine.set_status(Status::Ready);
                                //优先执行到时间的协程
                                self.ready.push_front(coroutine)
                            }
                        }
                    }
                }
            }
        }
    }

    //todo 提供一个block版，如果suspend和ready没有，则把自己挂起
    pub fn schedule(&mut self) -> ObjectList {
        let mut scheduled = ObjectList::new();
        while !self.suspend.is_empty() || !self.ready.is_empty() {
            let mut temp = self.try_schedule();
            while !temp.is_empty() {
                scheduled.push_back_raw(temp.pop_front_raw().unwrap());
            }
        }
        scheduled
    }

    pub fn get_ready(&self) -> &ObjectList {
        &self.ready
    }

    pub fn get_finished(&self) -> &ObjectList {
        &self.finished
    }
}

#[cfg(test)]
mod tests {
    use crate::coroutine::Coroutine;
    use crate::scheduler::Scheduler;
    use std::os::raw::c_void;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn simple() {
        let x = 1;
        let y = 2;
        let mut scheduler = Scheduler::new();
        scheduler.execute(Coroutine::new(
            2048,
            |param| {
                print!("env {} ", x);
                match param {
                    Some(param) => {
                        println!("coroutine1 {}", param as usize);
                    }
                    None => {
                        println!("coroutine1 no param");
                    }
                }
                param
            },
            Some(1usize as *mut c_void),
        ));
        scheduler.execute(Coroutine::new(
            2048,
            |param| {
                print!("env {} ", y);
                match param {
                    Some(param) => {
                        println!("coroutine2 {}", param as usize);
                    }
                    None => {
                        println!("coroutine2 no param");
                    }
                }
                param
            },
            Some(2usize as *mut c_void),
        ));
        scheduler.try_schedule();
    }

    #[test]
    fn test() {
        let mut scheduler = Scheduler::new();
        let coroutine = Coroutine::new(
            2048,
            |param| {
                match param {
                    Some(param) => {
                        println!("user_function1 {}", param as usize);
                    }
                    None => {
                        println!("user_function1 no param");
                    }
                }
                param
            },
            Some(1usize as *mut c_void),
        );
        scheduler.delay(Duration::from_millis(500), coroutine);
        assert_eq!(0, scheduler.try_schedule().len());
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(1, scheduler.suspend.len());
        let entry = scheduler.suspend.front().unwrap();
        assert_eq!(1, entry.len());

        scheduler.execute(Coroutine::new(
            2048,
            |param| {
                match param {
                    Some(param) => {
                        println!("user_function2 {}", param as usize);
                    }
                    None => {
                        println!("user_function2 no param");
                    }
                }
                param
            },
            Some(2usize as *mut c_void),
        ));
        assert_eq!(1, scheduler.try_schedule().len());

        //往下睡500+ms，才会轮询到
        thread::sleep(Duration::from_millis(501));
        assert_eq!(1, scheduler.try_schedule().len());
        assert_eq!(0, scheduler.ready.len());
        assert_eq!(0, scheduler.suspend.len());
    }

    #[test]
    fn schedule() {
        let mut scheduler = Scheduler::new();
        scheduler.delay(
            Duration::from_millis(500),
            Coroutine::new(
                2048,
                |param| {
                    println!("coroutine1");
                    param
                },
                None,
            ),
        );
        scheduler.execute(Coroutine::new(
            2048,
            |param| {
                println!("coroutine2");
                param
            },
            None,
        ));
        assert_eq!(2, scheduler.schedule().len());
    }

    #[test]
    fn delay() {
        let mut scheduler = Scheduler::new();
        let mut coroutine = Coroutine::new(
            2048,
            |param| {
                println!("coroutine1");
                param
            },
            None,
        );
        coroutine.set_delay(Duration::from_millis(500));
        scheduler.execute(coroutine);
        assert_eq!(0, scheduler.try_schedule().len());
    }

    #[test]
    fn try_schedule() {
        let mut scheduler = Scheduler::new();
        scheduler.delay(
            Duration::from_millis(500),
            Coroutine::new(
                2048,
                |param| {
                    println!("coroutine1");
                    param
                },
                None,
            ),
        );
        assert_eq!(0, scheduler.try_schedule().len());
    }

    #[test]
    fn schedule_with_timeout() {
        let mut scheduler = Scheduler::new();
        scheduler.delay(
            Duration::from_millis(500),
            Coroutine::new(2048, |param| param, None),
        );
        assert_eq!(
            0,
            scheduler
                .try_timed_schedule(Duration::from_millis(10))
                .len()
        );
    }

    #[test]
    fn global() {
        let scheduler1 = Scheduler::global();
        scheduler1.execute(Coroutine::new(
            2048,
            |param| param,
            Some(2usize as *mut c_void),
        ));
        let scheduler2 = Scheduler::global();
        assert_eq!(scheduler1, scheduler2);
    }

    #[test]
    fn current() {
        let scheduler1 = Scheduler::current();
        scheduler1.execute(Coroutine::new(
            2048,
            |param| param,
            Some(2usize as *mut c_void),
        ));
        let scheduler2 = Scheduler::current();
        assert_eq!(scheduler1, scheduler2);
    }
}
