use std::os::raw::c_void;
use std::ptr;
use crate::context::{Context, Transfer};
use crate::stack::{ProtectedFixedSizeStack, Stack};

#[derive(Debug)]
pub struct Coroutine<'a, F> {
    stack: &'a Stack,
    sp: Transfer,
    //用户函数
    proc: Box<F>,
    //调用用户函数的参数
    param: Option<*mut c_void>,
    //调用用户函数的结果
    result: Option<*mut c_void>,
    //上下文切换栈，方便用户在N个协程中任意切换
    context_stack: Vec<&'a Coroutine<'a, F>>,
}

impl<'a, F> Coroutine<'a, F>
    where F: FnOnce(Option<*mut c_void>) -> Option<*mut c_void>
{
    extern "C" fn coroutine_function(mut t: Transfer) {
        unsafe {
            loop {
                let context = t.data as *mut Coroutine<F>;
                let param = (*context).param as Option<*mut c_void>;
                match param {
                    Some(data) => { print!("coroutine_function {} => ", data as usize) }
                    None => { print!("coroutine_function no param => ") }
                }
                // copy stack
                let mut context_stack: Vec<&Coroutine<F>> = Vec::new();
                unsafe {
                    for data in (*context).context_stack.iter() {
                        context_stack.push(data);
                    }
                }
                context_stack.push(&*context);
                let mut func = ptr::read((*context).proc.as_ref());
                let mut new_context = Coroutine::init((*context).stack, Box::new(func), context_stack);
                //调用用户函数
                func = ptr::read((*context).proc.as_ref());
                new_context.set_param(param)
                    .set_result(func(param))
                    //调用完用户函数后，需要清理context_stack
                    .context_stack.pop();
                t = t.resume(&mut new_context as *mut Coroutine<F> as *mut c_void);
            }
        }
    }

    pub fn new(stack: &'a Stack, proc: F) -> Self {
        Coroutine::init(stack, Box::new(proc), Vec::new())
    }

    fn init(stack: &'a Stack, proc: Box<F>, context_stack: Vec<&'a Coroutine<'a, F>>) -> Self {
        let inner = Context::new(stack, Coroutine::<F>::coroutine_function);
        // Allocate a Context on the stack.
        let mut sp = Transfer::new(inner, 0 as *mut c_void);
        let mut context = Coroutine {
            stack,
            sp,
            proc,
            param: None,
            result: None,
            context_stack,
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
        let mut context_stack: Vec<&Coroutine<F>> = Vec::new();
        unsafe {
            for data in (*context).context_stack.iter() {
                context_stack.push(data);
            }
            Coroutine {
                stack: self.stack,
                sp,
                proc: Box::new(unsafe { ptr::read(self.proc.as_ref()) }),
                param: (*context).param,
                result: (*context).result,
                context_stack,
            }
        }
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
        });
        for i in 0..10 {
            print!("Resuming {} => ", i);
            c = c.resume_with(Some(i as *mut c_void));
            match c.get_result() {
                Some(result) => { println!("Got {}", result as usize) }
                None => { println!("No result") }
            }
        }
        println!("context test finished!");
    }
}