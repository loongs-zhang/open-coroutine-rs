use crate::context::{Context, Transfer};
use crate::scheduler::Scheduler;
use id_generator::IdGenerator;
use memory_pool::memory::Memory;
use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use std::time::Duration;
use std::{ptr, thread};

#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Status {
    ///协程被创建
    Created,
    ///等待运行
    Ready,
    ///运行中
    Running,
    ///被挂起
    Suspend,
    ///执行系统调用
    SystemCall,
    ///栈扩/缩容时
    CopyStack,
    ///调用用户函数完成，但未退出
    Finished,
    ///已退出
    Exited,
}

#[repr(C)]
#[derive(Debug)]
pub struct Coroutine<F: ?Sized> {
    id: usize,
    stack: ManuallyDrop<Memory>,
    sp: Transfer,
    status: Status,
    //用户函数
    proc: Box<F>,
    //调用用户函数的参数
    param: Option<*mut c_void>,
    //调用用户函数的结果
    result: Option<*mut c_void>,
    //下一次应该执行协程体的时间
    exec_time: u64,
    //下一个执行的协程
    next: Option<*mut c_void>,
    //入口协程，也是出口
    entrance: Option<*mut c_void>,
    scheduler: Option<*mut Scheduler>,
}

impl<F> Coroutine<F>
where
    F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void> + Sized,
{
    extern "C" fn coroutine_function(mut t: Transfer) {
        unsafe {
            loop {
                let context = t.data as *mut Coroutine<F>;
                if timer::now() < (*context).exec_time {
                    //优先继续执行其他协程，否则让出CPU时间片
                    match (*context).scheduler {
                        Some(scheduler) => {
                            (*scheduler).try_schedule();
                        }
                        None => thread::yield_now(),
                    }
                    continue;
                }
                //设置协程状态为运行中
                (*context).status = Status::Running;
                let param = (*context).param as Option<*mut c_void>;
                match param {
                    Some(data) => {
                        print!("coroutine_function {} => ", data as usize)
                    }
                    None => {
                        print!("coroutine_function no param => ")
                    }
                }
                //调用用户函数
                let result = (*context).invoke();
                match (*context).next {
                    Some(pointer) => {
                        //不回跳，继续执行下一个指定的协程
                        let next = pointer
                            as *mut Coroutine<
                                dyn FnOnce(Option<*mut c_void>) -> Option<*mut c_void>,
                            >;
                        std::mem::forget((*next).resume());
                    }
                    None => {
                        match (*context).entrance {
                            Some(pointer) => {
                                //跳回主线程
                                t = t.resume(pointer);
                            }
                            None => {
                                //不回跳，直接执行下一个协程
                                if let Some(scheduler) = (*context).scheduler {
                                    (*scheduler).try_schedule();
                                }
                                //如果没有，也能跑，只不过回跳次数会更多
                            }
                        }
                    }
                }
                //返回新的上下文
                let func = ptr::read_unaligned((*context).proc.as_ref());
                let mut new_context = Coroutine::init(
                    //设置协程状态为已完成，resume的时候，已经调用完了用户函数
                    (*context).id,
                    (*context).stack,
                    Status::Finished,
                    Box::new(func),
                    param,
                    None,
                );
                new_context.set_result(result);
                t = t.resume(&mut new_context as *mut Coroutine<F> as *mut c_void);
            }
        }
    }

    pub fn new(size: usize, proc: F, param: Option<*mut c_void>) -> Self {
        let stack = memory_pool::allocate(size).expect("allocate stack failed !");
        Coroutine::init(
            IdGenerator::next_id("coroutine"),
            stack,
            Status::Created,
            Box::new(proc),
            param,
            None,
        )
    }

    fn init(
        id: usize,
        stack: ManuallyDrop<Memory>,
        status: Status,
        proc: Box<F>,
        param: Option<*mut c_void>,
        next: Option<*mut c_void>,
    ) -> Self {
        let inner = Context::new(stack, Coroutine::<F>::coroutine_function);
        // Allocate a Context on the stack.
        let sp = Transfer::new(inner, ptr::null_mut());
        let mut context = Coroutine {
            id,
            stack,
            sp,
            status,
            proc,
            param,
            result: None,
            //默认轮询到了立刻执行
            exec_time: 0,
            next,
            entrance: None,
            scheduler: None,
        };
        context.sp.data = &mut context as *mut Coroutine<F> as *mut c_void;
        context
    }

    fn invoke(&mut self) -> Option<*mut c_void> {
        unsafe {
            let func = ptr::read_unaligned(self.proc.as_ref());
            let result = func(self.param);
            self.set_result(result);
            self.set_status(Status::Finished);
            result
        }
    }
}

impl<F: ?Sized> Coroutine<F> {
    pub fn resume(&self) -> Self {
        //使用构造方法传入的参数，直接切换上下文
        self.switch(&self.sp)
    }

    pub fn resume_with(&mut self, param: Option<*mut c_void>) -> Self {
        //覆盖用户参数
        self.set_param(param);
        //切换上下文
        self.switch(&self.sp)
    }

    pub fn resume_to(
        &self,
        to: &Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void>>,
    ) -> Self {
        self.switch(&to.sp)
    }

    fn switch(&self, to: &Transfer) -> Self {
        let sp = Transfer::switch(to);
        let context = sp.data as *mut Coroutine<F>;
        unsafe { ptr::read_unaligned(context) }
    }

    pub fn delay(&mut self, delay: Duration) -> Self {
        self.set_delay(delay).set_status(Status::Suspend).resume()
    }

    pub fn delay_with(&mut self, delay: Duration, param: Option<*mut c_void>) -> Self {
        self.set_delay(delay)
            .set_status(Status::Suspend)
            //覆盖用户参数
            .resume_with(param)
    }

    pub fn exit(&mut self) {
        self.set_status(Status::Exited);
        //只归还，不删除
        memory_pool::revert(self.stack);
    }

    ///下方开始get/set
    pub fn get_id(&self) -> usize {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).id }
    }

    pub fn set_param(&mut self, param: Option<*mut c_void>) -> &mut Self {
        self.param = param;
        unsafe {
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).param = param;
        }
        self
    }

    pub fn get_param(&self) -> Option<*mut c_void> {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).param }
    }

    pub fn set_result(&mut self, result: Option<*mut c_void>) -> &mut Self {
        self.result = result;
        unsafe {
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).result = result;
        }
        self
    }

    pub fn get_result(&self) -> Option<*mut c_void> {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).result }
    }

    pub fn set_status(&mut self, status: Status) -> &mut Self {
        self.status = status;
        unsafe {
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).status = status;
        }
        self
    }

    pub fn get_status(&self) -> Status {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).status }
    }

    pub fn set_delay(&mut self, delay: Duration) -> &mut Self {
        let time = timer::get_timeout_time(delay);
        self.set_execute_time(time)
    }

    pub fn get_execute_time(&self) -> u64 {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).exec_time }
    }

    pub fn set_execute_time(&mut self, time: u64) -> &mut Self {
        //覆盖执行时间
        self.exec_time = time;
        unsafe {
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).exec_time = time;
        }
        self
    }

    pub fn get_next(&self) -> Option<*mut c_void> {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).next }
    }

    pub fn set_next(
        &mut self,
        next: &mut Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void> + ?Sized>,
    ) -> &mut Self {
        let pointer = next as *mut _ as *mut c_void;
        self.set_next_ptr(pointer)
    }

    pub fn set_next_ptr(&mut self, pointer: *mut c_void) -> &mut Self {
        self.next = Some(pointer);
        unsafe {
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).next = Some(pointer);
        }
        self
    }

    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }

    pub fn get_entrance(&self) -> Option<*mut c_void> {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).entrance }
    }

    #[allow(unused)]
    pub(crate) fn set_entrance(
        &mut self,
        entrance: &Coroutine<impl FnOnce(Option<*mut c_void>) -> Option<*mut c_void> + ?Sized>,
    ) -> &mut Self {
        unsafe {
            let mut entrance = ptr::read_unaligned(entrance.sp.context.0);
            let pointer = &mut entrance as *mut c_void;
            self.entrance = Some(pointer);
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).entrance = Some(pointer);
        }
        self
    }

    #[allow(unused)]
    pub(crate) fn set_scheduler(&mut self, scheduler: &mut Scheduler) -> &mut Self {
        unsafe {
            self.scheduler = Some(scheduler as *mut Scheduler);
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).scheduler = Some(scheduler as *mut Scheduler);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::coroutine::Coroutine;
    use std::os::raw::c_void;
    use std::time::Duration;

    #[test]
    fn test() {
        println!("context test started !");
        let mut c = Coroutine::new(
            2048,
            |param| {
                match param {
                    Some(param) => {
                        print!("user_function {} => ", param as usize);
                    }
                    None => {
                        print!("user_function no param => ");
                    }
                }
                param
            },
            None,
        );
        for i in 0..10 {
            print!("Resuming {} => ", i);
            c = c.delay_with(Duration::from_millis(100), Some(i as *mut c_void));
            match c.get_result() {
                Some(result) => {
                    println!("Got {}", result as usize)
                }
                None => {
                    println!("No result")
                }
            }
        }
        c.exit();
        println!("context test finished!");
    }

    #[test]
    fn next() {
        let mut head = Coroutine::new(
            2048,
            |param| {
                println!("1");
                param
            },
            None,
        );
        let mut middle = Coroutine::new(
            2048,
            |param| {
                println!("2");
                param
            },
            None,
        );
        let mut tail = Coroutine::new(
            2048,
            |param| {
                println!("3");
                param
            },
            None,
        );
        head.set_next(&mut middle);
        middle.set_next(&mut tail);

        //设置出口为主线程
        tail.set_entrance(&head);
        head.resume();
    }
}
