use std::any::Any;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use crate::api::{Coroutine, MainCoroutine, State};
use crate::register::Register;
use crate::{register, Stack};

struct MainCoroutineImpl {
    /// 寄存器
    register: Register,
}

impl MainCoroutineImpl {
    fn new() -> Self {
        MainCoroutineImpl { register: Register::new() }
    }
}

impl MainCoroutine for MainCoroutineImpl {
    fn get_register(&self) -> &Register {
        &self.register
    }

    fn resume(&self, coroutine: Box<dyn Coroutine>) {
        if coroutine.get_state() == State::Exited {
            // 不能resume到已退出的协程
            return;
        }
        let mut from = *self.register;
        let to = coroutine.get_register();
        unsafe { register::swap(&mut from, to) }
    }

    fn destroy(&self, coroutine: Box<dyn Coroutine>) {
        todo!()
    }

    fn exit(&self) {}
}

struct CoroutineImpl {
    /// 主协程
    main: Box<dyn MainCoroutine>,
    /// 寄存器
    register: Register,
    /// 独有栈
    stack: Box<dyn Stack>,
    /// 指向参数的指针
    param: Option<usize>,
    /// 协程状态
    state: Rc<RefCell<State>>,
    /// 函数指针
    function: Box<dyn FnOnce(dyn Any) -> ()>,
}

impl CoroutineImpl {
    fn new(main: Box<dyn MainCoroutine>,
           stack: Box<dyn Stack>,
           function: Box<dyn FnOnce(dyn Any) -> ()>,
           param_pointer: Option<usize>) -> Self {
        CoroutineImpl {
            main,
            register: Register::new(),
            stack,
            param: param_pointer,
            state: Rc::new(RefCell::new(State::Created)),
            function,
        }
    }
}

impl Coroutine for CoroutineImpl {
    fn get_register(&self) -> &Register {
        &self.register
    }

    fn yields(&self) {
        let mut from = *self.register;
        let to = self.main.get_register();
        unsafe { register::swap(&mut from, to) }
    }

    fn exit(&self) {
        *self.state.borrow_mut() = State::Exited;
        self.yields();
    }

    fn get_state(&self) -> State {
        return *self.state.borrow();
    }

    fn get_main_coroutine(&self) -> &dyn MainCoroutine {
        &*self.main
    }

    fn set_param(&mut self, param_pointer: usize) {
        self.param = Some(param_pointer);
    }

    fn get_param(&self) -> Option<usize> {
        self.param
    }
}