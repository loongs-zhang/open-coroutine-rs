use std::os::raw::c_void;
use std::ptr;
use crate::context::{Context, Transfer};
use crate::stack::{ProtectedFixedSizeStack, Stack};

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
pub struct Coroutine<'a, F> {
    stack: &'a Stack,
    sp: Transfer,
    status: Status,
    //用户函数
    proc: Box<F>,
    //调用用户函数的参数
    param: Option<*mut c_void>,
    //调用用户函数的结果
    result: Option<*mut c_void>,
}

impl<'a, F> Coroutine<'a, F>
    where F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void>
{
    extern "C" fn coroutine_function(mut t: Transfer) {
        unsafe {
            loop {
                let context = t.data as *mut Coroutine<F>;
                //设置协程状态为运行中
                (*context).status = Status::Running;
                let param = (*context).param as Option<*mut c_void>;
                match param {
                    Some(data) => { print!("coroutine_function {} => ", data as usize) }
                    None => { print!("coroutine_function no param => ") }
                }
                let mut func = ptr::read((*context).proc.as_ref());
                let mut new_context = Coroutine::init(
                    //设置协程状态为已完成，resume的时候，已经调用完了用户函数
                    (*context).stack, Status::Finished, Box::new(func), param);
                //调用用户函数
                func = ptr::read((*context).proc.as_ref());
                new_context.set_result(func(param));
                //todo 不回跳，继续执行下一个ready的协程
                t = t.resume(&mut new_context as *mut Coroutine<F> as *mut c_void);
            }
        }
    }

    pub fn new(stack: &'a Stack, proc: F, param: Option<*mut c_void>) -> Self {
        Coroutine::init(stack, Status::Created, Box::new(proc), param)
        //todo 加到ready队列中，status再置为ready
    }

    fn init(stack: &'a Stack, status: Status, proc: Box<F>, param: Option<*mut c_void>) -> Self {
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
        };
        context.sp.data = &mut context as *mut Coroutine<F> as *mut c_void;
        context
    }
}

impl<'a, F> Coroutine<'a, F> {
    pub fn resume(&mut self) -> Self {
        //没有用户参数，直接切换上下文
        self.switch(&self.sp)
    }

    pub fn resume_with(&mut self, param: Option<*mut c_void>) -> Self {
        //设置用户参数
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

    pub fn get_status(&self) -> Status {
        let context = self.sp.data as *mut Coroutine<F>;
        unsafe { (*context).status }
    }

    pub fn exit(&mut self) {
        self.status = Status::Exited;
        self.stack.drop();
    }
}

#[cfg(test)]
mod tests {
    use std::os::raw::c_void;
    use crate::coroutine::Coroutine;
    use crate::stack::ProtectedFixedSizeStack;

    #[test]
    fn test() {
        println!("context test started !");
        let stack = ProtectedFixedSizeStack::new(2048)
            .expect("allocate stack failed !");
        let mut c = Coroutine::new(&stack, |param| {
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
            c = c.resume_with(Some(i as *mut c_void));
            match c.get_result() {
                Some(result) => { println!("Got {}", result as usize) }
                None => { println!("No result") }
            }
        }
        c.exit();
        println!("context test finished!");
    }
}