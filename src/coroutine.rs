use std::os::raw::c_void;
use std::{ptr, thread};
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::time::Duration;
use memory_pool::memory::Memory;
use crate::context::{Context, Transfer};
use crate::timer;

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

#[derive(Debug)]
pub struct Coroutine<F: ?Sized> {
    //todo 增加id字段
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
    next: Option<*mut Coroutine<F>>,
    //入口协程，也是出口
    entrance: Option<*mut Coroutine<F>>,
}

impl<F> Coroutine<F>
    where F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void>+Sized
{
    extern "C" fn coroutine_function(mut t: Transfer) {
        unsafe {
            loop {
                let context = t.data as *mut Coroutine<F>;
                if timer::now() < (*context).exec_time {
                    //让出CPU的执行权
                    thread::yield_now();
                    continue;
                }
                //设置协程状态为运行中
                (*context).status = Status::Running;
                let param = (*context).param as Option<*mut c_void>;
                match param {
                    Some(data) => { print!("coroutine_function {} => ", data as usize) }
                    None => { print!("coroutine_function no param => ") }
                }
                //调用用户函数
                let result = (*context).invoke();
                //返回新的上下文
                let func = ptr::read((*context).proc.as_ref());
                let mut new_context = Coroutine::init(
                    //设置协程状态为已完成，resume的时候，已经调用完了用户函数
                    (*context).stack, Status::Finished, Box::new(func), param, None);
                new_context.set_result(result);
                //todo 不回跳，继续执行下一个ready的协程
                t = t.resume(&mut new_context as *mut Coroutine<F> as *mut c_void);
            }
        }
    }

    pub fn new(size: usize, proc: F, param: Option<*mut c_void>) -> Self {
        let stack = memory_pool::allocate(size)
            .expect("allocate stack failed !");
        Coroutine::init(stack, Status::Created, Box::new(proc), param, None)
        //todo 加到ready队列中，status再置为ready
    }

    fn init(stack: ManuallyDrop<Memory>, status: Status,
            proc: Box<F>, param: Option<*mut c_void>,
            next: Option<*mut Coroutine<F>>) -> Self {
        let inner = Context::new(stack, Coroutine::<F>::coroutine_function);
        // Allocate a Context on the stack.
        let mut sp = Transfer::new(inner, 0 as *mut c_void);
        let mut context = Coroutine {
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
        };
        context.sp.data = &mut context as *mut Coroutine<F> as *mut c_void;
        context
    }

    fn invoke(&self) -> Option<*mut c_void> {
        unsafe {
            let mut func = ptr::read(self.proc.as_ref());
            func(self.param)
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

    pub fn resume_to(&self, to: &Coroutine<F>) -> Self {
        self.switch(&to.sp)
    }

    fn switch(&self, to: &Transfer) -> Self {
        let mut sp = Transfer::switch(to);
        let context = sp.data as *mut Coroutine<F>;
        unsafe { ptr::read(context) }
    }

    pub fn delay(&mut self, delay: Duration) -> Self {
        self.set_delay(delay)
            .set_status(Status::Suspend)
            .resume()
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
    pub fn set_param(&mut self, param: Option<*mut c_void>) -> &mut Self {
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

    pub(crate) fn set_status(&mut self, status: Status) -> &mut Self {
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
        //覆盖执行时间
        unsafe {
            let context = self.sp.data as *mut Coroutine<F>;
            (*context).exec_time = timer::get_timeout_time(delay);
        }
        self
    }

    pub fn get_execute_time(&self) -> u64 {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).exec_time }
    }
}

#[cfg(test)]
mod tests {
    use std::os::raw::c_void;
    use std::time::Duration;
    use crate::coroutine::Coroutine;

    #[test]
    fn test() {
        println!("context test started !");
        let mut c = Coroutine::new(2048, |param| {
            match param {
                Some(param) => {
                    print!("user_function {} => ", param as usize);
                }
                None => {
                    print!("user_function no param => ");
                }
            }
            param
        }, None);
        for i in 0..10 {
            print!("Resuming {} => ", i);
            c = c.delay_with(Duration::from_millis(100), Some(i as *mut c_void));
            match c.get_result() {
                Some(result) => { println!("Got {}", result as usize) }
                None => { println!("No result") }
            }
        }
        c.exit();
        println!("context test finished!");
    }
}